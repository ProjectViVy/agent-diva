//! Guardian - Automatic approval integration for ToolOrchestrator
//!
//! This module provides:
//! - GuardianReviewer trait for automatic approval decisions
//! - GuardianRejectionCircuitBreaker for safety limits
//! - Integration with ToolOrchestrator for auto-approval flow
//!
//! Inspired by Codex CLI's Guardian system for safe auto-approval.

use crate::approval::{CommandApprovalKey, ReviewDecision};
use crate::exec_policy::{ApprovalRequirement, ExecPolicyManager};
use crate::policy::AskForApproval;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

// ============================================================================
// GuardianConfig
// ============================================================================

/// Configuration for Guardian behavior
#[derive(Debug, Clone)]
pub struct GuardianConfig {
    /// Maximum consecutive rejections before circuit breaker triggers
    pub max_consecutive_rejections: usize,

    /// Time window for counting rejections (seconds)
    pub rejection_window_secs: u64,

    /// Whether to auto-approve commands matching existing Allow rules
    pub auto_approve_known_safe: bool,

    /// Whether to auto-approve read-only commands
    pub auto_approve_read_only: bool,

    /// Minimum command execution time before allowing auto-approve (ms)
    /// Commands that run very quickly may be auto-approved more liberally
    pub min_execution_time_for_approval_ms: u64,

    /// Whether to learn from approvals (create Allow rules)
    pub enable_auto_learning: bool,
}

impl Default for GuardianConfig {
    fn default() -> Self {
        Self {
            max_consecutive_rejections: 5,
            rejection_window_secs: 60,
            auto_approve_known_safe: false,
            auto_approve_read_only: false,
            min_execution_time_for_approval_ms: 100,
            enable_auto_learning: false,
        }
    }
}

impl GuardianConfig {
    /// Create a strict configuration (no auto-approve)
    pub fn strict() -> Self {
        Self {
            max_consecutive_rejections: 3,
            rejection_window_secs: 30,
            auto_approve_known_safe: false,
            auto_approve_read_only: false,
            min_execution_time_for_approval_ms: 1000,
            enable_auto_learning: false,
        }
    }

    /// Create a liberal configuration (more auto-approve)
    pub fn liberal() -> Self {
        Self {
            max_consecutive_rejections: 10,
            rejection_window_secs: 120,
            auto_approve_known_safe: true,
            auto_approve_read_only: true,
            min_execution_time_for_approval_ms: 50,
            enable_auto_learning: true,
        }
    }
}

// ============================================================================
// GuardianDecision
// ============================================================================

/// Decision returned by GuardianReviewer
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardianDecision {
    /// Auto-approve the command
    AutoApprove {
        /// Whether to persist approval for session
        session_approval: bool,
        /// Whether to create an Allow rule (learning)
        create_rule: bool,
    },

    /// Require user approval
    RequireApproval {
        /// Reason for requiring approval
        reason: String,
    },

    /// Deny execution (circuit breaker triggered)
    Denied {
        /// Reason for denial
        reason: String,
    },

    /// Defer to default policy
    Defer,
}

impl GuardianDecision {
    /// Check if this is an auto-approve decision
    pub fn is_auto_approved(&self) -> bool {
        matches!(self, GuardianDecision::AutoApprove { .. })
    }

    /// Check if approval is required
    pub fn requires_approval(&self) -> bool {
        matches!(self, GuardianDecision::RequireApproval { .. })
    }

    /// Check if denied
    pub fn is_denied(&self) -> bool {
        matches!(self, GuardianDecision::Denied { .. })
    }

    /// Check if deferring to default
    pub fn is_defer(&self) -> bool {
        matches!(self, GuardianDecision::Defer)
    }

    /// Create an auto-approve decision
    pub fn auto_approve(session_approval: bool, create_rule: bool) -> Self {
        GuardianDecision::AutoApprove {
            session_approval,
            create_rule,
        }
    }

    /// Create a require-approval decision
    pub fn require_approval(reason: String) -> Self {
        GuardianDecision::RequireApproval { reason }
    }

    /// Create a denied decision
    pub fn denied(reason: String) -> Self {
        GuardianDecision::Denied { reason }
    }
}

// ============================================================================
// GuardianReviewer Trait
// ============================================================================

