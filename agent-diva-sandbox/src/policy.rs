//! Sandbox policy definitions
//!
//! Core types for controlling sandbox behavior, inspired by Codex CLI.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Sandbox policy determines execution restrictions for shell commands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum SandboxPolicy {
    /// No restrictions whatsoever. Use with caution.
    #[serde(rename = "danger-full-access")]
    DangerFullAccess,

    /// Read-only access configuration.
    #[serde(rename = "read-only")]
    ReadOnly {
        /// Read access granted while running under this policy.
        #[serde(default)]
        access: ReadOnlyAccess,
        /// When set to true, outbound network access is allowed.
        #[serde(default)]
        network_access: bool,
    },

    /// Indicates the process is already in an external sandbox (container environment).
    #[serde(rename = "external-sandbox")]
    ExternalSandbox {
        #[serde(default)]
        network_access: NetworkAccess,
    },

    /// Same as ReadOnly but additionally grants write access to the current working directory.
    #[serde(rename = "workspace-write")]
    WorkspaceWrite {
        /// Additional folders beyond cwd that should be writable.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        writable_roots: Vec<PathBuf>,
        /// Read access granted while running under this policy.
        #[serde(default)]
        read_only_access: ReadOnlyAccess,
        /// When set to true, outbound network access is allowed.
        #[serde(default)]
        network_access: bool,
        /// When set to true, will NOT include the per-user TMPDIR environment variable.
        #[serde(default)]
        exclude_tmpdir_env_var: bool,
    },
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        SandboxPolicy::WorkspaceWrite {
            writable_roots: Vec::new(),
            read_only_access: ReadOnlyAccess::default(),
            network_access: false,
            exclude_tmpdir_env_var: false,
        }
    }
}

/// Sandbox mode - configuration-friendly enum for sandbox level selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum SandboxMode {
    /// No sandbox, direct execution (dangerous)
    #[serde(rename = "danger-full-access")]
    DangerFullAccess,
    /// Read-only file system access
    #[default]
    #[serde(rename = "read-only")]
    ReadOnly,
    /// Write access to workspace directory
    #[serde(rename = "workspace-write")]
    WorkspaceWrite,
}

impl SandboxMode {
    /// Convert to full SandboxPolicy with default options
    pub fn to_policy(&self, workspace: PathBuf) -> SandboxPolicy {
        match self {
            SandboxMode::DangerFullAccess => SandboxPolicy::DangerFullAccess,
            SandboxMode::ReadOnly => SandboxPolicy::ReadOnly {
                access: ReadOnlyAccess::FullDisk,
                network_access: false,
            },
            SandboxMode::WorkspaceWrite => SandboxPolicy::WorkspaceWrite {
                writable_roots: vec![workspace],
                read_only_access: ReadOnlyAccess::FullDisk,
                network_access: false,
                exclude_tmpdir_env_var: false,
            },
        }
    }
}

/// Read-only access level for file system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum ReadOnlyAccess {
    /// Full disk read access (default)
    #[default]
    FullDisk,
    /// No read access
    None,
    /// Custom paths only (to be specified in FileSystemSandboxPolicy)
    Custom,
}

impl ReadOnlyAccess {
    /// Check if this has full disk read access
    pub fn has_full_disk_read_access(&self) -> bool {
        matches!(self, ReadOnlyAccess::FullDisk)
    }
}

/// Network access configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum NetworkAccess {
    /// Network access allowed
    Allowed,
    /// Network access denied (default)
    #[default]
    Denied,
}

impl NetworkAccess {
    /// Check if network access is allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, NetworkAccess::Allowed)
    }
}

impl From<bool> for NetworkAccess {
    fn from(value: bool) -> Self {
        if value {
            NetworkAccess::Allowed
        } else {
            NetworkAccess::Denied
        }
    }
}

/// Approval policy - when to ask for user approval before execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AskForApproval {
    /// Never request approval, always execute directly
    Never,
    /// Request approval only after sandbox execution fails
    #[default]
    #[serde(rename = "on-failure")]
    OnFailure,
    /// LLM decides when to request approval
    #[serde(rename = "on-request")]
    OnRequest,
    /// Request approval for untrusted commands only
    #[serde(rename = "unless-trusted")]
    UnlessTrusted,
}

impl AskForApproval {
    /// Check if we should ask for approval before first attempt
    pub fn should_ask_before_first_attempt(&self) -> bool {
        matches!(
            self,
            AskForApproval::UnlessTrusted | AskForApproval::OnRequest
        )
    }

    /// Check if we can retry without sandbox after sandbox failure
    pub fn allows_sandbox_failure_retry(&self) -> bool {
        matches!(
            self,
            AskForApproval::OnFailure | AskForApproval::UnlessTrusted
        )
    }
}

/// Windows sandbox level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum WindowsSandboxLevel {
    /// Windows sandbox disabled
    #[default]
    #[serde(rename = "disabled")]
    Disabled,
    /// Restricted Token sandbox (limited privileges)
    #[serde(rename = "restricted-token")]
    RestrictedToken,
    /// Elevated sandbox (requires setup)
    #[serde(rename = "elevated")]
    Elevated,
}
