//! Budget enforcement for token usage

use crate::error_category::{CategorizeError, ErrorCategory};
use crate::token_ledger::store::JsonlTokenLedger;
use std::path::Path;
use thiserror::Error;

/// Error returned when a session exceeds its token budget.
#[derive(Error, Debug)]
#[error("Token budget exceeded for session {session_id}: used {used} tokens, limit is {limit}")]
pub struct BudgetExceeded {
    pub session_id: String,
    pub used: u64,
    pub limit: u64,
}

impl CategorizeError for BudgetExceeded {
    fn category(&self) -> ErrorCategory {
        ErrorCategory::Fatal
    }
}

/// Check whether a session is within its token budget.
///
/// Returns `Ok(())` if the session is within budget.
/// Returns `Err(BudgetExceeded)` if cumulative usage exceeds the limit.
/// If the ledger cannot be read, returns `Err(BudgetExceeded)` with `used = 0`.
pub fn check_budget(
    ledger: &JsonlTokenLedger,
    session_id: &str,
    budget_limit: u64,
) -> Result<(), BudgetExceeded> {
    let used = ledger
        .session_total(session_id)
        .map_err(|_| BudgetExceeded {
            session_id: session_id.to_string(),
            used: 0,
            limit: budget_limit,
        })?;

    if used > budget_limit {
        return Err(BudgetExceeded {
            session_id: session_id.to_string(),
            used,
            limit: budget_limit,
        });
    }

    Ok(())
}

/// Convenience: check budget using data_root path (creates a temporary ledger handle).
pub fn check_budget_at_path(
    data_root: &Path,
    session_id: &str,
    budget_limit: u64,
) -> Result<(), BudgetExceeded> {
    let ledger = JsonlTokenLedger::new(data_root).map_err(|_| BudgetExceeded {
        session_id: session_id.to_string(),
        used: 0,
        limit: budget_limit,
    })?;
    check_budget(&ledger, session_id, budget_limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::token_ledger::store::TokenLedgerEntry;
    use tempfile::TempDir;

    fn setup() -> (TempDir, JsonlTokenLedger) {
        let dir = TempDir::new().expect("tempdir");
        let ledger = JsonlTokenLedger::new(dir.path()).expect("ledger creation");
        (dir, ledger)
    }

    #[test]
    fn test_budget_within_limit() {
        let (_dir, ledger) = setup();
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 100, 50))
            .expect("append");

        let result = check_budget(&ledger, "sess-1", 200);
        assert!(result.is_ok());
    }

    #[test]
    fn test_budget_exceeded() {
        let (_dir, ledger) = setup();
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 100, 50))
            .expect("append");
        // total_tokens = 150, limit = 100 → exceeded

        let result = check_budget(&ledger, "sess-1", 100);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.session_id, "sess-1");
        assert_eq!(err.used, 150);
        assert_eq!(err.limit, 100);
    }

    #[test]
    fn test_budget_exactly_at_limit() {
        let (_dir, ledger) = setup();
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 80, 20))
            .expect("append");
        // total_tokens = 100, limit = 100 → exactly at limit, still OK

        let result = check_budget(&ledger, "sess-1", 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_budget_exceeded_category_is_fatal() {
        use crate::error_category::CategorizeError;
        let err = BudgetExceeded {
            session_id: "sess-1".to_string(),
            used: 200,
            limit: 100,
        };
        assert_eq!(err.category(), ErrorCategory::Fatal);
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_budget_unknown_session_within_limit() {
        let (_dir, ledger) = setup();
        // No entries for this session → used = 0, always within budget
        let result = check_budget(&ledger, "unknown-sess", 100);
        assert!(result.is_ok());
    }

    #[test]
    fn test_check_budget_at_path() {
        let dir = TempDir::new().expect("tempdir");
        let ledger = JsonlTokenLedger::new(dir.path()).expect("ledger");
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 100, 50))
            .expect("append");
        drop(ledger);

        let result = check_budget_at_path(dir.path(), "sess-1", 200);
        assert!(result.is_ok());

        let result = check_budget_at_path(dir.path(), "sess-1", 100);
        assert!(result.is_err());
    }

    #[test]
    fn test_budget_multiple_entries_accumulate() {
        let (_dir, ledger) = setup();
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-a", 50, 25))
            .expect("append");
        ledger
            .append(TokenLedgerEntry::new("sess-1", "model-b", 40, 20))
            .expect("append");
        // total: 75 + 60 = 135

        let result = check_budget(&ledger, "sess-1", 140);
        assert!(result.is_ok());

        let result = check_budget(&ledger, "sess-1", 130);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().used, 135);
    }
}
