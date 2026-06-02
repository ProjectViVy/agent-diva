//! ExecPolicy manager for rule-based command approval
//!
//! This module provides the ExecPolicyManager which loads, manages,
//! and evaluates command rules. Inspired by Codex CLI's execpolicy system,
//! simplified to TOML format.

use crate::decision::{Decision, Evaluation};
use crate::policy::AskForApproval;
use crate::rules::{Policy, PrefixRule};
use parking_lot::Mutex;
use std::collections::HashSet;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tracing::info;

// ============================================================================
// Banned Prefix Suggestions
// ============================================================================

/// Dangerous command prefixes that should NEVER be auto-suggested as Allow rules
///
/// These commands are either:
/// - Interpreters with inline execution (python -c, bash -c, etc.)
/// - Privilege escalation (sudo, su)
/// - Shell environments (env, zsh)
/// - Code execution tools (node -e, perl -e, ruby -e)
pub static BANNED_PREFIX_SUGGESTIONS: &[&[&str]] = &[
    // Python interpreters
    &["python3"],
    &["python3", "-"],
    &["python3", "-c"],
    &["python"],
    &["python", "-"],
    &["python", "-c"],
    // Bash/sh shells
    &["bash"],
    &["bash", "-lc"],
    &["bash", "-c"],
    &["sh"],
    &["sh", "-c"],
    &["sh", "-lc"],
    &["zsh"],
    &["/bin/bash"],
    &["/bin/sh"],
    // Privilege escalation
    &["sudo"],
    &["su"],
    &["doas"],
    // PowerShell
    &["pwsh"],
    &["pwsh", "-Command"],
    &["powershell"],
    &["powershell", "-Command"],
    // Node.js
    &["node"],
    &["node", "-e"],
    &["node", "-c"],
    // Perl
    &["perl"],
    &["perl", "-e"],
    // Ruby
    &["ruby"],
    &["ruby", "-e"],
    // Environment
    &["env"],
    // Other interpreters
    &["php"],
    &["php", "-r"],
    &["lua"],
    &["lua", "-e"],
];

/// Check if a command prefix is in the banned list
pub fn is_banned_prefix(prefix: &[String]) -> bool {
    let prefix_strs: Vec<&str> = prefix.iter().map(|s| s.as_str()).collect();
    BANNED_PREFIX_SUGGESTIONS
        .iter()
        .any(|banned| banned.to_vec() == prefix_strs)
}

// ============================================================================
// Errors
// ============================================================================

/// ExecPolicy errors
#[derive(Debug, Error)]
pub enum ExecPolicyError {
    /// Failed to load rules file
    #[error("Failed to load rules file: {0}")]
    LoadError(String),

    /// Failed to save rules file
    #[error("Failed to save rules file: {0}")]
    SaveError(String),

    /// Failed to parse TOML
    #[error("TOML parse error: {0}")]
    ParseError(String),

    /// Rule already exists
    #[error("Rule already exists for pattern: {0}")]
    DuplicateRule(String),

    /// Banned prefix suggestion
    #[error("Cannot auto-suggest rule for banned prefix: {0}")]
    BannedPrefix(String),
}

// ============================================================================
// ExecPolicyAmendment
// ============================================================================

/// Amendment for auto-learning rules from user approval
#[derive(Debug, Clone)]
pub struct ExecPolicyAmendment {
    /// Command prefix to add as a rule
    pub command: Vec<String>,
}

impl ExecPolicyAmendment {
    /// Create a new amendment
    pub fn new(command: Vec<String>) -> Self {
        Self { command }
    }

    /// Check if this amendment is valid (not banned)
    pub fn is_valid(&self) -> bool {
        !is_banned_prefix(&self.command)
    }

    /// Convert to a PrefixRule with the given decision
    pub fn to_rule(&self, decision: Decision) -> PrefixRule {
        PrefixRule::new(self.command.clone(), decision)
    }
}

// ============================================================================
// ExecPolicyManager
// ============================================================================

/// Manager for ExecPolicy rules
///
/// Provides:
/// - Rule loading from TOML files
/// - Rule evaluation for commands
/// - Rule amendment (auto-learning from approvals)
pub struct ExecPolicyManager {
    /// Current policy (atomic for concurrent reads)
    policy: Arc<Policy>,

    /// Update lock for rule modifications
    update_lock: Mutex<()>,

    /// Rules file path (for persistence)
    rules_path: Option<PathBuf>,
}

impl ExecPolicyManager {
    /// Create a new manager with empty policy
    pub fn new() -> Self {
        Self {
            policy: Arc::new(Policy::empty()),
            update_lock: Mutex::new(()),
            rules_path: None,
        }
    }

