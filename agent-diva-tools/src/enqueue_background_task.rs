//! Enqueue background task tool
//!
//! This tool allows the agent to enqueue a background task into the
//! supervised run queue. The task is stored in the `supervised_runs` table
//! and can be claimed and executed by a worker (e.g. SubagentRunHandler).

use agent_diva_core::supervised::{RunKind, RunStore, SupervisedRunSpec};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Tool for enqueueing background tasks into the supervised run queue.
pub struct EnqueueBackgroundTaskTool {
    run_store: RunStore,
}

impl EnqueueBackgroundTaskTool {
    /// Create a new enqueue_background_task tool backed by a `RunStore`.
    pub fn new(run_store: RunStore) -> Self {
        Self { run_store }
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

        if let Some(channel) = args.get("channel").and_then(|v| v.as_str()) {
            spec = spec.with_channel(channel);
        }

        if let Some(priority) = args.get("priority").and_then(|v| v.as_i64()) {
            spec = spec.with_priority(priority as i32);
        }

        if let Some(label) = args.get("label").and_then(|v| v.as_str()) {
            let metadata = json!({ "label": label });
            spec = spec.with_metadata(metadata);
        }

        let record = self
            .run_store
            .create(&spec)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))?;

        Ok(record.id)
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
