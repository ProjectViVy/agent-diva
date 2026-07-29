//! Task Executor — claims, executes, and completes supervised runs

use super::store::RunStore;
use super::types::{RunExecutionError, RunKind, RunRecord, RunStatus};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tokio::time::{interval, interval_at, Duration, Instant};
use tracing::{debug, error, info, warn};

const HEARTBEAT_INTERVAL_SECS: u64 = 10;
const STATUS_POLL_INTERVAL_MS: u64 = 100;

/// Trait for run execution handlers.
///
/// Implementors receive a `RunRecord` and must return either a success
/// string or a `RunExecutionError`.
#[async_trait::async_trait]
pub trait RunHandler: Send + Sync {
    /// Execute the run. Called by `TaskExecutor` after claiming a run.
    async fn handle(&self, record: RunRecord) -> Result<String, RunExecutionError>;
}

/// A test handler that sleeps for a configured duration then returns success.
///
/// Useful for integration tests and load simulations.
pub struct SleepHandler {
    sleep_ms: u64,
    result: String,
}

impl SleepHandler {
    /// Create a new sleep handler.
    ///
    /// * `sleep_ms` — milliseconds to sleep
    /// * `result` — string to return on success
    pub fn new(sleep_ms: u64, result: impl Into<String>) -> Self {
        Self {
            sleep_ms,
            result: result.into(),
        }
    }
}

#[async_trait::async_trait]
impl RunHandler for SleepHandler {
    async fn handle(&self, _record: RunRecord) -> Result<String, RunExecutionError> {
        tokio::time::sleep(Duration::from_millis(self.sleep_ms)).await;
        Ok(self.result.clone())
    }
}

/// Worker that claims runs from the store, dispatches them to the
/// appropriate handler by `RunKind`, and reports completion or failure.
///
/// Includes an internal heartbeat loop that ticks every 10 seconds
/// while a run is owned.
pub struct TaskExecutor {
    store: RunStore,
    handlers: HashMap<RunKind, Arc<dyn RunHandler>>,
    worker_id: String,
}

enum ExecutionOutcome {
    Finished(Result<String, RunExecutionError>),
    Cancelled,
    Lost,
    TimedOut,
    ExecutorShutdown,
}

impl TaskExecutor {
    /// Create a new executor with no handlers registered.
    ///
    /// * `store` — the `RunStore` to claim from
    /// * `worker_id` — unique identifier for this worker
    pub fn new(store: RunStore, worker_id: impl Into<String>) -> Self {
        Self {
            store,
            handlers: HashMap::new(),
            worker_id: worker_id.into(),
        }
    }

    /// Register a handler for a specific `RunKind`.
    pub fn register_handler(&mut self, kind: RunKind, handler: Arc<dyn RunHandler>) {
        self.handlers.insert(kind, handler);
    }

