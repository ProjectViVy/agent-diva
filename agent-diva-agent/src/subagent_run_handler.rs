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

        let context = parse_context(&record)?;

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
            .run_supervised(task.clone(), label, context)
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

fn parse_context(record: &RunRecord) -> Result<SupervisedSubagentContext, RunExecutionError> {
    let context = record.context.clone().ok_or_else(|| {
        RunExecutionError::HandlerError("subagent run missing typed context".to_string())
    })?;
    Ok(SupervisedSubagentContext::from_run_context(
        context,
        record.parent_id.clone(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::channel::{ChannelAddress, ChannelOrigin, ChannelRoute, Correlation};
    use agent_diva_core::supervised::types::{RunKind, SupervisedRunContext, SupervisedRunSpec};

    #[tokio::test]
    async fn test_parse_context_requires_typed_route() {
        let spec = SupervisedRunSpec::from_spec("test task").with_kind(RunKind::Subagent);
        let record = agent_diva_core::supervised::types::RunRecord::from_spec(&spec);
        let error = parse_context(&record).expect_err("missing typed context should fail");
        assert!(error.to_string().contains("missing typed context"));
    }

    #[tokio::test]
    async fn test_parse_context_preserves_opaque_route() {
        let mut address = ChannelAddress::new("telegram", "room-42");
        address.thread_id = Some("thread-7".to_string());
        let mut correlation = Correlation::new("opaque-session");
        correlation.request_id = Some("request-1".to_string());
        correlation.trace_id = Some("trace-1".to_string());
        correlation.message_id = Some("message-1".to_string());
        let spec = SupervisedRunSpec::from_spec("test task")
            .with_kind(RunKind::Subagent)
            .with_channel("telegram")
            .with_context(SupervisedRunContext {
                route: ChannelRoute::new(address, correlation, ChannelOrigin::ExternalUser),
                token_budget_limit: Some(1234),
                mask_config: None,
            })
            .with_parent_id("run-1");
        let record = agent_diva_core::supervised::types::RunRecord::from_spec(&spec);
        let context = parse_context(&record).expect("typed context");
        assert_eq!(context.route.address.channel, "telegram");
        assert_eq!(context.route.address.thread_id.as_deref(), Some("thread-7"));
        assert_eq!(context.route.correlation.session_key, "opaque-session");
        assert_eq!(
            context.route.correlation.trace_id.as_deref(),
            Some("trace-1")
        );
        assert_eq!(context.parent_id.as_deref(), Some("run-1"));
        assert_eq!(context.token_budget_limit, Some(1234));
    }
}
