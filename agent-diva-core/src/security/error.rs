//! Security-related error types

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
        }
    }

    /// Check if this error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimitExceeded { .. } | Self::ActionBudgetExhausted
        )
    }

    /// Machine-readable error code, stable across releases.
    pub fn error_code(&self) -> &'static str {
        match self {
            SecurityError::PathNotAllowed { .. } => "SE-001",
            SecurityError::PathEscapesWorkspace { .. } => "SE-002",
            SecurityError::ForbiddenComponent { .. } => "SE-003",
            SecurityError::RateLimitExceeded { .. } => "SE-004",
            SecurityError::ActionBudgetExhausted => "SE-005",
            SecurityError::ReadOnlyMode => "SE-006",
            SecurityError::SymlinkNotAllowed { .. } => "SE-007",
            SecurityError::InvalidPathFormat { .. } => "SE-008",
            SecurityError::FileTooLarge { .. } => "SE-009",
            SecurityError::ForbiddenExtension { .. } => "SE-010",
        }
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
    fn test_is_retryable() {
        assert!(SecurityError::RateLimitExceeded { count: 1, max: 0 }.is_retryable());

        assert!(!SecurityError::PathNotAllowed {
            path: "/test".to_string()
        }
        .is_retryable());
    }

    #[test]
    fn test_error_codes() {
        use std::collections::HashSet;
        let mut codes = HashSet::new();
        let variants = [
            SecurityError::PathNotAllowed { path: "p".into() },
            SecurityError::PathEscapesWorkspace { resolved: PathBuf::from("p") },
            SecurityError::ForbiddenComponent { component: "c".into() },
            SecurityError::RateLimitExceeded { count: 1, max: 1 },
            SecurityError::ActionBudgetExhausted,
            SecurityError::ReadOnlyMode,
            SecurityError::SymlinkNotAllowed { path: PathBuf::from("p") },
            SecurityError::InvalidPathFormat { reason: "p".into() },
            SecurityError::FileTooLarge { size: 1, max_size: 2 },
            SecurityError::ForbiddenExtension { ext: "exe".into() },
        ];
        assert_eq!(variants[0].error_code(), "SE-001");
        assert_eq!(variants[5].error_code(), "SE-006");
        for v in &variants {
            assert!(codes.insert(v.error_code()), "Duplicate error_code: {}", v.error_code());
        }
        assert_eq!(codes.len(), 10);
    }
}
