//! Task Executor — claims, executes, and completes supervised runs

use super::store::RunStore;
use super::types::{RunExecutionError, RunKind, RunRecord};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{interval, Duration, Instant};
use tracing::{debug, error, info, warn};

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
            tokio::select! {
                _ = cancel.cancelled() => {
                    info!("executor shutting down");
                    break;
                }
                result = self.tick() => {
                    if let Err(e) = result {
                        warn!(error = %e, "executor tick failed");
                    }
                    // Brief pause between ticks to avoid tight-loop CPU burn
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }
    }

    /// Single claim → execute → complete/fail cycle.
    ///
    /// Public so integration tests can drive a single tick.
    pub async fn tick(&self) -> Result<(), String> {
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
                let mut ticker = interval(Duration::from_secs(10));
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

        // Look up handler by kind
        let handler = self.handlers.get(&kind);
        let result = if let Some(handler) = handler {
            // Execute the handler
            let start = Instant::now();
            let result = handler.handle(record).await;
            let duration = start.elapsed();
            info!(run_id = %run_id, duration_ms = %duration.as_millis(), "handler executed");
            result
        } else {
            warn!(run_id = %run_id, kind = %kind.as_str(), "no handler registered for run kind");
            Err(RunExecutionError::HandlerError(format!(
                "no handler registered for run kind: {}",
                kind.as_str()
            )))
        };

        // Cancel heartbeat before completing
        hb_cancel.cancel();
        let _ = hb_task.await;

        match result {
            Ok(summary) => {
                info!(run_id = %run_id, "run completed");
                if let Err(e) = self
                    .store
                    .complete_supervised(&run_id, &self.worker_id, Some(summary))
                    .await
                {
                    error!(run_id = %run_id, error = %e, "complete failed");
                }
            }
            Err(e) => {
                warn!(run_id = %run_id, error = %e, "run failed");
                if let Err(e) = self
                    .store
                    .fail_supervised(&run_id, &self.worker_id, Some(e.to_string()))
                    .await
                {
                    error!(run_id = %run_id, error = %e, "fail failed");
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn setup() -> (TempDir, RunStore) {
        let dir = TempDir::new().expect("tempdir");
        let store = RunStore::new(dir.path()).await.expect("store creation");
        (dir, store)
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
}
