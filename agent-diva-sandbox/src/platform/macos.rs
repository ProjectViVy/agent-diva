//! macOS sandbox implementation using Seatbelt (sandbox-exec)
//!
//! This module implements process isolation on macOS using:
//! - Seatbelt (sandbox-exec) for process sandboxing
//! - .sbpl (Seatbelt Profile Language) policy files
//!
//! Inspired by OpenAI Codex CLI's seatbelt.rs architecture.

use crate::error::{SandboxError, SandboxResult};
use crate::filesystem::{FileSystemSandboxKind, FileSystemSandboxPolicy, WritableRoot};
use crate::policy::{ReadOnlyAccess, SandboxPolicy};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;
use tracing::{debug, info, warn};

// ============================================================================
// Constants
// ============================================================================

/// Platform default read-only paths (system binaries and libraries)
pub const MACOS_PLATFORM_DEFAULT_READ_ROOTS: &[&str] = &[
    "/bin",
    "/sbin",
    "/usr",
    "/System",
    "/Library",
    "/Applications",
];

/// Seatbelt base policy template
const SEATBELT_BASE_POLICY: &str = r#"
(version 1)
(debug deny)

; Allow basic process operations
(allow process-exec (literal "/bin/sh"))
(allow process-exec (literal "/bin/bash"))
(allow process-exec (literal "/bin/zsh"))
(allow process-exec (literal "/usr/bin/env"))

; Allow file operations on system paths
(allow file-read* (subpath "/usr"))
(allow file-read* (subpath "/System"))
(allow file-read* (subpath "/Library"))
(allow file-read* (subpath "/bin"))
(allow file-read* (subpath "/sbin"))

; Allow basic signals
(allow signal (target self))
(allow process-fork)
(allow process-exec)
"#;

/// Seatbelt network policy (for isolation mode)
const SEATBELT_NETWORK_ISOLATED: &str = r#"
; Deny all network operations
(deny network*)
"#;

/// Seatbelt network policy (for allowed mode)
const SEATBELT_NETWORK_ALLOWED: &str = r#"
; Allow network operations
(allow network*)
"#;

// ============================================================================
// Seatbelt Policy Generation
// ============================================================================

/// Create a Seatbelt policy string from SandboxPolicy and FileSystemSandboxPolicy
pub fn create_seatbelt_policy(
    sandbox_policy: &SandboxPolicy,
    fs_policy: &FileSystemSandboxPolicy,
    cwd: &Path,
) -> String {
    let mut policy_parts = vec![SEATBELT_BASE_POLICY.to_string()];

    // Add filesystem policy based on SandboxPolicy type
    match sandbox_policy {
        SandboxPolicy::DangerFullAccess => {
            // Allow all file operations
            policy_parts.push("(allow file*)".to_string());
            policy_parts.push("(allow file-read*)".to_string());
            policy_parts.push("(allow file-write*)".to_string());
        }
        SandboxPolicy::ReadOnly { .. } => {
            // Read-only: deny writes, allow reads
            policy_parts.push(create_read_only_policy(fs_policy, cwd));
        }
        SandboxPolicy::WorkspaceWrite {
            writable_roots,
            read_only_access,
            ..
        } => {
            // Workspace write: allow writes to specific paths
            policy_parts.push(create_workspace_write_policy(
                writable_roots,
                read_only_access,
                fs_policy,
                cwd,
            ));
        }
        SandboxPolicy::ExternalSandbox { .. } => {
            // Already in external sandbox, minimal restrictions
            policy_parts.push("(allow file*)".to_string());
        }
    }

    // Add network policy
    let network_access = match sandbox_policy {
        SandboxPolicy::DangerFullAccess => true,
        SandboxPolicy::ReadOnly { network_access, .. } => network_access,
        SandboxPolicy::WorkspaceWrite { network_access, .. } => network_access,
        SandboxPolicy::ExternalSandbox { network_access } => network_access.is_allowed(),
    };

    if network_access {
        policy_parts.push(SEATBELT_NETWORK_ALLOWED.to_string());
    } else {
        policy_parts.push(SEATBELT_NETWORK_ISOLATED.to_string());
    }

    policy_parts.join("\n")
}

