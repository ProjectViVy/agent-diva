//! ExecPolicy rule types for command matching
//!
//! This module defines the rule types used for command approval decisions.
//! Inspired by Codex CLI's execpolicy system, simplified to TOML format.

use crate::decision::{Decision, Evaluation, RuleMatch};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Prefix rule for command matching
///
/// Matches commands by their prefix (first few arguments).
/// Example: pattern ["git", "status"] matches "git status" and "git status -s"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrefixRule {
    /// Command pattern to match (e.g., ["git", "status"])
    pub pattern: Vec<String>,

    /// Decision for matched commands
    pub decision: Decision,

    /// Optional justification for the rule (for audit/logging)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
}

impl PrefixRule {
    /// Create a new prefix rule
    pub fn new(pattern: Vec<String>, decision: Decision) -> Self {
        Self {
            pattern,
            decision,
            justification: None,
        }
    }

    /// Create with justification
    pub fn with_justification(
        pattern: Vec<String>,
        decision: Decision,
        justification: String,
    ) -> Self {
        Self {
            pattern,
            decision,
            justification: Some(justification),
        }
    }

    /// Check if this rule matches a command
    ///
    /// A rule matches if the command starts with the pattern.
    /// Example: pattern ["git"] matches ["git", "status", "-s"]
    pub fn matches(&self, command: &[String]) -> bool {
        if command.len() < self.pattern.len() {
            return false;
        }

        // Compare prefix elements
        for (rule_token, cmd_token) in self.pattern.iter().zip(command.iter()) {
            if rule_token != cmd_token {
                return false;
            }
        }

        true
    }

    /// Create a RuleMatch for this rule against a command
    pub fn create_match(&self, command: &[String]) -> Option<RuleMatch> {
        if self.matches(command) {
            Some(RuleMatch::new(
                self.decision,
                self.pattern.len() == command.len(),
                self.pattern.clone(),
            ))
        } else {
            None
        }
    }
}

/// Policy container for prefix rules
///
/// Holds a collection of rules and provides evaluation methods.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Policy {
    /// Prefix rules for command matching
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefix_rules: Vec<PrefixRule>,
}

impl Policy {
    /// Create an empty policy
    pub fn empty() -> Self {
        Self::default()
    }

    /// Create a policy with rules
    pub fn from_rules(rules: Vec<PrefixRule>) -> Self {
        Self {
            prefix_rules: rules,
        }
    }

    /// Add a rule to the policy
    pub fn add_rule(&mut self, rule: PrefixRule) {
        self.prefix_rules.push(rule);
    }

    /// Find all matching rules for a command
    pub fn matches_for_command(&self, command: &[String]) -> Vec<RuleMatch> {
        self.prefix_rules
            .iter()
            .filter_map(|rule| rule.create_match(command))
            .collect()
    }

    /// Evaluate a single command against the policy
    pub fn evaluate(&self, command: &[String]) -> Evaluation {
        let matches = self.matches_for_command(command);
        Evaluation::from_matches(matches)
    }

    /// Evaluate multiple commands and aggregate decisions
    pub fn evaluate_multiple(&self, commands: &[Vec<String>]) -> Evaluation {
        let all_matches: Vec<RuleMatch> = commands
            .iter()
            .flat_map(|cmd| self.matches_for_command(cmd))
            .collect();

        Evaluation::from_matches(all_matches)
    }

    /// Check if policy has any rules
    pub fn is_empty(&self) -> bool {
        self.prefix_rules.is_empty()
    }

    /// Get rule count
    pub fn rule_count(&self) -> usize {
        self.prefix_rules.len()
    }

    /// Find a rule by pattern (for deduplication)
    pub fn find_rule_by_pattern(&self, pattern: &[String]) -> Option<&PrefixRule> {
        self.prefix_rules.iter().find(|r| r.pattern == pattern)
    }

    /// Parse policy from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Load policy from a TOML file
    pub fn load_from_file(path: &PathBuf) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(path)?;
        Self::from_toml(&content).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("TOML parse error: {}", e),
            )
        })
    }

    /// Serialize policy to TOML string
    pub fn to_toml(&self) -> Result<String, toml::ser::Error> {
        toml::to_string_pretty(self)
    }

    /// Save policy to a TOML file
    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let content = self.to_toml().map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("TOML serialization error: {}", e),
            )
        })?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

/// ExecPolicy rules file structure
///
/// This is the root structure for a TOML rules file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesFile {
    /// Version of the rules format (for future compatibility)
    #[serde(default = "default_rules_version")]
    pub version: String,

    /// Prefix rules
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefix_rules: Vec<PrefixRule>,
}

fn default_rules_version() -> String {
    "1.0".to_string()
}

impl RulesFile {
    /// Parse from TOML string
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Convert to Policy
    pub fn to_policy(&self) -> Policy {
        Policy::from_rules(self.prefix_rules.clone())
    }
}