/// Trait for automatic approval reviewers
///
/// Implementations can provide custom logic for auto-approving
/// commands based on various criteria (safety analysis, command patterns, etc.)
pub trait GuardianReviewer: Send + Sync {
    /// Review a command for automatic approval
    ///
    /// Returns a GuardianDecision indicating whether to:
    /// - Auto-approve
    /// - Require user approval
    /// - Deny (circuit breaker triggered)
    /// - Defer to default policy
    fn review(
        &self,
        command: &[String],
        cwd: &Path,
        approval_requirement: &ApprovalRequirement,
        config: &GuardianConfig,
    ) -> GuardianDecision;

    /// Get the name of this reviewer
    fn name(&self) -> &str;
}

// ============================================================================
// DefaultGuardianReviewer
// ============================================================================

/// Default implementation of GuardianReviewer
///
/// Uses ExecPolicy rules and approval policy to make decisions.
pub struct DefaultGuardianReviewer {
    /// ExecPolicy manager (optional)
    exec_policy: Option<Arc<ExecPolicyManager>>,

    /// Approval policy
    approval_policy: AskForApproval,
}

impl DefaultGuardianReviewer {
    /// Create a new default reviewer
    pub fn new(approval_policy: AskForApproval) -> Self {
        Self {
            exec_policy: None,
            approval_policy,
        }
    }

    /// Create with ExecPolicy
    pub fn with_exec_policy(
        approval_policy: AskForApproval,
        exec_policy: Arc<ExecPolicyManager>,
    ) -> Self {
        Self {
            exec_policy: Some(exec_policy),
            approval_policy,
        }
    }

    /// Check if command matches a known safe pattern
    fn is_known_safe(&self, command: &[String]) -> bool {
        if let Some(policy) = &self.exec_policy {
            // Check if there's an explicit Allow rule
            policy.has_allow_rule(command)
        } else {
            false
        }
    }

    /// Check if command appears to be read-only
    fn appears_read_only(&self, command: &[String]) -> bool {
        if command.is_empty() {
            return false;
        }

        // Common read-only command patterns
        let first = command[0].as_str();

        // Git read-only operations
        if first == "git" {
            if let Some(second) = command.get(1) {
                let read_only_git = [
                    "status",
                    "log",
                    "diff",
                    "show",
                    "branch",
                    "remote",
                    "tag",
                    "ls-files",
                    "ls-tree",
                    "rev-parse",
                    "describe",
                    "fetch",
                ];
                if read_only_git.contains(&second.as_str()) {
                    return true;
                }
            }
        }

        // Read-only file operations
        let read_only_first = [
            "ls", "cat", "head", "tail", "less", "more", "file", "stat", "wc", "grep", "find",
            "which", "whereis", "type", "echo", "pwd", "whoami", "id", "uname", "date", "uptime",
            "cargo", "rustc", "rustup", // Build tools (when just checking)
        ];

        if read_only_first.contains(&first) {
            // Check for write flags
            let write_flags = ["-w", "--write", "-o", "--output", ">"];
            for flag in write_flags {
                if command.iter().any(|c| c.contains(flag)) {
                    return false;
                }
            }
            return true;
        }

        false
    }

    /// Check if command is potentially dangerous
    fn is_potentially_dangerous(&self, command: &[String]) -> bool {
        if command.is_empty() {
            return false;
        }

        let first = command[0].as_str();

        // Privilege escalation
        if ["sudo", "su", "doas", "run0"].contains(&first) {
            return true;
        }

        // File removal
        if first == "rm" {
            return true;
        }

        // Shell interpreters with inline execution
        if let Some(second) = command.get(1) {
            let inline_flags = ["-c", "-e", "--eval", "--command"];
            if [
                "bash", "sh", "zsh", "python", "python3", "node", "perl", "ruby", "php", "lua",
            ]
            .contains(&first)
                && inline_flags.contains(&second.as_str())
            {
                return true;
            }
        }

        // Package managers with install/remove
        if ["npm", "yarn", "pnpm", "pip", "pip3", "cargo"].contains(&first) {
            if let Some(second) = command.get(1) {
                let dangerous_pkg = ["install", "uninstall", "remove", "update", "upgrade"];
                if dangerous_pkg.contains(&second.as_str()) {
                    // These might modify system state
                    return true;
                }
            }
        }

        false
    }
}