    /// Create a manager with a pre-loaded policy
    pub fn with_policy(policy: Policy) -> Self {
        Self {
            policy: Arc::new(policy),
            update_lock: Mutex::new(()),
            rules_path: None,
        }
    }

    /// Create a manager that loads rules from a file
    pub fn from_file(path: PathBuf) -> Result<Self, ExecPolicyError> {
        let policy = Policy::load_from_file(&path)
            .map_err(|e| ExecPolicyError::LoadError(format!("{}: {}", path.display(), e)))?;

        Ok(Self {
            policy: Arc::new(policy),
            update_lock: Mutex::new(()),
            rules_path: Some(path),
        })
    }

    /// Load rules from a TOML file, merging with existing rules
    pub fn load_rules(&mut self, path: &PathBuf) -> Result<(), ExecPolicyError> {
        let new_policy = Policy::load_from_file(path)
            .map_err(|e| ExecPolicyError::LoadError(format!("{}: {}", path.display(), e)))?;

        // Merge rules (new rules are appended)
        let mut merged = (*self.policy).clone();
        for rule in new_policy.prefix_rules {
            // Skip duplicates
            if merged.find_rule_by_pattern(&rule.pattern).is_none() {
                merged.add_rule(rule);
            }
        }

        let rule_count = merged.rule_count();
        self.policy = Arc::new(merged);
        self.rules_path = Some(path.clone());

        info!("Loaded {} rules from {}", rule_count, path.display());
        Ok(())
    }

    /// Get the current policy
    pub fn policy(&self) -> &Arc<Policy> {
        &self.policy
    }

    /// Evaluate a command against the policy
    pub fn evaluate(&self, command: &[String]) -> Evaluation {
        self.policy.evaluate(command)
    }

    /// Evaluate multiple commands and aggregate decisions
    pub fn evaluate_multiple(&self, commands: &[Vec<String>]) -> Evaluation {
        self.policy.evaluate_multiple(commands)
    }

    /// Check if a command has an explicit Allow rule
    pub fn has_allow_rule(&self, command: &[String]) -> bool {
        let matches = self.policy.matches_for_command(command);
        matches.iter().any(|m| m.decision == Decision::Allow)
    }

    /// Get approval requirement for a command
    pub fn get_approval_requirement(
        &self,
        command: &[String],
        approval_policy: AskForApproval,
    ) -> ApprovalRequirement {
        let eval = self.evaluate(command);

        match eval.decision {
            Decision::Forbidden => ApprovalRequirement::Forbidden {
                reason: "Command forbidden by policy rules".to_string(),
            },
            Decision::Prompt => {
                // Check if approval policy allows prompting
                if approval_policy == AskForApproval::Never {
                    ApprovalRequirement::Forbidden {
                        reason: "Prompt rule requires approval, but approval_policy=Never"
                            .to_string(),
                    }
                } else {
                    ApprovalRequirement::NeedsApproval {
                        reason: "Command requires approval per policy".to_string(),
                        amendment: Some(ExecPolicyAmendment::new(command.to_vec())),
                    }
                }
            }
            Decision::Allow => {
                // Check if this was an explicit rule match
                if eval.has_matches {
                    ApprovalRequirement::Skip {
                        bypass_sandbox: false, // Allow rules don't bypass sandbox by default
                        amendment: None,
                    }
                } else {
                    // No explicit rule, use approval policy defaults
                    match approval_policy {
                        AskForApproval::Never => ApprovalRequirement::Skip {
                            bypass_sandbox: false,
                            amendment: None,
                        },
                        AskForApproval::OnFailure => ApprovalRequirement::Skip {
                            bypass_sandbox: false,
                            amendment: None,
                        },
                        AskForApproval::OnRequest | AskForApproval::UnlessTrusted => {
                            ApprovalRequirement::NeedsApproval {
                                reason: "Untrusted command, no matching Allow rule".to_string(),
                                amendment: Some(ExecPolicyAmendment::new(command.to_vec())),
                            }
                        }
                    }
                }
            }
        }
    }

    /// Append an amendment to the rules file and update policy
    pub fn append_amendment(
        &mut self,
        amendment: &ExecPolicyAmendment,
    ) -> Result<(), ExecPolicyError> {
        // Validate amendment
        if !amendment.is_valid() {
            return Err(ExecPolicyError::BannedPrefix(amendment.command.join(" ")));
        }

        // Check for duplicate
        if self
            .policy
            .find_rule_by_pattern(&amendment.command)
            .is_some()
        {
            return Err(ExecPolicyError::DuplicateRule(amendment.command.join(" ")));
        }

        // Lock for update
        let _guard = self.update_lock.lock();

        // Create the rule
        let rule = amendment.to_rule(Decision::Allow);

        // Append to file if path is set
        if let Some(path) = &self.rules_path {
            blocking_append_rule_to_file(path, &rule)?;
        }

        // Update in-memory policy
        let mut new_policy = (*self.policy).clone();
        new_policy.add_rule(rule);
        self.policy = Arc::new(new_policy);

        info!("Added rule: {} -> Allow", amendment.command.join(" "));
        Ok(())
    }

