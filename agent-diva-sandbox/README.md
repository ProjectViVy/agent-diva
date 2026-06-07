# agent-diva-sandbox

Process isolation for shell command execution, inspired by OpenAI Codex CLI's sandbox architecture.

## Overview

This crate provides sandbox isolation for shell command execution in agent-diva. It supports multiple platforms with different isolation mechanisms:

- **Windows**: Restricted Token with LUA_TOKEN and WRITE_RESTRICTED flags
- **Linux**: Landlock LSM + Seccomp-BPF + Bubblewrap (bwrap)
- **macOS**: Seatbelt (sandbox-exec) with .sbpl policy files

## Features

### Core Capabilities (Phase 1 MVP)

- **Sandbox Policy Types**: `DangerFullAccess`, `ReadOnly`, `WorkspaceWrite`, `ExternalSandbox`
- **FileSystem Access Control**: Read/write/none modes with path-based rules
- **Protected Paths**: `.git`, `.diva`, `.agents`, `.env`, `*.pem`, `*.key` are read-only by default
- **Approval Cache**: Session-level caching for user approval decisions
- **WritableRoot**: Workspace directories with protected subpath exclusion

### Extended Capabilities (Phase 2)

- **Linux Landlock**: Kernel-level filesystem access control (ABI V1/V2/V3)
- **Linux Seccomp**: Network syscall filtering with configurable modes
- **Linux Bubblewrap**: Filesystem namespace isolation with bwrap
- **macOS Seatbelt**: sandbox-exec with dynamic policy generation
- **ExecPolicy**: Rule-based command approval system
- **Guardian**: Automatic approval integration with circuit breaker safety
- **ToolOrchestrator**: Full execution orchestration flow

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     ToolOrchestrator                             │
│  ┌─────────────────┐  ┌──────────────────┐  ┌─────────────────┐ │
│  │ GuardianManager │→ │ ExecPolicyManager│→ │ SandboxManager  │ │
│  │ (Auto-Approval) │  │ (Rule Evaluation)│  │ (Policy Apply)  │ │
│  └─────────────────┘  └──────────────────┘  └─────────────────┘ │
│                              ↓                                   │
│  ┌─────────────────────────────────────────────────────────────┐│
│  │                   Platform Executors                         ││
│  │  ┌──────────────┐  ┌──────────────┐  ┌─────────────────────┐││
│  │  │WindowsSandbox│  │LinuxSandbox  │  │MacOsSandboxExecutor │││
│  │  │(Restricted   │  │(Landlock+    │  │(sandbox-exec +      │││
│  │  │Token)        │  │Seccomp+bwrap)│  │Seatbelt)            │││
│  │  └──────────────┘  └──────────────┘  └─────────────────────┘││
│  └─────────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────────┘
```

## Quick Start

```rust
use agent_diva_sandbox::{SandboxManager, SandboxConfig, SandboxMode};

// Create sandbox manager with workspace-write mode
let config = SandboxConfig::workspace_write(PathBuf::from("/workspace"));
let manager = SandboxManager::new(&config);

