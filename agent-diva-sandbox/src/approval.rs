//! Approval system for sandbox execution
//!
//! Types for caching user approval decisions and determining approval requirements.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Shared approval store handle reused across runtime components.
pub type SharedApprovalStore = Arc<Mutex<ApprovalStore>>;

/// User's decision for command approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewDecision {
    /// Command denied, do not execute
    Denied,
    /// Approved for this single execution only
    ApprovedOnce,
    /// Approved for the entire session, no future prompts needed
    ApprovedForSession,
}

impl ReviewDecision {
    /// Check if this decision allows execution
    pub fn allows_execution(&self) -> bool {
        matches!(
            self,
            ReviewDecision::ApprovedOnce | ReviewDecision::ApprovedForSession
        )
    }

    /// Check if this decision persists for the session
    pub fn is_session_approval(&self) -> bool {
        matches!(self, ReviewDecision::ApprovedForSession)
    }
}

/// Key for approval cache lookup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct CommandApprovalKey {
    /// The command to be executed
    pub command: String,
    /// Working directory for the command
    pub cwd: PathBuf,
}

impl CommandApprovalKey {
    /// Create a new approval key
    pub fn new(command: String, cwd: PathBuf) -> Self {
        Self { command, cwd }
    }

    /// Generate a string key for HashMap storage
    pub fn to_cache_key(&self) -> String {
        // Serialize to JSON string as cache key
        serde_json::to_string(self)
            .unwrap_or_else(|_| format!("{}:{}", self.command, self.cwd.display()))
    }
}

/// Execution approval requirement determined by policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecApprovalRequirement {
    /// No approval needed, can skip directly to execution
    Skip {
        /// Whether to bypass sandbox on first attempt
        bypass_sandbox: bool,
    },
    /// Approval is needed before execution
    NeedsApproval {
        /// Reason for requiring approval
        reason: Option<String>,
    },
    /// Execution is forbidden, cannot proceed
    Forbidden {
        /// Reason for forbidden status
        reason: String,
    },
}

impl ExecApprovalRequirement {
    /// Create a skip requirement
    pub fn skip() -> Self {
        ExecApprovalRequirement::Skip {
            bypass_sandbox: false,
        }
    }

    /// Create a skip requirement that bypasses sandbox
    pub fn skip_bypass_sandbox() -> Self {
        ExecApprovalRequirement::Skip {
            bypass_sandbox: true,
        }
    }

    /// Create a needs approval requirement
    pub fn needs_approval(reason: Option<String>) -> Self {
        ExecApprovalRequirement::NeedsApproval { reason }
    }

    /// Create a forbidden requirement
    pub fn forbidden(reason: String) -> Self {
        ExecApprovalRequirement::Forbidden { reason }
    }

    /// Check if execution is allowed
    pub fn is_allowed(&self) -> bool {
        matches!(
            self,
            ExecApprovalRequirement::Skip { .. } | ExecApprovalRequirement::NeedsApproval { .. }
        )
    }

    /// Check if approval is needed
    pub fn requires_approval(&self) -> bool {
        matches!(self, ExecApprovalRequirement::NeedsApproval { .. })
    }

    /// Check if execution is forbidden
    pub fn is_forbidden(&self) -> bool {
        matches!(self, ExecApprovalRequirement::Forbidden { .. })
    }

    /// Check if sandbox should be bypassed on first attempt
    pub fn bypass_sandbox(&self) -> bool {
        match self {
            ExecApprovalRequirement::Skip { bypass_sandbox } => *bypass_sandbox,
            _ => false,
        }
    }
}

/// Approval cache store.
///
/// Stores user approval decisions for commands to avoid repeated prompts.
#[derive(Debug, Default)]
pub struct ApprovalStore {
    /// Cache of approval decisions by command key
    cache: HashMap<String, ReviewDecision>,
}

impl ApprovalStore {
    /// Create a new empty approval store
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new shared approval store wrapped in Arc<Mutex>
    pub fn new_shared() -> SharedApprovalStore {
        Arc::new(Mutex::new(Self::new()))
    }

    /// Get a cached approval decision
    pub fn get(&self, key: &CommandApprovalKey) -> Option<ReviewDecision> {
        self.cache.get(&key.to_cache_key()).copied()
    }

    /// Store an approval decision
    pub fn put(&mut self, key: CommandApprovalKey, decision: ReviewDecision) {
        self.cache.insert(key.to_cache_key(), decision);
    }

    /// Check if a command is approved for the session
    pub fn is_approved_for_session(&self, key: &CommandApprovalKey) -> bool {
        self.get(key)
            .map(|d| d.is_session_approval())
            .unwrap_or(false)
    }

