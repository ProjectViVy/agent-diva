//! Sandbox manager - coordinates policy, approval, and platform execution
//!
//! The SandboxManager is the main entry point for sandbox execution.

use crate::approval::{
    ApprovalStore, CommandApprovalKey, ExecApprovalRequirement, ReviewDecision, SharedApprovalStore,
};
use crate::error::{SandboxError, SandboxResult};
use crate::filesystem::{
    FileSystemAccessMode, FileSystemSandboxEntry, FileSystemSandboxPolicy, WritableRoot,
};
use crate::is_sandbox_disabled;
use crate::platform::{current_platform_sandbox_type, SandboxType};
use crate::policy::{AskForApproval, SandboxMode, SandboxPolicy, WindowsSandboxLevel};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

#[cfg(windows)]
use crate::platform::windows::WindowsSandboxExecutor;

#[cfg(target_os = "linux")]
use crate::platform::linux::LinuxSandboxExecutor;

#[cfg(target_os = "macos")]
use crate::platform::macos::MacOsSandboxExecutor;

/// Sandbox command representation
#[derive(Debug, Clone)]
pub struct SandboxCommand {
    /// Program to execute
    pub program: String,
    /// Arguments to pass
    pub args: Vec<String>,
    /// Working directory
    pub cwd: PathBuf,
    /// Environment variables
    pub env: HashMap<String, String>,
}

impl SandboxCommand {
    /// Create a new sandbox command
    pub fn new(program: String, args: Vec<String>, cwd: PathBuf) -> Self {
        Self {
            program,
            args,
            cwd,
            env: HashMap::new(),
        }
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: String, value: String) -> Self {
        self.env.insert(key, value);
        self
    }

    /// Convert to command string
    pub fn to_command_string(&self) -> String {
        if cfg!(windows) {
            self.to_powershell_command_string()
        } else {
            self.to_posix_command_string()
        }
    }

    fn to_posix_command_string(&self) -> String {
        std::iter::once(self.program.as_str())
            .chain(self.args.iter().map(String::as_str))
            .map(shell_words::quote)
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn to_powershell_command_string(&self) -> String {
        let mut parts = Vec::with_capacity(self.args.len() + 1);
        parts.push(format!("& {}", quote_for_powershell(&self.program)));
        parts.extend(self.args.iter().map(|arg| quote_for_powershell(arg)));
        parts.join(" ")
    }
}

fn quote_for_powershell(input: &str) -> String {
    format!("'{}'", input.replace('\'', "''"))
}

/// Sandbox execution request
#[derive(Debug)]
pub struct SandboxExecRequest {
    /// Command to execute (as string)
    pub command: String,
    /// Working directory
    pub cwd: PathBuf,
    /// Environment variables
    pub env: HashMap<String, String>,
    /// Timeout in seconds
    pub timeout_secs: u64,
}

impl SandboxExecRequest {
    /// Create a new execution request
    pub fn new(command: String, cwd: PathBuf) -> Self {
        Self {
            command,
            cwd,
            env: HashMap::new(),
            timeout_secs: 60,
        }
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
}

/// Sandbox configuration for manager initialization
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SandboxConfig {
    /// Sandbox mode
    pub mode: SandboxMode,
    /// Windows sandbox level
    pub windows_level: WindowsSandboxLevel,
    /// Network access allowed
    pub network_access: bool,
    /// Approval policy
    pub approval_policy: AskForApproval,
    /// Additional writable roots
    pub writable_roots: Vec<PathBuf>,
    /// Protected paths (glob patterns)
    pub protected_paths: Vec<String>,
    /// Patterns for commands that should be denied
    #[serde(default)]
    pub deny_patterns: Vec<String>,
    /// Timeout for commands (in seconds)
    #[serde(alias = "default_timeout")]
    pub timeout_seconds: u64,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            mode: SandboxMode::default(),
            windows_level: WindowsSandboxLevel::default(),
            network_access: false,
            approval_policy: AskForApproval::default(),
            writable_roots: Vec::new(),
            protected_paths: crate::filesystem::default_protected_paths(),
            deny_patterns: Vec::new(),
            timeout_seconds: 60,
        }
    }
}

