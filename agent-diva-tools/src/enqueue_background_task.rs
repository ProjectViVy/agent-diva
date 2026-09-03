//! Enqueue background task tool
//!
//! This tool allows the agent to enqueue a background task into the
//! supervised run queue. The task is stored in the `supervised_runs` table
//! and can be claimed and executed by a worker (e.g. SubagentRunHandler).

use agent_diva_core::channel::ChannelRoute;
use agent_diva_core::config::schema::MaskConfig;
use agent_diva_core::supervised::{RunKind, RunStore, SupervisedRunContext, SupervisedRunSpec};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Default routing and budget context inherited from the active agent turn.
#[derive(Debug, Clone, Default)]
pub struct BackgroundTaskContext {
    /// Authoritative parent channel route captured from the typed envelope.
    pub route: Option<ChannelRoute>,
    /// Dedicated supervised-run lineage; never encoded in generic metadata.
    pub parent_id: Option<String>,
    pub token_budget_limit: Option<u64>,
    /// Immutable parent-turn mask inherited by the supervised subagent.
    pub mask_config: Option<MaskConfig>,
}

/// Tool for enqueueing background tasks into the supervised run queue.
pub struct EnqueueBackgroundTaskTool {
    run_store: RunStore,
    context: BackgroundTaskContext,
}

impl EnqueueBackgroundTaskTool {
    /// Create a new enqueue_background_task tool backed by a `RunStore`.
    pub fn new(run_store: RunStore) -> Self {
        Self {
            run_store,
            context: BackgroundTaskContext::default(),
        }
    }

    /// Create a new tool with inherited turn context.
    pub fn with_context(run_store: RunStore, context: BackgroundTaskContext) -> Self {
        Self { run_store, context }
    }
}

#[async_trait]
impl Tool for EnqueueBackgroundTaskTool {
    fn name(&self) -> &str {
        "enqueue_background_task"
    }

    fn description(&self) -> &str {
        "Enqueue a background task into the supervised run queue. \
         The task will be picked up and executed by a background worker. \
         Returns the run ID of the created task."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "The task description or instruction"
                },
                "priority": {
                    "type": "integer",
                    "description": "Optional priority (higher = executed sooner). Defaults to 0."
                },
                "label": {
                    "type": "string",
                    "description": "Optional label stored in metadata for display"
                },
                "token_budget_limit": {
                    "type": "integer",
                    "description": "Optional parent session token budget limit"
                }
            },
            "required": ["message"]
        })
    }

    async fn execute(&self, args: Value) -> agent_diva_tooling::Result<String> {
        let message = match args.get("message").and_then(|v| v.as_str()) {
            Some(m) => m.to_string(),
            None => {
                return Err(ToolError::InvalidArguments(
                    "'message' parameter is required".to_string(),
                ))
            }
        };

        let mut spec = SupervisedRunSpec::from_spec(message).with_kind(RunKind::Subagent);

        if let Some(priority) = args.get("priority").and_then(|v| v.as_i64()) {
            spec = spec.with_priority(priority as i32);
        }

        let mut metadata = serde_json::Map::new();
        insert_string_arg_or_default(&mut metadata, &args, "label", None);
        let token_budget_limit = args
            .get("token_budget_limit")
            .and_then(|v| v.as_u64())
            .or(self.context.token_budget_limit);
        if let Some(route) = self.context.route.clone() {
            spec = spec
                .with_channel(route.address.channel.clone())
                .with_context(SupervisedRunContext {
                    route,
                    token_budget_limit,
                    mask_config: self.context.mask_config.clone(),
                });
        }
        if let Some(parent_id) = self.context.parent_id.clone() {
            spec = spec.with_parent_id(parent_id);
        }

        if !metadata.is_empty() {
            spec = spec.with_metadata(Value::Object(metadata));
        }

        let record = self
            .run_store
            .create(&spec)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(record.id)
    }
}

