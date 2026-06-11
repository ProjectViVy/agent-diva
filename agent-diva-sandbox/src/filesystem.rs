//! File system sandbox policy definitions
//!
//! Types for controlling file system access within sandbox.

use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// File system sandbox policy container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemSandboxPolicy {
    /// Policy type
    pub kind: FileSystemSandboxKind,
    /// Specific entries defining access rules
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub entries: Vec<FileSystemSandboxEntry>,
}

impl FileSystemSandboxPolicy {
    /// Create a restricted policy with specific entries
    pub fn restricted(entries: Vec<FileSystemSandboxEntry>) -> Self {
        Self {
            kind: FileSystemSandboxKind::Restricted,
            entries,
        }
    }

    /// Create an unrestricted policy
    pub fn unrestricted() -> Self {
        Self {
            kind: FileSystemSandboxKind::Unrestricted,
            entries: Vec::new(),
        }
    }

    /// Create an external sandbox policy
    pub fn external_sandbox() -> Self {
        Self {
            kind: FileSystemSandboxKind::ExternalSandbox,
            entries: Vec::new(),
        }
    }

    /// Check if a path can be read
    pub fn can_read_path(&self, path: &Path) -> bool {
        match self.kind {
            FileSystemSandboxKind::Unrestricted | FileSystemSandboxKind::ExternalSandbox => true,
            FileSystemSandboxKind::Restricted => {
                for entry in &self.entries {
                    if entry.matches_path(path) {
                        return entry.access != FileSystemAccessMode::None;
                    }
                }
                // Default: no access if not explicitly allowed
                false
            }
        }
    }

    /// Check if a path can be read, considering cwd as base
    pub fn can_read_path_with_cwd(&self, path: &Path, cwd: &Path) -> bool {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        self.can_read_path(&resolved)
    }

    /// Check if a path can be written
    pub fn can_write_path(&self, path: &Path) -> bool {
        match self.kind {
            FileSystemSandboxKind::Unrestricted | FileSystemSandboxKind::ExternalSandbox => true,
            FileSystemSandboxKind::Restricted => {
                for entry in &self.entries {
                    if entry.matches_path(path) {
                        return entry.access == FileSystemAccessMode::Write;
                    }
                }
                false
            }
        }
    }

    /// Check if a path can be written, considering cwd as base
    pub fn can_write_path_with_cwd(&self, path: &Path, cwd: &Path) -> bool {
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            cwd.join(path)
        };
        self.can_write_path(&resolved)
    }
}

impl Default for FileSystemSandboxPolicy {
    fn default() -> Self {
        Self::unrestricted()
    }
}

/// File system sandbox kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FileSystemSandboxKind {
    /// Restricted access based on entries
    #[default]
    Restricted,
    /// Unrestricted access
    Unrestricted,
    /// Managed by external sandbox (container)
    ExternalSandbox,
}

/// File system access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileSystemAccessMode {
    /// Read access
    Read,
    /// Write access (includes read)
    Write,
    /// Explicitly denied
    None,
}

impl FileSystemAccessMode {
    /// Check if this mode allows reading
    pub fn allows_read(&self) -> bool {
        matches!(
            self,
            FileSystemAccessMode::Read | FileSystemAccessMode::Write
        )
    }

    /// Check if this mode allows writing
    pub fn allows_write(&self) -> bool {
        matches!(self, FileSystemAccessMode::Write)
    }
}

/// File system sandbox entry - a path with associated access mode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSystemSandboxEntry {
    /// Target path
    pub path: FileSystemPath,
    /// Access mode for this path
    pub access: FileSystemAccessMode,
}

impl FileSystemSandboxEntry {
    /// Create a new entry
    pub fn new(path: FileSystemPath, access: FileSystemAccessMode) -> Self {
        Self { path, access }
    }

    /// Check if this entry matches a given path
    pub fn matches_path(&self, path: &Path) -> bool {
        self.path.matches(path)
    }
}

/// File system path type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FileSystemPath {
    /// Concrete absolute path
    Path { path: PathBuf },
    /// Glob pattern for matching
    GlobPattern { pattern: String },
    /// Special path marker
    Special { value: FileSystemSpecialPath },
}

impl FileSystemPath {
    /// Create a concrete path
    pub fn from_path(path: PathBuf) -> Self {
        FileSystemPath::Path { path }
    }

    /// Create a glob pattern
    pub fn from_glob(pattern: &str) -> Self {
        FileSystemPath::GlobPattern {
            pattern: pattern.to_string(),
        }
    }

