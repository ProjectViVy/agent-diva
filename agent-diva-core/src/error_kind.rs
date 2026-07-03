//! User-facing error kind classification.
//!
//! `ErrorKind` provides a simpler, user-oriented categorization
//! that sits alongside [`ErrorCategory`] (which is designed for
//! retry/fatal/internal routing decisions). The two enums serve
//! different audiences and use cases.
//!
//! [`ErrorCategory`]: crate::error_category::ErrorCategory

use crate::error_category::ErrorCategory;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Simplified, user-facing error classification.
///
/// Unlike [`ErrorCategory`] which focuses on retry/fatal/config/internal
/// routing, `ErrorKind` describes what the *user* should know about
/// an error. It is suitable for displaying in chat messages, logs,
/// and status reports.
///
/// # Mapping to ErrorCategory
///
/// | ErrorKind     | ErrorCategory |
/// |---------------|---------------|
/// | `RateLimited` | `Retryable`   |
/// | `Auth`        | `Auth`        |
/// | `Transient`   | `Retryable`   |
/// | `Permanent`   | `Fatal`       |
/// | `ToolSchema`  | `Unknown`     |
/// | `Timeout`     | `Timeout`     |
///
/// [`ErrorCategory`]: crate::error_category::ErrorCategory
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ErrorKind {
    /// The request was rate-limited. Retry after backoff.
    RateLimited,

    /// Authentication or authorization failure (wrong key, expired token, etc.).
    Auth,

    /// A transient failure (network blip, temporary overload). Retryable.
    Transient,

    /// A permanent error that will not succeed on retry (invalid input, not found, etc.).
    Permanent,

    /// A tool returned output that does not match the expected schema.
    ToolSchema,

    /// The operation timed out.
    Timeout,
}

impl ErrorKind {
    /// Convert this `ErrorKind` to the corresponding [`ErrorCategory`].
    ///
    /// This mapping is intentionally one-way: `ErrorCategory` has more
    /// variants (Config, Unknown) that don't have a natural user-facing
    /// equivalent.
    pub fn to_category(self) -> ErrorCategory {
        match self {
            ErrorKind::RateLimited => ErrorCategory::Retryable,
            ErrorKind::Auth => ErrorCategory::Auth,
            ErrorKind::Transient => ErrorCategory::Retryable,
            ErrorKind::Permanent => ErrorCategory::Fatal,
            ErrorKind::ToolSchema => ErrorCategory::Unknown,
            ErrorKind::Timeout => ErrorCategory::Timeout,
        }
    }
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            ErrorKind::RateLimited => "rate limited",
            ErrorKind::Auth => "authentication error",
            ErrorKind::Transient => "transient error",
            ErrorKind::Permanent => "permanent error",
            ErrorKind::ToolSchema => "tool schema error",
            ErrorKind::Timeout => "timeout",
        };
        write!(f, "{s}")
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_six_variants_exist() {
        // Verify every variant is constructable
        let variants = [
            ErrorKind::RateLimited,
            ErrorKind::Auth,
            ErrorKind::Transient,
            ErrorKind::Permanent,
            ErrorKind::ToolSchema,
            ErrorKind::Timeout,
        ];
        assert_eq!(variants.len(), 6);
    }

    #[test]
    fn display_output() {
        assert_eq!(ErrorKind::RateLimited.to_string(), "rate limited");
        assert_eq!(ErrorKind::Auth.to_string(), "authentication error");
        assert_eq!(ErrorKind::Transient.to_string(), "transient error");
        assert_eq!(ErrorKind::Permanent.to_string(), "permanent error");
        assert_eq!(ErrorKind::ToolSchema.to_string(), "tool schema error");
        assert_eq!(ErrorKind::Timeout.to_string(), "timeout");
    }

    #[test]
    fn category_mapping() {
        assert_eq!(ErrorKind::RateLimited.to_category(), ErrorCategory::Retryable);
        assert_eq!(ErrorKind::Auth.to_category(), ErrorCategory::Auth);
        assert_eq!(ErrorKind::Transient.to_category(), ErrorCategory::Retryable);
        assert_eq!(ErrorKind::Permanent.to_category(), ErrorCategory::Fatal);
        assert_eq!(ErrorKind::ToolSchema.to_category(), ErrorCategory::Unknown);
        assert_eq!(ErrorKind::Timeout.to_category(), ErrorCategory::Timeout);
    }

    #[test]
    fn serialization_roundtrip() {
        let original = ErrorKind::Auth;
        let json = serde_json::to_string(&original).expect("serialize");
        let roundtripped: ErrorKind = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(original, roundtripped);
    }

    #[test]
    fn all_variants_serialize_roundtrip() {
        for kind in &[
            ErrorKind::RateLimited,
            ErrorKind::Auth,
            ErrorKind::Transient,
            ErrorKind::Permanent,
            ErrorKind::ToolSchema,
            ErrorKind::Timeout,
        ] {
            let json = serde_json::to_string(kind).expect("serialize");
            let back: ErrorKind = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(*kind, back);
        }
    }

    #[test]
    fn equality_and_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(ErrorKind::Auth);
        set.insert(ErrorKind::Auth); // duplicate
        assert_eq!(set.len(), 1);
        set.insert(ErrorKind::Timeout);
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn error_category_unchanged() {
        // Sanity check: ErrorCategory variants haven't been modified
        // by accident. If this test fails, someone changed ErrorCategory
        // and the mapping in this file may need review.
        let categories = [
            ErrorCategory::Retryable,
            ErrorCategory::Fatal,
            ErrorCategory::Config,
            ErrorCategory::Auth,
            ErrorCategory::Timeout,
            ErrorCategory::Unknown,
        ];
        assert_eq!(categories.len(), 6);
    }

    #[test]
    fn to_category_is_exhaustive() {
        // Ensure match is exhaustive — will fail to compile if not
        #[allow(unreachable_patterns)]
        let _ = match ErrorKind::RateLimited {
            ErrorKind::RateLimited => (),
            ErrorKind::Auth => (),
            ErrorKind::Transient => (),
            ErrorKind::Permanent => (),
            ErrorKind::ToolSchema => (),
            ErrorKind::Timeout => (),
        };
    }
}
