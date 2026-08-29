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

use crate::subagent::{SubagentManager, SupervisedSubagentContext};

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
        let (origin_channel, origin_chat_id) = parse_routing(&record)?;
        let context = parse_context(&record);

        // Delegate to the supervised execution path, which waits for the real
        // subagent work to finish before returning to TaskExecutor.
        let label: Option<String> = record
            .metadata
            .as_ref()
            .and_then(|m: &serde_json::Value| m.get("label"))
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(String::from);

        let run_result = self
            .subagent_manager
            .run_supervised(task.clone(), label, origin_channel, origin_chat_id, context)
            .await;

        match run_result {
            Ok(summary) => {
                info!(run_id = %run_id, "subagent completed successfully");
                Ok(summary)
            }
            Err(e) => {
                let err_msg = format!("subagent run failed: {}", e);
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
/// 2. Missing routing is treated as a malformed supervised run.
fn parse_routing(record: &RunRecord) -> Result<(String, String), RunExecutionError> {
    let Some(channel) = record.channel.clone() else {
        return Err(RunExecutionError::HandlerError(
            "subagent run missing channel metadata".to_string(),
        ));
    };

    let Some(chat_id) = record
        .metadata
        .as_ref()
        .and_then(|m: &serde_json::Value| m.get("chat_id"))
        .and_then(|v: &serde_json::Value| v.as_str())
        .map(str::to_string)
    else {
        return Err(RunExecutionError::HandlerError(
            "subagent run missing chat_id metadata".to_string(),
        ));
    };

    Ok((channel, chat_id))
}

fn parse_context(record: &RunRecord) -> SupervisedSubagentContext {
    let metadata = record.metadata.as_ref();
    SupervisedSubagentContext {
        session_key: metadata
            .and_then(|m| m.get("session_key"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        trace_id: metadata
            .and_then(|m| m.get("trace_id"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        parent_run_id: metadata
            .and_then(|m| m.get("parent_run_id"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
        token_budget_limit: metadata
            .and_then(|m| m.get("token_budget_limit"))
            .and_then(|v| v.as_u64()),
        mask_config: metadata
            .and_then(|m| m.get("mask_config"))
            .and_then(|value| serde_json::from_value(value.clone()).ok()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::supervised::types::{RunKind, SupervisedRunSpec};

    #[tokio::test]
    async fn test_parse_routing_requires_channel_and_chat_id() {
        let spec = SupervisedRunSpec::from_spec("test task").with_kind(RunKind::Subagent);
        let record = agent_diva_core::supervised::types::RunRecord::from_spec(&spec);
        let error = parse_routing(&record).expect_err("missing routing should fail");
        assert!(error.to_string().contains("missing channel metadata"));
    }

    #[tokio::test]
    async fn test_parse_routing_from_channel_and_metadata() {
        let metadata = serde_json::json!({"chat_id": "room-42"});
        let spec = SupervisedRunSpec::from_spec("test task")
            .with_kind(RunKind::Subagent)
            .with_channel("telegram")
            .with_metadata(metadata);
        let record = agent_diva_core::supervised::types::RunRecord::from_spec(&spec);
        let (ch, chat) = parse_routing(&record).expect("routing");
        assert_eq!(ch, "telegram");
        assert_eq!(chat, "room-42");
    }

    #[tokio::test]
    async fn test_parse_context_from_metadata() {
        let metadata = serde_json::json!({
            "chat_id": "room-42",
            "session_key": "telegram:room-42",
            "trace_id": "trace-1",
            "parent_run_id": "run-1",
            "token_budget_limit": 1234
        });
        let spec = SupervisedRunSpec::from_spec("test task")
            .with_kind(RunKind::Subagent)
            .with_channel("telegram")
            .with_metadata(metadata);
        let record = agent_diva_core::supervised::types::RunRecord::from_spec(&spec);
        let context = parse_context(&record);
        assert_eq!(context.session_key.as_deref(), Some("telegram:room-42"));
        assert_eq!(context.trace_id.as_deref(), Some("trace-1"));
        assert_eq!(context.parent_run_id.as_deref(), Some("run-1"));
        assert_eq!(context.token_budget_limit, Some(1234));
    }
}
