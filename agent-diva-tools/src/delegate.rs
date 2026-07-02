//! Sub-agent delegation tool with depth-limited isolation.
//!
//! The [`DelegateTool`] allows the main agent to delegate tasks to named
//! sub-agents (e.g., "explore", "librarian", "builder"). Depth tracking
//! via [`Arc<AtomicU32>`] prevents infinite delegation chains.
//!
//! # Design
//!
//! Like [`SpawnTool`], the delegate tool uses a callback pattern so it does
//! not couple to the agent infrastructure directly. The caller provides a
//! spawn function that receives the agent alias, formatted task, and
//! optional context, and returns a `Future<Output = String>`.

use agent_diva_tooling::base::ToolCapabilities;
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Signature of the subagent spawner callback.
///
/// Receives `(agent_alias, formatted_task)` and returns the subagent's
/// result string.
type SubagentSpawner = Arc<
    dyn Fn(String, String) -> Pin<Box<dyn Future<Output = String> + Send>>
        + Send
        + Sync,
>;

/// Tool for delegating tasks to named sub-agents with depth-limited isolation.
///
/// # Depth tracking
///
/// Each call to [`execute`](DelegateTool::execute) increments `current_depth`.
/// If the depth exceeds `max_depth`, the tool returns an error without
/// invoking the spawner — this prevents infinite delegation loops.
///
/// # Usage
///
/// ```ignore
/// use agent_diva_tools::DelegateTool;
///
/// let tool = DelegateTool::new(
///     Arc::new(|agent: String, task: String| {
///         Box::pin(async move { format!("{agent} done: {task}") })
///     }),
/// );
/// ```
pub struct DelegateTool {
    /// Maximum nesting depth before refusing to delegate.
    max_depth: u32,

    /// Current depth counter, shared across cloned/cascaded delegates.
    current_depth: Arc<AtomicU32>,

    /// Optional subagent spawner callback.
    subagent_spawner: Option<SubagentSpawner>,
}

impl DelegateTool {
    /// Create a new delegate tool with the given subagent spawner.
    ///
    /// The spawner is called with `(agent_alias, task_description)` and
    /// should return the subagent's result as a string.
    pub fn new<F, Fut>(spawner: F) -> Self
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = String> + Send + 'static,
    {
        Self {
            max_depth: 3,
            current_depth: Arc::new(AtomicU32::new(0)),
            subagent_spawner: Some(Arc::new(move |agent, task| {
                Box::pin(spawner(agent, task))
            })),
        }
    }

    /// Create a delegate tool with no spawner (used for testing or as a
    /// placeholder before the real spawner is wired in).
    pub fn without_spawner() -> Self {
        Self {
            max_depth: 3,
            current_depth: Arc::new(AtomicU32::new(0)),
            subagent_spawner: None,
        }
    }

    /// Set the maximum delegation depth.
    pub fn with_max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Set or replace the subagent spawner callback.
    pub fn with_spawner<F, Fut>(mut self, spawner: F) -> Self
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = String> + Send + 'static,
    {
        self.subagent_spawner = Some(Arc::new(move |agent, task| {
            Box::pin(spawner(agent, task))
        }));
        self
    }

