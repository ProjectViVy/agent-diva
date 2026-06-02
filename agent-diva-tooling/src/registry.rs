//! Tool registry.

use crate::Tool;
use agent_diva_core::error_context::{find_problematic_chars, ErrorContext};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, warn};

const ERROR_HINT: &str = "\n\n[Analyze the error above and try a different approach.]";

/// Registry of available tools.
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new tool registry.
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
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

    /// Execute a tool by name with given parameters.
    pub async fn execute(&self, name: &str, params: Value) -> String {
        let tool = match self.tools.get(name) {
            Some(tool) => tool,
            None => {
                let ctx = ErrorContext::new("tool_lookup", format!("Tool '{}' not found", name))
                    .with_metadata("tool_name", name.to_string())
                    .with_metadata("available_tools", self.tool_names().join(", "));
                warn!("{}", ctx.to_detailed_string());
                return format!("Error: Tool '{}' not found{}", name, ERROR_HINT);
            }
        };

        let errors = tool.validate_params(&params);
        if !errors.is_empty() {
            let params_str = serde_json::to_string(&params).unwrap_or_default();
            let problems = find_problematic_chars(&params_str);
            let ctx = ErrorContext::new("tool_validation", errors.join("; "))
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
            return format!(
                "Error: Invalid parameters for tool '{}': {}{}",
                name,
                errors.join("; "),
                ERROR_HINT,
            );
        }

        match tool.execute(params.clone()).await {
            Ok(result) => {
                if result.starts_with("Error") {
                    let params_str = serde_json::to_string(&params).unwrap_or_default();
                    let ctx = ErrorContext::new("tool_execution", &result)
                        .with_content(&params_str)
                        .with_metadata("tool_name", name.to_string());
                    warn!("{}", ctx.to_detailed_string());
                    format!("{}{}", result, ERROR_HINT)
                } else {
                    result
                }
            }
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
                format!("Error executing {}: {}{}", name, e, ERROR_HINT)
            }
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

    struct MockTool;

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
        assert_eq!(result, "mock result");
    }

    #[tokio::test]
    async fn test_execute_unknown_tool() {
        let registry = ToolRegistry::new();
        let result = registry.execute("nonexistent", serde_json::json!({})).await;
        assert!(result.contains("Tool 'nonexistent' not found"));
        assert!(result.contains("[Analyze the error above"));
    }
}