/// Create read-only filesystem policy
fn create_read_only_policy(fs_policy: &FileSystemSandboxPolicy, cwd: &Path) -> String {
    let mut policy = String::new();

    if fs_policy.kind == FileSystemSandboxKind::Unrestricted {
        // Allow all reads
        policy.push_str("(allow file-read*)\n");
    } else {
        // Restricted: only allow specific paths
        // Add platform default read roots
        for root in MACOS_PLATFORM_DEFAULT_READ_ROOTS {
            policy.push_str(&format!("(allow file-read* (subpath \"{}\"))\n", root));
        }

        // Add entries from policy
        for entry in &fs_policy.entries {
            if entry.access.allows_read() {
                let path_str = resolve_entry_path(&entry.path, cwd);
                policy.push_str(&format!("(allow file-read* (literal \"{}\"))\n", path_str));
                // Also allow reading subpaths
                policy.push_str(&format!("(allow file-read* (subpath \"{}\"))\n", path_str));
            }
        }

        // Add cwd
        policy.push_str(&format!(
            "(allow file-read* (subpath \"{}\"))\n",
            cwd.display()
        ));
    }

    // Deny writes
    policy.push_str("(deny file-write*)\n");

    policy
}

/// Create workspace write policy
fn create_workspace_write_policy(
    writable_roots: &[PathBuf],
    read_only_access: &ReadOnlyAccess,
    fs_policy: &FileSystemSandboxPolicy,
    cwd: &Path,
) -> String {
    let mut policy = String::new();

    // Read access
    if read_only_access.has_full_disk_read_access() {
        policy.push_str("(allow file-read*)\n");
    } else {
        // Restricted read: add platform defaults and specific paths
        for root in MACOS_PLATFORM_DEFAULT_READ_ROOTS {
            policy.push_str(&format!("(allow file-read* (subpath \"{}\"))\n", root));
        }

        // Add cwd
        policy.push_str(&format!(
            "(allow file-read* (subpath \"{}\"))\n",
            cwd.display()
        ));

        // Add read entries from policy
        for entry in &fs_policy.entries {
            if entry.access.allows_read() {
                let path_str = resolve_entry_path(&entry.path, cwd);
                policy.push_str(&format!("(allow file-read* (subpath \"{}\"))\n", path_str));
            }
        }
    }

    // Write access to writable roots
    for root in writable_roots {
        policy.push_str(&format!(
            "(allow file-write* (subpath \"{}\"))\n",
            root.display()
        ));
        policy.push_str(&format!(
            "(allow file-write-create (subpath \"{}\"))\n",
            root.display()
        ));
        policy.push_str(&format!(
            "(allow file-write-data (subpath \"{}\"))\n",
            root.display()
        ));
    }

    // Always allow write to cwd
    policy.push_str(&format!(
        "(allow file-write* (subpath \"{}\"))\n",
        cwd.display()
    ));

    // Add write entries from policy
    for entry in &fs_policy.entries {
        if entry.access.allows_write() {
            let path_str = resolve_entry_path(&entry.path, cwd);
            policy.push_str(&format!("(allow file-write* (subpath \"{}\"))\n", path_str));
        }
    }

    // Deny writes to protected paths
    policy.push_str("(deny file-write* (subpath \".git\"))\n");
    policy.push_str("(deny file-write* (subpath \".diva\"))\n");
    policy.push_str("(deny file-write* (literal \".env\"))\n");
    policy.push_str("(deny file-write* (literal \"*.pem\"))\n");
    policy.push_str("(deny file-write* (literal \"*.key\"))\n");

    policy
}

/// Resolve entry path to a string
fn resolve_entry_path(path: &crate::filesystem::FileSystemPath, cwd: &Path) -> String {
    match path {
        crate::filesystem::FileSystemPath::Path { path } => {
            if path.is_absolute() {
                path.display().to_string()
            } else {
                cwd.join(path).display().to_string()
            }
        }
        crate::filesystem::FileSystemPath::Special { value } => match value {
            crate::filesystem::FileSystemSpecialPath::CurrentWorkingDirectory => {
                cwd.display().to_string()
            }
            crate::filesystem::FileSystemSpecialPath::Root => "/".to_string(),
            crate::filesystem::FileSystemSpecialPath::Tmpdir => {
                std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string())
            }
            _ => cwd.display().to_string(),
        },
        crate::filesystem::FileSystemPath::GlobPattern { .. } => {
            // Glob patterns are harder to handle in Seatbelt
            // We'll skip them for now
            cwd.display().to_string()
        }
    }
}