// Execute a command in sandbox
let request = SandboxExecRequest::new("cargo build".to_string(), PathBuf::from("/workspace"));
let result = manager.execute_sandboxed(&request).await;
```

## Policy Types

### SandboxMode (Configuration-Friendly)

```rust
pub enum SandboxMode {
    DangerFullAccess,  // No sandbox - direct execution
    ReadOnly,          // Read-only filesystem access
    WorkspaceWrite,    // Write access to workspace directory (default)
}
```

### SandboxPolicy (Full Control)

```rust
pub enum SandboxPolicy {
    DangerFullAccess,
    ReadOnly {
        access: ReadOnlyAccess,
        network_access: bool,
    },
    WorkspaceWrite {
        writable_roots: Vec<PathBuf>,
        read_only_access: ReadOnlyAccess,
        network_access: bool,
        exclude_tmpdir_env_var: bool,
    },
    ExternalSandbox {
        network_access: NetworkAccess,
    },
}
```

## FileSystem Policy

### FileSystemSandboxPolicy

```rust
let policy = FileSystemSandboxPolicy::restricted(vec![
    FileSystemSandboxEntry::new(
        FileSystemPath::from_path(PathBuf::from("/workspace")),
        FileSystemAccessMode::Write,
    ),
    FileSystemSandboxEntry::new(
        FileSystemPath::cwd(),
        FileSystemAccessMode::Read,
    ),
]);
```

### WritableRoot with Protected Subpaths

```rust
let root = WritableRoot::new(PathBuf::from("/workspace"));
// Automatically adds .git, .diva, .agents as read-only subpaths
assert!(root.is_path_writable(Path::new("/workspace/src/main.rs")));
assert!(!root.is_path_writable(Path::new("/workspace/.git/config")));
```

## Approval System

### AskForApproval Policy

```rust
pub enum AskForApproval {
    Never,        // Never request approval, execute directly
    OnFailure,    // Request approval after sandbox failure (default)
    OnRequest,    // LLM decides when to request approval
    UnlessTrusted, // Request approval for untrusted commands
}
```

### ApprovalStore

```rust
let store = ApprovalStore::new_shared();
let key = CommandApprovalKey::new("cargo build".to_string(), cwd);

// Record approval
store.put(key.clone(), ReviewDecision::ApprovedForSession);

// Check approval
if store.is_approved_for_session(&key) {
    // Skip approval prompt
}
```

## ExecPolicy (Rule-Based Approval)

### Rules File Format (TOML)

```toml
[[prefix_rules]]
pattern = ["git", "status"]
decision = "Allow"

[[prefix_rules]]
pattern = ["git", "checkout"]
decision = "Prompt"

[[prefix_rules]]
pattern = ["rm", "-rf"]
decision = "Forbidden"
```

### ExecPolicyManager

```rust
let manager = ExecPolicyManager::from_file(PathBuf::from(".diva/exec.rules"))?;

let requirement = manager.get_approval_requirement(
    &["git".to_string(), "status".to_string()],
    AskForApproval::OnFailure,
);

match requirement {
    ApprovalRequirement::Skip { .. } => { /* Execute directly */ },
    ApprovalRequirement::NeedsApproval { reason, amendment } => { /* Prompt user */ },
    ApprovalRequirement::Forbidden { reason } => { /* Deny execution */ },
}
```

### BANNED_PREFIX_SUGGESTIONS

The system maintains a list of 44 dangerous command prefixes that cannot be auto-suggested as Allow rules:

- Python interpreters: `python`, `python3`, `python -c`
- Shell environments: `bash`, `sh`, `zsh`, `/bin/bash`
- Privilege escalation: `sudo`, `su`, `doas`
- Node.js: `node`, `node -e`
- PowerShell: `powershell`, `pwsh -Command`
- Other interpreters: `perl -e`, `ruby -e`, `php -r`, `lua -e`

## Guardian (Auto-Approval)

### GuardianConfig

```rust
let config = GuardianConfig {
    max_consecutive_rejections: 5,    // Circuit breaker threshold
    rejection_window_secs: 60,        // Time window for counting
    auto_approve_known_safe: true,    // Auto-approve if Allow rule exists
    auto_approve_read_only: false,    // Auto-approve read-only commands
    enable_auto_learning: true,       // Create Allow rules from approvals
    min_execution_time_for_approval_ms: 100,
};
```

### GuardianManager

```rust
let guardian = GuardianManager::with_default_reviewer(
    GuardianConfig::default(),
    AskForApproval::OnRequest,
);

let decision = guardian.review(
    &["git".to_string(), "status".to_string()],
    &cwd,
    &approval_requirement,
);

match decision {
    GuardianDecision::AutoApprove { session_approval, create_rule } => { /* Execute */ },
    GuardianDecision::RequireApproval { reason } => { /* Prompt */ },
    GuardianDecision::Denied { reason } => { /* Block */ },
    GuardianDecision::Defer => { /* Use default policy */ },
}
```

### CircuitBreaker

The `GuardianRejectionCircuitBreaker` prevents auto-approval spam when too many commands are denied:

- Triggers after `max_consecutive_rejections` denials within `rejection_window_secs`
- Blocks auto-approve decisions when triggered
- Resets after successful approval

## ToolOrchestrator

The full orchestration flow:

```
1. Guardian Auto-Approval Phase
   ├─ Check circuit breaker state
   ├─ Review command against GuardianConfig
   └─ Return AutoApprove/RequireApproval/Denied/Defer

