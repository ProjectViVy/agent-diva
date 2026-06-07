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

pub mod approval;
pub mod decision;
pub mod error;
pub mod exec_policy;
pub mod filesystem;
pub mod guardian;
pub mod manager;
pub mod orchestrator;
pub mod platform;
pub mod policy;
pub mod rules;

// Re-export public API
pub use approval::{
    ApprovalStore, CommandApprovalKey, ExecApprovalRequirement, ReviewDecision, SharedApprovalStore,
};
pub use decision::{Decision, Evaluation, RuleMatch};
pub use error::{SandboxError, SandboxResult};
pub use exec_policy::{
    is_banned_prefix, ApprovalRequirement, ExecPolicyAmendment, ExecPolicyError, ExecPolicyManager,
    BANNED_PREFIX_SUGGESTIONS,
};
pub use filesystem::{
    default_protected_paths, FileSystemAccessMode, FileSystemSandboxEntry, FileSystemSandboxKind,
    FileSystemSandboxPolicy, WritableRoot,
};
pub use guardian::{
    DefaultGuardianReviewer, GuardianConfig, GuardianDecision, GuardianManager,
    GuardianRejectionCircuitBreaker, GuardianReviewer,
};
pub use manager::{SandboxCommand, SandboxExecRequest, SandboxManager};
pub use orchestrator::{
    Approvable, OrchestratorRunResult, SandboxAttempt, SandboxOverride, SandboxPermissions,
    Sandboxable, ToolOrchestrator,
};
pub use policy::{AskForApproval, NetworkAccess, ReadOnlyAccess, SandboxMode, SandboxPolicy};
pub use rules::{Policy, PrefixRule, RulesFile};

/// Environment variable to completely disable sandbox
pub const ENV_SANDBOX_DISABLED: &str = "AGENT_DIVA_SANDBOX_DISABLED";

/// Check if sandbox is disabled via environment variable
pub fn is_sandbox_disabled() -> bool {
    std::env::var(ENV_SANDBOX_DISABLED)
        .map(|v| v.eq_ignore_ascii_case("1") || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}