// ============================================================================
// sandbox-exec Command Generation
// ============================================================================

/// Create sandbox-exec command arguments
pub fn create_sandbox_exec_command(policy_file: &Path, command: Vec<String>) -> Vec<String> {
    vec![
        "-f".to_string(),
        policy_file.display().to_string(),
        "--".to_string(),
    ]
    .into_iter()
    .chain(command)
    .collect()
}

/// Write Seatbelt policy to a temporary file
pub fn write_seatbelt_policy_file(policy: &str) -> SandboxResult<PathBuf> {
    // Create a temporary file for the policy
    let temp_dir = std::env::temp_dir();
    let policy_file = temp_dir.join(format!(
        "agent-diva-sandbox-policy-{}.sbpl",
        std::process::id()
    ));

    std::fs::write(&policy_file, policy).map_err(|e| {
        SandboxError::Internal(format!("Failed to write Seatbelt policy file: {}", e))
    })?;

    debug!("Seatbelt policy written to: {}", policy_file.display());
    Ok(policy_file)
}

// ============================================================================
// macOS Sandbox Executor
// ============================================================================

/// macOS sandbox executor using Seatbelt (sandbox-exec)
pub struct MacOsSandboxExecutor;

impl MacOsSandboxExecutor {
    /// Create a new macOS sandbox executor
    pub fn new() -> Self {
        Self
    }

