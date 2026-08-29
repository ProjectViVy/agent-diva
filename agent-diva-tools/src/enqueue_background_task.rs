//! Enqueue background task tool
//!
//! This tool allows the agent to enqueue a background task into the
//! supervised run queue. The task is stored in the `supervised_runs` table
//! and can be claimed and executed by a worker (e.g. SubagentRunHandler).

use agent_diva_core::config::schema::MaskConfig;
use agent_diva_core::supervised::{RunKind, RunStore, SupervisedRunSpec};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Default routing and budget context inherited from the active agent turn.
#[derive(Debug, Clone, Default)]
pub struct BackgroundTaskContext {
    pub channel: Option<String>,
    pub chat_id: Option<String>,
    pub session_key: Option<String>,
    pub trace_id: Option<String>,
    pub parent_run_id: Option<String>,
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
                "channel": {
                    "type": "string",
                    "description": "Optional channel context for the task"
                },
                "priority": {
                    "type": "integer",
                    "description": "Optional priority (higher = executed sooner). Defaults to 0."
                },
                "label": {
                    "type": "string",
                    "description": "Optional label stored in metadata for display"
                },
                "chat_id": {
                    "type": "string",
                    "description": "Optional chat id override for result routing"
                },
                "session_key": {
                    "type": "string",
                    "description": "Optional parent session key for token ledger inheritance"
                },
                "trace_id": {
                    "type": "string",
                    "description": "Optional parent trace id"
                },
                "parent_run_id": {
                    "type": "string",
                    "description": "Optional parent run id for lineage"
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

        let channel = args
            .get("channel")
            .and_then(|v| v.as_str())
            .map(str::to_string)
            .or_else(|| self.context.channel.clone());
        if let Some(channel) = channel {
            spec = spec.with_channel(channel);
        }

        if let Some(priority) = args.get("priority").and_then(|v| v.as_i64()) {
            spec = spec.with_priority(priority as i32);
        }

        let mut metadata = serde_json::Map::new();
        insert_string_arg_or_default(&mut metadata, &args, "label", None);
        insert_string_arg_or_default(
            &mut metadata,
            &args,
            "chat_id",
            self.context.chat_id.clone(),
        );
        insert_string_arg_or_default(
            &mut metadata,
            &args,
            "session_key",
            self.context.session_key.clone(),
        );
        insert_string_arg_or_default(
            &mut metadata,
            &args,
            "trace_id",
            self.context.trace_id.clone(),
        );
        insert_string_arg_or_default(
            &mut metadata,
            &args,
            "parent_run_id",
            self.context.parent_run_id.clone(),
        );
        let token_budget_limit = args
            .get("token_budget_limit")
            .and_then(|v| v.as_u64())
            .or(self.context.token_budget_limit);
        if let Some(limit) = token_budget_limit {
            metadata.insert("token_budget_limit".to_string(), json!(limit));
        }
        if let Some(mask) = self.context.mask_config.as_ref() {
            metadata.insert("mask_config".to_string(), json!(mask));
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
        assert!(params["properties"]["channel"].is_object());
        assert!(params["properties"]["priority"].is_object());
        assert!(params["properties"]["label"].is_object());
        assert!(params["properties"]["chat_id"].is_object());
        assert!(params["properties"]["session_key"].is_object());
        assert!(params["properties"]["trace_id"].is_object());
        assert!(params["properties"]["parent_run_id"].is_object());
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
            "channel": "telegram",
            "priority": 10
        });

        let run_id = tool.execute(args).await.expect("execute");

        let record = store
            .get_record(&run_id)
            .await
            .expect("get_record")
            .expect("record should exist");
        assert_eq!(record.channel, Some("telegram".to_string()));
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
    async fn test_enqueue_background_task_inherits_context_metadata() {
        let (_dir, store) = setup_store().await;
        let tool = EnqueueBackgroundTaskTool::with_context(
            store.clone(),
            BackgroundTaskContext {
                channel: Some("api".to_string()),
                chat_id: Some("chat-1".to_string()),
                session_key: Some("api:chat-1".to_string()),
                trace_id: Some("trace-1".to_string()),
                parent_run_id: Some("parent-1".to_string()),
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
        assert_eq!(metadata["chat_id"], "chat-1");
        assert_eq!(metadata["session_key"], "api:chat-1");
        assert_eq!(metadata["trace_id"], "trace-1");
        assert_eq!(metadata["parent_run_id"], "parent-1");
        assert_eq!(metadata["token_budget_limit"], 2048);
        assert!(metadata["mask_config"].is_object());
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
