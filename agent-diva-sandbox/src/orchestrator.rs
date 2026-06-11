//! ToolOrchestrator - Execution orchestration with approval and sandbox
//!
//! This module implements the orchestration flow for tool execution:
//! Approval → Sandbox Selection → Attempt → Retry/Escalation
//!
//! Inspired by Codex CLI's orchestrator.rs architecture.

use crate::approval::{CommandApprovalKey, ReviewDecision};
use crate::error::{SandboxError, SandboxResult};
use crate::exec_policy::{ApprovalRequirement, ExecPolicyManager};
use crate::filesystem::FileSystemSandboxPolicy;
use crate::guardian::{GuardianDecision, GuardianManager};
use crate::manager::{SandboxExecRequest, SandboxManager};
use crate::policy::{AskForApproval, SandboxPolicy};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{debug, info, warn};

// ============================================================================
// SandboxOverride
// ============================================================================

/// Override for sandbox behavior on first attempt
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SandboxOverride {
    /// Normal sandbox selection
    #[default]
    NoOverride,

    /// Bypass sandbox on first attempt (for escalated permissions)
    BypassSandboxFirstAttempt,
}

impl SandboxOverride {
    /// Check if sandbox should be bypassed
    pub fn should_bypass(&self) -> bool {
        matches!(self, SandboxOverride::BypassSandboxFirstAttempt)
    }
}

// ============================================================================
// SandboxAttempt
// ============================================================================

/// Represents a sandbox execution attempt
#[derive(Debug)]
pub struct SandboxAttempt<'a> {
    /// Sandbox policy to apply
    pub policy: &'a SandboxPolicy,

    /// File system policy
    pub fs_policy: &'a FileSystemSandboxPolicy,

    /// Approval requirement
    pub approval: ApprovalRequirement,

    /// Sandbox override
    pub override_: SandboxOverride,

    /// Working directory
    pub cwd: PathBuf,

    /// Environment variables
    pub env: HashMap<String, String>,

    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl<'a> SandboxAttempt<'a> {
    /// Create a new sandbox attempt
    pub fn new(
        policy: &'a SandboxPolicy,
        fs_policy: &'a FileSystemSandboxPolicy,
        cwd: PathBuf,
    ) -> Self {
        Self {
            policy,
            fs_policy,
            approval: ApprovalRequirement::Skip {
                bypass_sandbox: false,
                amendment: None,
            },
            override_: SandboxOverride::NoOverride,
            cwd,
            env: HashMap::new(),
            timeout_secs: 60,
        }
    }

    /// Set approval requirement
    pub fn with_approval(mut self, approval: ApprovalRequirement) -> Self {
        self.approval = approval;
        self
    }

    /// Set sandbox override
    pub fn with_override(mut self, override_: SandboxOverride) -> Self {
        self.override_ = override_;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout_secs: u64) -> Self {
        self.timeout_secs = timeout_secs;
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.insert(key, value);
        self
    }

    /// Check if this attempt needs approval
    pub fn needs_approval(&self) -> bool {
        self.approval.needs_approval()
    }

    /// Check if this attempt is forbidden
    pub fn is_forbidden(&self) -> bool {
        self.approval.is_forbidden()
    }

    /// Check if sandbox should be bypassed
    pub fn should_bypass_sandbox(&self) -> bool {
        self.override_.should_bypass() || self.approval.bypass_sandbox()
    }
}

// ============================================================================
// OrchestratorRunResult
// ============================================================================

/// Result of orchestrator run
#[derive(Debug)]
pub struct OrchestratorRunResult {
    /// Execution output
    pub output: String,

    /// Whether execution was successful
    pub success: bool,

    /// Whether sandbox was used
    pub used_sandbox: bool,

    /// Whether approval was requested
    pub approval_requested: bool,

    /// Amendment suggestion (if applicable)
    pub amendment: Option<Vec<String>>,
}

impl OrchestratorRunResult {
    /// Create a successful result
    pub fn success(output: String, used_sandbox: bool) -> Self {
        Self {
            output,
            success: true,
            used_sandbox,
            approval_requested: false,
            amendment: None,
        }
    }

