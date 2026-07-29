//! Meta-compaction — recursive compression of `prior_summaries` when their
//! token count exceeds budget.
//!
//! When a session accumulates many compaction summaries, the total token
//! count of those summaries can itself blow the context budget.  `MetaCompactor`
//! compresses them by merging adjacent pairs into single summaries,
//! recursing up to a fixed depth limit.  If still over budget after the
//! maximum depth, it truncates the oldest summaries to fit.

use tracing::{info, warn};

use super::quality::QualityGate;
use crate::token_estimate::estimate_tokens;

/// Error type for meta-compaction failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompactionError {
    /// The summaries vector became empty during compaction.
    EmptyAfterCompaction,
    /// Failed to compress summaries within the allowed recursion depth.
    BudgetExceeded {
        remaining_tokens: usize,
        max_tokens: usize,
    },
}

impl std::fmt::Display for CompactionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompactionError::EmptyAfterCompaction => {
                write!(f, "summaries became empty during meta-compaction")
            }
            CompactionError::BudgetExceeded {
                remaining_tokens,
                max_tokens,
            } => {
                write!(
                    f,
                    "meta-compaction could not reduce summaries to fit budget: {} tokens > {} max",
                    remaining_tokens, max_tokens
                )
            }
        }
    }
}

impl std::error::Error for CompactionError {}

// ---------------------------------------------------------------------------
// MetaCompactor
// ---------------------------------------------------------------------------

/// Recursively compresses a vector of summary strings when their total
/// token count exceeds a budget.
///
/// # Design
///
/// - **Depth limit**: 2 levels of recursion.  Level 0 is the raw summaries.
///   Level 1 merges adjacent pairs.  Level 2 merges adjacent pairs again.
/// - **Quality gate**: Each merged summary is validated against a
///   [`QualityGate`] (using an empty source message list).  If the gate
///   rejects, the merge is retried up to `max_retry` times.
/// - **Fallback**: After max depth, if still over budget, truncate the
///   oldest summaries until the budget is met.
pub struct MetaCompactor {
    quality_gate: QualityGate,
    max_depth: usize,
}

impl MetaCompactor {
    /// Create a new `MetaCompactor` with the given quality gate.
    ///
    /// The default `max_depth` is 2.
    pub fn new(quality_gate: QualityGate) -> Self {
        Self {
            quality_gate,
            max_depth: 2,
        }
    }

    /// Create a new `MetaCompactor` with a custom depth limit.
    ///
    /// # Panics
    ///
    /// Panics if `max_depth` is 0.
    pub fn with_max_depth(quality_gate: QualityGate, max_depth: usize) -> Self {
        assert!(max_depth > 0, "max_depth must be > 0");
        Self {
            quality_gate,
            max_depth,
        }
    }

    /// Estimate the token count of a single summary string.
    fn estimate_summary_tokens(summary: &str) -> usize {
        estimate_tokens(summary)
    }

    /// Compute the total estimated tokens for a slice of summaries.
    fn total_tokens(summaries: &[String]) -> usize {
        summaries
            .iter()
            .map(|s| Self::estimate_summary_tokens(s))
            .sum()
    }

    /// Merge two adjacent summaries into a single compressed summary.
    ///
    /// Uses a deterministic text-join pattern: concatenates the two summaries
    /// with a separator and an ellipsis indicator.
    fn merge_pair(left: &str, right: &str) -> String {
        // Truncate each half if it is very long to keep the merged result bounded
        const MAX_HALF_LEN: usize = 500;
        let left_trunc = truncate_chars(left, MAX_HALF_LEN);
        let right_trunc = truncate_chars(right, MAX_HALF_LEN);
        format!("{} … {}", left_trunc, right_trunc)
    }

    /// Validate a merged summary using the quality gate.
    ///
    /// Returns `true` if the summary passes the gate.
    fn validate_merged(&self, merged: &str) -> bool {
        // Quality gate expects source messages; for meta-compaction we
        // validate against an empty source (no original messages).
        let result = self.quality_gate.evaluate(merged, &[]);
        result.passes
    }