impl GuardianReviewer for DefaultGuardianReviewer {
    fn review(
        &self,
        command: &[String],
        _cwd: &Path,
        approval_requirement: &ApprovalRequirement,
        config: &GuardianConfig,
    ) -> GuardianDecision {
        // If forbidden, deny immediately
        if approval_requirement.is_forbidden() {
            return GuardianDecision::denied("Command forbidden by policy".to_string());
        }

        // If needs approval is not required by policy, defer
        if approval_requirement.can_skip() {
            return GuardianDecision::Defer;
        }

        // Check approval policy
        match self.approval_policy {
            AskForApproval::Never => {
                // Never request approval - auto-approve if safe
                if config.auto_approve_known_safe && self.is_known_safe(command) {
                    return GuardianDecision::auto_approve(true, false);
                }
                if config.auto_approve_read_only && self.appears_read_only(command) {
                    return GuardianDecision::auto_approve(true, false);
                }
                // Even with Never, check for dangerous commands
                if self.is_potentially_dangerous(command) {
                    return GuardianDecision::denied(
                        "Dangerous command detected, approval policy=Never".to_string(),
                    );
                }
                GuardianDecision::auto_approve(false, config.enable_auto_learning)
            }
            AskForApproval::OnFailure => {
                // Allow first, prompt on failure - defer for now
                GuardianDecision::Defer
            }
            AskForApproval::OnRequest | AskForApproval::UnlessTrusted => {
                // Check if command is known safe
                if config.auto_approve_known_safe && self.is_known_safe(command) {
                    return GuardianDecision::auto_approve(true, false);
                }

                // Check if read-only
                if config.auto_approve_read_only && self.appears_read_only(command) {
                    return GuardianDecision::auto_approve(true, false);
                }

                // Check if dangerous
                if self.is_potentially_dangerous(command) {
                    return GuardianDecision::require_approval(
                        "Potentially dangerous command requires approval".to_string(),
                    );
                }

                // Require approval for unknown commands
                GuardianDecision::require_approval("Unknown command requires approval".to_string())
            }
        }
    }

    fn name(&self) -> &str {
        "DefaultGuardianReviewer"
    }
}

// ============================================================================
// GuardianRejectionCircuitBreaker
// ============================================================================

/// Circuit breaker that triggers after too many rejections
///
/// Prevents auto-approval spam when the system is denying many commands.
pub struct GuardianRejectionCircuitBreaker {
    /// Configuration
    config: GuardianConfig,

    /// Rejection timestamps (for window tracking)
    rejections: Mutex<Vec<Instant>>,

    /// Whether circuit breaker is currently triggered
    triggered: Mutex<bool>,
}

impl GuardianRejectionCircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: GuardianConfig) -> Self {
        Self {
            config,
            rejections: Mutex::new(Vec::new()),
            triggered: Mutex::new(false),
        }
    }

    /// Record a rejection
    pub fn record_rejection(&self) {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.rejection_window_secs);

        let mut rejections = self.rejections.lock();

        // Remove old rejections outside window
        rejections.retain(|t| now.duration_since(*t) < window);

        // Add new rejection
        rejections.push(now);

        // Check if threshold reached
        if rejections.len() >= self.config.max_consecutive_rejections {
            *self.triggered.lock() = true;
            warn!(
                "Circuit breaker triggered after {} rejections in {}s window",
                rejections.len(),
                self.config.rejection_window_secs
            );
        }

        debug!(
            "Recorded rejection, count={} in window, threshold={}",
            rejections.len(),
            self.config.max_consecutive_rejections
        );
    }

    /// Record an approval (resets circuit breaker)
    pub fn record_approval(&self) {
        let mut rejections = self.rejections.lock();
        rejections.clear();
        *self.triggered.lock() = false;

        debug!("Circuit breaker reset after approval");
    }

    /// Check if circuit breaker is triggered
    pub fn is_triggered(&self) -> bool {
        *self.triggered.lock()
    }

    /// Get current rejection count in window
    pub fn rejection_count(&self) -> usize {
        let now = Instant::now();
        let window = Duration::from_secs(self.config.rejection_window_secs);

        let rejections = self.rejections.lock();
        rejections
            .iter()
            .filter(|t| now.duration_since(**t) < window)
            .count()
    }

    /// Reset circuit breaker manually
    pub fn reset(&self) {
        let mut rejections = self.rejections.lock();
        rejections.clear();
        *self.triggered.lock() = false;

        info!("Circuit breaker manually reset");
    }

    /// Modify a GuardianDecision based on circuit breaker state
    pub fn apply_to_decision(&self, decision: GuardianDecision) -> GuardianDecision {
        if self.is_triggered() {
            match decision {
                GuardianDecision::AutoApprove { .. } => {
                    // Block auto-approve when circuit breaker is triggered
                    GuardianDecision::denied(
                        "Circuit breaker triggered, too many recent rejections".to_string(),
                    )
                }
                GuardianDecision::Defer => {
                    // Defer becomes require-approval when triggered
                    GuardianDecision::require_approval(
                        "Circuit breaker triggered, manual approval required".to_string(),
                    )
                }
                other => other,
            }
        } else {
            decision
        }
    }
}

