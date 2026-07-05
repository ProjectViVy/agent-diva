//! Error types for sandbox operations

use thiserror::Error;

/// Sandbox operation errors
#[derive(Debug, Error)]
pub enum SandboxError {
    /// Command was denied by sandbox policy
    #[error("Command denied by sandbox policy: {reason}")]
    Denied { reason: String },

    /// Execution requires explicit user approval before it can proceed
    #[error("Command requires approval: {reason}")]
    ApprovalRequired { reason: String },

    /// Permission denied for file system access
    #[error("Permission denied for path '{path}': {reason}")]
    PermissionDenied { path: String, reason: String },

    /// Failed to create restricted token (Windows)
    #[cfg(windows)]
    #[error("Failed to create restricted token: {0}")]
    TokenCreation(String),

    /// Failed to spawn sandboxed process
    #[error("Failed to spawn sandboxed process: {0}")]
    SpawnFailed(String),

    /// Command execution timed out
    #[error("Command timed out after {secs} seconds")]
    Timeout { secs: u64 },

    /// Command completed with a non-zero exit code
    #[error("Command failed with exit code {code}")]
    ExecutionFailed {
        code: i32,
        stdout: String,
        stderr: String,
    },

    /// Invalid command or parameters
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    /// Platform not supported for sandboxing
    #[error("Sandbox not supported on this platform")]
    PlatformNotSupported,

    /// Platform-specific error (e.g., WSL1, missing dependencies)
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Platform sandbox is unavailable and direct execution must not be used implicitly
    #[error("Sandbox unavailable on {platform}: {reason}")]
    PlatformUnavailable {
        platform: &'static str,
        reason: String,
    },

    /// Sandbox is disabled via environment variable
    #[error("Sandbox disabled by environment variable")]
    Disabled,

    /// Internal error
    #[error("Internal sandbox error: {0}")]
    Internal(String),
}

/// Convenient result type for sandbox operations
pub type SandboxResult<T> = std::result::Result<T, SandboxError>;

impl agent_diva_core::error_category::CategorizeError for SandboxError {
    fn category(&self) -> agent_diva_core::error_category::ErrorCategory {
        match self {
            Self::Timeout { .. } => agent_diva_core::error_category::ErrorCategory::Timeout,
            Self::Denied { .. } | Self::ApprovalRequired { .. } | Self::PermissionDenied { .. } => {
                agent_diva_core::error_category::ErrorCategory::Auth
            }
            Self::ExecutionFailed { .. } | Self::SpawnFailed(_) => {
                agent_diva_core::error_category::ErrorCategory::Retryable
            }
            Self::InvalidCommand(_) => agent_diva_core::error_category::ErrorCategory::Fatal,
            Self::PlatformUnavailable { .. } | Self::Disabled | Self::PlatformNotSupported => {
                agent_diva_core::error_category::ErrorCategory::Config
            }
            Self::PlatformError(_) | Self::Internal(_) => {
                agent_diva_core::error_category::ErrorCategory::Unknown
            }
            #[cfg(windows)]
            Self::TokenCreation(_) => agent_diva_core::error_category::ErrorCategory::Fatal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::error_category::{CategorizeError, ErrorCategory};

    #[test]
    fn sandbox_timeout_is_retryable() {
        let err = SandboxError::Timeout { secs: 2 };
        assert_eq!(err.category(), ErrorCategory::Timeout);
        assert!(err.is_retryable());
    }

    #[test]
    fn sandbox_platform_unavailable_is_config() {
        let err = SandboxError::PlatformUnavailable {
            platform: "linux",
            reason: "disabled".to_string(),
        };
        assert_eq!(err.category(), ErrorCategory::Config);
    }

    #[test]
    fn sandbox_spawn_failure_is_retryable() {
        let err = SandboxError::SpawnFailed("busy".to_string());
        assert_eq!(err.category(), ErrorCategory::Retryable);
        assert!(err.is_retryable());
    }
}