impl SandboxConfig {
    /// Create config for danger-full-access mode (no sandbox)
    pub fn danger_full_access() -> Self {
        Self {
            mode: SandboxMode::DangerFullAccess,
            windows_level: WindowsSandboxLevel::Disabled,
            network_access: true,
            approval_policy: AskForApproval::Never,
            writable_roots: Vec::new(),
            protected_paths: Vec::new(),
            deny_patterns: Vec::new(),
            timeout_seconds: 60,
        }
    }

    /// Convert from agent-diva-core SandboxConfig
    pub fn from_core_config(core_config: &agent_diva_core::config::SandboxConfig) -> Self {
        use agent_diva_core::config::{
            AskForApproval as CoreApproval, SandboxMode as CoreMode,
            WindowsSandboxLevel as CoreLevel,
        };

        let mode = match core_config.mode {
            CoreMode::DangerFullAccess => SandboxMode::DangerFullAccess,
            CoreMode::ReadOnly => SandboxMode::ReadOnly,
            CoreMode::WorkspaceWrite => SandboxMode::WorkspaceWrite,
        };

        let approval_policy = match core_config.approval_policy {
            CoreApproval::Never => AskForApproval::Never,
            CoreApproval::OnFailure => AskForApproval::OnFailure,
            CoreApproval::OnRequest => AskForApproval::OnRequest,
            CoreApproval::UnlessTrusted => AskForApproval::UnlessTrusted,
        };

        let windows_level = match core_config.windows_level {
            CoreLevel::Disabled => WindowsSandboxLevel::Disabled,
            CoreLevel::RestrictedToken => WindowsSandboxLevel::RestrictedToken,
            CoreLevel::Elevated => WindowsSandboxLevel::Elevated,
        };

        Self {
            mode,
            windows_level,
            network_access: core_config.network_access,
            approval_policy,
            writable_roots: core_config
                .writable_roots
                .iter()
                .map(PathBuf::from)
                .collect(),
            protected_paths: core_config.protected_paths.clone(),
            deny_patterns: core_config.deny_patterns.clone(),
            timeout_seconds: core_config.timeout_seconds,
        }
    }

    /// Create config for workspace-write mode
    pub fn workspace_write(workspace: PathBuf) -> Self {
        Self {
            mode: SandboxMode::WorkspaceWrite,
            windows_level: WindowsSandboxLevel::RestrictedToken,
            network_access: false,
            approval_policy: AskForApproval::OnFailure,
            writable_roots: vec![workspace],
            protected_paths: crate::filesystem::default_protected_paths(),
            deny_patterns: Vec::new(),
            timeout_seconds: 60,
        }
    }
}

/// Sandbox manager - coordinates all sandbox components
pub struct SandboxManager {
    /// Sandbox policy
    policy: SandboxPolicy,
    /// File system policy
    fs_policy: FileSystemSandboxPolicy,
    /// Windows sandbox level
    windows_level: WindowsSandboxLevel,
    /// Approval policy
    approval_policy: AskForApproval,
    /// Approval store for caching decisions
    approval_store: SharedApprovalStore,
    /// Default timeout (used when request has no timeout specified)
    #[allow(dead_code)]
    default_timeout: u64,
    /// Whether sandbox is disabled
    disabled: bool,
}

impl SandboxManager {
    /// Create a new sandbox manager with configuration
    pub fn new(config: &SandboxConfig) -> Self {
        Self::new_with_approval_store(config, ApprovalStore::new_shared())
    }

