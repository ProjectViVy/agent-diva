//! Quality gating for LLM-generated summaries.
//!
//! The [`SummaryQualityGate`] evaluates a summary against the original messages
//! by computing keyword coverage — the fraction of extracted keywords from the
//! original messages that appear in the summary. If coverage falls below a
//! configurable threshold, the gate reports a failure with details.
//!
//! # No external NLP dependencies
//!
//! Keyword extraction uses simple string splitting and filtering. No
//! NLP/ML crates or tokenizers are required.

use agent_diva_providers::Message;

use crate::summary_compaction::Summary;

/// The result of a single quality evaluation pass.
#[derive(Debug, Clone)]
pub struct QualityResult {
    /// Whether the summary passed the quality gate.
    pub passed: bool,
    /// Overall quality score (0.0 – 1.0). Currently equal to
    /// `keyword_coverage`; reserved for future multi-metric expansion.
    pub score: f64,
    /// Fraction of extracted keywords that appear in the summary.
    pub keyword_coverage: f64,
    /// Human-readable reason when the summary did not pass, if any.
    pub failure_reason: Option<String>,
}

/// A quality gate that validates LLM-generated summaries by checking keyword
/// coverage against the original conversation messages.
///
/// # Example
///
/// ```rust,ignore
/// let gate = SummaryQualityGate::new(0.6, 2);
/// let result = gate.evaluate(&summary, &messages);
/// if !result.passed {
///     eprintln!("Quality rejected: {}", result.failure_reason.unwrap());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct SummaryQualityGate {
    /// Minimum required keyword coverage (0.0 – 1.0).
    keyword_coverage_threshold: f64,
    /// Maximum number of re-generation attempts allowed by the engine.
    max_retries: u32,
}

impl Default for SummaryQualityGate {
    fn default() -> Self {
        Self {
            keyword_coverage_threshold: 0.6,
            max_retries: 2,
        }
    }
}

impl SummaryQualityGate {
    /// Create a new quality gate with the given threshold and retry limit.
    ///
    /// * `keyword_coverage_threshold` — fraction [0.0, 1.0]; summaries below
    ///   this value will be rejected.
    /// * `max_retries` — how many times the engine may re-generate.
    pub fn new(keyword_coverage_threshold: f64, max_retries: u32) -> Self {
        Self {
            keyword_coverage_threshold,
            max_retries,
        }
    }

    /// Return the configured keyword coverage threshold.
    pub fn threshold(&self) -> f64 {
        self.keyword_coverage_threshold
    }

    /// Return the configured maximum number of retries.
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Evaluate the quality of a summary against the original conversation
    /// messages using keyword coverage heuristics.
    ///
    /// Keywords are extracted from the original messages by:
    ///
    /// 1. Concatenating all message text via `to_text_lossy()`.
    /// 2. Splitting on whitespace.
    /// 3. Keeping unique terms longer than 3 characters.
    ///
    /// Keyword coverage is then computed as the fraction of those keywords
    /// that appear (case-insensitive substring match) in the summary content.
    ///
    /// When no keywords can be extracted (e.g., all messages are empty or only
    /// contain short tokens), the gate passes by default with `score = 1.0`.
    pub fn evaluate(&self, summary: &Summary, original_messages: &[Message]) -> QualityResult {
        let keywords = extract_keywords(original_messages);

        if keywords.is_empty() {
            return QualityResult {
                passed: true,
                score: 1.0,
                keyword_coverage: 1.0,
                failure_reason: None,
            };
        }

        let summary_lower = summary.content.to_lowercase();
        let matched = keywords
            .iter()
            .filter(|kw| summary_lower.contains(&kw.to_lowercase()))
            .count();

        let keyword_coverage = matched as f64 / keywords.len() as f64;
        let passed = keyword_coverage >= self.keyword_coverage_threshold;

        QualityResult {
            passed,
            score: keyword_coverage,
            keyword_coverage,
            failure_reason: if passed {
                None
            } else {
                Some(format!(
                    "Keyword coverage {:.1}% is below threshold {:.0}% \
                     ({matched}/{} keywords matched)",
                    keyword_coverage * 100.0,
                    self.keyword_coverage_threshold * 100.0,
                    keywords.len(),
                ))
            },
        }
    }
}

