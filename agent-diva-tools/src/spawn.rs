//! Subagent spawning tool

use agent_diva_core::channel::{ChannelAddress, ChannelOrigin, ChannelRoute, Correlation};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::sync::Arc;

/// Callback function type for spawning subagents
type SpawnCallback = Arc<
    dyn Fn(
            String,
            Option<String>,
            ChannelRoute,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = std::result::Result<String, ToolError>> + Send>,
        > + Send
        + Sync,
>;

/// Spawn tool for creating subagents
///
/// This tool allows the main agent to spawn subagents for background task execution.
/// The subagent runs asynchronously and announces its result back when complete.
pub struct SpawnTool {
    spawn_callback: SpawnCallback,
    origin_route: Arc<tokio::sync::RwLock<ChannelRoute>>,
}

impl SpawnTool {
    /// Create a new spawn tool with a callback to the SubagentManager
    pub fn new<F, Fut>(spawn_fn: F) -> Self
    where
        F: Fn(String, Option<String>, ChannelRoute) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = std::result::Result<String, ToolError>> + Send + 'static,
    {
        Self {
            spawn_callback: Arc::new(move |task, label, route| {
                Box::pin(spawn_fn(task, label, route))
            }),
            origin_route: Arc::new(tokio::sync::RwLock::new(Self::default_route())),
        }
    }

    /// Set the typed parent route for subagent announcements.
    pub async fn set_context(&self, route: ChannelRoute) {
        *self.origin_route.write().await = route;
    }

    /// Bind the route captured by the current typed AgentLoop turn.
    pub fn with_context(self, route: ChannelRoute) -> Self {
        Self {
            spawn_callback: self.spawn_callback,
            origin_route: Arc::new(tokio::sync::RwLock::new(route)),
        }
    }

    fn default_route() -> ChannelRoute {
        ChannelRoute::new(
            ChannelAddress::new("cli", "direct"),
            Correlation::new("cli:direct"),
            ChannelOrigin::OwnerFrontend,
        )
    }
}

#[async_trait]
impl Tool for SpawnTool {
    fn name(&self) -> &str {
        "spawn"
    }

    fn description(&self) -> &str {
        "Spawn a subagent to handle a task in the background. \
         Use this for complex or time-consuming tasks that can run independently. \
         The subagent will complete the task and report back when done."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "task": {
                    "type": "string",
                    "description": "The task for the subagent to complete"
                },
                "label": {
                    "type": "string",
                    "description": "Optional short label for the task (for display)"
                }
            },
            "required": ["task"]
        })
    }

    async fn execute(&self, args: Value) -> std::result::Result<String, ToolError> {
        let task = match args.get("task").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => {
                return Err(ToolError::InvalidArguments(
                    "'task' parameter is required".to_string(),
                ))
            }
        };

        let label = args.get("label").and_then(|v| v.as_str()).map(String::from);

        let route = self.origin_route.read().await.clone();

        (self.spawn_callback)(task, label, route).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawn_tool_name() {
        let tool = SpawnTool::new(|_task, _label, _route| async { Ok("spawned".to_string()) });
        assert_eq!(tool.name(), "spawn");
    }

    #[tokio::test]
    async fn test_spawn_tool_parameters() {
        let tool = SpawnTool::new(|_task, _label, _route| async { Ok("spawned".to_string()) });
        let params = tool.parameters();
        assert!(params["properties"]["task"].is_object());
        assert_eq!(params["required"][0], "task");
    }

    #[tokio::test]
    async fn test_spawn_tool_execute() {
        let tool = SpawnTool::new(|task, label, route| async move {
            Ok(format!(
                "Spawned: {} (label: {:?}, channel: {}, chat_id: {}, session: {})",
                task,
                label,
                route.address.channel,
                route.address.chat_id,
                route.correlation.session_key
            ))
        });

        let args = json!({
            "task": "Test task",
            "label": "test"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("Spawned: Test task"));
        assert!(result.contains("label: Some(\"test\")"));
        assert!(result.contains("channel: cli"));
        assert!(result.contains("chat_id: direct"));
        assert!(result.contains("session: cli:direct"));
    }

    #[tokio::test]
    async fn test_spawn_tool_execute_without_label() {
        let tool = SpawnTool::new(|task, label, _route| async move {
            Ok(format!("Task: {}, Label: {:?}", task, label))
        });

        let args = json!({
            "task": "Another task"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("Task: Another task"));
        assert!(result.contains("Label: None"));
    }

    #[tokio::test]
    async fn test_spawn_tool_set_context() {
        let tool = SpawnTool::new(|_task, _label, route| async move {
            Ok(format!(
                "Channel: {}, Chat: {}, Session: {}",
                route.address.channel, route.address.chat_id, route.correlation.session_key
            ))
        });

        tool.set_context(ChannelRoute::new(
            ChannelAddress::new("telegram", "12345"),
            Correlation::new("opaque:telegram:12345"),
            ChannelOrigin::ExternalUser,
        ))
        .await;

        let args = json!({
            "task": "Test"
        });

        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("Channel: telegram"));
        assert!(result.contains("Chat: 12345"));
        assert!(result.contains("Session: opaque:telegram:12345"));
    }

    #[tokio::test]
    async fn test_spawn_tool_error_handling() {
        let tool = SpawnTool::new(|_task, _label, _route| async {
            Err(ToolError::ExecutionFailed("Spawn failed".to_string()))
        });

        let args = json!({
            "task": "Test"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Spawn failed"));
    }

    #[tokio::test]
    async fn test_spawn_tool_missing_task() {
        let tool =
            SpawnTool::new(|_task, _label, _route| async { Ok("should not reach".to_string()) });

        let args = json!({
            "label": "test"
        });

        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("'task' parameter is required"));
    }
}