    /// Create a new sandbox manager with a shared approval store.
    pub fn new_with_approval_store(
        config: &SandboxConfig,
        approval_store: SharedApprovalStore,
    ) -> Self {
        let disabled = is_sandbox_disabled() || config.mode == SandboxMode::DangerFullAccess;

        // Build policy from config
        let policy = if disabled {
            SandboxPolicy::DangerFullAccess
        } else {
            config.mode.to_policy(
                config
                    .writable_roots
                    .first()
                    .cloned()
                    .unwrap_or_else(|| PathBuf::from(".")),
            )
        };

        // Build file system policy
        let fs_policy = if disabled {
            FileSystemSandboxPolicy::unrestricted()
        } else {
            Self::build_fs_policy(config)
        };

        Self {
            policy,
            fs_policy,
            windows_level: config.windows_level,
            approval_policy: config.approval_policy,
            approval_store,
            default_timeout: config.timeout_seconds,
            disabled,
        }
    }

    /// Build file system policy from configuration
    fn build_fs_policy(config: &SandboxConfig) -> FileSystemSandboxPolicy {
        let mut entries = Vec::new();

        // Add writable roots
        for root in &config.writable_roots {
            let writable_root = WritableRoot::new(root.clone());
            entries.push(FileSystemSandboxEntry::new(
                crate::filesystem::FileSystemPath::from_path(writable_root.root.clone()),
                FileSystemAccessMode::Write,
            ));
        }

        // Add read-only access for cwd by default
        entries.push(FileSystemSandboxEntry::new(
            crate::filesystem::FileSystemPath::cwd(),
            FileSystemAccessMode::Read,
        ));

        FileSystemSandboxPolicy::restricted(entries)
    }

    /// Create with default configuration
    pub fn with_defaults() -> Self {
        Self::new(&SandboxConfig::default())
    }

    /// Create disabled (danger-full-access) manager
    pub fn disabled() -> Self {
        Self::new(&SandboxConfig::danger_full_access())
    }

    /// Check if sandbox is disabled
    pub fn is_disabled(&self) -> bool {
        self.disabled
    }

    /// Get current sandbox policy
    pub fn policy(&self) -> &SandboxPolicy {
        &self.policy
    }

    /// Get file system policy
    pub fn fs_policy(&self) -> &FileSystemSandboxPolicy {
        &self.fs_policy
    }

    /// Get approval policy
    pub fn approval_policy(&self) -> AskForApproval {
        self.approval_policy
    }

    /// Check approval requirement for a command
    pub fn check_approval_requirement(&self, command: &str, cwd: &Path) -> ExecApprovalRequirement {
        // If sandbox is disabled, always skip
        if self.disabled {
            return ExecApprovalRequirement::skip_bypass_sandbox();
        }

        // Check approval cache first
        let key = CommandApprovalKey::new(command.to_string(), cwd.to_path_buf());
        {
            let store = self.approval_store.lock().unwrap();
            if store.is_approved_for_session(&key) {
                return ExecApprovalRequirement::skip();
            }
            if store.is_denied(&key) {
                return ExecApprovalRequirement::forbidden("Previously denied by user".to_string());
            }
        }

        // Check approval policy
        match self.approval_policy {
            AskForApproval::Never => ExecApprovalRequirement::skip(),
            AskForApproval::OnFailure => ExecApprovalRequirement::skip(),
            AskForApproval::OnRequest => ExecApprovalRequirement::needs_approval(None),
            AskForApproval::UnlessTrusted => {
                // Check if command is "trusted" - for now, prompt for all
                ExecApprovalRequirement::needs_approval(Some("Untrusted command".to_string()))
            }
        }
    }

    /// Check if a path is allowed for reading
    pub fn can_read_path(&self, path: &Path, cwd: &Path) -> bool {
        if self.disabled {
            return true;
        }
        self.fs_policy.can_read_path_with_cwd(path, cwd)
    }

    /// Check if a path is allowed for writing
    pub fn can_write_path(&self, path: &Path, cwd: &Path) -> bool {
        if self.disabled {
            return true;
        }
        self.fs_policy.can_write_path_with_cwd(path, cwd)
    }