    /// Run the executor loop until cancelled.
    ///
    /// The loop:
    /// 1. Claims the next queued run
    /// 2. Spawns a heartbeat task (10s interval)
    /// 3. Looks up handler by RunKind and calls it
    /// 4. Marks complete or failed
    /// 5. Cancels the heartbeat task
    pub async fn run(&self, cancel: tokio_util::sync::CancellationToken) {
        info!(worker_id = %self.worker_id, "executor started");

        loop {
            if cancel.is_cancelled() {
                info!("executor shutting down");
                break;
            }

            if let Err(e) = self.tick_inner(Some(cancel.clone())).await {
                warn!(error = %e, "executor tick failed");
            }

            // Brief pause between ticks to avoid tight-loop CPU burn
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Single claim → execute → complete/fail cycle.
    ///
    /// Public so integration tests can drive a single tick.
    pub async fn tick(&self) -> Result<(), String> {
        self.tick_inner(None).await
    }

    async fn tick_inner(
        &self,
        executor_cancel: Option<tokio_util::sync::CancellationToken>,
    ) -> Result<(), String> {
        let record = match self.store.claim_next_supervised(&self.worker_id).await {
            Ok(Some(r)) => r,
            Ok(None) => {
                debug!("no queued runs");
                return Ok(());
            }
            Err(e) => {
                return Err(format!("claim failed: {}", e));
            }
        };

        let run_id = record.id.clone();
        let kind = record.kind;
        info!(run_id = %run_id, kind = %kind.as_str(), "claimed run");

        // Spawn heartbeat loop
        let hb_cancel = tokio_util::sync::CancellationToken::new();
        let hb_task = {
            let store = self.store.clone();
            let worker_id = self.worker_id.clone();
            let run_id = run_id.clone();
            let hb_cancel = hb_cancel.clone();
            tokio::spawn(async move {
                let heartbeat_interval = Duration::from_secs(HEARTBEAT_INTERVAL_SECS);
                let mut ticker =
                    interval_at(Instant::now() + heartbeat_interval, heartbeat_interval);
                loop {
                    tokio::select! {
                        _ = ticker.tick() => {
                            if let Err(e) = store.heartbeat_supervised(&run_id, &worker_id).await {
                                warn!(run_id = %run_id, error = %e, "heartbeat failed");
                                break;
                            }
                            debug!(run_id = %run_id, "heartbeat sent");
                        }
                        _ = hb_cancel.cancelled() => break,
                    }
                }
            })
        };

        let outcome = if let Some(handler) = self.handlers.get(&kind).cloned() {
            let timeout = timeout_duration(record.timeout_secs);
            let run_task = tokio::spawn(async move {
                let start = Instant::now();
                let result = handler.handle(record).await;
                let duration = start.elapsed();
                (result, duration)
            });

            self.await_execution_outcome(&run_id, run_task, timeout, executor_cancel.as_ref())
                .await
        } else {
            warn!(run_id = %run_id, kind = %kind.as_str(), "no handler registered for run kind");
            ExecutionOutcome::Finished(Err(RunExecutionError::HandlerError(format!(
                "no handler registered for run kind: {}",
                kind.as_str()
            ))))
        };

        // Cancel heartbeat before completing
        hb_cancel.cancel();
        let _ = hb_task.await;

        match outcome {
            ExecutionOutcome::Finished(Ok(summary)) => {
                info!(run_id = %run_id, "run completed");
                self.complete_if_running(&run_id, Some(summary)).await?;
            }
            ExecutionOutcome::Finished(Err(RunExecutionError::Cancelled))
            | ExecutionOutcome::Cancelled => {
                warn!(run_id = %run_id, "run cancelled during execution");
                self.cancel_if_running(&run_id, "run was cancelled").await?;
            }
            ExecutionOutcome::Lost => {
                warn!(run_id = %run_id, "run marked lost during execution");
            }
            ExecutionOutcome::TimedOut => {
                warn!(run_id = %run_id, "run timed out during execution");
                self.fail_if_running(&run_id, RunExecutionError::Timeout)
                    .await?;
            }
            ExecutionOutcome::ExecutorShutdown => {
                warn!(run_id = %run_id, "executor shutdown interrupted run");
                self.cancel_if_running(&run_id, "executor shutdown").await?;
            }
            ExecutionOutcome::Finished(Err(e)) => {
                warn!(run_id = %run_id, error = %e, "run failed");
                self.fail_if_running(&run_id, e).await?;
            }
        }

        Ok(())
    }

    async fn await_execution_outcome(
        &self,
        run_id: &str,
        mut run_task: JoinHandle<(Result<String, RunExecutionError>, Duration)>,
        timeout: Option<Duration>,
        executor_cancel: Option<&tokio_util::sync::CancellationToken>,
    ) -> ExecutionOutcome {
        let mut status_ticker = interval(Duration::from_millis(STATUS_POLL_INTERVAL_MS));
        let timeout_sleep = async {
            match timeout {
                Some(duration) => {
                    tokio::time::sleep(duration).await;
                    Some(())
                }
                None => std::future::pending::<Option<()>>().await,
            }
        };
        tokio::pin!(timeout_sleep);

        loop {
            tokio::select! {
                join = &mut run_task => {
                    return match join {
                        Ok((result, duration)) => {
                            info!(run_id = %run_id, duration_ms = %duration.as_millis(), "handler executed");
                            ExecutionOutcome::Finished(result)
                        }
                        Err(join_err) if join_err.is_cancelled() => {
                            ExecutionOutcome::Cancelled
                        }
                        Err(join_err) => {
                            ExecutionOutcome::Finished(Err(RunExecutionError::HandlerError(format!(
                                "handler task join error: {join_err}"
                            ))))
                        }
                    };
                }
                _ = &mut timeout_sleep => {
                    run_task.abort();
                    let _ = run_task.await;
                    return ExecutionOutcome::TimedOut;
                }
                _ = status_ticker.tick() => {
                    match self.store.get_record(run_id).await {
                        Ok(Some(record)) => {
                            match record.status {
                                RunStatus::Cancelled => {
                                    run_task.abort();
                                    let _ = run_task.await;
                                    return ExecutionOutcome::Cancelled;
                                }
                                RunStatus::Lost => {
                                    run_task.abort();
                                    let _ = run_task.await;
                                    return ExecutionOutcome::Lost;
                                }
                                status if status != RunStatus::Running => {
                                    run_task.abort();
                                    let _ = run_task.await;
                                    return ExecutionOutcome::Cancelled;
                                }
                                _ => {}
                            }
                        }
                        Ok(None) => {
                            run_task.abort();
                            let _ = run_task.await;
                            return ExecutionOutcome::Cancelled;
                        }
                        Err(err) => {
                            warn!(run_id = %run_id, error = %err, "failed to refresh run status");
                        }
                    }
                }
                _ = async {
                    if let Some(cancel) = executor_cancel {
                        cancel.cancelled().await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {
                    run_task.abort();
                    let _ = run_task.await;
                    return ExecutionOutcome::ExecutorShutdown;
                }
            }
        }
    }

    async fn complete_if_running(
        &self,
        run_id: &str,
        summary: Option<String>,
    ) -> Result<(), String> {
        match self.store.get_record(run_id).await {
            Ok(Some(record))
                if record.status == RunStatus::Running
                    && record.claimed_by.as_deref() == Some(self.worker_id.as_str()) =>
            {
                if let Err(e) = self
                    .store
                    .complete_supervised(run_id, &self.worker_id, summary)
                    .await
                {
                    error!(run_id = %run_id, error = %e, "complete failed");
                    return Err(format!("complete failed: {e}"));
                }
            }
            Ok(Some(record)) => {
                info!(run_id = %run_id, status = %record.status.as_str(), "skipping completion for terminal run");
            }
            Ok(None) => {
                warn!(run_id = %run_id, "run disappeared before completion");
            }
            Err(e) => {
                error!(run_id = %run_id, error = %e, "failed to load run before completion");
                return Err(format!("failed to load run before completion: {e}"));
            }
        }
        Ok(())
    }

    async fn fail_if_running(&self, run_id: &str, error: RunExecutionError) -> Result<(), String> {
        match self.store.get_record(run_id).await {
            Ok(Some(record))
                if record.status == RunStatus::Running
                    && record.claimed_by.as_deref() == Some(self.worker_id.as_str()) =>
            {
                if let Err(e) = self
                    .store
                    .fail_supervised(run_id, &self.worker_id, Some(error.to_string()))
                    .await
                {
                    error!(run_id = %run_id, error = %e, "fail failed");
                    return Err(format!("fail failed: {e}"));
                }
            }
            Ok(Some(record)) => {
                info!(run_id = %run_id, status = %record.status.as_str(), "skipping failure for terminal run");
            }
            Ok(None) => {
                warn!(run_id = %run_id, "run disappeared before failure handling");
            }
            Err(e) => {
                error!(run_id = %run_id, error = %e, "failed to load run before failure handling");
                return Err(format!("failed to load run before failure handling: {e}"));
            }
        }
        Ok(())
    }

    async fn cancel_if_running(&self, run_id: &str, reason: &str) -> Result<(), String> {
        match self.store.get_record(run_id).await {
            Ok(Some(record))
                if record.status == RunStatus::Running
                    && record.claimed_by.as_deref() == Some(self.worker_id.as_str()) =>
            {
                if let Err(e) = self
                    .store
                    .cancel_supervised(run_id, Some(reason.to_string()))
                    .await
                {
                    error!(run_id = %run_id, error = %e, "cancel failed");
                    return Err(format!("cancel failed: {e}"));
                }
            }
            Ok(Some(record)) => {
                info!(run_id = %run_id, status = %record.status.as_str(), "skipping cancel for terminal run");
            }
            Ok(None) => {
                warn!(run_id = %run_id, "run disappeared before cancellation handling");
            }
            Err(e) => {
                error!(run_id = %run_id, error = %e, "failed to load run before cancellation handling");
                return Err(format!(
                    "failed to load run before cancellation handling: {e}"
                ));
            }
        }
        Ok(())
    }
}

fn timeout_duration(timeout_secs: Option<i64>) -> Option<Duration> {
    let secs = u64::try_from(timeout_secs?).ok()?;
    Some(Duration::from_secs(secs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, RunStore) {
        let dir = TempDir::new().expect("tempdir");
        let store = RunStore::new(dir.path()).await.expect("store creation");
        (dir, store)
    }

    async fn wait_for_status(store: &RunStore, run_id: &str, expected: RunStatus) {
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                let status = store
                    .get_record(run_id)
                    .await
                    .expect("get record while waiting")
                    .expect("record while waiting")
                    .status;
                if status == expected {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap_or_else(|_| panic!("run {run_id} did not reach status {}", expected.as_str()));
    }

    #[tokio::test]
    async fn test_sleep_handler_success() {
        let handler = SleepHandler::new(10, "done");
        use super::super::types::SupervisedRunSpec;
        let record = RunRecord::from_spec(&SupervisedRunSpec::from_spec("test"));

        let result = handler.handle(record).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "done");
    }

    #[tokio::test]
    async fn test_executor_claims_and_completes() {
        let (_dir, store) = setup().await;
        use super::super::types::{RunKind, RunStatus, SupervisedRunSpec};

        let spec = SupervisedRunSpec::from_spec("exec test").with_kind(RunKind::Generic);
        let created = store.create(&spec).await.expect("create");

        let mut executor = TaskExecutor::new(store.clone(), "worker-test");
        executor.register_handler(
            RunKind::Generic,
            Arc::new(SleepHandler::new(10, "completed")),
        );

        // Run a single tick
        executor.tick().await.expect("tick");

        let reloaded = store
            .get_record(&created.id)
            .await
            .expect("get")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Completed);
        assert_eq!(reloaded.result_summary, Some("completed".to_string()));
        assert_eq!(reloaded.claimed_by, Some("worker-test".to_string()));
    }

    #[tokio::test]
    async fn test_executor_no_work_when_empty() {
        let (_dir, store) = setup().await;

        let mut executor = TaskExecutor::new(store.clone(), "worker-test");
        executor.register_handler(
            RunKind::Generic,
            Arc::new(SleepHandler::new(10, "completed")),
        );

        // Should return Ok(()) immediately when no queued runs
        let result = executor.tick().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_executor_fails_when_no_handler() {
        let (_dir, store) = setup().await;
        use super::super::types::{RunKind, RunStatus, SupervisedRunSpec};

        // Create a run with kind Subagent but no handler registered
        let spec = SupervisedRunSpec::from_spec("subagent test").with_kind(RunKind::Subagent);
        let created = store.create(&spec).await.expect("create");

        let executor = TaskExecutor::new(store.clone(), "worker-test");
        // No handlers registered

        // Run a single tick
        executor.tick().await.expect("tick");

        let reloaded = store
            .get_record(&created.id)
            .await
            .expect("get")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Failed);
        assert!(reloaded
            .error_message
            .as_ref()
            .unwrap()
            .contains("no handler registered"));
    }

    #[tokio::test]
    async fn test_executor_times_out_run() {
        let (_dir, store) = setup().await;
        use super::super::types::{RunKind, RunStatus, SupervisedRunSpec};

        let spec = SupervisedRunSpec::from_spec("timeout test")
            .with_kind(RunKind::Generic)
            .with_timeout_secs(0);
        let created = store.create(&spec).await.expect("create");

        let mut executor = TaskExecutor::new(store.clone(), "worker-test");
        executor.register_handler(
            RunKind::Generic,
            Arc::new(SleepHandler::new(200, "completed")),
        );

        executor.tick().await.expect("tick");

        let reloaded = store
            .get_record(&created.id)
            .await
            .expect("get")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Failed);
        assert_eq!(reloaded.error_message, Some("run timed out".to_string()));
    }

    #[tokio::test]
    async fn test_executor_stops_after_external_cancel() {
        let (_dir, store) = setup().await;
        use super::super::types::{RunKind, RunStatus, SupervisedRunSpec};

        let spec = SupervisedRunSpec::from_spec("cancel test").with_kind(RunKind::Generic);
        let created = store.create(&spec).await.expect("create");

        let mut executor = TaskExecutor::new(store.clone(), "worker-test");
        executor.register_handler(
            RunKind::Generic,
            Arc::new(SleepHandler::new(5_000, "completed")),
        );

        let tick = tokio::spawn({
            let executor = executor;
            async move { executor.tick().await }
        });

        wait_for_status(&store, &created.id, RunStatus::Running).await;
        store
            .cancel_supervised(&created.id, Some("user request".to_string()))
            .await
            .expect("cancel");

        tokio::time::timeout(Duration::from_secs(2), tick)
            .await
            .expect("tick should finish")
            .expect("join")
            .expect("tick");

        let reloaded = store
            .get_record(&created.id)
            .await
            .expect("get")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Cancelled);
        assert_eq!(reloaded.error_message, Some("user request".to_string()));
    }

    #[tokio::test]
    async fn test_executor_stops_after_run_marked_lost() {
        let (_dir, store) = setup().await;
        use super::super::types::{RunKind, RunStatus, SupervisedRunSpec};

        let spec = SupervisedRunSpec::from_spec("lost test").with_kind(RunKind::Generic);
        let created = store.create(&spec).await.expect("create");

        let mut executor = TaskExecutor::new(store.clone(), "worker-test");
        executor.register_handler(
            RunKind::Generic,
            Arc::new(SleepHandler::new(5_000, "completed")),
        );

        let tick = tokio::spawn({
            let executor = executor;
            async move { executor.tick().await }
        });

        wait_for_status(&store, &created.id, RunStatus::Running).await;
        let lost = store
            .mark_lost(Utc::now() + chrono::Duration::seconds(1))
            .await
            .expect("mark_lost");
        assert_eq!(lost.len(), 1);

        tokio::time::timeout(Duration::from_secs(2), tick)
            .await
            .expect("tick should finish")
            .expect("join")
            .expect("tick");

        let reloaded = store
            .get_record(&created.id)
            .await
            .expect("get")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Lost);
    }
}
