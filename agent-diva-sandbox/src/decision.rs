//! Decision type for ExecPolicy rules
//!
//! This module defines the Decision enum used by the rule-based
//! command approval system. Inspired by Codex CLI's execpolicy.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Decision outcome for a command rule evaluation
///
/// This enum represents the three possible outcomes when evaluating
/// a command against the ExecPolicy rules:
/// - `Allow`: Command can execute without approval
/// - `Prompt`: Command requires user approval before execution
/// - `Forbidden`: Command is explicitly blocked
///
/// The enum implements `Ord` to allow aggregating multiple decisions
/// where the highest (most restrictive) value wins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    /// Command is allowed to execute without approval
    Allow,

    /// Command requires user approval before execution
    /// When approval_policy is "never", this becomes Forbidden
    #[default]
    Prompt,

    /// Command is explicitly forbidden, cannot be executed
    Forbidden,
}

impl Decision {
    /// Check if this decision allows execution
    pub fn allows_execution(&self) -> bool {
        matches!(self, Decision::Allow)
    }

    /// Check if this decision requires approval
    pub fn requires_approval(&self) -> bool {
        matches!(self, Decision::Prompt)
    }

    /// Check if this decision blocks execution
    pub fn blocks_execution(&self) -> bool {
        matches!(self, Decision::Forbidden)
    }

    /// Aggregate multiple decisions, taking the most restrictive
    pub fn aggregate(decisions: &[Decision]) -> Decision {
        decisions.iter().max().copied().unwrap_or(Decision::Prompt)
    }
}

impl FromStr for Decision {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "allow" => Ok(Decision::Allow),
            "prompt" => Ok(Decision::Prompt),
            "forbidden" => Ok(Decision::Forbidden),
            _ => Err(format!("Invalid decision: {}", s)),
        }
    }
}

impl std::fmt::Display for Decision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Decision::Allow => write!(f, "allow"),
            Decision::Prompt => write!(f, "prompt"),
            Decision::Forbidden => write!(f, "forbidden"),
        }
    }
}

/// Rule match result containing the matched rule and decision
#[derive(Debug, Clone)]
pub struct RuleMatch {
    /// The matched rule's decision
    pub decision: Decision,
    /// Whether this was an exact prefix match
    pub is_exact_match: bool,
    /// The pattern that matched (for debugging)
    pub matched_pattern: Vec<String>,
}

impl RuleMatch {
    /// Create a new rule match
    pub fn new(decision: Decision, is_exact_match: bool, matched_pattern: Vec<String>) -> Self {
        Self {
            decision,
            is_exact_match,
            matched_pattern,
        }
    }

    /// Get the decision from this match
    pub fn decision(&self) -> Decision {
        self.decision
    }
}

/// Evaluation result for a command against policy rules
#[derive(Debug, Clone)]
pub struct Evaluation {
    /// Final aggregated decision
    pub decision: Decision,
    /// All rule matches that contributed to this decision
    pub matches: Vec<RuleMatch>,
    /// Whether any rules matched
    pub has_matches: bool,
}

impl Evaluation {
    /// Create an evaluation from matches
    pub fn from_matches(matches: Vec<RuleMatch>) -> Self {
        let decisions: Vec<Decision> = matches.iter().map(|m| m.decision).collect();
        let decision = Decision::aggregate(&decisions);
        let has_matches = !matches.is_empty();

        Self {
            decision,
            matches,
            has_matches,
        }
    }

    /// Create an empty evaluation with default decision
    pub fn empty() -> Self {
        Self {
            decision: Decision::Prompt,
            matches: Vec::new(),
            has_matches: false,
        }
    }

    /// Create an evaluation with a heuristic decision (no rule match)
    pub fn heuristic(decision: Decision) -> Self {
        Self {
            decision,
            matches: Vec::new(),
            has_matches: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decision_ordering() {
        assert!(Decision::Forbidden > Decision::Prompt);
        assert!(Decision::Prompt > Decision::Allow);
        assert!(Decision::Forbidden > Decision::Allow);
    }

    #[test]
    fn test_decision_aggregate() {
        let decisions = [Decision::Allow, Decision::Prompt, Decision::Allow];
        assert_eq!(Decision::aggregate(&decisions), Decision::Prompt);

        let decisions = [Decision::Allow, Decision::Allow];
        assert_eq!(Decision::aggregate(&decisions), Decision::Allow);

        let decisions = [Decision::Forbidden, Decision::Allow];
        assert_eq!(Decision::aggregate(&decisions), Decision::Forbidden);
    }

    #[test]
    fn test_decision_parse() {
        assert_eq!(Decision::from_str("allow"), Ok(Decision::Allow));
        assert_eq!(Decision::from_str("Allow"), Ok(Decision::Allow));
        assert_eq!(Decision::from_str("prompt"), Ok(Decision::Prompt));
        assert_eq!(Decision::from_str("forbidden"), Ok(Decision::Forbidden));
        assert!(Decision::from_str("invalid").is_err());
    }

    #[test]
    fn test_decision_helpers() {
        assert!(Decision::Allow.allows_execution());
        assert!(!Decision::Allow.requires_approval());
        assert!(!Decision::Allow.blocks_execution());

        assert!(!Decision::Prompt.allows_execution());
        assert!(Decision::Prompt.requires_approval());
        assert!(!Decision::Prompt.blocks_execution());

        assert!(!Decision::Forbidden.allows_execution());
        assert!(!Decision::Forbidden.requires_approval());
        assert!(Decision::Forbidden.blocks_execution());
    }

    #[test]
    fn test_rule_match() {
        let rule_match = RuleMatch::new(
            Decision::Allow,
            true,
            vec!["git".to_string(), "status".to_string()],
        );
        assert_eq!(rule_match.decision(), Decision::Allow);
        assert!(rule_match.is_exact_match);
    }

    #[test]
    fn test_evaluation() {
        let matches = vec![
            RuleMatch::new(Decision::Allow, true, vec!["git".to_string()]),
            RuleMatch::new(
                Decision::Prompt,
                false,
                vec!["git".to_string(), "checkout".to_string()],
            ),
        ];
        let eval = Evaluation::from_matches(matches);
        assert_eq!(eval.decision, Decision::Prompt);
        assert!(eval.has_matches);
    }

    #[test]
    fn test_evaluation_empty() {
        let eval = Evaluation::empty();
        assert_eq!(eval.decision, Decision::Prompt);
        assert!(!eval.has_matches);
    }

    #[test]
    fn test_serialization() {
        let decision = Decision::Allow;
        let json = serde_json::to_string(&decision).unwrap();
        assert_eq!(json, "\"allow\"");

        let parsed: Decision = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, Decision::Allow);
    }
}