    /// Create a failed result
    pub fn failed(output: String, used_sandbox: bool) -> Self {
        Self {
            output,
            success: false,
            used_sandbox,
            approval_requested: false,
            amendment: None,
        }
    }

    /// Create with amendment
    pub fn with_amendment(mut self, amendment: Vec<String>) -> Self {
        self.amendment = Some(amendment);
        self
    }
}

// ============================================================================
// Sandbox Permissions
// ============================================================================

/// Sandbox permissions level
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SandboxPermissions {
    /// Normal sandbox restrictions
    #[default]
    Normal,

    /// Elevated permissions (can bypass sandbox)
    Elevated,

    /// No sandbox
    None,
}

impl SandboxPermissions {
    /// Check if elevated permissions are needed
    pub fn requires_escalated_permissions(&self) -> bool {
        matches!(
            self,
            SandboxPermissions::Elevated | SandboxPermissions::None
        )
    }
}

// ============================================================================
// Traits: Approvable and Sandboxable
// ============================================================================

/// Trait for tools that can request approval
pub trait Approvable {
    /// Get approval keys for this tool request
    fn approval_keys(&self) -> Vec<String>;

    /// Check if approval should bypass sandbox
    fn wants_no_sandbox_approval(&self, policy: AskForApproval) -> bool {
        matches!(
            policy,
            AskForApproval::OnFailure | AskForApproval::UnlessTrusted
        )
    }

    /// Check if approval should be bypassed (already approved)
    fn should_bypass_approval(&self, policy: AskForApproval, already_approved: bool) -> bool {
        if already_approved {
            return true;
        }
        matches!(policy, AskForApproval::Never)
    }
}

/// Trait for tools that can be sandboxed
pub trait Sandboxable {
    /// Check if escalation on failure is allowed
    fn escalate_on_failure(&self) -> bool {
        true // Default: allow escalation
    }

    /// Get sandbox permissions level
    fn sandbox_permissions(&self) -> SandboxPermissions {
        SandboxPermissions::Normal
    }

    /// Check if tool requires elevated permissions
    fn requires_elevated_permissions(&self) -> bool {
        self.sandbox_permissions().requires_escalated_permissions()
    }
}

// ============================================================================
// ToolOrchestrator
// ============================================================================

/// Orchestrator for tool execution with approval and sandbox
///
/// Implements the flow:
/// 1. Guardian Auto-Approval Phase - Check if auto-approval is possible
/// 2. Approval Phase - Check if approval is needed
/// 3. First Attempt - Execute with or without sandbox
/// 4. Sandbox Denial Handling - Escalate if needed
/// 5. Retry Attempt - Execute without sandbox (if escalated)
pub struct ToolOrchestrator {
    /// Sandbox manager
    sandbox_manager: Arc<SandboxManager>,

    /// ExecPolicy manager (optional)
    exec_policy: Option<ExecPolicyManager>,

    /// Approval policy
    approval_policy: AskForApproval,

    /// Guardian manager for auto-approval (optional)
    guardian: Option<Arc<GuardianManager>>,
}

impl ToolOrchestrator {
    /// Create a new orchestrator
    pub fn new(sandbox_manager: Arc<SandboxManager>, approval_policy: AskForApproval) -> Self {
        Self {
            sandbox_manager,
            exec_policy: None,
            approval_policy,
            guardian: None,
        }
    }

    /// Create with ExecPolicy
    pub fn with_exec_policy(
        sandbox_manager: Arc<SandboxManager>,
        approval_policy: AskForApproval,
        exec_policy: ExecPolicyManager,
    ) -> Self {
        Self {
            sandbox_manager,
            exec_policy: Some(exec_policy),
            approval_policy,
            guardian: None,
        }
    }

    /// Create with Guardian
    pub fn with_guardian(
        sandbox_manager: Arc<SandboxManager>,
        approval_policy: AskForApproval,
        guardian: Arc<GuardianManager>,
    ) -> Self {
        Self {
            sandbox_manager,
            exec_policy: None,
            approval_policy,
            guardian: Some(guardian),
        }
    }