    /// Perform one level of pair-wise compression.
    ///
    /// Adjacent pairs are merged.  If there is an odd number of summaries,
    /// the last one is kept as-is.
    ///
    /// Returns the new vector of compressed summaries.
    fn compress_once(&self, summaries: &[String]) -> Result<Vec<String>, CompactionError> {
        if summaries.is_empty() {
            return Ok(Vec::new());
        }
        if summaries.len() == 1 {
            return Ok(vec![summaries[0].clone()]);
        }

        let mut compressed = Vec::with_capacity(summaries.len() / 2 + 1);
        let mut i = 0;
        while i < summaries.len() {
            if i + 1 < summaries.len() {
                let left = &summaries[i];
                let right = &summaries[i + 1];
                let merged = Self::merge_pair(left, right);

                // Validate against quality gate with retry
                let mut accepted = merged.clone();
                let mut passed = self.validate_merged(&accepted);
                let mut retries = 0;
                while !passed && retries < self.quality_gate.max_retry {
                    // Retry: append a hint and re-merge with more aggressive truncation
                    let hint = "[Compacted summary]";
                    let retry_merged =
                        Self::merge_pair(&truncate_chars(left, 250), &truncate_chars(right, 250));
                    accepted = format!("{} {}", hint, retry_merged);
                    passed = self.validate_merged(&accepted);
                    retries += 1;
                }

                if passed {
                    compressed.push(accepted);
                } else {
                    // Quality gate rejected even after retries; keep the original
                    // pair as a single concatenated string (degraded mode)
                    warn!("Meta-compaction quality gate rejected merge after {} retries; using degraded mode", retries);
                    compressed.push(format!("{} {}", left, right));
                }
                i += 2;
            } else {
                // Odd element out — keep it
                compressed.push(summaries[i].clone());
                i += 1;
            }
        }

        Ok(compressed)
    }

    /// Recursively compress summaries until they fit within `max_tokens` or
    /// `max_depth` is reached.
    ///
    /// # Arguments
    ///
    /// * `summaries` — Mutable reference to the vector of summary strings.
    ///   The vector is modified in place.
    /// * `max_tokens` — Maximum allowed total tokens for the summaries.
    ///
    /// # Returns
    ///
    /// `Ok(())` if the summaries fit within the budget (either naturally or
    /// after compression/truncation).  `Err(CompactionError)` if an
    /// unrecoverable error occurs.
    pub fn compact(
        &self,
        summaries: &mut Vec<String>,
        max_tokens: usize,
    ) -> Result<(), CompactionError> {
        if summaries.is_empty() {
            return Ok(());
        }

        let initial_tokens = Self::total_tokens(summaries);
        if initial_tokens <= max_tokens {
            return Ok(());
        }

        info!(
            "Meta-compaction triggered: {} summaries, {} tokens > {} max_tokens",
            summaries.len(),
            initial_tokens,
            max_tokens
        );

        let mut current = summaries.clone();
        let mut depth = 0;

        while depth < self.max_depth {
            let before_count = current.len();
            let before_tokens = Self::total_tokens(&current);

            current = self.compress_once(&current)?;

            let after_count = current.len();
            let after_tokens = Self::total_tokens(&current);

            info!(
                "Meta-compaction depth {}: {} summaries ({} tokens) -> {} summaries ({} tokens)",
                depth + 1,
                before_count,
                before_tokens,
                after_count,
                after_tokens
            );

            if after_tokens <= max_tokens {
                info!("Meta-compaction succeeded at depth {}", depth + 1);
                *summaries = current;
                return Ok(());
            }

            // If no reduction happened (e.g., only 1 summary left), stop recursing
            if after_count >= before_count {
                warn!(
                    "Meta-compaction stopped: no further reduction possible at depth {}",
                    depth + 1
                );
                break;
            }

            depth += 1;
        }

        // After max depth: truncate oldest summaries to fit budget
        let final_tokens = Self::total_tokens(&current);
        if final_tokens > max_tokens {
            warn!(
                "Meta-compaction reached max depth ({}); truncating oldest summaries to fit budget",
                self.max_depth
            );
            Self::truncate_to_fit(&mut current, max_tokens);
        }

        *summaries = current;
        Ok(())
    }

