//! Error classification for tool failures.
//!
//! `ErrorKind` provides a coarse-grained taxonomy of tool errors so that
//! callers can decide on retry strategies, user-facing messaging, and
//! telemetry without inspecting every variant of the concrete error types.
//!
//! Error codes use the format `TE-XXX` where XXX is a three-digit zero-padded
//! number. Codes are assigned to `ToolError` variants and are stable across
//! releases.

use std::fmt;

/// Coarse error category used for retry decisions and reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The caller has been rate-limited and should back off.
    RateLimited,
    /// Authentication or authorization failure.
    Auth,
    /// A transient failure that may succeed if retried.
    Transient,
    /// A permanent failure that will not be fixed by retrying.
    Permanent,
    /// The tool was called with an invalid schema or arguments.
    ToolSchema,
    /// The operation timed out.
    Timeout,
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RateLimited => write!(f, "rate limited"),
            Self::Auth => write!(f, "authentication failure"),
            Self::Transient => write!(f, "transient error"),
            Self::Permanent => write!(f, "permanent error"),
            Self::ToolSchema => write!(f, "invalid tool schema or arguments"),
            Self::Timeout => write!(f, "operation timed out"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_kind_display() {
        assert_eq!(ErrorKind::RateLimited.to_string(), "rate limited");
        assert_eq!(ErrorKind::Auth.to_string(), "authentication failure");
        assert_eq!(ErrorKind::Transient.to_string(), "transient error");
        assert_eq!(ErrorKind::Permanent.to_string(), "permanent error");
        assert_eq!(
            ErrorKind::ToolSchema.to_string(),
            "invalid tool schema or arguments"
        );
        assert_eq!(ErrorKind::Timeout.to_string(), "operation timed out");
    }
}