    /// Create with ExecPolicy and Guardian
    pub fn with_exec_policy_and_guardian(
        sandbox_manager: Arc<SandboxManager>,
        approval_policy: AskForApproval,
        exec_policy: ExecPolicyManager,
        guardian: Arc<GuardianManager>,
    ) -> Self {
        Self {
            sandbox_manager,
            exec_policy: Some(exec_policy),
            approval_policy,
            guardian: Some(guardian),
        }
    }

    /// Get the sandbox manager
    pub fn sandbox_manager(&self) -> &Arc<SandboxManager> {
        &self.sandbox_manager
    }

    /// Get approval policy
    pub fn approval_policy(&self) -> AskForApproval {
        self.approval_policy
    }

    /// Get the Guardian manager
    pub fn guardian(&self) -> Option<&Arc<GuardianManager>> {
        self.guardian.as_ref()
    }

    /// Check if Guardian auto-approval is available
    pub fn has_guardian(&self) -> bool {
        self.guardian.is_some()
    }

    /// Check approval requirement for a command
    pub fn check_approval(&self, command: &[String]) -> ApprovalRequirement {
        if let Some(policy) = &self.exec_policy {
            policy.get_approval_requirement(command, self.approval_policy)
        } else {
            // No ExecPolicy, use approval policy defaults
            match self.approval_policy {
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
                        reason: "Approval policy requires it".to_string(),
                        amendment: None,
                    }
                }
            }
        }
    }

    /// Run a command through the orchestration flow
    pub async fn run(&self, command: &str, cwd: &PathBuf) -> SandboxResult<OrchestratorRunResult> {
        info!("ToolOrchestrator.run: '{}' in {:?}", command, cwd);

        let command_parts =
            shell_words::split(command).map_err(|e| SandboxError::InvalidCommand(e.to_string()))?;
        if let Some(result) = self.preflight_guardian(command, cwd, &command_parts).await? {
            return Ok(result);
        }

        let resolution = self.resolve_approval(command, cwd, &command_parts)?;
        let attempt = self.select_sandbox(cwd, &resolution)?;
        let result = self.execute(&command_parts, &attempt).await;

        match result {
            Ok(output) => Ok(self.build_success_result(output, &attempt, &resolution.approval)),
            Err(error) => self
                .handle_failure(
                    command,
                    cwd,
                    &command_parts,
                    &resolution.approval_key,
                    &resolution.approval,
                    error,
                )
                .await,
        }
    }

    async fn preflight_guardian(
        &self,
        command: &str,
        cwd: &PathBuf,
        command_parts: &[String],
    ) -> SandboxResult<Option<OrchestratorRunResult>> {
        if let Some(guardian) = &self.guardian {
            let approval = self.check_approval(&command_parts);
            let guardian_decision = guardian.review(&command_parts, cwd, &approval);

            match guardian_decision {
                GuardianDecision::AutoApprove {
                    session_approval,
                    create_rule,
                } => {
                    info!(
                        "Guardian auto-approved command (session={}, create_rule={})",
                        session_approval, create_rule
                    );

                    // Record approval for session
                    if session_approval {
                        let key = crate::approval::CommandApprovalKey::new(
                            command.to_string(),
                            cwd.clone(),
                        );
                        guardian.record_approval(key, ReviewDecision::ApprovedForSession);
                    }

                    // Create Allow rule if configured
                    if create_rule {
                        if let Some(_policy) = &self.exec_policy {
                            let _amendment =
                                crate::exec_policy::ExecPolicyAmendment::new(command_parts.to_vec());
                            // Note: This would need mutable access, skip for now
                            debug!("Would create Allow rule for: {}", command_parts.join(" "));
                        }
                    }

                    // Execute directly with sandbox
                    let attempt = SandboxAttempt::new(
                        self.sandbox_manager.policy(),
                        self.sandbox_manager.fs_policy(),
                        cwd.clone(),
                    );

                    let result = self.execute(&command_parts, &attempt).await;
                    return match result {
                        Ok(output) => Ok(Some(OrchestratorRunResult::success(output, true))),
                        Err(e) => Err(e),
                    };
                }
                GuardianDecision::Denied { reason } => {
                    warn!("Guardian denied command: {}", reason);
                    return Err(SandboxError::Denied { reason });
                }
                GuardianDecision::RequireApproval { reason } => {
                    debug!("Guardian requires approval: {}", reason);
                    // Continue to normal approval flow
                }
                GuardianDecision::Defer => {
                    debug!("Guardian defers to default policy");
                    // Continue to normal approval flow
                }
            }
        }

        Ok(None)
    }

    fn resolve_approval(
        &self,
        command: &str,
        cwd: &PathBuf,
        command_parts: &[String],
    ) -> SandboxResult<ApprovalResolution> {
        let approval_key = CommandApprovalKey::new(command.to_string(), cwd.clone());
        let approval = self.check_approval(&command_parts);

        if approval.is_forbidden() {
            warn!("Command forbidden by policy: {}", command);
            return Err(SandboxError::Denied {
                reason: match approval {
                    ApprovalRequirement::Forbidden { reason } => reason,
                    _ => "Command forbidden".to_string(),
                },
            });
        }

        let sandbox_override = self.resolve_initial_override(&approval_key, &approval)?;
        Ok(ApprovalResolution {
            approval_key,
            approval,
            sandbox_override,
        })
    }

    fn select_sandbox<'a>(
        &'a self,
        cwd: &PathBuf,
        resolution: &ApprovalResolution,
    ) -> SandboxResult<SandboxAttempt<'a>> {
        Ok(SandboxAttempt::new(
            self.sandbox_manager.policy(),
            self.sandbox_manager.fs_policy(),
            cwd.clone(),
        )
        .with_approval(resolution.approval.clone())
        .with_override(resolution.sandbox_override))
    }

    async fn execute(
        &self,
        command_parts: &[String],
        attempt: &SandboxAttempt<'_>,
    ) -> SandboxResult<String> {
        // Build the command string
        let command = command_parts.join(" ");

        // Build execution request
        let request = SandboxExecRequest::new(command, attempt.cwd.clone())
            .with_timeout(attempt.timeout_secs);

        // Execute with or without sandbox
        if attempt.should_bypass_sandbox() {
            debug!("Executing without sandbox (bypass)");
            self.sandbox_manager.execute_unsandboxed(&request).await
        } else {
            debug!("Executing with sandbox");
            self.sandbox_manager.execute_sandboxed(&request).await
        }
    }

    async fn handle_failure(
        &self,
        command: &str,
        cwd: &PathBuf,
        command_parts: &[String],
        approval_key: &CommandApprovalKey,
        approval: &ApprovalRequirement,
        error: SandboxError,
    ) -> SandboxResult<OrchestratorRunResult> {
        match error {
            SandboxError::Denied { reason } => {
                debug!("Sandbox denied: {}", reason);
                self.retry_after_sandbox_failure(
                    command,
                    cwd,
                    command_parts,
                    approval_key,
                    approval,
                    SandboxError::Denied { reason },
                )
                .await
            }
            err if self.should_offer_escalation(&err) => {
                self.retry_after_sandbox_failure(
                    command,
                    cwd,
                    command_parts,
                    approval_key,
                    approval,
                    err,
                )
                .await
            }
            err => Err(err),
        }
    }

    fn build_success_result(
        &self,
        output: String,
        attempt: &SandboxAttempt<'_>,
        approval: &ApprovalRequirement,
    ) -> OrchestratorRunResult {
        let amendment = match approval {
            ApprovalRequirement::NeedsApproval { amendment, .. } => amendment
                .as_ref()
                .map(|candidate| candidate.command.clone()),
            _ => None,
        };

        OrchestratorRunResult::success(output, !attempt.should_bypass_sandbox())
            .with_amendment(amendment.unwrap_or_default())
    }

    fn resolve_initial_override(
        &self,
        approval_key: &CommandApprovalKey,
        approval: &ApprovalRequirement,
    ) -> SandboxResult<SandboxOverride> {
        if approval.bypass_sandbox() {
            return Ok(SandboxOverride::BypassSandboxFirstAttempt);
        }

        match self.approval_policy {
            AskForApproval::OnRequest | AskForApproval::UnlessTrusted => {
                match self.cached_decision(approval_key) {
                    Some(ReviewDecision::ApprovedForSession) => {
                        Ok(SandboxOverride::BypassSandboxFirstAttempt)
                    }
                    Some(ReviewDecision::ApprovedOnce) => {
                        self.consume_approved_once(approval_key);
                        Ok(SandboxOverride::BypassSandboxFirstAttempt)
                    }
                    Some(ReviewDecision::Denied) => Err(SandboxError::Denied {
                        reason: "Previously denied by user".to_string(),
                    }),
                    None => Err(SandboxError::ApprovalRequired {
                        reason: self.approval_prompt_reason(approval),
                    }),
                }
            }
            AskForApproval::Never | AskForApproval::OnFailure => Ok(SandboxOverride::NoOverride),
        }
    }

    async fn retry_after_sandbox_failure(
        &self,
        command: &str,
        cwd: &PathBuf,
        command_parts: &[String],
        approval_key: &CommandApprovalKey,
        approval: &ApprovalRequirement,
        original_error: SandboxError,
    ) -> SandboxResult<OrchestratorRunResult> {
        match self.resolve_retry_permission(command_parts, approval_key, approval)? {
            RetryPermission::RetryDirect => {
                info!("Escalating to no-sandbox execution after recorded approval");
                let escalated_attempt = SandboxAttempt::new(
                    self.sandbox_manager.policy(),
                    self.sandbox_manager.fs_policy(),
                    cwd.clone(),
                )
                .with_override(SandboxOverride::BypassSandboxFirstAttempt);

                self.execute(command_parts, &escalated_attempt)
                    .await
                    .map(|output| OrchestratorRunResult::success(output, false))
            }
            RetryPermission::NeedsApproval(reason) => Err(SandboxError::ApprovalRequired {
                reason: format!("{} [{}]", reason, command),
            }),
            RetryPermission::Forbidden => Err(original_error),
        }
    }

    fn resolve_retry_permission(
        &self,
        command_parts: &[String],
        approval_key: &CommandApprovalKey,
        approval: &ApprovalRequirement,
    ) -> SandboxResult<RetryPermission> {
        if !self.approval_policy.allows_sandbox_failure_retry() {
            return Ok(RetryPermission::Forbidden);
        }

        if crate::exec_policy::is_banned_prefix(command_parts) {
            warn!("Cannot escalate banned prefix command");
            return Ok(RetryPermission::Forbidden);
        }

        match self.cached_decision(approval_key) {
            Some(ReviewDecision::ApprovedForSession) => Ok(RetryPermission::RetryDirect),
            Some(ReviewDecision::ApprovedOnce) => {
                self.consume_approved_once(approval_key);
                Ok(RetryPermission::RetryDirect)
            }
            Some(ReviewDecision::Denied) => Err(SandboxError::Denied {
                reason: "Previously denied by user".to_string(),
            }),
            None => Ok(RetryPermission::NeedsApproval(
                self.approval_prompt_reason(approval),
            )),
        }
    }

    fn approval_prompt_reason(&self, approval: &ApprovalRequirement) -> String {
        match approval {
            ApprovalRequirement::NeedsApproval { reason, .. } => reason.clone(),
            _ => "Sandbox execution requires explicit approval to retry without isolation"
                .to_string(),
        }
    }

    fn cached_decision(&self, approval_key: &CommandApprovalKey) -> Option<ReviewDecision> {
        self.sandbox_manager.check_cached_decision(approval_key)
    }

    fn consume_approved_once(&self, approval_key: &CommandApprovalKey) {
        self.sandbox_manager.consume_approved_once(approval_key);
    }

    fn should_offer_escalation(&self, error: &SandboxError) -> bool {
        matches!(
            error,
            SandboxError::Denied { .. }
                | SandboxError::PermissionDenied { .. }
                | SandboxError::PlatformError(_)
                | SandboxError::PlatformUnavailable { .. }
                | SandboxError::PlatformNotSupported
                | SandboxError::ApprovalRequired { .. }
        )
    }

    /// Record approval decision (for caching)
    pub fn record_approval(&self, command: &str, cwd: &Path, decision: ReviewDecision) {
        let key = crate::approval::CommandApprovalKey::new(command.to_string(), cwd.to_path_buf());
        self.sandbox_manager.record_approval(key, decision);
    }
}