2. Approval Phase
   ├─ Check ExecPolicy rules
   ├─ Evaluate ApprovalRequirement
   └─ Skip/NeedsApproval/Forbidden

3. First Attempt
   ├─ Create SandboxAttempt with policy
   ├─ Execute with platform sandbox

4. Sandbox Denial Handling
   ├─ Check escalation eligibility
   ├─ Verify not banned prefix
   └─ Create retry attempt without sandbox

5. Retry Attempt (if escalated)
   └ Execute with bypass_sandbox flag
```

### Usage

```rust
let orchestrator = ToolOrchestrator::with_exec_policy_and_guardian(
    sandbox_manager,
    AskForApproval::OnFailure,
    exec_policy,
    guardian,
);

let result = orchestrator.run("cargo build", &cwd).await?;

println!("Success: {}", result.success);
println!("Used sandbox: {}", result.used_sandbox);
println!("Amendment: {:?}", result.amendment);
```

## Platform Implementations

### Windows (Restricted Token)

Uses Windows API `CreateRestrictedToken` with:
- `DISABLE_MAX_PRIVILEGE`: Removes elevated privileges
- `LUA_TOKEN`: Least-privileged User Account token
- `WRITE_RESTRICTED`: Restricts write access

```rust
let executor = WindowsSandboxExecutor::new(WindowsSandboxLevel::RestrictedToken);
let result = executor.execute(&command, &cwd, env, 60, &policy, &fs_policy).await;
```

### Linux (Landlock + Seccomp + Bubblewrap)

- **Landlock LSM**: Kernel-level filesystem ACL (kernel 5.13+)
- **Seccomp-BPF**: Syscall filtering for network operations
- **Bubblewrap**: User namespace isolation with filesystem binds

```rust
// Check support
if is_landlock_supported() {
    let ruleset = build_landlock_from_fs_policy(&fs_policy, &cwd)?;
    ruleset.restrict_current_thread()?;
}

// Execute with bwrap
let executor = LinuxSandboxExecutor::with_options(BwrapOptions {
    mount_proc: true,
    network_mode: BwrapNetworkMode::Isolated,
});
```

#### WSL1 Detection

WSL1 cannot use bubblewrap due to lack of user namespace support:

```rust
if is_wsl1() {
    // Fallback to direct execution or warn
}
```

### macOS (Seatbelt)

Uses `sandbox-exec` with dynamically generated `.sbpl` policy:

```rust
let policy = create_seatbelt_policy(&sandbox_policy, &fs_policy, &cwd);
let executor = MacOsSandboxExecutor::new();
let result = executor.execute(&command, &cwd, env, 60, &policy, &fs_policy).await;
```

## Environment Variables

- `AGENT_DIVA_SANDBOX_DISABLED=1` or `true`: Completely disable sandbox

## Error Handling

```rust
pub enum SandboxError {
    Denied { reason: String },
    SpawnFailed(String),
    Timeout { secs: u64 },
    TokenCreation(String),
    PlatformError(String),
    InvalidCommand(String),
    Internal(String),
}
```

## Testing

```bash
# Run all tests
cargo test -p agent-diva-sandbox

# Run specific test
cargo test -p agent-diva-sandbox test_writable_root

# Run with output
cargo test -p agent-diva-sandbox -- --nocapture
```

## Dependencies

- `tokio`: Async runtime
- `serde/serde_json`: Serialization
- `globset`: Glob pattern matching for protected paths
- `shell-words`: Command parsing
- `toml`: ExecPolicy rules file parsing
- `parking_lot`: ArcSwap for ExecPolicy

### Platform-Specific

- **Windows**: `windows` crate (Win32 Security/Threading)
- **Linux**: `landlock` + `seccompiler` crates

## License

MIT