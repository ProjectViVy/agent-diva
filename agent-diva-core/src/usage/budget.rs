//! Token budget control module

use serde::{Deserialize, Serialize};

/// Token budget configuration and tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Total token limit (0 = unlimited)
    pub total_limit: i64,
    /// Tokens used so far
    pub used: i64,
    /// Warning threshold ratio (e.g., 0.8 = warn at 80% usage)
    pub warning_threshold: f64,
}

impl TokenBudget {
    /// Create an unlimited budget
    pub fn unlimited() -> Self {
        Self {
            total_limit: 0,
            used: 0,
            warning_threshold: 0.8,
        }
    }

    /// Create a budget with a specific limit
    pub fn with_limit(limit: i64) -> Self {
        Self {
            total_limit: limit,
            used: 0,
            warning_threshold: 0.8,
        }
    }

    /// Set warning threshold
    pub fn with_warning_threshold(mut self, threshold: f64) -> Self {
        self.warning_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Get remaining tokens
    pub fn remaining(&self) -> i64 {
        if self.total_limit <= 0 {
            // Return a large number for unlimited
            999_999_999
        } else {
            std::cmp::max(0, self.total_limit - self.used)
        }
    }

    /// Get usage ratio (0.0 to 1.0+)
    pub fn usage_ratio(&self) -> f64 {
        if self.total_limit <= 0 {
            0.0
        } else {
            self.used as f64 / self.total_limit as f64
        }
    }

    /// Check if budget is exceeded
    pub fn is_exceeded(&self) -> bool {
        self.total_limit > 0 && self.used >= self.total_limit
    }

    /// Check if warning threshold is reached
    pub fn should_warn(&self) -> bool {
        self.usage_ratio() >= self.warning_threshold && !self.is_exceeded()
    }

    /// Add tokens to used count
    pub fn add_usage(&mut self, tokens: i64) {
        self.used += tokens;
    }

    /// Get warning message if threshold is reached
    pub fn get_warning_message(&self) -> Option<String> {
        if self.is_exceeded() {
            Some("[TOKEN BUDGET EXCEEDED] Please wrap up immediately.".to_string())
        } else if self.should_warn() {
            Some(format!(
                "[TOKEN BUDGET WARNING] {}% used.",
                (self.usage_ratio() * 100.0) as u32
            ))
        } else {
            None
        }
    }

    /// Parse budget directive from user message
    /// Patterns: +500k, +1m, +100000
    pub fn parse_directive(text: &str) -> Option<i64> {
        // Pattern: +XXXk (thousands)
        if let Some(captures) = regex::Regex::new(r"\+(\d+)k\b")
            .ok()
            .and_then(|re| re.captures(text))
        {
            if let Some(m) = captures.get(1) {
                let num: i64 = m.as_str().parse().ok()?;
                return Some(num * 1000);
            }
        }

        // Pattern: +XXXm (millions)
        if let Some(captures) = regex::Regex::new(r"\+(\d+)m\b")
            .ok()
            .and_then(|re| re.captures(text))
        {
            if let Some(m) = captures.get(1) {
                let num: i64 = m.as_str().parse().ok()?;
                return Some(num * 1_000_000);
            }
        }

        // Pattern: +XXXXX (direct number, 4+ digits)
        if let Some(captures) = regex::Regex::new(r"\+(\d{4,})\b")
            .ok()
            .and_then(|re| re.captures(text))
        {
            if let Some(m) = captures.get(1) {
                return m.as_str().parse().ok();
            }
        }

        None
    }

    /// Reset the budget
    pub fn reset(&mut self) {
        self.used = 0;
    }

    /// Update the limit
    pub fn set_limit(&mut self, limit: i64) {
        self.total_limit = limit;
    }
}

impl Default for TokenBudget {
    fn default() -> Self {
        Self::unlimited()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unlimited_budget() {
        let budget = TokenBudget::unlimited();
        assert!(!budget.is_exceeded());
        assert!(!budget.should_warn());
        assert_eq!(budget.remaining(), 999_999_999);
    }

    #[test]
    fn test_budget_limit() {
        let budget = TokenBudget::with_limit(1000);
        assert_eq!(budget.total_limit, 1000);
        assert_eq!(budget.remaining(), 1000);
    }

    #[test]
    fn test_budget_usage() {
        let mut budget = TokenBudget::with_limit(1000);
        budget.add_usage(800);
        assert!(budget.should_warn());
        assert!(!budget.is_exceeded());
        assert_eq!(budget.remaining(), 200);
    }

    #[test]
    fn test_budget_exceeded() {
        let mut budget = TokenBudget::with_limit(1000);
        budget.add_usage(1000);
        assert!(budget.is_exceeded());
        assert!(!budget.should_warn());
        assert_eq!(budget.remaining(), 0);
    }

    #[test]
    fn test_parse_directive_k() {
        let result = TokenBudget::parse_directive("let's add +500k tokens");
        assert_eq!(result, Some(500_000));
    }

    #[test]
    fn test_parse_directive_m() {
        let result = TokenBudget::parse_directive("increase by +1m");
        assert_eq!(result, Some(1_000_000));
    }

    #[test]
    fn test_parse_directive_direct() {
        let result = TokenBudget::parse_directive("+100000 tokens");
        assert_eq!(result, Some(100_000));
    }

    #[test]
    fn test_warning_message() {
        let mut budget = TokenBudget::with_limit(1000).with_warning_threshold(0.8);
        budget.add_usage(800);
        let msg = budget.get_warning_message();
        assert!(msg.is_some());
        assert!(msg.unwrap().contains("80%"));
    }
}