/// Extract distinct keywords from a slice of messages.
///
/// The current heuristic splits all message content on whitespace, filters
/// out tokens that are at most 3 characters long, and returns the unique
/// set of remaining terms.  This deliberately avoids external NLP
/// dependencies.
fn extract_keywords(messages: &[Message]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    let mut keywords = Vec::new();

    for msg in messages {
        let text = msg.content.to_text_lossy();
        for token in text.split_whitespace() {
            let cleaned: String = token
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                .collect();

            if cleaned.len() >= 3 && seen.insert(cleaned.to_lowercase()) {
                keywords.push(cleaned);
            }
        }
    }

    keywords
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::summary_compaction::Summary;
    use chrono::Utc;

    // ── Helpers ──────────────────────────────────────────────────────────

    fn make_message(role: &str, text: &str) -> Message {
        Message {
            role: role.to_string(),
            content: agent_diva_providers::MessageContent::Text(text.to_string()),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
            thinking_blocks: None,
        }
    }

    fn make_summary(content: &str) -> Summary {
        Summary {
            id: "test-summary".to_string(),
            content: content.to_string(),
            created_at: Utc::now(),
            prev_summary_id: None,
            source_message_range: (0, 0),
            token_count: 100,
            source_summary_ids: Vec::new(),
            is_meta: false,
        }
    }

    // ── Gate defaults ────────────────────────────────────────────────────

    #[test]
    fn test_default_threshold() {
        let gate = SummaryQualityGate::default();
        assert!((gate.threshold() - 0.6).abs() < f64::EPSILON);
        assert_eq!(gate.max_retries(), 2);
    }

    #[test]
    fn test_custom_gate() {
        let gate = SummaryQualityGate::new(0.8, 5);
        assert!((gate.threshold() - 0.8).abs() < f64::EPSILON);
        assert_eq!(gate.max_retries(), 5);
    }

    // ── Evaluation: passing cases ────────────────────────────────────────

    #[test]
    fn test_evaluate_high_coverage_passes() {
        let gate = SummaryQualityGate::default();
        let messages = vec![
            make_message("user", "The quick brown fox jumps over the lazy dog"),
            make_message("assistant", "The fox is quick and the dog is lazy"),
        ];
        let summary =
            make_summary("The quick brown fox and the lazy dog were discussed");

        let result = gate.evaluate(&summary, &messages);
        assert!(result.passed);
        assert!(result.keyword_coverage >= 0.6);
        assert!(result.failure_reason.is_none());
    }

    #[test]
    fn test_evaluate_empty_messages_passes_by_default() {
        let gate = SummaryQualityGate::default();
        let messages: Vec<Message> = vec![];
        let summary = make_summary("No original messages to summarize");

        let result = gate.evaluate(&summary, &messages);
        assert!(result.passed);
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_evaluate_short_only_messages_passes_by_default() {
        let gate = SummaryQualityGate::default();
        let messages = vec![make_message("user", "a b c d")];
        let summary = make_summary("All tokens are short so no keywords");

        let result = gate.evaluate(&summary, &messages);
        assert!(result.passed);
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    // ── Evaluation: failing cases ────────────────────────────────────────

    #[test]
    fn test_evaluate_low_coverage_fails() {
        let gate = SummaryQualityGate::new(0.5, 2);
        let messages =
            vec![make_message("user", "alpha beta gamma delta epsilon")];
        // Summary mentions only one of the five keywords
        let summary = make_summary("Some unrelated text about alpha");

        let result = gate.evaluate(&summary, &messages);
        assert!(!result.passed);
        assert!(result.failure_reason.is_some());
        assert!(result.keyword_coverage < 0.5);
    }

    #[test]
    fn test_evaluate_extreme_threshold() {
        // Threshold 1.0 → must match every keyword; "detail" is absent
        // from the summary so coverage is 2/3 → fails.
        let gate = SummaryQualityGate::new(1.0, 2);
        let messages = vec![make_message("user", "important concept detail")];
        let summary = make_summary("The important concept was covered");

        let result = gate.evaluate(&summary, &messages);
        assert!(!result.passed);
        assert!((result.keyword_coverage - 2.0 / 3.0).abs() < f64::EPSILON);
    }

    // ── Keyword extraction ───────────────────────────────────────────────

    #[test]
    fn test_extract_keywords_skips_short_tokens() {
        // Words with 1–2 characters are excluded; 3+ character words
        // are included (changed from >3 to >=3 per Iteration 2).
        let messages = vec![make_message("user", "a an the for big small")];
        let keywords = extract_keywords(&messages);
        assert!(keywords.contains(&"the".to_string()));
        assert!(keywords.contains(&"for".to_string()));
        assert!(keywords.contains(&"big".to_string()));
        assert!(keywords.contains(&"small".to_string()));
        assert!(!keywords.contains(&"a".to_string()));
        assert!(!keywords.contains(&"an".to_string()));
    }

    #[test]
    fn test_extract_keywords_deduplicates() {
        let messages = vec![
            make_message("user", "apple banana apple"),
            make_message("assistant", "banana cherry"),
        ];
        let keywords = extract_keywords(&messages);
        assert_eq!(keywords.len(), 3); // apple, banana, cherry
        assert!(keywords.contains(&"apple".to_string()));
        assert!(keywords.contains(&"banana".to_string()));
        assert!(keywords.contains(&"cherry".to_string()));
    }

    #[test]
    fn test_extract_keywords_empty_messages() {
        let messages: Vec<Message> = vec![];
        let keywords = extract_keywords(&messages);
        assert!(keywords.is_empty());
    }
}