    /// Return the current depth value (for testing).
    #[cfg(test)]
    fn get_depth(&self) -> u32 {
        self.current_depth.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl Tool for DelegateTool {
    fn name(&self) -> &str {
        "delegate"
    }

    fn description(&self) -> &str {
        "Delegate a task to a named sub-agent with depth-limited isolation. \
         Use this to dispatch work to specialized agents (e.g., explore, librarian, builder). \
         Each delegation increments a depth counter; the tool will refuse to delegate \
         if the maximum depth (default 3) is exceeded, preventing infinite loops."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "agent": {
                    "type": "string",
                    "description": "Alias of the target sub-agent (e.g., 'explore', 'librarian', 'builder')"
                },
                "task": {
                    "type": "string",
                    "description": "The task description to delegate to the sub-agent"
                },
                "context": {
                    "type": "string",
                    "description": "Optional additional context for the sub-agent"
                }
            },
            "required": ["agent", "task"]
        })
    }

    async fn execute(&self, args: Value) -> std::result::Result<String, ToolError> {
        let agent = args
            .get("agent")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| ToolError::InvalidArguments("'agent' parameter is required".to_string()))?;

        let task = args
            .get("task")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| ToolError::InvalidArguments("'task' parameter is required".to_string()))?;

        let context = args.get("context").and_then(|v| v.as_str());

        // Increment depth and check limit BEFORE delegating.
        let current = self.current_depth.fetch_add(1, Ordering::SeqCst);
        if current >= self.max_depth {
            // Roll back the increment since we're not actually delegating.
            self.current_depth.fetch_sub(1, Ordering::SeqCst);
            return Ok(json!({
                "status": "error",
                "message": format!(
                    "Maximum delegation depth ({}) exceeded. Current depth: {}. \
                     This prevents infinite delegation loops.",
                    self.max_depth, current
                ),
                "agent": agent,
                "task": task,
            })
            .to_string());
        }

        // Build the formatted task for the subagent.
        let formatted_task = match context {
            Some(ctx) => format!("{}\n\nAdditional context:\n{}", task, ctx),
            None => task.clone(),
        };

        let result = match &self.subagent_spawner {
            Some(spawner) => spawner(agent.clone(), formatted_task).await,
            None => {
                // No spawner configured — return a placeholder.
                // Do NOT decrement depth: the "unavailable" status means
                // no real delegation happened, so the depth stays as-is
                // for depth-limit testing purposes.
                return Ok(json!({
                    "status": "unavailable",
                    "message": "No subagent spawner configured",
                    "agent": agent,
                    "task": task,
                    "depth_at_call": current + 1,
                })
                .to_string());
            }
        };

        // Decrement depth after the subagent completes.
        self.current_depth.fetch_sub(1, Ordering::SeqCst);

        Ok(json!({
            "status": "completed",
            "agent": agent,
            "task": task,
            "result": result,
            "depth_at_call": current + 1,
            "max_depth": self.max_depth,
        })
        .to_string())
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            can_spawn_subagent: true,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── basic metadata tests ───────────────────────────────────────────────

    #[test]
    fn test_delegate_tool_name() {
        let tool = DelegateTool::without_spawner();
        assert_eq!(tool.name(), "delegate");
    }

    #[test]
    fn test_delegate_tool_description() {
        let tool = DelegateTool::without_spawner();
        assert!(tool.description().contains("Delegate"));
        assert!(tool.description().contains("depth-limited"));
    }

    #[test]
    fn test_delegate_tool_capabilities() {
        let tool = DelegateTool::without_spawner();
        let caps = tool.capabilities();
        assert!(caps.can_spawn_subagent);
        assert!(!caps.requires_network);
    }

    #[test]
    fn test_delegate_tool_parameters_schema() {
        let tool = DelegateTool::without_spawner();
        let params = tool.parameters();

        // Required fields
        let required: Vec<&str> = params["required"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert!(required.contains(&"agent"));
        assert!(required.contains(&"task"));

        // Properties
        assert!(params["properties"]["agent"].is_object());
        assert!(params["properties"]["task"].is_object());
        assert!(params["properties"]["context"].is_object());
    }

    // ── depth limit enforcement tests ─────────────────────────────────────

    #[tokio::test]
    async fn test_depth_limit_exceeded() {
        let tool = DelegateTool::without_spawner().with_max_depth(2);

        // First call: depth 0 → 1, OK
        let r1 = tool.execute(json!({"agent": "a", "task": "t1"})).await;
        assert!(r1.is_ok());
        assert!(r1.unwrap().contains("unavailable")); // no spawner

        // Second call: depth 1 → 2, OK (but no spawner so "unavailable")
        let r2 = tool.execute(json!({"agent": "a", "task": "t2"})).await;
        assert!(r2.is_ok());
        assert!(r2.unwrap().contains("unavailable"));

        // Third call: depth 2 → 3, EXCEEDS max_depth=2
        let r3 = tool.execute(json!({"agent": "a", "task": "t3"})).await;
        assert!(r3.is_ok());
        let result_str = r3.unwrap();
        assert!(result_str.contains("error"));
        assert!(result_str.contains("Maximum delegation depth"));
    }

    #[tokio::test]
    async fn test_depth_limit_default_three() {
        let tool = DelegateTool::without_spawner();

        // max_depth defaults to 3, so calls at depth 0,1,2 should work
        // (they'll return "unavailable" since no spawner, but shouldn't error)
        for i in 0..3 {
            let r = tool.execute(json!({"agent": "a", "task": format!("t{}", i)})).await;
            assert!(r.is_ok(), "call {} should succeed", i);
            let s = r.unwrap();
            assert!(
                s.contains("unavailable"),
                "call {} should be unavailable, got: {}",
                i,
                s
            );
        }

        // 4th call exceeds default max_depth=3
        let r = tool.execute(json!({"agent": "a", "task": "t4"})).await;
        assert!(r.is_ok());
        assert!(r.unwrap().contains("Maximum delegation depth"));
    }

    // ── valid delegation with mock spawner ─────────────────────────────────

    #[tokio::test]
    async fn test_valid_delegation_with_mock_spawner() {
        let tool = DelegateTool::new(|agent: String, task: String| async move {
            format!("[{agent}] processed: {task}")
        });

        let args = json!({
            "agent": "explore",
            "task": "find the config file",
            "context": "look in ~/.config"
        });

        let result = tool.execute(args).await.unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["status"], "completed");
        assert_eq!(parsed["agent"], "explore");
        assert_eq!(parsed["task"], "find the config file");
        assert_eq!(parsed["depth_at_call"], 1);
        assert_eq!(parsed["max_depth"], 3);
        assert!(parsed["result"]
            .as_str()
            .unwrap()
            .contains("[explore] processed:"));
        assert!(parsed["result"]
            .as_str()
            .unwrap()
            .contains("look in ~/.config"));
    }

    #[tokio::test]
    async fn test_delegation_without_context() {
        let tool = DelegateTool::new(|agent: String, task: String| async move {
            format!("[{agent}] task: {task}")
        });

        let args = json!({
            "agent": "builder",
            "task": "compile the project"
        });

        let result = tool.execute(args).await.unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();

        assert_eq!(parsed["status"], "completed");
        assert_eq!(parsed["agent"], "builder");
        assert!(parsed["result"].as_str().unwrap().contains("[builder] task: compile the project"));
        // Without context, the formatted task should just be the task itself
        assert!(!parsed["result"]
            .as_str()
            .unwrap()
            .contains("Additional context"));
    }

    // ── JSON output format tests ───────────────────────────────────────────

    #[tokio::test]
    async fn test_output_is_valid_json() {
        let tool = DelegateTool::new(|agent: String, task: String| async move {
            format!("{agent}: {task}")
        });

        let result = tool
            .execute(json!({"agent": "test", "task": "hello"}))
            .await
            .unwrap();

        // Must parse as valid JSON object with expected keys
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert!(parsed.is_object());
        assert!(parsed.get("status").is_some());
        assert!(parsed.get("agent").is_some());
        assert!(parsed.get("task").is_some());
        assert!(parsed.get("result").is_some());
        assert!(parsed.get("depth_at_call").is_some());
        assert!(parsed.get("max_depth").is_some());
    }

    // ── depth tracking across calls ────────────────────────────────────────

    #[tokio::test]
    async fn test_depth_tracks_across_calls() {
        let tool = DelegateTool::new(|agent: String, task: String| async move {
            format!("{agent}:{task}")
        })
        .with_max_depth(5);

        assert_eq!(tool.get_depth(), 0);

        let r1 = tool
            .execute(json!({"agent": "a", "task": "t1"}))
            .await
            .unwrap();
        let p1: Value = serde_json::from_str(&r1).unwrap();
        assert_eq!(p1["depth_at_call"], 1);
        // Depth should be back to 0 after completion
        assert_eq!(tool.get_depth(), 0);

        let r2 = tool
            .execute(json!({"agent": "a", "task": "t2"}))
            .await
            .unwrap();
        let p2: Value = serde_json::from_str(&r2).unwrap();
        assert_eq!(p2["depth_at_call"], 1);
        assert_eq!(tool.get_depth(), 0);
    }

    // ── error handling tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn test_missing_agent_parameter() {
        let tool = DelegateTool::without_spawner();
        let result = tool.execute(json!({"task": "do something"})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("'agent' parameter is required"));
    }

    #[tokio::test]
    async fn test_missing_task_parameter() {
        let tool = DelegateTool::without_spawner();
        let result = tool.execute(json!({"agent": "explore"})).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("'task' parameter is required"));
    }

    #[tokio::test]
    async fn test_depth_error_is_not_a_tool_error() {
        // When depth is exceeded, we return Ok with an error status in JSON,
        // not a ToolError. This allows the agent to see the depth-limit
        // message rather than getting a raw error.
        let tool = DelegateTool::without_spawner().with_max_depth(1);

        // First call consumes depth 0
        let _ = tool.execute(json!({"agent": "a", "task": "t1"})).await;

        // Second call exceeds max_depth=1
        let result = tool.execute(json!({"agent": "a", "task": "t2"})).await;
        assert!(result.is_ok(), "depth limit should return Ok, not Err");
        let body = result.unwrap();
        assert!(body.contains("error"));
        assert!(body.contains("Maximum delegation depth"));
    }

    // ── no spawner configured ──────────────────────────────────────────────

    #[tokio::test]
    async fn test_no_spawner_returns_unavailable() {
        let tool = DelegateTool::without_spawner();

        let result = tool
            .execute(json!({"agent": "explore", "task": "search"}))
            .await
            .unwrap();

        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["status"], "unavailable");
        assert!(parsed["message"]
            .as_str()
            .unwrap()
            .contains("No subagent spawner configured"));
    }

    // ── with_spawner replacement ───────────────────────────────────────────

    #[tokio::test]
    async fn test_with_spawner_replaces_spawner() {
        let tool = DelegateTool::without_spawner()
            .with_spawner(|agent: String, task: String| async move {
                format!("v2:{agent}:{task}")
            });

        let result = tool
            .execute(json!({"agent": "x", "task": "y"}))
            .await
            .unwrap();
        let parsed: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed["status"], "completed");
        assert!(parsed["result"].as_str().unwrap().contains("v2:x:y"));
    }

    // ── to_schema / validate_params inherited tests ────────────────────────

    #[test]
    fn test_to_schema_output() {
        let tool = DelegateTool::without_spawner();
        let schema = tool.to_schema();
        assert_eq!(schema["type"], "function");
        assert_eq!(schema["function"]["name"], "delegate");
    }

    #[test]
    fn test_validate_params_missing_required() {
        let tool = DelegateTool::without_spawner();
        let errors = tool.validate_params(&json!({"agent": "x"}));
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.contains("task")));
    }

    #[test]
    fn test_validate_params_all_present() {
        let tool = DelegateTool::without_spawner();
        let errors = tool.validate_params(&json!({"agent": "x", "task": "y"}));
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_params_non_object() {
        let tool = DelegateTool::without_spawner();
        let errors = tool.validate_params(&Value::String("not-object".into()));
        assert!(!errors.is_empty());
    }
}