    /// Check if sandbox-exec is available
    pub fn is_available(&self) -> bool {
        // Check for sandbox-exec binary
        std::process::Command::new("/usr/bin/sandbox-exec")
            .arg("-h")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// Execute a command in the sandbox
    pub async fn execute(
        &self,
        command: &str,
        cwd: &Path,
        env: HashMap<String, String>,
        timeout_secs: u64,
        policy: &SandboxPolicy,
        fs_policy: &FileSystemSandboxPolicy,
    ) -> SandboxResult<String> {
        info!("Executing command in macOS Seatbelt sandbox: {}", command);

        // Check if sandbox-exec is available
        if !self.is_available() {
            warn!("sandbox-exec not available, executing directly");
            return self.execute_direct(command, cwd, env, timeout_secs).await;
        }

        // Generate Seatbelt policy
        let seatbelt_policy = create_seatbelt_policy(policy, fs_policy, cwd);
        debug!("Generated Seatbelt policy:\n{}", seatbelt_policy);

        // Write policy to temporary file
        let policy_file = write_seatbelt_policy_file(&seatbelt_policy)?;

        // Parse command into arguments
        let command_args =
            shell_words::split(command).map_err(|e| SandboxError::InvalidCommand(e.to_string()))?;

        // Create sandbox-exec arguments
        let sandbox_args = create_sandbox_exec_command(&policy_file, command_args);

        // Execute via sandbox-exec
        let result = self
            .execute_sandbox_exec(sandbox_args, cwd, env, timeout_secs)
            .await;

        // Clean up policy file
        if let Err(e) = std::fs::remove_file(&policy_file) {
            warn!("Failed to remove temporary policy file: {}", e);
        }

        result
    }

    /// Execute via sandbox-exec
    async fn execute_sandbox_exec(
        &self,
        args: Vec<String>,
        cwd: &Path,
        env: HashMap<String, String>,
        timeout_secs: u64,
    ) -> SandboxResult<String> {
        use tokio::process::Command;
        use tokio::time::timeout;

        debug!("Executing sandbox-exec with args: {:?}", args);

        let mut cmd = Command::new("/usr/bin/sandbox-exec");
        for arg in args {
            cmd.arg(arg);
        }

        cmd.current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .map_err(|e| SandboxError::SpawnFailed(e.to_string()))?;

        let output_future = child.wait_with_output();
        let output_result = timeout(Duration::from_secs(timeout_secs), output_future).await;

        let output = match output_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(SandboxError::SpawnFailed(e.to_string())),
            Err(_) => return Err(SandboxError::Timeout { secs: timeout_secs }),
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

    /// Execute directly without sandbox
    pub async fn execute_direct(
        &self,
        command: &str,
        cwd: &Path,
        env: HashMap<String, String>,
        timeout_secs: u64,
    ) -> SandboxResult<String> {
        info!("Executing command directly (no sandbox): {}", command);

        use tokio::process::Command;
        use tokio::time::timeout;

        let mut cmd = Command::new("sh");
        cmd.arg("-c")
            .arg(command)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .map_err(|e| SandboxError::SpawnFailed(e.to_string()))?;

        let output_future = child.wait_with_output();
        let output_result = timeout(Duration::from_secs(timeout_secs), output_future).await;

        let output = match output_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(SandboxError::SpawnFailed(e.to_string())),
            Err(_) => return Err(SandboxError::Timeout { secs: timeout_secs }),
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
}

impl Default for MacOsSandboxExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::{FileSystemAccessMode, FileSystemPath, FileSystemSandboxEntry};

    #[test]
    fn test_seatbelt_base_policy() {
        let policy = SEATBELT_BASE_POLICY;
        assert!(policy.contains("(version 1)"));
        assert!(policy.contains("(allow process-exec"));
    }

    #[test]
    fn test_create_seatbelt_policy_danger_full_access() {
        let sandbox_policy = SandboxPolicy::DangerFullAccess;
        let fs_policy = FileSystemSandboxPolicy::unrestricted();
        let cwd = Path::new("/workspace");

        let policy = create_seatbelt_policy(&sandbox_policy, &fs_policy, cwd);
        assert!(policy.contains("(allow file*)"));
        assert!(policy.contains("(allow network*)"));
    }

    #[test]
    fn test_create_seatbelt_policy_read_only() {
        let sandbox_policy = SandboxPolicy::ReadOnly {
            access: ReadOnlyAccess::FullDisk,
            network_access: false,
        };
        let fs_policy = FileSystemSandboxPolicy::unrestricted();
        let cwd = Path::new("/workspace");

        let policy = create_seatbelt_policy(&sandbox_policy, &fs_policy, cwd);
        assert!(policy.contains("(deny file-write*)"));
        assert!(policy.contains("(deny network*)"));
    }

    #[test]
    fn test_create_seatbelt_policy_workspace_write() {
        let sandbox_policy = SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![PathBuf::from("/workspace")],
            read_only_access: ReadOnlyAccess::FullDisk,
            network_access: true,
            exclude_tmpdir_env_var: false,
        };
        let fs_policy = FileSystemSandboxPolicy::unrestricted();
        let cwd = Path::new("/workspace");

        let policy = create_seatbelt_policy(&sandbox_policy, &fs_policy, cwd);
        assert!(policy.contains("(allow file-write* (subpath \"/workspace\"))"));
        assert!(policy.contains("(allow network*)"));
    }

    #[test]
    fn test_create_sandbox_exec_command() {
        let policy_file = Path::new("/tmp/policy.sbpl");
        let command = vec!["ls".to_string(), "-la".to_string()];

        let args = create_sandbox_exec_command(policy_file, command);
        assert_eq!(args[0], "-f");
        assert_eq!(args[1], "/tmp/policy.sbpl");
        assert_eq!(args[2], "--");
        assert_eq!(args[3], "ls");
        assert_eq!(args[4], "-la");
    }

    #[test]
    fn test_resolve_entry_path_absolute() {
        let path = FileSystemPath::from_path(PathBuf::from("/usr/bin"));
        let cwd = Path::new("/workspace");

        let resolved = resolve_entry_path(&path, cwd);
        assert_eq!(resolved, "/usr/bin");
    }

    #[test]
    fn test_resolve_entry_path_relative() {
        let path = FileSystemPath::from_path(PathBuf::from("src/main.rs"));
        let cwd = Path::new("/workspace");

        let resolved = resolve_entry_path(&path, cwd);
        // Note: path display may differ on Windows
        assert!(resolved.contains("src"));
    }

    #[test]
    fn test_resolve_entry_path_cwd() {
        let path = FileSystemPath::Special {
            value: crate::filesystem::FileSystemSpecialPath::CurrentWorkingDirectory,
        };
        let cwd = Path::new("/workspace");

        let resolved = resolve_entry_path(&path, cwd);
        assert!(resolved.contains("workspace"));
    }
}
