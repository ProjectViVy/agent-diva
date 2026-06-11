//! Agent Diva Sandbox - Process isolation for command execution
//!
//! This crate provides sandbox isolation for shell command execution,
//! inspired by OpenAI Codex CLI's sandbox architecture.
//!
//! # Features (Phase 1 MVP)
//! - Windows Restricted Token isolation
//! - FileSystem access control (read/write/none)
//! - Protected paths (.git, .diva, .env)
//! - Approval cache for user decisions
//!
//! # Features (Phase 2)
//! - Linux Bubblewrap/Landlock/Seccomp isolation
//! - ExecPolicy rule-based command approval
//! - BANNED_PREFIX_SUGGESTIONS safety limits
//! - ToolOrchestrator execution orchestration
//! - Guardian automatic approval system
//!
//! # Example
//! ```ignore
//! use agent_diva_sandbox::{SandboxManager, SandboxPolicy};
//!
//! let manager = SandboxManager::new(&config);
//! let result = manager.execute_sandboxed("echo hello", &cwd).await;
//! ```

#[cfg(any(feature = "approval", feature = "manager", feature = "orchestrator"))]
pub mod approval;
pub mod decision;
pub mod error;
#[cfg(any(feature = "guardian", feature = "manager", feature = "orchestrator"))]
pub mod exec_policy;
#[cfg(any(
    feature = "filesystem",
    feature = "manager",
    feature = "orchestrator",
    feature = "platform"
))]
pub mod filesystem;
#[cfg(any(feature = "guardian", feature = "manager", feature = "orchestrator"))]
pub mod guardian;
#[cfg(feature = "manager")]
pub mod manager;
#[cfg(any(feature = "manager", feature = "orchestrator"))]
pub mod orchestrator;
#[cfg(feature = "platform")]
pub mod platform;
pub mod policy;
pub mod rules;

// Re-export public API
#[cfg(any(feature = "approval", feature = "manager", feature = "orchestrator"))]
pub use approval::{
    ApprovalStore, CommandApprovalKey, ExecApprovalRequirement, ReviewDecision, SharedApprovalStore,
};
pub use decision::{Decision, Evaluation, RuleMatch};
pub use error::{SandboxError, SandboxResult};
#[cfg(any(feature = "guardian", feature = "manager", feature = "orchestrator"))]
pub use exec_policy::{
    is_banned_prefix, ApprovalRequirement, ExecPolicyAmendment, ExecPolicyError, ExecPolicyManager,
    BANNED_PREFIX_SUGGESTIONS,
};
#[cfg(any(
    feature = "filesystem",
    feature = "manager",
    feature = "orchestrator",
    feature = "platform"
))]
pub use filesystem::{
    default_protected_paths, FileSystemAccessMode, FileSystemSandboxEntry, FileSystemSandboxKind,
    FileSystemSandboxPolicy, WritableRoot,
};
#[cfg(any(feature = "guardian", feature = "manager", feature = "orchestrator"))]
pub use guardian::{
    DefaultGuardianReviewer, GuardianConfig, GuardianDecision, GuardianManager,
    GuardianRejectionCircuitBreaker, GuardianReviewer,
};
#[cfg(feature = "manager")]
pub use manager::{SandboxCommand, SandboxExecRequest, SandboxManager};
#[cfg(any(feature = "manager", feature = "orchestrator"))]
pub use orchestrator::{
    Approvable, OrchestratorRunResult, SandboxAttempt, SandboxOverride, SandboxPermissions,
    Sandboxable, ToolOrchestrator,
};
pub use policy::{
    AskForApproval, NetworkAccess, ReadOnlyAccess, SandboxMode, SandboxPolicy,
    SecurityPolicySandboxExt,
};
pub use rules::{Policy, PrefixRule, RulesFile};

/// Environment variable to completely disable sandbox
pub const ENV_SANDBOX_DISABLED: &str = "AGENT_DIVA_SANDBOX_DISABLED";

/// Check if sandbox is disabled via environment variable
pub fn is_sandbox_disabled() -> bool {
    std::env::var(ENV_SANDBOX_DISABLED)
        .map(|v| v.eq_ignore_ascii_case("1") || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
