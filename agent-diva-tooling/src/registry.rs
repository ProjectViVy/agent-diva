//! Tool registry.

use crate::{Result, Tool, ToolError};
use agent_diva_core::error_context::{find_problematic_chars, ErrorContext};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;
use tracing::{error, warn};

const DEFAULT_TOOL_TIMEOUT_SECS: u64 = 60;

/// Maximum length for tool results (in characters) to prevent oversized API
/// requests. Matches the limit used by `agent_diva_tools::sanitize`.
const MAX_TOOL_RESULT_CHARS: usize = 80_000;

/// Truncate tool result to prevent oversized API requests.
/// This is a safety net to avoid 400 errors from LLM providers.
fn truncate_tool_result(result: &str) -> String {
    let char_count = result.chars().count();
    if char_count <= MAX_TOOL_RESULT_CHARS {
        result.to_string()
    } else {
        let truncated: String = result.chars().take(MAX_TOOL_RESULT_CHARS).collect();
        format!(
            "{}\n\n... [Result truncated: {} total characters, showing first {}]",
            truncated, char_count, MAX_TOOL_RESULT_CHARS
        )
    }
}

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
    timeout_secs: u64,
}

impl ToolRegistry {
    /// Create a new tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
            timeout_secs: DEFAULT_TOOL_TIMEOUT_SECS,
        }
    }

    /// Create a new tool registry with a specific default timeout.
    pub fn with_timeout_secs(timeout_secs: u64) -> Self {
        Self {
            timeout_secs,
            ..Self::new()
        }
    }

    /// Register a tool.
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.name().to_string();
        self.tools.insert(name, tool);
    }

    /// Unregister a tool by name.
    pub fn unregister(&mut self, name: &str) {
        self.tools.remove(name);
    }

    /// Get a tool by name.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Check if a tool is registered.
    pub fn has(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Get all tool definitions in OpenAI format.
    pub fn get_definitions(&self) -> Vec<Value> {
        self.tools.values().map(|tool| tool.to_schema()).collect()
    }

    /// Get the registry-level default tool timeout in seconds.
    pub fn timeout_secs(&self) -> u64 {
        self.timeout_secs
    }

    /// Execute a tool by name with given parameters.
    ///
    /// Returns `Ok(truncated_result)` on success (result is truncated to
    /// `MAX_TOOL_RESULT_CHARS`), or a `ToolError` on failure.
    pub async fn execute(&self, name: &str, params: Value) -> Result<String> {
        let tool = match self.tools.get(name) {
            Some(tool) => tool,
            None => {
                let msg = format!("Tool '{}' not found", name);
                let ctx = ErrorContext::new("tool_lookup", &msg)
                    .with_metadata("tool_name", name.to_string())
                    .with_metadata("available_tools", self.tool_names().join(", "));
                warn!("{}", ctx.to_detailed_string());
                return Err(ToolError::Error(msg));
            }
        };

        let errors = tool.validate_params(&params);
        if !errors.is_empty() {
            let params_str = serde_json::to_string(&params).unwrap_or_default();
            let problems = find_problematic_chars(&params_str);
            let msg = format!(
                "Invalid parameters for tool '{}': {}",
                name,
                errors.join("; "),
            );
            let ctx = ErrorContext::new("tool_validation", &msg)
                .with_content(&params_str)
                .with_metadata("tool_name", name.to_string());
            let ctx_str = ctx.to_detailed_string();
            if problems.is_empty() {
                warn!("{}", ctx_str);
            } else {
                warn!(
                    "{}\n  Problematic characters found:\n    - {}",
                    ctx_str,
                    problems.join("\n    - ")
                );
            }
            return Err(ToolError::InvalidParams(msg));
        }

        match timeout(
            Duration::from_secs(self.timeout_secs),
            tool.execute(params.clone()),
        )
        .await
        {
            Err(_) => {
                let params_str = serde_json::to_string(&params).unwrap_or_default();
                let problems = find_problematic_chars(&params_str);
                let msg = format!(
                    "Tool '{}' timed out after {} seconds",
                    name, self.timeout_secs
                );
                let ctx = ErrorContext::new("tool_execution_timeout", &msg)
                    .with_content(&params_str)
                    .with_metadata("tool_name", name.to_string())
                    .with_metadata("timeout_secs", self.timeout_secs.to_string());
                let ctx_str = ctx.to_detailed_string();
                if problems.is_empty() {
                    error!("{}", ctx_str);
                } else {
                    error!(
                        "{}\n  Problematic characters found:\n    - {}",
                        ctx_str,
                        problems.join("\n    - ")
                    );
                }
                Err(ToolError::ExecutionFailed(msg))
            }
            Ok(result) => match result {
                Ok(output) => Ok(truncate_tool_result(&output)),
                Err(e) => {
                    let params_str = serde_json::to_string(&params).unwrap_or_default();
                    let problems = find_problematic_chars(&params_str);
                    let ctx = ErrorContext::new("tool_execution", e.to_string())
                        .with_content(&params_str)
                        .with_metadata("tool_name", name.to_string());
                    let ctx_str = ctx.to_detailed_string();
                    if problems.is_empty() {
                        error!("{}", ctx_str);
                    } else {
                        error!(
                            "{}\n  Problematic characters found:\n    - {}",
                            ctx_str,
                            problems.join("\n    - ")
                        );
                    }
                    Err(e)
                }
            },
        }
    }

    /// Get list of registered tool names.
    pub fn tool_names(&self) -> Vec<String> {
        self.tools.keys().cloned().collect()
    }

    /// Get number of registered tools.
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty.
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use tokio::time::sleep;

    struct MockTool;
    struct SlowTool;

    #[async_trait]
    impl Tool for MockTool {
        fn name(&self) -> &str {
            "mock"
        }

        fn description(&self) -> &str {
            "A mock tool"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        async fn execute(&self, _args: Value) -> crate::Result<String> {
            Ok("mock result".to_string())
        }
    }

    #[async_trait]
    impl Tool for SlowTool {
        fn name(&self) -> &str {
            "slow"
        }

        fn description(&self) -> &str {
            "A slow mock tool"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            })
        }

        async fn execute(&self, _args: Value) -> crate::Result<String> {
            sleep(Duration::from_millis(50)).await;
            Ok("too late".to_string())
        }
    }

    #[test]
    fn test_register_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));
        assert_eq!(registry.len(), 1);
        assert!(registry.has("mock"));
    }

    #[test]
    fn test_unregister_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));
        registry.unregister("mock");
        assert_eq!(registry.len(), 0);
        assert!(!registry.has("mock"));
    }

    #[tokio::test]
    async fn test_execute_tool() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool));
        let result = registry.execute("mock", serde_json::json!({})).await;
        assert_eq!(result.unwrap(), "mock result");
    }

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("nonexistent", serde_json::json!({})).await;
        let err = result.unwrap_err();
        assert!(err.to_string().contains("Tool 'nonexistent' not found"));
    }

    #[test]
    fn test_registry_timeout_defaults_to_sixty_seconds() {
        let registry = ToolRegistry::new();
        assert_eq!(registry.timeout_secs(), 60);
    }

    #[tokio::test]
    async fn test_execute_tool_timeout_wrapped() {
        let mut registry = ToolRegistry::with_timeout_secs(0);
        registry.register(Arc::new(SlowTool));

        let result = registry.execute("slow", serde_json::json!({})).await;
        let err = result.unwrap_err();
        assert!(err.to_string().contains("timed out after 0 seconds"));
    }

    #[tokio::test]
    async fn test_execute_truncates_large_results() {
        struct BigResultTool;

        #[async_trait]
        impl Tool for BigResultTool {
            fn name(&self) -> &str {
                "big"
            }
            fn description(&self) -> &str {
                "Returns a large result"
            }
            fn parameters(&self) -> Value {
                serde_json::json!({"type": "object", "properties": {}, "required": []})
            }
            async fn execute(&self, _args: Value) -> crate::Result<String> {
                Ok("x".repeat(MAX_TOOL_RESULT_CHARS + 5000))
            }
        }

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(BigResultTool));
        let result = registry
            .execute("big", serde_json::json!({}))
            .await
            .unwrap();
        assert!(result.len() < MAX_TOOL_RESULT_CHARS + 5000);
        assert!(result.contains("Result truncated"));
    }
}
