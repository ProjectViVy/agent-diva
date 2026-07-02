//! Base trait for tools.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Describes what a tool is capable of doing.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolCapabilities {
    /// Whether this tool requires network access.
    #[serde(default)]
    pub requires_network: bool,

    /// Whether this tool requires filesystem access.
    #[serde(default)]
    pub requires_filesystem: bool,

    /// Whether this tool is read-only (does not mutate state).
    #[serde(default)]
    pub is_read_only: bool,

    /// Whether this tool can spawn sub-agents.
    #[serde(default)]
    pub can_spawn_subagent: bool,

    /// Whether this tool supports streaming output.
    #[serde(default)]
    pub supports_streaming: bool,
}

/// Metadata about a tool for registration and discovery.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToolMetadata {
    /// Semantic version of the tool.
    #[serde(default = "default_version")]
    pub version: String,

    /// Author of the tool.
    #[serde(default)]
    pub author: String,

    /// Category for grouping (e.g., "filesystem", "network", "ai").
    #[serde(default)]
    pub category: String,
}

fn default_version() -> String {
    "0.1.0".to_string()
}

/// Trait for tools.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name.
    fn name(&self) -> &str;

    /// Get the tool description.
    fn description(&self) -> &str;

    /// Get the tool parameters schema (JSON Schema format).
    fn parameters(&self) -> Value;

    /// Execute the tool with arguments.
    async fn execute(&self, args: Value) -> Result<String>;

    /// Validate parameters against the schema.
    fn validate_params(&self, params: &Value) -> Vec<String> {
        let schema = self.parameters();

        if !params.is_object() {
            return vec!["Parameters must be an object".to_string()];
        }

        let mut errors = Vec::new();

        if let Some(required) = schema.get("required").and_then(|r| r.as_array()) {
            let params_obj = params.as_object().expect("validated object");
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

    /// Convert tool to OpenAI function schema format.
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

    /// Get the tool's capabilities.
    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities::default()
    }

    /// Get the tool's metadata.
    fn metadata(&self) -> ToolMetadata {
        ToolMetadata::default()
    }

    /// Perform a health check on the tool.
    fn health_check(&self) -> Result<()> {
        Ok(())
    }
}

/// Tool errors.
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

    #[error("Tool execution timed out after {secs}s")]
    Timeout { secs: u64 },
}

pub type Result<T> = std::result::Result<T, ToolError>;

impl agent_diva_core::error_category::CategorizeError for ToolError {
    fn category(&self) -> agent_diva_core::error_category::ErrorCategory {
        match self {
            Self::Timeout { .. } => agent_diva_core::error_category::ErrorCategory::Timeout,
            Self::ExecutionFailed(_) | Self::Io(_) => {
                agent_diva_core::error_category::ErrorCategory::Fatal
            }
            Self::InvalidParams(_) | Self::InvalidArguments(_) => {
                agent_diva_core::error_category::ErrorCategory::Config
            }
            Self::Error(_) => agent_diva_core::error_category::ErrorCategory::Unknown,
        }
    }
}