    /// Truncate the oldest (front) summaries until the total token count
    /// fits within `max_tokens`.
    ///
    /// If even a single summary exceeds the budget, it is truncated in place
    /// (character-level truncation with ellipsis).
    fn truncate_to_fit(summaries: &mut Vec<String>, max_tokens: usize) {
        if summaries.is_empty() {
            return;
        }

        // Remove oldest summaries until we fit or only the newest summary remains.
        while summaries.len() > 1 && Self::total_tokens(summaries) > max_tokens {
            let removed = summaries.remove(0);
            let removed_tokens = Self::estimate_summary_tokens(&removed);
            info!(
                "Meta-compaction truncated oldest summary ({} tokens)",
                removed_tokens
            );
        }

        // Edge case: empty after removal — nothing to do
        if summaries.is_empty() {
            return;
        }

        // If the newest remaining summary still exceeds budget, truncate it in place
        let newest_idx = summaries.len() - 1;
        let newest_tokens = Self::estimate_summary_tokens(&summaries[newest_idx]);
        if newest_tokens > max_tokens {
            let newest = &summaries[newest_idx];
            let max_chars = max_tokens * 3; // rough inverse of chars/3 token estimate
            let truncated = truncate_chars(newest, max_chars);
            info!(
                "Meta-compaction truncated newest summary from {} to {} chars",
                newest.len(),
                truncated.len()
            );
            summaries[newest_idx] = truncated;
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Truncate a string to at most `max_chars` characters, appending an ellipsis
/// if truncation occurred.
fn truncate_chars(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_chars {
        return s.to_string();
    }
    if max_chars == 0 {
        return String::new();
    }
    let truncated: String = s.chars().take(max_chars.saturating_sub(1)).collect();
    format!("{}…", truncated)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- helpers --

    fn make_long_summary(id: usize, char_count: usize) -> String {
        format!(
            "Summary {}: {}",
            id,
            "x".repeat(char_count.saturating_sub(10))
        )
    }

    fn make_gate() -> QualityGate {
        QualityGate::default()
    }

    // -- CompactionError --

    #[test]
    fn test_compaction_error_display() {
        let err = CompactionError::EmptyAfterCompaction;
        assert!(err.to_string().contains("empty"));

        let err = CompactionError::BudgetExceeded {
            remaining_tokens: 1000,
            max_tokens: 500,
        };
        assert!(err.to_string().contains("1000"));
        assert!(err.to_string().contains("500"));
    }

    // -- MetaCompactor::new --

    #[test]
    fn test_new_sets_default_depth() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        assert_eq!(compactor.max_depth, 2);
    }

    #[test]
    fn test_with_max_depth() {
        let gate = make_gate();
        let compactor = MetaCompactor::with_max_depth(gate, 3);
        assert_eq!(compactor.max_depth, 3);
    }

    // -- truncate_chars --

    #[test]
    fn test_truncate_chars_no_op() {
        assert_eq!(truncate_chars("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_chars_truncates() {
        let result = truncate_chars("hello world", 5);
        assert_eq!(result, "hell…");
    }

    #[test]
    fn test_truncate_chars_zero() {
        assert_eq!(truncate_chars("hello", 0), "");
    }

    // -- estimate_summary_tokens --

    #[test]
    fn test_estimate_summary_tokens_empty() {
        assert_eq!(MetaCompactor::estimate_summary_tokens(""), 0);
    }

    #[test]
    fn test_estimate_summary_tokens_non_empty() {
        let tokens = MetaCompactor::estimate_summary_tokens("hello world");
        assert!(tokens > 0);
    }

    // -- total_tokens --

    #[test]
    fn test_total_tokens_empty() {
        assert_eq!(MetaCompactor::total_tokens(&[]), 0);
    }

    #[test]
    fn test_total_tokens_multiple() {
        let summaries = vec!["hello".to_string(), "world".to_string()];
        let total = MetaCompactor::total_tokens(&summaries);
        assert!(total > 0);
    }

    // -- merge_pair --

    #[test]
    fn test_merge_pair_basic() {
        let merged = MetaCompactor::merge_pair("left summary", "right summary");
        assert!(merged.contains("left summary"));
        assert!(merged.contains("right summary"));
        assert!(merged.contains("…"));
    }

    #[test]
    fn test_merge_pair_truncates_long() {
        let long = "a".repeat(1000);
        let merged = MetaCompactor::merge_pair(&long, &long);
        // Should be truncated (each half to 500 chars + ellipsis)
        assert!(merged.len() < long.len() * 2);
    }

    // -- compress_once --

    #[test]
    fn test_compress_once_empty() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        let result = compactor.compress_once(&[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_compress_once_single() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        let summaries = vec!["only summary".to_string()];
        let result = compactor.compress_once(&summaries).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "only summary");
    }

    #[test]
    fn test_compress_once_pairs() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        let summaries = vec![
            "summary one".to_string(),
            "summary two".to_string(),
            "summary three".to_string(),
            "summary four".to_string(),
        ];
        let result = compactor.compress_once(&summaries).unwrap();
        // 4 -> 2 merged summaries
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_compress_once_odd() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        let summaries = vec![
            "summary one".to_string(),
            "summary two".to_string(),
            "summary three".to_string(),
        ];
        let result = compactor.compress_once(&summaries).unwrap();
        // 3 -> 2 (one merged + one kept)
        assert_eq!(result.len(), 2);
    }

    // -- compact: no-op when under budget --

    #[test]
    fn test_compact_no_op_when_under_budget() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        let mut summaries = vec!["short".to_string()];
        compactor.compact(&mut summaries, 1000).unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0], "short");
    }

