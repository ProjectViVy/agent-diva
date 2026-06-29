//! Quality gating for memory consolidation outputs.
//!
//! The [`ConsolidationQualityGate`] evaluates a consolidation output against
//! expected keywords by computing keyword coverage — the fraction of extracted
//! keywords from the source content that appear in the consolidation text. If
//! coverage falls below a configurable threshold, the gate reports a failure
//! with details.
//!
//! # No external NLP dependencies
//!
//! Keyword matching uses case-insensitive substring comparison. No NLP/ML
//! crates or tokenizers are required.

/// The result of a single quality evaluation pass.
#[derive(Debug, Clone)]
pub struct QualityResult {
    /// Whether the consolidation passed the quality gate.
    pub passed: bool,
    /// Overall quality score (0.0 – 1.0). Currently equal to
    /// `keyword_coverage`; reserved for future multi-metric expansion.
    pub score: f64,
    /// Fraction of extracted keywords that appear in the consolidation text.
    pub keyword_coverage: f64,
    /// Human-readable reason when the consolidation did not pass, if any.
    pub failure_reason: Option<String>,
}

/// A quality gate that validates memory consolidation outputs by checking
/// keyword coverage against expected keywords.
///
/// # Example
///
/// ```rust,ignore
/// let gate = ConsolidationQualityGate::new(0.6, 2);
/// let result = gate.evaluate(
///     "The user discussed project architecture and database schema.",
///     &["architecture", "database", "schema"].map(String::from),
/// );
/// if !result.passed {
///     eprintln!("Quality rejected: {}", result.failure_reason.unwrap());
/// }
/// ```
#[derive(Debug, Clone)]
pub struct ConsolidationQualityGate {
    /// Minimum required keyword coverage (0.0 – 1.0).
    threshold: f64,
    /// Maximum number of re-generation attempts allowed by the engine.
    max_retries: u32,
}

impl Default for ConsolidationQualityGate {
    fn default() -> Self {
        Self {
            threshold: 0.6,
            max_retries: 2,
        }
    }
}

impl ConsolidationQualityGate {
    /// Create a new quality gate with the given threshold and retry limit.
    ///
    /// * `threshold` — fraction [0.0, 1.0]; consolidation outputs below
    ///   this value will be rejected.
    /// * `max_retries` — how many times the engine may re-generate.
    pub fn new(threshold: f64, max_retries: u32) -> Self {
        Self {
            threshold,
            max_retries,
        }
    }

    /// Return the configured keyword coverage threshold.
    pub fn threshold(&self) -> f64 {
        self.threshold
    }

    /// Return the configured maximum number of retries.
    pub fn max_retries(&self) -> u32 {
        self.max_retries
    }

    /// Evaluate the quality of a consolidation output against the expected
    /// keywords using keyword coverage heuristics.
    ///
    /// Keyword coverage is computed as the fraction of provided keywords that
    /// appear (case-insensitive substring match) in the consolidation content.
    ///
    /// When the content is empty or no keywords are provided, the gate passes
    /// by default with `score = 1.0`.
    pub fn evaluate(&self, content: &str, keywords: &[String]) -> QualityResult {
        if keywords.is_empty() || content.trim().is_empty() {
            return QualityResult {
                passed: true,
                score: 1.0,
                keyword_coverage: 1.0,
                failure_reason: None,
            };
        }

        let content_lower = content.to_lowercase();
        let matched = keywords
            .iter()
            .filter(|kw| content_lower.contains(&kw.to_lowercase()))
            .count();

        let keyword_coverage = matched as f64 / keywords.len() as f64;
        let passed = keyword_coverage >= self.threshold;

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
                    self.threshold * 100.0,
                    keywords.len(),
                ))
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Helpers ──────────────────────────────────────────────────────────

    fn kw(words: &[&str]) -> Vec<String> {
        words.iter().map(|s| s.to_string()).collect()
    }

    // ── Gate defaults ────────────────────────────────────────────────────

    #[test]
    fn test_default_threshold() {
        let gate = ConsolidationQualityGate::default();
        assert!((gate.threshold() - 0.6).abs() < f64::EPSILON);
        assert_eq!(gate.max_retries(), 2);
    }

    #[test]
    fn test_gate_custom_threshold() {
        let gate = ConsolidationQualityGate::new(0.8, 5);
        assert!((gate.threshold() - 0.8).abs() < f64::EPSILON);
        assert_eq!(gate.max_retries(), 5);
    }

    // ── Evaluation: passing cases ────────────────────────────────────────

    #[test]
    fn test_gate_passes_high_coverage() {
        let gate = ConsolidationQualityGate::default();
        let keywords = kw(&["architecture", "database", "schema", "deployment"]);
        let content = "The system architecture uses a distributed database \
                       with a flexible schema designed for cloud deployment.";

        let result = gate.evaluate(content, &keywords);
        assert!(result.passed);
        assert!(result.keyword_coverage >= 0.6);
        assert!(result.failure_reason.is_none());
    }

    #[test]
    fn test_gate_empty_content_passes() {
        let gate = ConsolidationQualityGate::default();
        let keywords = kw(&["architecture", "database"]);

        let result = gate.evaluate("", &keywords);
        assert!(result.passed);
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_gate_empty_keywords_passes() {
        let gate = ConsolidationQualityGate::default();
        let keywords: Vec<String> = vec![];

        let result = gate.evaluate("Some consolidation content here", &keywords);
        assert!(result.passed);
        assert!((result.score - 1.0).abs() < f64::EPSILON);
    }

    // ── Evaluation: failing cases ────────────────────────────────────────

    #[test]
    fn test_gate_fails_low_coverage() {
        let gate = ConsolidationQualityGate::new(0.5, 2);
        let keywords = kw(&["alpha", "beta", "gamma", "delta", "epsilon"]);

        // Only one of five keywords appears
        let content = "Some unrelated text about alpha";

        let result = gate.evaluate(content, &keywords);
        assert!(!result.passed);
        assert!(result.failure_reason.is_some());
        assert!(result.keyword_coverage < 0.5);
    }

    #[test]
    fn test_gate_extreme_threshold() {
        // Threshold 1.0 → must match every keyword; "detail" is absent
        // so coverage is 2/3 → fails.
        let gate = ConsolidationQualityGate::new(1.0, 2);
        let keywords = kw(&["important", "concept", "detail"]);
        let content = "The important concept was covered";

        let result = gate.evaluate(content, &keywords);
        assert!(!result.passed);
        assert!((result.keyword_coverage - 2.0 / 3.0).abs() < f64::EPSILON);
    }
}
