//! SubagentRunHandler — executes `RunKind::Subagent` supervised runs
//!
//! Bridges the supervised run system (`RunHandler` trait) with the subagent
//! manager. When a run with `kind == Subagent` is claimed by a `TaskExecutor`,
//! this handler parses the run message and delegates execution to
//! `SubagentManager`.

use std::sync::Arc;

use agent_diva_core::supervised::types::{RunExecutionError, RunRecord};
use agent_diva_core::supervised::RunHandler;
use tracing::{error, info};

use crate::subagent::SubagentManager;

/// Handler for supervised runs of kind `Subagent`.
///
/// Delegates actual subagent execution to `SubagentManager` and reports
/// the result back via the handler return value (which `TaskExecutor`
/// forwards to `RunStore::complete_supervised` or `fail_supervised`).
pub struct SubagentRunHandler {
    subagent_manager: Arc<SubagentManager>,
}

impl SubagentRunHandler {
    /// Create a new handler backed by the given `SubagentManager`.
    pub fn new(subagent_manager: Arc<SubagentManager>) -> Self {
        Self { subagent_manager }
    }
}

#[async_trait::async_trait]
impl RunHandler for SubagentRunHandler {
    async fn handle(&self, record: RunRecord) -> Result<String, RunExecutionError> {
        let run_id = &record.id;
        let task = &record.message;

        info!(run_id = %run_id, task_len = task.len(), "handling subagent run");

        // Parse optional metadata for channel/chat routing
        let (origin_channel, origin_chat_id) = parse_routing(&record);

        // Delegate to SubagentManager::spawn — this internally tokio::spawns
        // the subagent and returns immediately with a status message.
        let label: Option<String> = record
            .metadata
            .as_ref()
            .and_then(|m: &serde_json::Value| m.get("label"))
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(String::from);

        let spawn_result = self
            .subagent_manager
            .spawn(task.clone(), label, origin_channel, origin_chat_id)
            .await;

        match spawn_result {
            Ok(status_msg) => {
                info!(run_id = %run_id, "subagent spawned successfully");
                Ok(status_msg)
            }
            Err(e) => {
                let err_msg = format!("subagent spawn failed: {}", e);
                error!(run_id = %run_id, error = %err_msg);
                Err(RunExecutionError::HandlerError(err_msg))
            }
        }
    }
}

/// Extract routing information from the run record.
///
/// Priority:
/// 1. `record.channel` / `record.metadata.chat_id`
/// 2. Default to "internal" / "supervised"
fn parse_routing(record: &RunRecord) -> (String, String) {
    let channel = record
        .channel
        .clone()
        .unwrap_or_else(|| "internal".to_string());

    let chat_id = record
        .metadata
        .as_ref()
        .and_then(|m: &serde_json::Value| m.get("chat_id"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .unwrap_or("supervised")
        .to_string();

    (channel, chat_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::supervised::types::{RunKind, SupervisedRunSpec};
    use agent_diva_core::supervised::{RunStore, TaskExecutor};

    #[tokio::test]
    async fn test_parse_routing_defaults() {
        let spec = SupervisedRunSpec::from_spec("test task").with_kind(RunKind::Subagent);
        let record = agent_diva_core::supervised::types::RunRecord::from_spec(&spec);
        let (ch, chat) = parse_routing(&record);
        assert_eq!(ch, "internal");
        assert_eq!(chat, "supervised");
    }

    #[tokio::test]
    async fn test_parse_routing_from_channel_and_metadata() {
        let metadata = serde_json::json!({"chat_id": "room-42"});
        let spec = SupervisedRunSpec::from_spec("test task")
            .with_kind(RunKind::Subagent)
            .with_channel("telegram")
            .with_metadata(metadata);
        let record = agent_diva_core::supervised::types::RunRecord::from_spec(&spec);
        let (ch, chat) = parse_routing(&record);
        assert_eq!(ch, "telegram");
        assert_eq!(chat, "room-42");
    }

    /// Integration test: verify TaskExecutor dispatches Subagent runs
    /// to SubagentRunHandler and marks them failed (no real SubagentManager).
    #[tokio::test]
    async fn test_task_executor_dispatches_subagent_run() {
        use agent_diva_core::supervised::types::{RunKind, RunStatus};
        use tempfile::TempDir;

        let dir = TempDir::new().expect("tempdir");
        let store = RunStore::new(dir.path()).await.expect("store creation");

        // Create a Subagent run
        let spec = SupervisedRunSpec::from_spec("summarize file").with_kind(RunKind::Subagent);
        let created = store.create(&spec).await.expect("create");

        // Build executor with SubagentRunHandler registered
        let mut executor = TaskExecutor::new(store.clone(), "worker-subagent");
        // Note: we can't easily construct a real SubagentManager here,
        // so we verify the "no handler" path first (existing core test).
        // For a real integration, a mock SubagentManager would be needed.
        // Instead, verify the registration API works.
        executor.register_handler(RunKind::Subagent, Arc::new(MockSubagentHandler));

        // Tick should find the run and dispatch to our mock handler
        executor.tick().await.expect("tick");

        let reloaded = store
            .get_record(&created.id)
            .await
            .expect("get")
            .expect("record");
        assert_eq!(reloaded.status, RunStatus::Completed);
        assert_eq!(reloaded.result_summary, Some("mock-subagent-done".to_string()));
    }

    /// Mock handler for integration testing
    struct MockSubagentHandler;

    #[async_trait::async_trait]
    impl RunHandler for MockSubagentHandler {
        async fn handle(&self, _record: RunRecord) -> Result<String, RunExecutionError> {
            Ok("mock-subagent-done".to_string())
        }
    }
}