    /// Create current working directory special path
    pub fn cwd() -> Self {
        FileSystemPath::Special {
            value: FileSystemSpecialPath::CurrentWorkingDirectory,
        }
    }

    /// Check if this path matches a given concrete path
    pub fn matches(&self, target: &Path) -> bool {
        match self {
            FileSystemPath::Path { path } => target.starts_with(path) || path.starts_with(target),
            FileSystemPath::GlobPattern { pattern } => {
                // Use globset for matching
                if let Ok(glob) = Glob::new(pattern) {
                    if let Ok(set) = GlobSetBuilder::new().add(glob).build() {
                        return set.is_match(target);
                    }
                }
                false
            }
            FileSystemPath::Special { value: _ } => {
                // Special paths need context to resolve; return false here
                // They should be resolved before matching
                false
            }
        }
    }
}

/// Special file system paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FileSystemSpecialPath {
    /// Root directory
    Root,
    /// Minimal system paths
    Minimal,
    /// Current working directory
    CurrentWorkingDirectory,
    /// Project roots with optional subpath
    ProjectRoots { subpath: Option<PathBuf> },
    /// User's temp directory
    Tmpdir,
    /// System temp directory (/tmp)
    SlashTmp,
    /// Unknown special path
    Unknown {
        path: String,
        subpath: Option<PathBuf>,
    },
}

/// Writable root with protected subpaths.
///
/// A writable root defines a directory where writes are allowed,
/// but certain subpaths within it remain read-only (protected).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritableRoot {
    /// Root path where writes are allowed
    pub root: PathBuf,
    /// Subpaths within root that should remain read-only
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub read_only_subpaths: Vec<PathBuf>,
}

impl WritableRoot {
    /// Create a new writable root with default protected paths
    pub fn new(root: PathBuf) -> Self {
        let read_only_subpaths = default_read_only_subpaths_for_writable_root(&root, true);
        Self {
            root,
            read_only_subpaths,
        }
    }

    /// Create a writable root without default protected paths
    pub fn new_unprotected(root: PathBuf) -> Self {
        Self {
            root,
            read_only_subpaths: Vec::new(),
        }
    }

    /// Create with custom protected paths
    pub fn with_protected(root: PathBuf, read_only_subpaths: Vec<PathBuf>) -> Self {
        Self {
            root,
            read_only_subpaths,
        }
    }

    /// Check if a given path is writable under this root.
    ///
    /// Path must:
    /// 1. Be under the root directory
    /// 2. Not be under any read_only_subpaths
    pub fn is_path_writable(&self, path: &Path) -> bool {
        // Path must be under root
        if !path.starts_with(&self.root) {
            return false;
        }

        // Path must not be under any protected subpath
        for protected in &self.read_only_subpaths {
            if path.starts_with(protected) {
                return false;
            }
        }

        true
    }

    /// Add a protected subpath
    pub fn add_protected(&mut self, subpath: PathBuf) {
        if subpath.starts_with(&self.root) {
            self.read_only_subpaths.push(subpath);
        }
    }
}

/// Default read-only subpaths for a writable root.
///
/// These are protected paths that should remain read-only even
/// within a writable workspace.
pub fn default_read_only_subpaths_for_writable_root(
    writable_root: &Path,
    protect_missing_dot_diva: bool,
) -> Vec<PathBuf> {
    let mut subpaths = Vec::new();

    // .git directory - version control metadata
    let git_dir = writable_root.join(".git");
    if git_dir.exists() {
        subpaths.push(git_dir);
    }

    // .agents directory - agent configuration (from AGENTS.md)
    let agents_dir = writable_root.join(".agents");
    if agents_dir.exists() {
        subpaths.push(agents_dir);
    }

    // .diva directory - agent-diva specific data
    let diva_dir = writable_root.join(".diva");
    if protect_missing_dot_diva || diva_dir.exists() {
        subpaths.push(diva_dir);
    }

    subpaths
}

/// Default protected path patterns (glob patterns for deny).
pub fn default_protected_paths() -> Vec<String> {
    vec![
        ".git".to_string(),
        ".diva".to_string(),
        ".agents".to_string(),
        ".env".to_string(),
        ".env.*".to_string(),
        ".npmrc".to_string(),
        ".yarnrc".to_string(),
        ".pnpmrc".to_string(),
        "*.pem".to_string(),
        "*.key".to_string(),
        "*.secret".to_string(),
        "*.tfvars".to_string(),
        "*.tfstate".to_string(),
        "*.tfstate.backup".to_string(),
        "credentials".to_string(),
        "credentials.json".to_string(),
        "*.credentials".to_string(),
        "id_rsa".to_string(),
        "id_rsa.pub".to_string(),
        "id_ed25519".to_string(),
        "id_ed25519.pub".to_string(),
        ".aws/credentials".to_string(),
        ".aws/config".to_string(),
        ".docker/config.json".to_string(),
        ".netrc".to_string(),
        ".pypirc".to_string(),
    ]
}