    /// Check if a command was denied
    pub fn is_denied(&self, key: &CommandApprovalKey) -> bool {
        self.get(key)
            .map(|d| matches!(d, ReviewDecision::Denied))
            .unwrap_or(false)
    }

    /// Check if we have any cached decision for this command
    pub fn has_decision(&self, key: &CommandApprovalKey) -> bool {
        self.cache.contains_key(&key.to_cache_key())
    }

    /// Remove a cached decision
    pub fn remove(&mut self, key: &CommandApprovalKey) -> Option<ReviewDecision> {
        self.cache.remove(&key.to_cache_key())
    }

    /// Clear all cached decisions
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Clear all cached decisions and return how many entries were removed.
    pub fn clear_and_count(&mut self) -> usize {
        let count = self.cache.len();
        self.cache.clear();
        count
    }

    /// Get number of cached decisions
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get all session-approved commands
    pub fn session_approved_commands(&self) -> Vec<CommandApprovalKey> {
        self.cache
            .iter()
            .filter(|(_, d)| d.is_session_approval())
            .filter_map(|(k, _)| serde_json::from_str(k).ok())
            .collect()
    }

    /// Get all denied commands
    pub fn denied_commands(&self) -> Vec<CommandApprovalKey> {
        self.cache
            .iter()
            .filter(|(_, d)| matches!(d, ReviewDecision::Denied))
            .filter_map(|(k, _)| serde_json::from_str(k).ok())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_review_decision() {
        assert!(ReviewDecision::ApprovedOnce.allows_execution());
        assert!(ReviewDecision::ApprovedForSession.allows_execution());
        assert!(!ReviewDecision::Denied.allows_execution());

        assert!(ReviewDecision::ApprovedForSession.is_session_approval());
        assert!(!ReviewDecision::ApprovedOnce.is_session_approval());
    }

    #[test]
    fn test_command_approval_key() {
        let key1 = CommandApprovalKey::new("ls -la".to_string(), PathBuf::from("/workspace"));
        let key2 = CommandApprovalKey::new("ls -la".to_string(), PathBuf::from("/workspace"));
        let key3 = CommandApprovalKey::new("ls -la".to_string(), PathBuf::from("/other"));

        assert_eq!(key1, key2);
        assert_ne!(key1, key3);
        assert_eq!(key1.to_cache_key(), key2.to_cache_key());
    }

    #[test]
    fn test_approval_store_basic() {
        let store = ApprovalStore::new();
        let key = CommandApprovalKey::new("cargo build".to_string(), PathBuf::from("/workspace"));

        assert!(!store.has_decision(&key));
        assert!(!store.is_approved_for_session(&key));
        assert!(!store.is_denied(&key));
    }

    #[test]
    fn test_approval_store_put_get() {
        let mut store = ApprovalStore::new();
        let key = CommandApprovalKey::new("cargo build".to_string(), PathBuf::from("/workspace"));

        store.put(key.clone(), ReviewDecision::ApprovedForSession);

        assert!(store.has_decision(&key));
        assert!(store.is_approved_for_session(&key));
        assert!(!store.is_denied(&key));

        let decision = store.get(&key);
        assert!(decision.is_some());
        assert_eq!(decision.unwrap(), ReviewDecision::ApprovedForSession);
    }

    #[test]
    fn test_approval_store_denied() {
        let mut store = ApprovalStore::new();
        let key = CommandApprovalKey::new("rm -rf /".to_string(), PathBuf::from("/workspace"));

        store.put(key.clone(), ReviewDecision::Denied);

        assert!(store.has_decision(&key));
        assert!(!store.is_approved_for_session(&key));
        assert!(store.is_denied(&key));
    }

    #[test]
    fn test_approval_store_clear() {
        let mut store = ApprovalStore::new();
        let key = CommandApprovalKey::new("cargo build".to_string(), PathBuf::from("/workspace"));

        store.put(key, ReviewDecision::ApprovedForSession);
        assert!(!store.is_empty());

        store.clear();
        assert!(store.is_empty());
    }

    #[test]
    fn test_exec_approval_requirement() {
        let skip = ExecApprovalRequirement::skip();
        assert!(skip.is_allowed());
        assert!(!skip.requires_approval());
        assert!(!skip.is_forbidden());
        assert!(!skip.bypass_sandbox());

        let skip_bypass = ExecApprovalRequirement::skip_bypass_sandbox();
        assert!(skip_bypass.bypass_sandbox());

        let needs = ExecApprovalRequirement::needs_approval(Some("Dangerous command".to_string()));
        assert!(needs.is_allowed());
        assert!(needs.requires_approval());
        assert!(!needs.is_forbidden());

        let forbidden = ExecApprovalRequirement::forbidden("System file access".to_string());
        assert!(!forbidden.is_allowed());
        assert!(!forbidden.requires_approval());
        assert!(forbidden.is_forbidden());
    }
}