    /// Get the rules file path
    pub fn rules_path(&self) -> Option<&PathBuf> {
        self.rules_path.as_ref()
    }
}

impl Default for ExecPolicyManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Approval Requirement
// ============================================================================

/// Approval requirement for command execution
#[derive(Debug, Clone)]
pub enum ApprovalRequirement {
    /// Skip approval (command is allowed)
    Skip {
        /// Whether to bypass sandbox
        bypass_sandbox: bool,
        /// Optional amendment to suggest
        amendment: Option<ExecPolicyAmendment>,
    },

    /// Need user approval
    NeedsApproval {
        /// Reason for needing approval
        reason: String,
        /// Optional amendment to suggest if approved
        amendment: Option<ExecPolicyAmendment>,
    },

    /// Forbidden (cannot execute)
    Forbidden {
        /// Reason for forbidding
        reason: String,
    },
}

impl ApprovalRequirement {
    /// Check if approval is needed
    pub fn needs_approval(&self) -> bool {
        matches!(self, ApprovalRequirement::NeedsApproval { .. })
    }

    /// Check if execution is forbidden
    pub fn is_forbidden(&self) -> bool {
        matches!(self, ApprovalRequirement::Forbidden { .. })
    }

    /// Check if approval can be skipped
    pub fn can_skip(&self) -> bool {
        matches!(self, ApprovalRequirement::Skip { .. })
    }

