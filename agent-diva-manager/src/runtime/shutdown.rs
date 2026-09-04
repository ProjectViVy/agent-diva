use super::*;
use std::time::Duration;

const GRACEFUL_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
// ChannelRuntime waits for each supervisor's bounded six-second shutdown
// budget before reporting a deterministic cleanup-pending state. Keep the
// owning boundary longer than that internal budget so it is not cancelled
// while it still owns the live RuntimeEntry set.
const CHANNEL_RUNTIME_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(8);
const ABORT_JOIN_TIMEOUT: Duration = Duration::from_secs(1);

// ---------------------------------------------------------------------------
//  Story 3.2 migration: Future shutdown will use Bootstrap::stop_all()
//  instead of manually aborting each task handle. The Bootstrap struct
//  (agent-diva-tooling) stops modules in reverse dependency order and
//  handles stop-failure gracefully.
//
//  Example (future):
//    let mut registry = ModuleRegistry::new();
//    registry.register(Arc::new(cron_module));
//    ...
//    let bootstrap = Bootstrap::new(registry);
//    bootstrap.stop_all().await;
// ---------------------------------------------------------------------------

pub(super) async fn wait_for_shutdown(tasks: &mut GatewayTasks) -> bool {
    let mut manager_handle_completed = false;
    tokio::select! {
        _ = tokio::signal::ctrl_c() => {}
        res = &mut tasks.manager_handle => {
            manager_handle_completed = true;
            match res {
                Ok(Err(e)) => tracing::error!("Manager loop error: {}", e),
                Err(e) => tracing::error!("Manager loop panicked or cancelled: {}", e),
                _ => {}
            }
        }
    }
    manager_handle_completed
}

pub(super) async fn shutdown_runtime(tasks: GatewayTasks, manager_handle_completed: bool) {
    let _ = tasks.server_shutdown_tx.send(());
    join_with_timeout(
        "HTTP server",
        tasks.server_handle,
        GRACEFUL_SHUTDOWN_TIMEOUT,
    )
    .await;

    if !manager_handle_completed {
        tasks.manager_handle.abort();
    }
    join_with_timeout("manager", tasks.manager_handle, ABORT_JOIN_TIMEOUT).await;

    tasks.fabric_ingress_handle.abort();
    join_with_timeout(
        "Fabric ingress",
        tasks.fabric_ingress_handle,
        ABORT_JOIN_TIMEOUT,
    )
    .await;

    tasks.outbound_dispatch_handle.abort();
    join_with_timeout(
        "outbound dispatcher",
        tasks.outbound_dispatch_handle,
        ABORT_JOIN_TIMEOUT,
    )
    .await;

    tasks.agent_handle.abort();
    join_with_timeout("agent runtime", tasks.agent_handle, ABORT_JOIN_TIMEOUT).await;

    tasks.supervised_executor_cancel.cancel();
    join_with_timeout(
        "supervised executor",
        tasks.supervised_executor_handle,
        GRACEFUL_SHUTDOWN_TIMEOUT,
    )
    .await;

    if tokio::time::timeout(
        CHANNEL_RUNTIME_SHUTDOWN_TIMEOUT,
        tasks.channel_runtime.shutdown(),
    )
    .await
    .is_err()
    {
        tracing::warn!("Native channel runtime did not stop within the shutdown timeout");
    }

    if tokio::time::timeout(GRACEFUL_SHUTDOWN_TIMEOUT, tasks.cron_service.stop())
        .await
        .is_err()
    {
        tracing::warn!("Cron service did not stop within the shutdown timeout");
    }
}

async fn join_with_timeout<T>(label: &str, mut handle: JoinHandle<T>, timeout: Duration) {
    if tokio::time::timeout(timeout, &mut handle).await.is_ok() {
        return;
    }

    tracing::warn!("{label} did not stop within the shutdown timeout; aborting task");
    handle.abort();
    if tokio::time::timeout(ABORT_JOIN_TIMEOUT, &mut handle)
        .await
        .is_err()
    {
        tracing::error!("{label} did not exit after abort");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn join_with_timeout_aborts_a_stuck_task() {
        let handle = tokio::spawn(async {
            std::future::pending::<()>().await;
        });

        let started = std::time::Instant::now();
        join_with_timeout("test task", handle, Duration::from_millis(20)).await;

        assert!(started.elapsed() < Duration::from_secs(1));
    }
}