enum RetryPermission {
    RetryDirect,
    NeedsApproval(String),
    Forbidden,
}

struct ApprovalResolution {
    approval_key: CommandApprovalKey,
    approval: ApprovalRequirement,
    sandbox_override: SandboxOverride,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approval::ApprovalStore;
    use crate::guardian::GuardianConfig;

    #[test]
    fn test_sandbox_override_default() {
        let override_ = SandboxOverride::default();
        assert_eq!(override_, SandboxOverride::NoOverride);
        assert!(!override_.should_bypass());
    }

    #[test]
    fn test_sandbox_override_bypass() {
        let override_ = SandboxOverride::BypassSandboxFirstAttempt;
        assert!(override_.should_bypass());
    }

    #[test]
    fn test_sandbox_permissions_default() {
        let perms = SandboxPermissions::default();
        assert_eq!(perms, SandboxPermissions::Normal);
        assert!(!perms.requires_escalated_permissions());
    }

    #[test]
    fn test_sandbox_permissions_elevated() {
        let perms = SandboxPermissions::Elevated;
        assert!(perms.requires_escalated_permissions());
    }

    #[test]
    fn test_sandbox_permissions_none() {
        let perms = SandboxPermissions::None;
        assert!(perms.requires_escalated_permissions());
    }

    #[test]
    fn test_sandbox_attempt_creation() {
        let policy = SandboxPolicy::default();
        let fs_policy = crate::filesystem::FileSystemSandboxPolicy::unrestricted();
        let cwd = PathBuf::from("/workspace");

        let attempt = SandboxAttempt::new(&policy, &fs_policy, cwd.clone());
        assert!(!attempt.should_bypass_sandbox());
        assert!(!attempt.needs_approval());
    }