impl Default for RulesFile {
    fn default() -> Self {
        Self {
            version: default_rules_version(),
            prefix_rules: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prefix_rule_creation() {
        let rule = PrefixRule::new(
            vec!["git".to_string(), "status".to_string()],
            Decision::Allow,
        );
        assert_eq!(rule.pattern.len(), 2);
        assert_eq!(rule.decision, Decision::Allow);
    }

    #[test]
    fn test_prefix_rule_matching() {
        let rule = PrefixRule::new(
            vec!["git".to_string(), "status".to_string()],
            Decision::Allow,
        );

        // Exact match
        assert!(rule.matches(&["git".to_string(), "status".to_string()]));

        // Prefix match (longer command)
        assert!(rule.matches(&["git".to_string(), "status".to_string(), "-s".to_string()]));

        // No match (different command)
        assert!(!rule.matches(&["git".to_string(), "log".to_string()]));

        // No match (shorter command)
        assert!(!rule.matches(&["git".to_string()]));

        // No match (completely different)
        assert!(!rule.matches(&["npm".to_string(), "install".to_string()]));
    }

    #[test]
    fn test_prefix_rule_create_match() {
        let rule = PrefixRule::new(vec!["git".to_string()], Decision::Allow);

        let match_result = rule.create_match(&["git".to_string(), "status".to_string()]);
        assert!(match_result.is_some());
        let m = match_result.unwrap();
        assert_eq!(m.decision, Decision::Allow);
        assert!(!m.is_exact_match); // Not exact, command is longer

        let exact_match = rule.create_match(&["git".to_string()]);
        assert!(exact_match.is_some());
        assert!(exact_match.unwrap().is_exact_match);
    }

    #[test]
    fn test_policy_empty() {
        let policy = Policy::empty();
        assert!(policy.is_empty());
        assert_eq!(policy.rule_count(), 0);
    }

    #[test]
    fn test_policy_evaluation() {
        let rules = vec![
            PrefixRule::new(
                vec!["git".to_string(), "status".to_string()],
                Decision::Allow,
            ),
            PrefixRule::new(
                vec!["git".to_string(), "checkout".to_string()],
                Decision::Prompt,
            ),
            PrefixRule::new(
                vec!["rm".to_string(), "-rf".to_string()],
                Decision::Forbidden,
            ),
        ];
        let policy = Policy::from_rules(rules);

        // Evaluate "git status"
        let eval = policy.evaluate(&["git".to_string(), "status".to_string()]);
        assert_eq!(eval.decision, Decision::Allow);
        assert!(eval.has_matches);

        // Evaluate "git checkout main"
        let eval = policy.evaluate(&[
            "git".to_string(),
            "checkout".to_string(),
            "main".to_string(),
        ]);
        assert_eq!(eval.decision, Decision::Prompt);

        // Evaluate "rm -rf /"
        let eval = policy.evaluate(&["rm".to_string(), "-rf".to_string(), "/".to_string()]);
        assert_eq!(eval.decision, Decision::Forbidden);

        // Evaluate unknown command
        let eval = policy.evaluate(&["ls".to_string()]);
        assert!(!eval.has_matches);
        // Default decision for no match is Prompt
        assert_eq!(eval.decision, Decision::Prompt);
    }

    #[test]
    fn test_policy_multiple_commands() {
        let rules = vec![
            PrefixRule::new(vec!["git".to_string()], Decision::Allow),
            PrefixRule::new(vec!["rm".to_string()], Decision::Forbidden),
        ];
        let policy = Policy::from_rules(rules);

        let commands = vec![
            vec!["git".to_string(), "status".to_string()],
            vec!["rm".to_string(), "-rf".to_string()],
        ];
        let eval = policy.evaluate_multiple(&commands);
        // Forbidden > Allow, so aggregate should be Forbidden
        assert_eq!(eval.decision, Decision::Forbidden);
    }

    #[test]
    fn test_toml_serialization() {
        let policy = Policy::from_rules(vec![PrefixRule::with_justification(
            vec!["git".to_string(), "status".to_string()],
            Decision::Allow,
            "Safe git read command".to_string(),
        )]);

        let toml_str = policy.to_toml().unwrap();
        assert!(toml_str.contains("git"));
        assert!(toml_str.contains("status"));
        assert!(toml_str.contains("allow"));
    }

    #[test]
    fn test_toml_deserialization() {
        let toml_str = r#"
[[prefix_rules]]
pattern = ["git", "status"]
decision = "allow"
justification = "Safe git command"
"#;

        let policy = Policy::from_toml(toml_str).unwrap();
        assert_eq!(policy.rule_count(), 1);
        assert_eq!(
            policy.prefix_rules[0].pattern,
            vec!["git".to_string(), "status".to_string()]
        );
        assert_eq!(policy.prefix_rules[0].decision, Decision::Allow);
    }

    #[test]
    fn test_rules_file() {
        let toml_str = r#"
version = "1.0"

[[prefix_rules]]
pattern = ["npm", "install"]
decision = "allow"

[[prefix_rules]]
pattern = ["rm", "-rf"]
decision = "forbidden"
"#;

        let rules_file = RulesFile::from_toml(toml_str).unwrap();
        assert_eq!(rules_file.version, "1.0");
        assert_eq!(rules_file.prefix_rules.len(), 2);

        let policy = rules_file.to_policy();
        assert_eq!(policy.rule_count(), 2);
    }
}
