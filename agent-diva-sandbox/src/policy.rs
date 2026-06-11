//! Sandbox policy definitions
//!
//! Core types for controlling sandbox behavior, inspired by Codex CLI.

use agent_diva_core::security::{SecurityConfig, SecurityLevel, SecurityPolicy};
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

impl SandboxPolicy {
    /// Convert this sandbox policy into the closest `SecurityPolicy` representation.
    ///
    /// Mapping table:
    /// - `DangerFullAccess` -> `SecurityLevel::Permissive`
    /// - `ReadOnly` -> `SecurityLevel::Paranoid`
    /// - `WorkspaceWrite` -> `SecurityLevel::Standard`
    /// - `ExternalSandbox` -> `SecurityLevel::Standard`
    ///
    /// This bridge is intentionally lossy because `SecurityPolicy` does not encode
    /// network access or external-sandbox provenance.
    pub fn to_security_policy(&self, workspace_dir: PathBuf) -> SecurityPolicy {
        let mut config = match self {
            SandboxPolicy::DangerFullAccess => SecurityConfig::from_level(SecurityLevel::Permissive),
            SandboxPolicy::ReadOnly { .. } => SecurityConfig::from_level(SecurityLevel::Paranoid),
            SandboxPolicy::WorkspaceWrite { .. } | SandboxPolicy::ExternalSandbox { .. } => {
                SecurityConfig::from_level(SecurityLevel::Standard)
            }
        };

        match self {
            SandboxPolicy::DangerFullAccess => {
                config.workspace_only = false;
                config.allowed_roots.clear();
                config.read_only = Some(false);
            }
            SandboxPolicy::ReadOnly { access, .. } => {
                config.workspace_only = !matches!(access, ReadOnlyAccess::FullDisk);
                config.read_only = Some(true);
            }
            SandboxPolicy::WorkspaceWrite { writable_roots, .. } => {
                config.workspace_only = true;
                config.allowed_roots = writable_roots.clone();
                config.read_only = Some(false);
            }
            SandboxPolicy::ExternalSandbox { .. } => {
                config.workspace_only = true;
                config.read_only = Some(false);
            }
        }

        SecurityPolicy::with_config(workspace_dir, config)
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

/// Bridge methods for converting the core `SecurityPolicy` into sandbox policies.
pub trait SecurityPolicySandboxExt {
    /// Convert a `SecurityPolicy` into the closest `SandboxPolicy` representation.
    ///
    /// Mapping table:
    /// - `Permissive` with `workspace_only=false` -> `DangerFullAccess`
    /// - read-only policies -> `ReadOnly`
    /// - all other policies -> `WorkspaceWrite`
    ///
    /// The conversion is intentionally lossy because `SecurityPolicy` has no direct
    /// network-access field and models allowed roots differently.
    fn to_sandbox_policy(&self) -> SandboxPolicy;
}

impl SecurityPolicySandboxExt for SecurityPolicy {
    fn to_sandbox_policy(&self) -> SandboxPolicy {
        let config = self.config();
        let workspace_dir = self.workspace_dir().to_path_buf();

        if config.level == SecurityLevel::Permissive && !config.workspace_only && !self.is_read_only()
        {
            return SandboxPolicy::DangerFullAccess;
        }

        if self.is_read_only() {
            return SandboxPolicy::ReadOnly {
                access: if config.workspace_only {
                    ReadOnlyAccess::Custom
                } else {
                    ReadOnlyAccess::FullDisk
                },
                network_access: false,
            };
        }

        let mut writable_roots = config.allowed_roots.clone();
        if !writable_roots.iter().any(|root| root == &workspace_dir) {
            writable_roots.insert(0, workspace_dir);
        }

        SandboxPolicy::WorkspaceWrite {
            writable_roots,
            read_only_access: if config.workspace_only {
                ReadOnlyAccess::Custom
            } else {
                ReadOnlyAccess::FullDisk
            },
            network_access: false,
            exclude_tmpdir_env_var: false,
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_policy_to_security_policy_danger_full_access() {
        let security =
            SandboxPolicy::DangerFullAccess.to_security_policy(PathBuf::from("/workspace"));

        assert_eq!(security.config().level, SecurityLevel::Permissive);
        assert!(!security.config().workspace_only);
        assert!(!security.is_read_only());
    }

    #[test]
    fn test_sandbox_policy_to_security_policy_workspace_write() {
        let workspace = PathBuf::from("/workspace");
        let other = PathBuf::from("/tmp/cache");
        let security = SandboxPolicy::WorkspaceWrite {
            writable_roots: vec![workspace.clone(), other.clone()],
            read_only_access: ReadOnlyAccess::Custom,
            network_access: false,
            exclude_tmpdir_env_var: false,
        }
        .to_security_policy(workspace.clone());

        assert_eq!(security.config().level, SecurityLevel::Standard);
        assert!(security.config().workspace_only);
        assert_eq!(security.config().allowed_roots, vec![workspace, other]);
    }

    #[test]
    fn test_security_policy_to_sandbox_policy_read_only() {
        let security =
            SecurityPolicy::from_level(PathBuf::from("/workspace"), SecurityLevel::Paranoid);

        assert!(matches!(
            security.to_sandbox_policy(),
            SandboxPolicy::ReadOnly {
                access: ReadOnlyAccess::Custom,
                network_access: false,
            }
        ));
    }

    #[test]
    fn test_security_policy_to_sandbox_policy_workspace_write() {
        let mut config = SecurityConfig::from_level(SecurityLevel::Standard);
        config.allowed_roots = vec![PathBuf::from("/tmp/cache")];
        let security = SecurityPolicy::with_config(PathBuf::from("/workspace"), config);

        match security.to_sandbox_policy() {
            SandboxPolicy::WorkspaceWrite { writable_roots, .. } => {
                assert_eq!(writable_roots[0], PathBuf::from("/workspace"));
                assert!(writable_roots.contains(&PathBuf::from("/tmp/cache")));
            }
            other => panic!("expected workspace-write policy, got {other:?}"),
        }
    }
}
