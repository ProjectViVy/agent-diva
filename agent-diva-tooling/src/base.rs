//! Base trait for tools.

use async_trait::async_trait;
use agent_diva_core::ErrorKind;
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
}

pub type Result<T> = std::result::Result<T, ToolError>;

impl ToolError {
    /// Classify this tool error into a coarse [`ErrorKind`] for retry
    /// decisions and reporting.
    pub fn error_kind(&self) -> ErrorKind {
        match self {
            Self::Error(_) | Self::ExecutionFailed(_) => ErrorKind::Permanent,
            Self::InvalidParams(_) | Self::InvalidArguments(_) => ErrorKind::ToolSchema,
            Self::Io(_) => ErrorKind::Transient,
        }
    }

    /// Whether this error is considered retryable.
    pub fn is_retryable(&self) -> bool {
        match self.error_kind() {
            ErrorKind::RateLimited | ErrorKind::Transient | ErrorKind::Timeout => true,
            ErrorKind::Auth | ErrorKind::Permanent | ErrorKind::ToolSchema => false,
        }
    }

    /// Return a stable machine-readable error code for this variant.
    /// Codes are of the form `TE-XXX` and do NOT change when `ErrorKind` or
    /// `is_retryable()` semantics change.
    pub fn error_code(&self) -> &'static str {
        match self {
            Self::Error(_) => "TE-000",
            Self::InvalidParams(_) => "TE-001",
            Self::InvalidArguments(_) => "TE-002",
            Self::ExecutionFailed(_) => "TE-003",
            Self::Io(_) => "TE-004",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_kind_mapping() {
        assert_eq!(
            ToolError::Error("test".into()).error_kind(),
            ErrorKind::Permanent
        );
        assert_eq!(
            ToolError::ExecutionFailed("test".into()).error_kind(),
            ErrorKind::Permanent
        );
        assert_eq!(
            ToolError::InvalidParams("test".into()).error_kind(),
            ErrorKind::ToolSchema
        );
        assert_eq!(
            ToolError::InvalidArguments("test".into()).error_kind(),
            ErrorKind::ToolSchema
        );
        assert_eq!(
            ToolError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")).error_kind(),
            ErrorKind::Transient
        );
    }

    #[test]
    fn is_retryable_mapping() {
        // Io → Transient → true
        assert!(ToolError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")).is_retryable());
        // Error → Permanent → false
        assert!(!ToolError::Error("test".into()).is_retryable());
        // ExecutionFailed → Permanent → false
        assert!(!ToolError::ExecutionFailed("test".into()).is_retryable());
        // InvalidParams → ToolSchema → false
        assert!(!ToolError::InvalidParams("test".into()).is_retryable());
        // InvalidArguments → ToolSchema → false
        assert!(!ToolError::InvalidArguments("test".into()).is_retryable());
    }

    #[test]
    fn error_code_mapping() {
        assert_eq!(ToolError::Error("test".into()).error_code(), "TE-000");
        assert_eq!(ToolError::InvalidParams("test".into()).error_code(), "TE-001");
        assert_eq!(ToolError::InvalidArguments("test".into()).error_code(), "TE-002");
        assert_eq!(ToolError::ExecutionFailed("test".into()).error_code(), "TE-003");
        assert_eq!(
            ToolError::Io(std::io::Error::new(std::io::ErrorKind::Other, "io")).error_code(),
            "TE-004"
        );
    }
}
