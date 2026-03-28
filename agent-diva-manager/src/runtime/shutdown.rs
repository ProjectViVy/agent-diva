use super::*;

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
    tasks.bus.stop().await;

    let _ = tasks.server_shutdown_tx.send(());
    let _ = tasks.server_handle.await;

    if !manager_handle_completed {
        tasks.manager_handle.abort();
        let _ = tasks.manager_handle.await;
    }

    tasks.inbound_bridge_handle.abort();
    let _ = tasks.inbound_bridge_handle.await;

    if let Some(handle) = tasks.neuro_link_bridge_handle {
        handle.abort();
        let _ = handle.await;
    }

    tasks.outbound_dispatch_handle.abort();
    let _ = tasks.outbound_dispatch_handle.await;

    tasks.agent_handle.abort();
    let _ = tasks.agent_handle.await;

    tasks.channel_handle.abort();
    let _ = tasks.channel_handle.await;

    if let Err(e) = tasks.channel_manager.stop_all().await {
        tracing::error!("Failed to stop channels: {}", e);
    }
    tasks.cron_service.stop().await;
}