    #[test]
    fn test_sandbox_attempt_with_override() {
        let policy = SandboxPolicy::default();
        let fs_policy = crate::filesystem::FileSystemSandboxPolicy::unrestricted();
        let cwd = PathBuf::from("/workspace");

        let attempt = SandboxAttempt::new(&policy, &fs_policy, cwd)
            .with_override(SandboxOverride::BypassSandboxFirstAttempt);
        assert!(attempt.should_bypass_sandbox());
    }

    #[test]
    fn test_orchestrator_run_result_success() {
        let result = OrchestratorRunResult::success("output".to_string(), true);
        assert!(result.success);
        assert!(result.used_sandbox);
        assert!(!result.approval_requested);
    }

    #[test]
    fn test_orchestrator_run_result_failed() {
        let result = OrchestratorRunResult::failed("error".to_string(), false);
        assert!(!result.success);
        assert!(!result.used_sandbox);
    }

    #[test]
    fn test_orchestrator_run_result_with_amendment() {
        let result = OrchestratorRunResult::success("output".to_string(), true)
            .with_amendment(vec!["git".to_string(), "status".to_string()]);
        assert!(result.amendment.is_some());
        assert_eq!(result.amendment.unwrap().len(), 2);
    }

    #[test]
    fn test_approvable_trait_defaults() {
        struct TestTool;
        impl Approvable for TestTool {
            fn approval_keys(&self) -> Vec<String> {
                vec!["test".to_string()]
            }
        }

        let tool = TestTool;
        assert!(tool.wants_no_sandbox_approval(AskForApproval::OnFailure));
        assert!(!tool.wants_no_sandbox_approval(AskForApproval::Never));
        assert!(tool.should_bypass_approval(AskForApproval::Never, false));
        assert!(tool.should_bypass_approval(AskForApproval::OnFailure, true));
    }