fn insert_string_arg_or_default(
    metadata: &mut serde_json::Map<String, Value>,
    args: &Value,
    key: &str,
    default: Option<String>,
) {
    let value = args
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .or(default);
    if let Some(value) = value {
        metadata.insert(key.to_string(), json!(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::supervised::RunStatus;
    use serde_json::json;
    use tempfile::TempDir;

    async fn setup_store() -> (TempDir, RunStore) {
        let dir = TempDir::new().expect("tempdir");
        let store = RunStore::new(dir.path()).await.expect("store creation");
        (dir, store)
    }

    #[tokio::test]
    async fn test_enqueue_background_task_name() {
        let (_dir, store) = setup_store().await;
        let tool = EnqueueBackgroundTaskTool::new(store);
        assert_eq!(tool.name(), "enqueue_background_task");
    }

    #[tokio::test]
    async fn test_enqueue_background_task_parameters() {
        let (_dir, store) = setup_store().await;
        let tool = EnqueueBackgroundTaskTool::new(store);
        let params = tool.parameters();
        assert!(params["properties"]["message"].is_object());
        assert_eq!(params["required"][0], "message");
        assert!(params["properties"]["priority"].is_object());
        assert!(params["properties"]["label"].is_object());
        assert!(params["properties"]["token_budget_limit"].is_object());
    }

    #[tokio::test]
    async fn test_enqueue_background_task_execute_basic() {
        let (_dir, store) = setup_store().await;
        let tool = EnqueueBackgroundTaskTool::new(store.clone());

        let args = json!({
            "message": "Analyze the codebase for dead code"
        });

        let run_id = tool.execute(args).await.expect("execute");
        assert!(!run_id.is_empty());

        // Verify the record exists in the supervised_runs table
        let record = store
            .get_record(&run_id)
            .await
            .expect("get_record")
            .expect("record should exist");
        assert_eq!(record.status, RunStatus::Queued);
        assert_eq!(record.message, "Analyze the codebase for dead code");
        assert_eq!(record.kind, RunKind::Subagent);
        assert_eq!(record.priority, 0);
    }

    #[tokio::test]
    async fn test_enqueue_background_task_execute_with_channel_and_priority() {
        let (_dir, store) = setup_store().await;
        let tool = EnqueueBackgroundTaskTool::new(store.clone());

        let args = json!({
            "message": "High priority cleanup",
            "priority": 10
        });

        let run_id = tool.execute(args).await.expect("execute");

        let record = store
            .get_record(&run_id)
            .await
            .expect("get_record")
            .expect("record should exist");
        assert_eq!(record.channel, None);
        assert_eq!(record.priority, 10);
        assert_eq!(record.kind, RunKind::Subagent);
    }

    #[tokio::test]
    async fn test_enqueue_background_task_execute_with_label() {
        let (_dir, store) = setup_store().await;
        let tool = EnqueueBackgroundTaskTool::new(store.clone());

        let args = json!({
            "message": "Refactor auth module",
            "label": "auth-refactor"
        });

        let run_id = tool.execute(args).await.expect("execute");

        let record = store
            .get_record(&run_id)
            .await
            .expect("get_record")
            .expect("record should exist");
        let metadata = record.metadata.expect("metadata");
        assert_eq!(metadata["label"], "auth-refactor");
    }

    #[tokio::test]
    async fn test_enqueue_background_task_persists_typed_context() {
        let (_dir, store) = setup_store().await;
        let mut address = agent_diva_core::channel::ChannelAddress::new("api", "chat-1");
        address.thread_id = Some("thread-1".to_string());
        let mut correlation = agent_diva_core::channel::Correlation::new("opaque-session");
        correlation.request_id = Some("request-1".to_string());
        correlation.trace_id = Some("trace-1".to_string());
        correlation.message_id = Some("message-1".to_string());
        let tool = EnqueueBackgroundTaskTool::with_context(
            store.clone(),
            BackgroundTaskContext {
                route: Some(ChannelRoute::new(
                    address,
                    correlation,
                    agent_diva_core::channel::ChannelOrigin::OwnerFrontend,
                )),
                parent_id: Some("parent-1".to_string()),
                token_budget_limit: Some(2048),
                mask_config: Some(MaskConfig::default()),
            },
        );

        let run_id = tool
            .execute(json!({
                "message": "Inherited context task",
                "label": "ctx"
            }))
            .await
            .expect("execute");

        let record = store
            .get_record(&run_id)
            .await
            .expect("get_record")
            .expect("record should exist");
        assert_eq!(record.channel.as_deref(), Some("api"));
        let metadata = record.metadata.expect("metadata");
        assert_eq!(metadata["label"], "ctx");
        assert_eq!(record.parent_id.as_deref(), Some("parent-1"));
        let context = record.context.expect("typed context");
        assert_eq!(context.route.correlation.session_key, "opaque-session");
        assert_eq!(
            context.route.correlation.request_id.as_deref(),
            Some("request-1")
        );
        assert_eq!(context.route.address.thread_id.as_deref(), Some("thread-1"));
        assert_eq!(context.token_budget_limit, Some(2048));
        assert!(context.mask_config.is_some());
    }

    #[tokio::test]
    async fn test_enqueue_background_task_missing_message() {
        let (_dir, store) = setup_store().await;
        let tool = EnqueueBackgroundTaskTool::new(store);

        let args = json!({
            "channel": "cli"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("message"),
            "error should mention missing message parameter"
        );
    }
}
