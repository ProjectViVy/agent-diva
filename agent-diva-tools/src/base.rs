//! Base trait for tools

use async_trait::async_trait;
use serde_json::Value;

/// Trait for tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name
    fn name(&self) -> &str;

    /// Get the tool description
    fn description(&self) -> &str;

    /// Get the tool parameters schema (JSON Schema format)
    fn parameters(&self) -> Value;

    /// Execute the tool with arguments
    async fn execute(&self, args: Value) -> Result<String>;

    /// Validate parameters against the schema
    fn validate_params(&self, params: &Value) -> Vec<String> {
        // Basic validation - can be enhanced with full JSON Schema validation
        let schema = self.parameters();

        if !params.is_object() {
            return vec!["Parameters must be an object".to_string()];
        }

        let mut errors = Vec::new();

        // Check required fields
        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            let params_obj = params.as_object().unwrap();
            for field in required {
                if let Some(field_name) = field.as_str() {
                    if !params_obj.contains_key(field_name) {
                        errors.push(format!("Missing required field: {}", field_name));
                    }
                }
            }
        }

        errors
    }

    /// Convert tool to OpenAI function schema format
    fn to_schema(&self) -> Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name(),
                "description": self.description(),
                "parameters": self.parameters(),
            }
        })
    }
}

/// Tool errors
#[derive(Debug, thiserror::Error)]
pub enum ToolError {
    #[error("Tool error: {0}")]
    Error(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Invalid arguments: {0}")]
    InvalidArguments(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ToolError>;
