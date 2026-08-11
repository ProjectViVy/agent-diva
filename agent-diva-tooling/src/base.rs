//! Base trait for tools.

use async_trait::async_trait;
use serde_json::Value;

/// Trait for tools.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Get the tool name.
    fn name(&self) -> &str;

    /// Get the tool description.
    fn description(&self) -> &str;

    /// Get the tool parameters schema (JSON Schema format).
    fn parameters(&self) -> Value;

    /// Optional execution timeout override for long-lived interactive tools.
    fn timeout_secs(&self) -> Option<u64> {
        None
    }

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

    /// The model attempted to mount a deferred tool before discovering it in
    /// the current session.
    #[error("tool_not_discovered: tool '{name}' must be returned by tool_search before mounting")]
    ToolNotDiscovered { name: String },

    /// The model attempted to call a discovered deferred tool before mounting
    /// it into the provider-facing tool set.
    #[error("tool_not_mounted: tool '{name}' must be mounted before execution")]
    ToolNotMounted { name: String },

    /// A session mount intent survived, but the currently authorized source
    /// no longer provides the tool.
    #[error("tool_unavailable: tool '{name}' is no longer available")]
    ToolUnavailable { name: String },

    /// The requested name is not a mountable deferred tool in the current
    /// authorized catalog.
    #[error("tool_mount_forbidden: tool '{name}' cannot be mounted in the current tool surface")]
    ToolMountForbidden { name: String },
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
            Self::Error(_)
            | Self::ToolNotDiscovered { .. }
            | Self::ToolNotMounted { .. }
            | Self::ToolUnavailable { .. }
            | Self::ToolMountForbidden { .. } => {
                agent_diva_core::error_category::ErrorCategory::Unknown
            }
        }
    }
}

impl ToolError {
    /// Return the stable machine-readable error code for this tool error.
    pub fn code(&self) -> &'static str {
        match self {
            Self::ToolNotDiscovered { .. } => "tool_not_discovered",
            Self::ToolNotMounted { .. } => "tool_not_mounted",
            Self::ToolUnavailable { .. } => "tool_unavailable",
            Self::ToolMountForbidden { .. } => "tool_mount_forbidden",
            Self::InvalidParams(_) => "invalid_params",
            Self::InvalidArguments(_) => "invalid_arguments",
            Self::ExecutionFailed(_) => "execution_failed",
            Self::Io(_) => "io_error",
            Self::Timeout { .. } => "timeout",
            Self::Error(_) => "error",
        }
    }
}