impl GuardianReviewer for GuardianRejectionCircuitBreaker {
    fn review(
        &self,
        _command: &[String],
        _cwd: &Path,
        _approval_requirement: &ApprovalRequirement,
        _config: &GuardianConfig,
    ) -> GuardianDecision {
        if self.is_triggered() {
            GuardianDecision::denied("Circuit breaker triggered".to_string())
        } else {
            GuardianDecision::Defer
        }
    }

    fn name(&self) -> &str {
        "GuardianRejectionCircuitBreaker"
    }
}

// ============================================================================
// GuardianManager
// ============================================================================

/// Manager for Guardian reviewers and circuit breaker
///
/// Coordinates multiple reviewers and applies circuit breaker.
pub struct GuardianManager {
    /// Configuration
    config: GuardianConfig,

    /// Primary reviewer
    reviewer: Arc<dyn GuardianReviewer>,

    /// Circuit breaker
    circuit_breaker: Arc<GuardianRejectionCircuitBreaker>,

    /// Approval cache (for tracking session approvals)
    approval_cache: Arc<Mutex<HashMap<String, ReviewDecision>>>,
}

impl GuardianManager {
    /// Create a new Guardian manager
    pub fn new(config: GuardianConfig, reviewer: Arc<dyn GuardianReviewer>) -> Self {
        Self {
            config: config.clone(),
            reviewer,
            circuit_breaker: Arc::new(GuardianRejectionCircuitBreaker::new(config)),
            approval_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create with default reviewer
    pub fn with_default_reviewer(config: GuardianConfig, approval_policy: AskForApproval) -> Self {
        let reviewer = Arc::new(DefaultGuardianReviewer::new(approval_policy));
        Self::new(config, reviewer)
    }

    /// Create with default reviewer and ExecPolicy
    pub fn with_exec_policy(
        config: GuardianConfig,
        approval_policy: AskForApproval,
        exec_policy: Arc<ExecPolicyManager>,
    ) -> Self {
        let reviewer = Arc::new(DefaultGuardianReviewer::with_exec_policy(
            approval_policy,
            exec_policy,
        ));
        Self::new(config, reviewer)
    }

    /// Review a command
    pub fn review(
        &self,
        command: &[String],
        cwd: &Path,
        approval_requirement: &ApprovalRequirement,
    ) -> GuardianDecision {
        // Get reviewer decision
        let decision = self
            .reviewer
            .review(command, cwd, approval_requirement, &self.config);

        // Apply circuit breaker
        let decision = self.circuit_breaker.apply_to_decision(decision);

        // Track result
        match decision {
            GuardianDecision::AutoApprove { .. } => {
                self.circuit_breaker.record_approval();
            }
            GuardianDecision::Denied { .. } => {
                self.circuit_breaker.record_rejection();
            }
            GuardianDecision::RequireApproval { .. } => {
                // Neutral - doesn't affect circuit breaker
            }
            GuardianDecision::Defer => {
                // Neutral - doesn't affect circuit breaker
            }
        }

        decision
    }

    /// Check if command is already approved for session
    pub fn is_session_approved(&self, key: &CommandApprovalKey) -> bool {
        let cache = self.approval_cache.lock();
        cache
            .get(&key.to_cache_key())
            .map(|d| d.is_session_approval())
            .unwrap_or(false)
    }

    /// Record an approval decision
    pub fn record_approval(&self, key: CommandApprovalKey, decision: ReviewDecision) {
        let mut cache = self.approval_cache.lock();
        cache.insert(key.to_cache_key(), decision);

        if decision.allows_execution() {
            self.circuit_breaker.record_approval();
        } else {
            self.circuit_breaker.record_rejection();
        }
    }

    /// Get circuit breaker
    pub fn circuit_breaker(&self) -> &Arc<GuardianRejectionCircuitBreaker> {
        &self.circuit_breaker
    }

    /// Get config
    pub fn config(&self) -> &GuardianConfig {
        &self.config
    }

    /// Check if circuit breaker is triggered
    pub fn is_circuit_breaker_triggered(&self) -> bool {
        self.circuit_breaker.is_triggered()
    }

    /// Reset circuit breaker
    pub fn reset_circuit_breaker(&self) {
        self.circuit_breaker.reset();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_guardian_config_default() {
        let config = GuardianConfig::default();
        assert_eq!(config.max_consecutive_rejections, 5);
        assert!(!config.auto_approve_known_safe);
        assert!(!config.auto_approve_read_only);
        assert!(!config.enable_auto_learning);
    }

    #[test]
    fn test_guardian_config_strict() {
        let config = GuardianConfig::strict();
        assert_eq!(config.max_consecutive_rejections, 3);
        assert!(!config.auto_approve_known_safe);
        assert!(!config.auto_approve_read_only);
        assert!(!config.enable_auto_learning);
    }

    #[test]
    fn test_guardian_config_liberal() {
        let config = GuardianConfig::liberal();
        assert_eq!(config.max_consecutive_rejections, 10);
        assert!(config.auto_approve_known_safe);
        assert!(config.auto_approve_read_only);
        assert!(config.enable_auto_learning);
    }

    #[test]
    fn test_guardian_decision_auto_approve() {
        let decision = GuardianDecision::auto_approve(true, true);
        assert!(decision.is_auto_approved());
        assert!(!decision.requires_approval());
        assert!(!decision.is_denied());
        assert!(!decision.is_defer());
    }

    #[test]
    fn test_guardian_decision_require_approval() {
        let decision = GuardianDecision::require_approval("test".to_string());
        assert!(!decision.is_auto_approved());
        assert!(decision.requires_approval());
        assert!(!decision.is_denied());
        assert!(!decision.is_defer());
    }

    #[test]
    fn test_guardian_decision_denied() {
        let decision = GuardianDecision::denied("test".to_string());
        assert!(!decision.is_auto_approved());
        assert!(!decision.requires_approval());
        assert!(decision.is_denied());
        assert!(!decision.is_defer());
    }

    #[test]
    fn test_guardian_decision_defer() {
        let decision = GuardianDecision::Defer;
        assert!(!decision.is_auto_approved());
        assert!(!decision.requires_approval());
        assert!(!decision.is_denied());
        assert!(decision.is_defer());
    }

    #[test]
    fn test_default_guardian_reviewer_read_only_detection() {
        let reviewer = DefaultGuardianReviewer::new(AskForApproval::OnRequest);

        // Read-only commands
        assert!(reviewer.appears_read_only(&["git".to_string(), "status".to_string()]));
        assert!(reviewer.appears_read_only(&["git".to_string(), "log".to_string()]));
        assert!(reviewer.appears_read_only(&["ls".to_string()]));
        assert!(reviewer.appears_read_only(&["cat".to_string(), "file.txt".to_string()]));
        assert!(reviewer.appears_read_only(&["pwd".to_string()]));

        // Write commands
        assert!(!reviewer.appears_read_only(&["git".to_string(), "commit".to_string()]));
        assert!(!reviewer.appears_read_only(&["rm".to_string(), "file.txt".to_string()]));
        assert!(!reviewer.appears_read_only(&["npm".to_string(), "install".to_string()]));
    }

    #[test]
    fn test_default_guardian_reviewer_dangerous_detection() {
        let reviewer = DefaultGuardianReviewer::new(AskForApproval::OnRequest);

        // Dangerous commands
        assert!(reviewer.is_potentially_dangerous(&["sudo".to_string()]));
        assert!(reviewer.is_potentially_dangerous(&["rm".to_string(), "-rf".to_string()]));
        assert!(reviewer.is_potentially_dangerous(&[
            "bash".to_string(),
            "-c".to_string(),
            "echo test".to_string()
        ]));
        assert!(reviewer.is_potentially_dangerous(&["python".to_string(), "-c".to_string()]));

        // Safe commands
        assert!(!reviewer.is_potentially_dangerous(&["git".to_string(), "status".to_string()]));
        assert!(!reviewer.is_potentially_dangerous(&["ls".to_string()]));
        assert!(!reviewer.is_potentially_dangerous(&["cargo".to_string(), "build".to_string()]));
    }

    #[test]
    fn test_circuit_breaker_basic() {
        let config = GuardianConfig {
            max_consecutive_rejections: 3,
            rejection_window_secs: 60,
            ..Default::default()
        };
        let cb = GuardianRejectionCircuitBreaker::new(config);

        assert!(!cb.is_triggered());
        assert_eq!(cb.rejection_count(), 0);

        // Record rejections
        cb.record_rejection();
        assert_eq!(cb.rejection_count(), 1);
        assert!(!cb.is_triggered());

        cb.record_rejection();
        assert_eq!(cb.rejection_count(), 2);
        assert!(!cb.is_triggered());

        cb.record_rejection();
        assert_eq!(cb.rejection_count(), 3);
        assert!(cb.is_triggered());

        // Reset
        cb.reset();
        assert!(!cb.is_triggered());
        assert_eq!(cb.rejection_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_approval_reset() {
        let config = GuardianConfig {
            max_consecutive_rejections: 2,
            rejection_window_secs: 60,
            ..Default::default()
        };
        let cb = GuardianRejectionCircuitBreaker::new(config);

        cb.record_rejection();
        cb.record_rejection();
        assert!(cb.is_triggered());

        // Approval resets
        cb.record_approval();
        assert!(!cb.is_triggered());
        assert_eq!(cb.rejection_count(), 0);
    }

    #[test]
    fn test_circuit_breaker_apply_to_decision() {
        let config = GuardianConfig {
            max_consecutive_rejections: 1,
            rejection_window_secs: 60,
            ..Default::default()
        };
        let cb = GuardianRejectionCircuitBreaker::new(config);

        // Not triggered - pass through
        let decision = GuardianDecision::auto_approve(true, false);
        let result = cb.apply_to_decision(decision);
        assert!(result.is_auto_approved());

        // Trigger circuit breaker
        cb.record_rejection();
        assert!(cb.is_triggered());

        // Auto-approve becomes denied
        let decision = GuardianDecision::auto_approve(true, false);
        let result = cb.apply_to_decision(decision);
        assert!(result.is_denied());

        // Defer becomes require-approval
        let decision = GuardianDecision::Defer;
        let result = cb.apply_to_decision(decision);
        assert!(result.requires_approval());

        // Denied stays denied
        let decision = GuardianDecision::denied("test".to_string());
        let result = cb.apply_to_decision(decision);
        assert!(result.is_denied());
    }

    #[test]
    fn test_guardian_manager_creation() {
        let config = GuardianConfig::default();
        let manager = GuardianManager::with_default_reviewer(config, AskForApproval::OnRequest);

        assert!(!manager.is_circuit_breaker_triggered());
        assert_eq!(manager.config().max_consecutive_rejections, 5);
    }

    #[test]
    fn test_guardian_manager_review_forbidden() {
        let config = GuardianConfig::default();
        let manager = GuardianManager::with_default_reviewer(config, AskForApproval::OnRequest);

        let forbidden = ApprovalRequirement::Forbidden {
            reason: "test".to_string(),
        };
        let decision = manager.review(&["rm".to_string()], &PathBuf::from("/"), &forbidden);

        assert!(decision.is_denied());
    }

    #[test]
    fn test_guardian_manager_review_skip() {
        let config = GuardianConfig::default();
        let manager = GuardianManager::with_default_reviewer(config, AskForApproval::OnRequest);

        let skip = ApprovalRequirement::Skip {
            bypass_sandbox: false,
            amendment: None,
        };
        let decision = manager.review(&["ls".to_string()], &PathBuf::from("/workspace"), &skip);

        assert!(decision.is_defer());
    }

    #[test]
    fn test_guardian_default_does_not_auto_approve_known_safe_commands() {
        let config = GuardianConfig::default();
        let manager = GuardianManager::with_default_reviewer(config, AskForApproval::OnRequest);

        let needs_approval = ApprovalRequirement::NeedsApproval {
            reason: "approval required".to_string(),
            amendment: None,
        };
        let decision = manager.review(
            &["git".to_string(), "status".to_string()],
            &PathBuf::from("/workspace"),
            &needs_approval,
        );

        assert!(decision.requires_approval());
    }
}
