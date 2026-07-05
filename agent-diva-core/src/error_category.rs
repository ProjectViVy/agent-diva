//! Error classification trait for cross-crate error categorization.
//!
//! Each domain error type can be classified into a standard `ErrorCategory`,
//! enabling callers to decide retry/backoff/fatal behavior without matching
//! on concrete error variants.

/// Broad classification of error categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    Retryable,
    Fatal,
    Config,
    Auth,
    Timeout,
    Unknown,
}

/// Trait for classifying domain errors into broad categories.
pub trait CategorizeError {
    fn category(&self) -> ErrorCategory;

    fn is_retryable(&self) -> bool {
        matches!(
            self.category(),
            ErrorCategory::Retryable | ErrorCategory::Timeout
        )
    }
}

// --- Impls for types owned by agent-diva-core ---

use crate::error::Error;

impl CategorizeError for Error {
    fn category(&self) -> ErrorCategory {
        match self {
            Error::Config(_) => ErrorCategory::Config,
            Error::Io(err) => {
                if matches!(err.kind(), std::io::ErrorKind::TimedOut) {
                    ErrorCategory::Timeout
                } else {
                    ErrorCategory::Retryable
                }
            }
            Error::Unauthorized(_) => ErrorCategory::Auth,
            Error::Session(_) | Error::Channel(_) | Error::Provider(_) => ErrorCategory::Retryable,
            Error::NotFound(_) | Error::Validation(_) => ErrorCategory::Fatal,
            Error::Serialization(_) | Error::Tool(_) | Error::Internal(_) => ErrorCategory::Unknown,
        }
    }
}

use crate::security::error::SecurityError;

impl CategorizeError for SecurityError {
    fn category(&self) -> ErrorCategory {
        match self {
            SecurityError::RateLimitExceeded { .. } | SecurityError::ActionBudgetExhausted => {
                ErrorCategory::Retryable
            }
            SecurityError::InjectionDetected { .. }
            | SecurityError::PiiDetected { .. }
            | SecurityError::ReadOnlyMode
            | SecurityError::PathNotAllowed { .. }
            | SecurityError::PathEscapesWorkspace { .. }
            | SecurityError::ForbiddenComponent { .. }
            | SecurityError::SymlinkNotAllowed { .. }
            | SecurityError::InvalidPathFormat { .. }
            | SecurityError::FileTooLarge { .. }
            | SecurityError::ForbiddenExtension { .. } => ErrorCategory::Fatal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::injection::InjectionKind;

    #[test]
    fn error_category_core_config() {
        let err = Error::Config("bad key".into());
        assert_eq!(err.category(), ErrorCategory::Config);
        assert!(!err.is_retryable());
    }

    #[test]
    fn error_category_core_io() {
        let err = Error::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe"));
        assert_eq!(err.category(), ErrorCategory::Retryable);
        assert!(err.is_retryable());
    }

    #[test]
    fn error_category_core_unauthorized() {
        let err = Error::Unauthorized("forbidden".into());
        assert_eq!(err.category(), ErrorCategory::Auth);
    }

    #[test]
    fn error_category_core_session_is_retryable() {
        let err = Error::Session("busy".into());
        assert_eq!(err.category(), ErrorCategory::Retryable);
    }

    #[test]
    fn error_category_core_provider_is_retryable() {
        let err = Error::Provider("overloaded".into());
        assert_eq!(err.category(), ErrorCategory::Retryable);
    }

    #[test]
    fn error_category_core_not_found_is_fatal() {
        let err = Error::NotFound("gone".into());
        assert_eq!(err.category(), ErrorCategory::Fatal);
    }

    #[test]
    fn error_category_core_internal_is_unknown() {
        let err = Error::Internal("bug".into());
        assert_eq!(err.category(), ErrorCategory::Unknown);
    }

    #[test]
    fn error_category_security_injection() {
        let err = SecurityError::InjectionDetected {
            kind: InjectionKind::Jailbreak,
            confidence: 0.95,
        };
        assert_eq!(err.category(), ErrorCategory::Fatal);
        assert!(!err.is_retryable());
    }

    #[test]
    fn error_category_security_pii() {
        let err = SecurityError::PiiDetected {
            count: 3,
            severity: crate::audit::PiiSeverity::Error,
        };
        assert_eq!(err.category(), ErrorCategory::Fatal);
    }

    #[test]
    fn error_category_security_rate_limit() {
        let err = SecurityError::RateLimitExceeded { count: 10, max: 5 };
        assert_eq!(err.category(), ErrorCategory::Retryable);
        assert!(err.is_retryable());
    }

    #[test]
    fn error_category_security_path_not_allowed() {
        let err = SecurityError::PathNotAllowed {
            path: "/etc/passwd".into(),
        };
        assert_eq!(err.category(), ErrorCategory::Fatal);
    }

    #[test]
    fn error_category_security_read_only() {
        assert_eq!(SecurityError::ReadOnlyMode.category(), ErrorCategory::Fatal);
    }

    #[test]
    fn unknown_fallback_for_uncategorized() {
        let err = Error::Serialization("bad json".into());
        assert_eq!(err.category(), ErrorCategory::Unknown);
    }

    #[test]
    fn default_is_retryable_matches_retryable() {
        let err = Error::Io(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe"));
        assert!(err.is_retryable());

        let err = Error::Config("x".into());
        assert!(!err.is_retryable());
    }

    #[test]
    fn timeout_category_is_retryable() {
        let err = Error::Io(std::io::Error::new(std::io::ErrorKind::TimedOut, "slow"));
        assert_eq!(err.category(), ErrorCategory::Timeout);
        assert!(err.is_retryable());
    }

    #[test]
    fn security_policy_denials_are_fatal() {
        assert_eq!(SecurityError::ReadOnlyMode.category(), ErrorCategory::Fatal);
        assert_eq!(
            SecurityError::PathNotAllowed {
                path: "/etc/passwd".into()
            }
            .category(),
            ErrorCategory::Fatal
        );
    }
}