    /// Get bypass_sandbox flag
    pub fn bypass_sandbox(&self) -> bool {
        match self {
            ApprovalRequirement::Skip { bypass_sandbox, .. } => *bypass_sandbox,
            _ => false,
        }
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Blockingly append a rule to a TOML file (with file locking)
fn blocking_append_rule_to_file(path: &Path, rule: &PrefixRule) -> Result<(), ExecPolicyError> {
    // Open file for appending
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .append(true)
        .open(path)
        .map_err(|e| ExecPolicyError::SaveError(e.to_string()))?;

    // Lock the file (Unix file lock)
    #[cfg(unix)]
    {
        use std::os::unix::fs::FileExt;
        file.lock_exclusive()
            .map_err(|e| ExecPolicyError::SaveError(format!("File lock error: {}", e)))?;
    }

    // Read existing content to check for duplicates
    // We need to read before writing, so we'll read the whole file first
    let existing_content = std::fs::read_to_string(path).unwrap_or_default();
    let existing_patterns: HashSet<Vec<String>> = existing_content
        .lines()
        .filter(|line| line.contains("pattern"))
        .filter_map(extract_pattern_from_line)
        .collect();

    // Check if pattern already exists
    if existing_patterns.contains(&rule.pattern) {
        #[cfg(unix)]
        file.unlock().ok();
        return Err(ExecPolicyError::DuplicateRule(rule.pattern.join(" ")));
    }

    // Format the rule as TOML
    let rule_toml = format!(
        "\n[[prefix_rules]]\npattern = [{}]\ndecision = \"{}\"\n",
        rule.pattern
            .iter()
            .map(|p| format!("\"{}\"", p))
            .collect::<Vec<_>>()
            .join(", "),
        rule.decision
    );

    // Write to file
    file.write_all(rule_toml.as_bytes())
        .map_err(|e| ExecPolicyError::SaveError(e.to_string()))?;

    // Unlock file
    #[cfg(unix)]
    file.unlock().ok();

    Ok(())
}

/// Extract pattern from a TOML line like: pattern = ["git", "status"]
fn extract_pattern_from_line(line: &str) -> Option<Vec<String>> {
    if !line.contains("pattern") || !line.contains("=") {
        return None;
    }

    // Find the array part
    let start = line.find('[')?;
    let end = line.rfind(']')?;
    let array_str = &line[start + 1..end];

    // Parse the quoted strings
    array_str
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .into()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_banned_prefix_detection() {
        assert!(is_banned_prefix(&["python3".to_string(), "-c".to_string()]));
        assert!(is_banned_prefix(&["bash".to_string()]));
        assert!(is_banned_prefix(&["sudo".to_string()]));
        assert!(is_banned_prefix(&["node".to_string(), "-e".to_string()]));

        // Safe commands are not banned
        assert!(!is_banned_prefix(&[
            "git".to_string(),
            "status".to_string()
        ]));
        assert!(!is_banned_prefix(&[
            "npm".to_string(),
            "install".to_string()
        ]));
        assert!(!is_banned_prefix(&[
            "cargo".to_string(),
            "build".to_string()
        ]));
    }

    #[test]
    fn test_amendment_creation() {
        let amendment = ExecPolicyAmendment::new(vec!["git".to_string(), "status".to_string()]);
        assert!(amendment.is_valid());

        let banned_amendment =
            ExecPolicyAmendment::new(vec!["python3".to_string(), "-c".to_string()]);
        assert!(!banned_amendment.is_valid());
    }

    #[test]
    fn test_amendment_to_rule() {
        let amendment = ExecPolicyAmendment::new(vec!["npm".to_string(), "install".to_string()]);
        let rule = amendment.to_rule(Decision::Allow);
        assert_eq!(rule.pattern, vec!["npm".to_string(), "install".to_string()]);
        assert_eq!(rule.decision, Decision::Allow);
    }

    #[test]
    fn test_exec_policy_manager_creation() {
        let manager = ExecPolicyManager::new();
        assert!(manager.policy().is_empty());
    }

    #[test]
    fn test_exec_policy_manager_with_policy() {
        let policy = Policy::from_rules(vec![PrefixRule::new(
            vec!["git".to_string()],
            Decision::Allow,
        )]);
        let manager = ExecPolicyManager::with_policy(policy);
        assert_eq!(manager.policy().rule_count(), 1);
    }

    #[test]
    fn test_evaluate_command() {
        let policy = Policy::from_rules(vec![PrefixRule::new(
            vec!["git".to_string(), "status".to_string()],
            Decision::Allow,
        )]);
        let manager = ExecPolicyManager::with_policy(policy);

        let eval = manager.evaluate(&["git".to_string(), "status".to_string()]);
        assert_eq!(eval.decision, Decision::Allow);
        assert!(eval.has_matches);

        let eval = manager.evaluate(&["npm".to_string(), "install".to_string()]);
        assert!(!eval.has_matches);
    }

    #[test]
    fn test_approval_requirement_allowed() {
        let policy = Policy::from_rules(vec![PrefixRule::new(
            vec!["git".to_string()],
            Decision::Allow,
        )]);
        let manager = ExecPolicyManager::with_policy(policy);

        let req = manager.get_approval_requirement(
            &["git".to_string(), "status".to_string()],
            AskForApproval::OnFailure,
        );
        assert!(req.can_skip());
        assert!(!req.bypass_sandbox());
    }

    #[test]
    fn test_approval_requirement_forbidden() {
        let policy = Policy::from_rules(vec![PrefixRule::new(
            vec!["rm".to_string(), "-rf".to_string()],
            Decision::Forbidden,
        )]);
        let manager = ExecPolicyManager::with_policy(policy);

        let req = manager.get_approval_requirement(
            &["rm".to_string(), "-rf".to_string(), "/".to_string()],
            AskForApproval::OnFailure,
        );
        assert!(req.is_forbidden());
    }

    #[test]
    fn test_approval_requirement_prompt_with_never_policy() {
        let policy = Policy::from_rules(vec![PrefixRule::new(
            vec!["git".to_string(), "checkout".to_string()],
            Decision::Prompt,
        )]);
        let manager = ExecPolicyManager::with_policy(policy);

        // With Never approval policy, Prompt becomes Forbidden
        let req = manager.get_approval_requirement(
            &[
                "git".to_string(),
                "checkout".to_string(),
                "main".to_string(),
            ],
            AskForApproval::Never,
        );
        assert!(req.is_forbidden());
    }

    #[test]
    fn test_approval_requirement_untrusted_command() {
        let manager = ExecPolicyManager::new();

        // No rules, OnRequest policy should need approval
        let req = manager.get_approval_requirement(
            &["some-random-command".to_string()],
            AskForApproval::OnRequest,
        );
        assert!(req.needs_approval());
    }

    #[test]
    fn test_append_amendment_banned() {
        let mut manager = ExecPolicyManager::new();
        let amendment = ExecPolicyAmendment::new(vec!["python3".to_string(), "-c".to_string()]);

        let result = manager.append_amendment(&amendment);
        assert!(result.is_err());
        assert!(matches!(result, Err(ExecPolicyError::BannedPrefix(_))));
    }

    #[test]
    fn test_append_amendment_duplicate() {
        let policy = Policy::from_rules(vec![PrefixRule::new(
            vec!["git".to_string()],
            Decision::Allow,
        )]);
        let mut manager = ExecPolicyManager::with_policy(policy);

        let amendment = ExecPolicyAmendment::new(vec!["git".to_string()]);
        let result = manager.append_amendment(&amendment);
        assert!(result.is_err());
        assert!(matches!(result, Err(ExecPolicyError::DuplicateRule(_))));
    }
}