    #[test]
    fn test_sandboxable_trait_defaults() {
        struct TestTool;
        impl Sandboxable for TestTool {}

        let tool = TestTool;
        assert!(tool.escalate_on_failure());
        assert_eq!(tool.sandbox_permissions(), SandboxPermissions::Normal);
        assert!(!tool.requires_elevated_permissions());
    }

    #[test]
    fn test_orchestrator_creation() {
        let manager = Arc::new(SandboxManager::with_defaults());
        let orchestrator = ToolOrchestrator::new(manager, AskForApproval::OnFailure);
        assert_eq!(orchestrator.approval_policy(), AskForApproval::OnFailure);
    }

    #[test]
    fn test_orchestrator_check_approval_no_policy() {
        let manager = Arc::new(SandboxManager::with_defaults());
        let orchestrator = ToolOrchestrator::new(manager, AskForApproval::Never);

        let approval = orchestrator.check_approval(&["git".to_string(), "status".to_string()]);
        assert!(approval.can_skip());
    }

    #[test]
    fn test_orchestrator_check_approval_needs_approval() {
        let manager = Arc::new(SandboxManager::with_defaults());
        let orchestrator = ToolOrchestrator::new(manager, AskForApproval::OnRequest);

        let approval = orchestrator.check_approval(&["git".to_string(), "status".to_string()]);
        assert!(approval.needs_approval());
    }

