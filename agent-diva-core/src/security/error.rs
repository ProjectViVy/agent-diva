//! Security-related error types

use crate::audit::PiiSeverity;
use crate::security::injection::InjectionKind;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during security policy enforcement
#[derive(Error, Debug, Clone)]
pub enum SecurityError {
    /// Path is not allowed by security policy
    #[error("Path not allowed: {path}")]
    PathNotAllowed { path: String },

    /// Resolved path escapes allowed workspace
    #[error("Path escapes workspace: {resolved:?}")]
    PathEscapesWorkspace { resolved: PathBuf },

    /// Path contains forbidden component (e.g., parent dir)
    #[error("Path contains forbidden component: {component}")]
    ForbiddenComponent { component: String },

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {count} actions in the last hour (max: {max})")]
    RateLimitExceeded { count: usize, max: u32 },

    /// Action budget exhausted
    #[error("Action budget exhausted")]
    ActionBudgetExhausted,

    /// Read-only mode
    #[error("Read-only mode: write operations are not allowed")]
    ReadOnlyMode,

    /// Path is a symbolic link (when not allowed)
    #[error("Symbolic links are not allowed: {path}")]
    SymlinkNotAllowed { path: PathBuf },

    /// Invalid path format
    #[error("Invalid path format: {reason}")]
    InvalidPathFormat { reason: String },

    /// File too large
    #[error("File too large: {size} bytes (max: {max_size})")]
    FileTooLarge { size: u64, max_size: u64 },

    /// Forbidden file extension
    #[error("Forbidden file extension: {ext}")]
    ForbiddenExtension { ext: String },

    /// PII detected at error severity
    #[error("PII detected: {count} item(s) of severity {severity:?}")]
    PiiDetected { count: usize, severity: PiiSeverity },

    /// Prompt injection detected
    #[error("Injection detected: {kind:?} with confidence {confidence}")]
    InjectionDetected {
        kind: InjectionKind,
        confidence: f32,
    },
}

impl SecurityError {
    /// Get a user-friendly error message
    pub fn user_message(&self) -> String {
        match self {
            Self::PathNotAllowed { path } => {
                format!("Access to '{}' is not allowed by security policy", path)
            }
            Self::PathEscapesWorkspace { .. } => {
                "The specified path is outside the allowed workspace".to_string()
            }
            Self::ForbiddenComponent { component } => {
                format!("Path contains forbidden component: {}", component)
            }
            Self::RateLimitExceeded { count, max } => {
                format!(
                    "Too many file operations ({} in the last hour, max: {}). Please try again later.",
                    count, max
                )
            }
            Self::ActionBudgetExhausted => {
                "Action budget exhausted. Please try again later.".to_string()
            }
            Self::ReadOnlyMode => "Write operations are disabled in read-only mode".to_string(),
            Self::SymlinkNotAllowed { path } => {
                format!("Symbolic links are not allowed: {}", path.display())
            }
            Self::InvalidPathFormat { reason } => {
                format!("Invalid path: {}", reason)
            }
            Self::FileTooLarge { size, max_size } => {
                format!("File too large ({} bytes, max: {} bytes)", size, max_size)
            }
            Self::ForbiddenExtension { ext } => {
                format!("Files with extension '{}' are not allowed", ext)
            }
            Self::PiiDetected { count, severity } => {
                format!("PII detected: {} item(s) of severity {:?}", count, severity)
            }
            Self::InjectionDetected { kind, confidence } => {
                format!(
                    "Prompt injection detected: {:?} (confidence: {:.2})",
                    kind, confidence
                )
            }
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimitExceeded { .. } | Self::ActionBudgetExhausted
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_messages() {
        let err = SecurityError::PathNotAllowed {
            path: "/etc/passwd".to_string(),
        };
        assert!(err.user_message().contains("not allowed"));

        let err = SecurityError::RateLimitExceeded {
            count: 150,
            max: 100,
        };
        assert!(err.user_message().contains("Too many"));
    }

    #[test]
    fn test_injection_error_message() {
        let err = SecurityError::InjectionDetected {
            kind: InjectionKind::RoleOverride,
            confidence: 0.9,
        };
        assert!(err.user_message().contains("injection"));
        assert!(err.user_message().contains("RoleOverride"));
    }

    #[test]
    fn test_is_retryable() {
        assert!(SecurityError::RateLimitExceeded { count: 1, max: 0 }.is_retryable());

        assert!(!SecurityError::PathNotAllowed {
            path: "/test".to_string()
        }
        .is_retryable());

        assert!(!SecurityError::InjectionDetected {
            kind: InjectionKind::Jailbreak,
            confidence: 0.85,
        }
        .is_retryable());
    }
}