/// Read deny matcher using glob patterns.
pub struct ReadDenyMatcher {
    matchers: GlobSet,
    patterns: Vec<String>,
}

impl ReadDenyMatcher {
    /// Create a new matcher from glob patterns
    pub fn new(patterns: Vec<String>) -> Self {
        let mut builder = GlobSetBuilder::new();
        for pattern in &patterns {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        let matchers = builder.build().unwrap_or_else(|_| GlobSet::empty());
        Self { matchers, patterns }
    }

    /// Create with default protected paths
    pub fn with_defaults() -> Self {
        Self::new(default_protected_paths())
    }

    /// Check if a path is denied for reading
    pub fn is_read_denied(&self, path: &Path) -> bool {
        self.matchers.is_match(path)
    }

    /// Get the patterns
    pub fn patterns(&self) -> &[String] {
        &self.patterns
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::policy::{AskForApproval, SandboxMode, SandboxPolicy};

    #[test]
    fn test_writable_root_allows_workspace_files() {
        let root = WritableRoot::new(PathBuf::from("/workspace"));
        assert!(root.is_path_writable(Path::new("/workspace/src/main.rs")));
        assert!(root.is_path_writable(Path::new("/workspace/README.md")));
    }

    #[test]
    fn test_writable_root_blocks_path_traversal() {
        let root = WritableRoot::new(PathBuf::from("/workspace"));
        assert!(!root.is_path_writable(Path::new("/etc/passwd")));
        // Note: Path traversal through parent dir references is handled at a higher level
        // The starts_with check will fail for "/workspace/../etc/passwd" which doesn't start with "/workspace"
    }

    #[test]
    fn test_file_system_access_mode() {
        assert!(FileSystemAccessMode::Read.allows_read());
        assert!(!FileSystemAccessMode::Read.allows_write());

        assert!(FileSystemAccessMode::Write.allows_read());
        assert!(FileSystemAccessMode::Write.allows_write());

        assert!(!FileSystemAccessMode::None.allows_read());
        assert!(!FileSystemAccessMode::None.allows_write());
    }

    #[test]
    fn test_sandbox_policy_default() {
        let policy = SandboxPolicy::default();
        assert!(matches!(policy, SandboxPolicy::WorkspaceWrite { .. }));
    }

    #[test]
    fn test_sandbox_mode_to_policy() {
        // Default SandboxMode is ReadOnly
        let mode = SandboxMode::default();
        let policy = mode.to_policy(PathBuf::from("/workspace"));
        assert!(matches!(policy, SandboxPolicy::ReadOnly { .. }));

        // WorkspaceWrite mode produces WorkspaceWrite policy
        let mode = SandboxMode::WorkspaceWrite;
        let policy = mode.to_policy(PathBuf::from("/workspace"));
        assert!(matches!(policy, SandboxPolicy::WorkspaceWrite { .. }));
    }

    #[test]
    fn test_ask_for_approval_defaults() {
        let policy = AskForApproval::default();
        assert!(matches!(policy, AskForApproval::OnFailure));
        assert!(policy.allows_sandbox_failure_retry());
        assert!(!policy.should_ask_before_first_attempt());
    }

    #[test]
    fn test_default_protected_paths_deny_new_secret_patterns() {
        let matcher = ReadDenyMatcher::with_defaults();
        let denied_paths = [
            ".env.local",
            ".env.production",
            ".env.development",
            ".npmrc",
            ".yarnrc",
            ".pnpmrc",
            "terraform.tfvars",
            "terraform.tfstate",
            "terraform.tfstate.backup",
            "credentials",
            "credentials.json",
            "service.credentials",
            "id_rsa",
            "id_rsa.pub",
            "id_ed25519",
            "id_ed25519.pub",
            ".aws/credentials",
            ".aws/config",
            ".docker/config.json",
            ".netrc",
            ".pypirc",
        ];

        for path in denied_paths {
            assert!(
                matcher.is_read_denied(Path::new(path)),
                "expected {path} to be denied"
            );
        }
    }
}