    #[test]
    fn test_orchestrator_with_guardian() {
        let manager = Arc::new(SandboxManager::with_defaults());
        let guardian = Arc::new(GuardianManager::with_default_reviewer(
            GuardianConfig::default(),
            AskForApproval::OnRequest,
        ));
        let orchestrator =
            ToolOrchestrator::with_guardian(manager, AskForApproval::OnRequest, guardian);

        assert!(orchestrator.has_guardian());
        assert!(orchestrator.guardian().is_some());
    }

    #[test]
    fn test_orchestrator_without_guardian() {
        let manager = Arc::new(SandboxManager::with_defaults());
        let orchestrator = ToolOrchestrator::new(manager, AskForApproval::OnRequest);

        assert!(!orchestrator.has_guardian());
        assert!(orchestrator.guardian().is_none());
    }

    #[tokio::test]
    async fn test_on_request_requires_cached_approval_before_execution() {
        let config = crate::manager::SandboxConfig::danger_full_access();
        let store = ApprovalStore::new_shared();
        let manager = Arc::new(SandboxManager::new_with_approval_store(&config, store));
        let orchestrator = ToolOrchestrator::new(manager.clone(), AskForApproval::OnRequest);
        let cwd = PathBuf::from(".");

        let err = orchestrator.run("echo hello", &cwd).await.unwrap_err();
        assert!(matches!(err, SandboxError::ApprovalRequired { .. }));

        manager.record_approval(
            CommandApprovalKey::new("echo hello".to_string(), cwd.clone()),
            ReviewDecision::ApprovedForSession,
        );

        let result = orchestrator.run("echo hello", &cwd).await.unwrap();
        assert!(result.output.contains("hello"));
        assert!(!result.used_sandbox);
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn test_on_failure_retry_requires_cached_approval() {
        let mut config = crate::manager::SandboxConfig::default();
        config.mode = crate::policy::SandboxMode::WorkspaceWrite;
        let store = ApprovalStore::new_shared();
        let manager = Arc::new(SandboxManager::new_with_approval_store(&config, store));
        let orchestrator = ToolOrchestrator::new(manager.clone(), AskForApproval::OnFailure);
        let cwd = PathBuf::from(".");

        let err = orchestrator.run("echo hello", &cwd).await.unwrap_err();
        assert!(matches!(err, SandboxError::ApprovalRequired { .. }));

        manager.record_approval(
            CommandApprovalKey::new("echo hello".to_string(), cwd.clone()),
            ReviewDecision::ApprovedForSession,
        );

        let result = orchestrator.run("echo hello", &cwd).await.unwrap();
        assert!(result.output.contains("hello"));
    }
}