    /// Execute a command in the sandbox
    pub async fn execute_sandboxed(&self, request: &SandboxExecRequest) -> SandboxResult<String> {
        info!(
            "Executing sandboxed command: '{}' in {:?}",
            request.command, request.cwd
        );

        // If sandbox is disabled, execute directly
        if self.disabled {
            return self.execute_direct(request).await;
        }

        // Check approval requirement
        let approval = self.check_approval_requirement(&request.command, &request.cwd);

        match approval {
            ExecApprovalRequirement::Forbidden { reason } => {
                return Err(SandboxError::Denied { reason });
            }
            ExecApprovalRequirement::NeedsApproval { reason } => {
                // For now, return error - GUI needs to handle approval
                return Err(SandboxError::ApprovalRequired {
                    reason: reason.unwrap_or_else(|| "Approval required".to_string()),
                });
            }
            ExecApprovalRequirement::Skip { bypass_sandbox } => {
                if bypass_sandbox {
                    return self.execute_direct(request).await;
                }
            }
        }

        // Execute with sandbox
        self.execute_with_platform(request).await
    }

    /// Execute without sandbox (direct execution)
    async fn execute_direct(&self, request: &SandboxExecRequest) -> SandboxResult<String> {
        use std::time::Duration;
        use tokio::process::Command;
        use tokio::time::timeout;

        debug!("Executing directly: {}", request.command);

        let (shell, args) = if cfg!(target_os = "windows") {
            (
                "powershell",
                vec!["-NoProfile", "-NonInteractive", "-Command"],
            )
        } else {
            ("sh", vec!["-c"])
        };

        let mut cmd = Command::new(shell);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.arg(&request.command)
            .current_dir(&request.cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        for (key, value) in &request.env {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .map_err(|e| SandboxError::SpawnFailed(e.to_string()))?;

        let output_future = child.wait_with_output();
        let output_result = timeout(Duration::from_secs(request.timeout_secs), output_future).await;

        let output = match output_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(SandboxError::SpawnFailed(e.to_string())),
            Err(_) => {
                return Err(SandboxError::Timeout {
                    secs: request.timeout_secs,
                })
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if output.status.success() {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            let code = output.status.code().unwrap_or(-1);
            Ok(format!(
                "Exit code: {}\nstdout: {}\nstderr: {}",
                code, stdout, stderr
            ))
        }
    }

    /// Execute without sandbox after an explicit bypass/escalation decision.
    pub async fn execute_unsandboxed(&self, request: &SandboxExecRequest) -> SandboxResult<String> {
        self.execute_direct(request).await
    }

    /// Execute with platform-specific sandbox
    async fn execute_with_platform(&self, request: &SandboxExecRequest) -> SandboxResult<String> {
        #[cfg(windows)]
        {
            let executor = WindowsSandboxExecutor::new(self.windows_level);
            if executor.is_available() {
                executor
                    .execute(
                        &request.command,
                        &request.cwd,
                        request.env.clone(),
                        request.timeout_secs,
                        &self.policy,
                        &self.fs_policy,
                    )
                    .await
            } else {
                Err(SandboxError::PlatformError(
                    "Windows sandbox is unavailable for the configured level".to_string(),
                ))
            }
        }

        #[cfg(target_os = "linux")]
        {
            let executor = LinuxSandboxExecutor::new();
            if executor.is_available() {
                executor
                    .execute(
                        &request.command,
                        &request.cwd,
                        request.env.clone(),
                        request.timeout_secs,
                        &self.policy,
                        &self.fs_policy,
                    )
                    .await
            } else {
                Err(SandboxError::PlatformError(
                    "Linux sandbox is unavailable for the configured level".to_string(),
                ))
            }
        }

        #[cfg(target_os = "macos")]
        {
            let executor = MacOsSandboxExecutor::new();
            if executor.is_available() {
                executor
                    .execute(
                        &request.command,
                        &request.cwd,
                        request.env.clone(),
                        request.timeout_secs,
                        &self.policy,
                        &self.fs_policy,
                    )
                    .await
            } else {
                Err(SandboxError::PlatformError(
                    "macOS sandbox is unavailable for the configured level".to_string(),
                ))
            }
        }

        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        {
            let _ = request;
            Err(SandboxError::PlatformNotSupported)
        }
    }

    /// Record an approval decision
    pub fn record_approval(&self, key: CommandApprovalKey, decision: ReviewDecision) {
        let mut store = self.approval_store.lock().unwrap();
        store.put(key, decision);
    }

    /// Get the approval store
    pub fn approval_store(&self) -> SharedApprovalStore {
        self.approval_store.clone()
    }

    /// Get current platform sandbox type
    pub fn sandbox_type(&self) -> SandboxType {
        if self.disabled {
            SandboxType::None
        } else {
            current_platform_sandbox_type()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_manager_default() {
        let manager = SandboxManager::with_defaults();
        assert!(!manager.is_disabled());
    }

    #[test]
    fn test_sandbox_manager_disabled() {
        let manager = SandboxManager::disabled();
        assert!(manager.is_disabled());
    }

    #[test]
    fn test_sandbox_config_default() {
        let config = SandboxConfig::default();
        assert!(matches!(config.mode, SandboxMode::ReadOnly));
        assert!(!config.network_access);
    }

    #[test]
    fn test_sandbox_config_danger_full_access() {
        let config = SandboxConfig::danger_full_access();
        assert!(matches!(config.mode, SandboxMode::DangerFullAccess));
        assert!(config.network_access);
    }

    #[test]
    fn test_sandbox_command() {
        let cmd = SandboxCommand::new(
            "cargo".to_string(),
            vec!["build".to_string()],
            PathBuf::from("."),
        );
        let expected = if cfg!(windows) {
            "& 'cargo' 'build'"
        } else {
            "cargo build"
        };
        assert_eq!(cmd.to_command_string(), expected);
    }

    #[test]
    fn test_sandbox_command_quotes_metacharacters() {
        let dangerous = "value;|&$`<>".to_string();
        let cmd = SandboxCommand::new(
            "echo".to_string(),
            vec![dangerous.clone(), "with space".to_string()],
            PathBuf::from("."),
        );

        if cfg!(windows) {
            assert_eq!(
                cmd.to_command_string(),
                "& 'echo' 'value;|&$`<>' 'with space'"
            );
        } else {
            let parsed = shell_words::split(&cmd.to_command_string()).unwrap();
            assert_eq!(
                parsed,
                vec!["echo".to_string(), dangerous, "with space".to_string()]
            );
        }
    }

    #[test]
    fn test_approval_requirement_disabled() {
        let manager = SandboxManager::disabled();
        let cwd = PathBuf::from(".");
        let approval = manager.check_approval_requirement("ls", &cwd);
        assert!(approval.bypass_sandbox());
    }

    #[tokio::test]
    async fn test_execute_direct() {
        let manager = SandboxManager::disabled();
        let request = SandboxExecRequest::new("echo hello".to_string(), PathBuf::from("."));

        let result = manager.execute_direct(&request).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("hello"));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn test_to_command_string_prevents_shell_injection() {
        let manager = SandboxManager::disabled();
        let dangerous_arg = "safe; printf injected".to_string();
        let command = SandboxCommand::new(
            "printf".to_string(),
            vec!["%s".to_string(), dangerous_arg.clone()],
            PathBuf::from("."),
        );
        let request = SandboxExecRequest::new(command.to_command_string(), PathBuf::from("."));

        let output = manager.execute_direct(&request).await.unwrap();
        assert_eq!(output, dangerous_arg);
        assert!(!output.contains("injected"));
    }

    #[cfg(windows)]
    #[tokio::test]
    async fn test_to_command_string_prevents_powershell_injection() {
        let manager = SandboxManager::disabled();
        let dangerous_arg = "safe; Write-Output injected".to_string();
        let command = SandboxCommand::new(
            "Write-Output".to_string(),
            vec![dangerous_arg.clone()],
            PathBuf::from("."),
        );
        let request = SandboxExecRequest::new(command.to_command_string(), PathBuf::from("."));

        let output = manager.execute_direct(&request).await.unwrap();
        assert_eq!(output.trim(), dangerous_arg);
        assert!(!output.contains("\ninjected"));
    }
}
