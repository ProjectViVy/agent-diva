//! Unified token usage type used across providers, sessions, and events.

use serde::{Deserialize, Serialize};

/// Normalized token usage for a single LLM response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Usage {
    /// Number of tokens in the prompt (input).
    pub prompt_tokens: i64,

    /// Number of tokens in the completion (output).
    pub completion_tokens: i64,

    /// Total tokens consumed (prompt + completion).
    pub total_tokens: i64,
}

impl Usage {
    /// Create a new `Usage` from prompt and completion token counts.
    pub fn new(prompt_tokens: i64, completion_tokens: i64) -> Self {
        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens.saturating_add(completion_tokens),
        }
    }

    /// Returns true if this usage carries no token information.
    pub fn is_empty(&self) -> bool {
        self.prompt_tokens == 0 && self.completion_tokens == 0 && self.total_tokens == 0
    }

    /// Accumulate another `Usage` into this one.
    pub fn accumulate(&mut self, other: &Usage) {
        self.prompt_tokens = self.prompt_tokens.saturating_add(other.prompt_tokens);
        self.completion_tokens = self.completion_tokens.saturating_add(other.completion_tokens);
        self.total_tokens = self.total_tokens.saturating_add(other.total_tokens);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_new_computes_total() {
        let u = Usage::new(100, 50);
        assert_eq!(u.prompt_tokens, 100);
        assert_eq!(u.completion_tokens, 50);
        assert_eq!(u.total_tokens, 150);
    }

    #[test]
    fn test_usage_default_is_zero() {
        let u = Usage::default();
        assert!(u.is_empty());
    }

    #[test]
    fn test_usage_is_empty() {
        assert!(Usage::default().is_empty());
        assert!(!Usage::new(1, 0).is_empty());
        assert!(!Usage::new(0, 1).is_empty());
    }

    #[test]
    fn test_usage_accumulate() {
        let mut a = Usage::new(100, 50);
        let b = Usage::new(200, 100);
        a.accumulate(&b);
        assert_eq!(a.prompt_tokens, 300);
        assert_eq!(a.completion_tokens, 150);
        assert_eq!(a.total_tokens, 450);
    }

    #[test]
    fn test_usage_roundtrip_json() {
        let u = Usage::new(100, 50);
        let json = serde_json::to_string(&u).unwrap();
        let u2: Usage = serde_json::from_str(&json).unwrap();
        assert_eq!(u.prompt_tokens, u2.prompt_tokens);
        assert_eq!(u.completion_tokens, u2.completion_tokens);
        assert_eq!(u.total_tokens, u2.total_tokens);
    }
}