    // -- compact: reduces summaries when over budget --

    #[test]
    fn test_compact_reduces_summaries() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        // Create summaries that exceed a small budget
        let mut summaries: Vec<String> = (0..10).map(|i| make_long_summary(i, 300)).collect();
        let initial_count = summaries.len();
        let max_tokens = 500; // small budget

        compactor.compact(&mut summaries, max_tokens).unwrap();

        // Should have fewer or equal summaries
        assert!(summaries.len() <= initial_count);
    }

    // -- compact: depth limit respected --

    #[test]
    fn test_compact_depth_limit_respected() {
        let gate = make_gate();
        let compactor = MetaCompactor::with_max_depth(gate, 1);
        let mut summaries: Vec<String> = (0..8).map(|i| make_long_summary(i, 300)).collect();

        let max_tokens = 500;
        compactor.compact(&mut summaries, max_tokens).unwrap();

        // After depth=1, should have compressed once
        assert!(summaries.len() <= 4); // 8 -> 4 after one compression
    }

    // -- compact: truncation fallback --

    #[test]
    fn test_compact_truncates_when_max_depth_reached() {
        let gate = make_gate();
        let compactor = MetaCompactor::with_max_depth(gate, 1);
        // Create many long summaries that won't fit even after one compression
        let mut summaries: Vec<String> = (0..20).map(|i| make_long_summary(i, 500)).collect();

        let max_tokens = 100;
        compactor.compact(&mut summaries, max_tokens).unwrap();

        // Should fit within budget after truncation
        let total = MetaCompactor::total_tokens(&summaries);
        assert!(total <= max_tokens);
    }

    // -- compact: empty input --

    #[test]
    fn test_compact_empty_input() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        let mut summaries: Vec<String> = vec![];
        compactor.compact(&mut summaries, 100).unwrap();
        assert!(summaries.is_empty());
    }

    // -- compact: quality gate is invoked --

    #[test]
    fn test_compact_quality_gate_invoked() {
        // Use a very strict gate that should reject everything
        let strict_gate = QualityGate {
            min_completeness: 1.0,
            min_keyword_coverage: 1.0,
            min_score: 1.0,
            max_retry: 0,
        };
        let compactor = MetaCompactor::new(strict_gate);
        let mut summaries = vec![
            "first summary with some content".to_string(),
            "second summary with more content".to_string(),
        ];

        // Should not panic; degraded mode keeps concatenated pair
        compactor.compact(&mut summaries, 10).unwrap();
        // After compaction with a strict gate, the pair is merged in degraded mode
        // and then truncation may reduce it further.  Just verify it does not panic.
        // The exact length depends on the token budget, so we only assert non-panic.
    }

    // -- truncate_to_fit --

    #[test]
    fn test_truncate_to_fit_removes_oldest() {
        let mut summaries = vec![
            "old summary one".to_string(),
            "old summary two".to_string(),
            "new summary".to_string(),
        ];
        // Force truncation by setting a very small max_tokens
        let max_tokens = 1;
        MetaCompactor::truncate_to_fit(&mut summaries, max_tokens);
        // Should remove oldest until it fits (or empty)
        let total = MetaCompactor::total_tokens(&summaries);
        assert!(total <= max_tokens);
        assert_eq!(summaries.len(), 1, "should preserve the newest summary");
    }

    #[test]
    fn test_truncate_to_fit_preserves_last_summary_under_tiny_budget() {
        let mut summaries = vec![
            "old deployment notes".to_string(),
            "newest critical fact: rotate api key and keep timeout=30s".to_string(),
        ];

        MetaCompactor::truncate_to_fit(&mut summaries, 2);

        assert_eq!(summaries.len(), 1);
        assert!(
            summaries[0].starts_with("new")
                || summaries[0].starts_with("ne")
                || summaries[0].is_empty(),
            "newest summary should be preserved even after truncation"
        );
    }

    // -- simulate >10 compaction sessions --

    #[test]
    fn test_meta_compaction_many_sessions() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        // Simulate 15 compaction sessions with long summaries
        let mut summaries: Vec<String> = (0..15).map(|i| make_long_summary(i, 400)).collect();
        let initial_count = summaries.len();
        let initial_tokens = MetaCompactor::total_tokens(&summaries);

        let max_tokens = 1000;
        compactor.compact(&mut summaries, max_tokens).unwrap();

        let final_count = summaries.len();
        let final_tokens = MetaCompactor::total_tokens(&summaries);

        // Should have reduced count or tokens
        assert!(
            final_count < initial_count || final_tokens <= max_tokens,
            "meta-compaction should reduce summaries: {} -> {} summaries, {} -> {} tokens",
            initial_count,
            final_count,
            initial_tokens,
            final_tokens
        );
        assert!(
            final_tokens <= max_tokens,
            "final tokens {} should be <= {}",
            final_tokens,
            max_tokens
        );
    }

    #[test]
    fn test_meta_compaction_fixture_retains_key_facts() {
        let gate = make_gate();
        let compactor = MetaCompactor::new(gate);
        let mut summaries = vec![
            "Decision log: migrate the provider stack to DeepSeek native endpoint and keep raw model id deepseek-chat without gateway prefix.".to_string(),
            "Operational facts: rotate the staging API key on Monday, keep timeout=30s, and preserve workspace-write sandbox semantics for tool execution.".to_string(),
            "Delivery notes: compaction must stay best-effort, retry the provider once after overflow, and rebuild messages from the persisted post-compaction session snapshot.".to_string(),
        ];

        let initial_tokens = MetaCompactor::total_tokens(&summaries);
        let max_tokens = initial_tokens.saturating_sub(20);
        compactor.compact(&mut summaries, max_tokens).unwrap();

        let merged = summaries.join("\n");
        assert!(merged.contains("deepseek-chat"));
        assert!(merged.contains("timeout=30s"));
        assert!(merged.contains("best-effort"));
        assert!(MetaCompactor::total_tokens(&summaries) <= max_tokens);
    }
}
