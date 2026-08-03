//! Workspace isolation and sandbox path validation
//!
//! Provides dual-root workspace isolation (`data_root` for agent state,
//! `project_root` for user files) and path validation that rejects
//! directory traversal (`..`) and symlink escape attempts.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Which workspace root to resolve a path against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SandboxRoot {
    /// Agent state directory (configs, databases, logs)
    Data,
    /// User project directory (source files, assets)
    Project,
}

/// Dual-root workspace configuration.
///
/// `data_root` holds agent-internal state (databases, session data, logs).
/// `project_root` holds user-facing project files (source code, assets).
/// Both roots are configurable and must be absolute, canonical paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRoots {
    /// Root directory for agent state (databases, session data, logs)
    pub data_root: PathBuf,
    /// Root directory for user project files (source code, assets)
    pub project_root: PathBuf,
}

impl WorkspaceRoots {
    /// Create a new WorkspaceRoots, canonicalizing both paths.
    ///
    /// Both paths must exist on disk. If either path does not exist,
    /// an error is returned.
    pub fn new(data_root: &Path, project_root: &Path) -> Result<Self> {
        let data_root = data_root
            .canonicalize()
            .map_err(|e| Error::Validation(format!("data_root does not exist: {e}")))?;
        let project_root = project_root
            .canonicalize()
            .map_err(|e| Error::Validation(format!("project_root does not exist: {e}")))?;
        Ok(Self {
            data_root,
            project_root,
        })
    }

    /// Get the root path for the given sandbox root type.
    pub fn root_for(&self, root: SandboxRoot) -> &Path {
        match root {
            SandboxRoot::Data => &self.data_root,
            SandboxRoot::Project => &self.project_root,
        }
    }
}

/// Resolve and validate a path within a sandbox root.
///
/// This function:
/// 1. Rejects paths containing `..` components (directory traversal)
/// 2. Joins the input path to the specified root
/// 3. Canonicalizes the result (resolving symlinks)
/// 4. Verifies the canonical path is still within the root
/// 5. Rejects the path if it escapes the root via symlink
///
/// # Errors
///
/// Returns an error if:
/// - The input path contains `..` components
/// - The resolved path does not exist on disk
/// - The resolved path escapes the sandbox root (symlink attack)
pub fn resolve_sandbox_path(
    roots: &WorkspaceRoots,
    input: &str,
    root: SandboxRoot,
) -> Result<PathBuf> {
    // Step 1: Reject ".." components in the input
    let input_path = Path::new(input);
    for component in input_path.components() {
        if let std::path::Component::ParentDir = component {
            return Err(Error::Validation(
                "path traversal rejected: '..' components are not allowed".to_string(),
            ));
        }
    }

    // Step 2: Join to the sandbox root
    let base = roots.root_for(root);
    let joined = base.join(input_path);

    // Step 3: Canonicalize (resolves symlinks)
    let canonical = joined.canonicalize().map_err(|e| {
        Error::Validation(format!("path does not exist or cannot be resolved: {e}"))
    })?;

    // Step 4: Verify the canonical path is within the root
    // We compare against the canonical version of the root
    let canonical_root = base
        .canonicalize()
        .map_err(|e| Error::Validation(format!("root path cannot be canonicalized: {e}")))?;

    if !canonical.starts_with(&canonical_root) {
        return Err(Error::Validation(
            "sandbox escape rejected: resolved path is outside the workspace root".to_string(),
        ));
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper to create a WorkspaceRoots backed by temp directories.
    fn setup() -> (TempDir, TempDir, WorkspaceRoots) {
        let data_dir = TempDir::new().expect("data tempdir");
        let project_dir = TempDir::new().expect("project tempdir");
        let roots =
            WorkspaceRoots::new(data_dir.path(), project_dir.path()).expect("workspace roots");
        (data_dir, project_dir, roots)
    }

    #[test]
    fn test_workspace_roots_stores_both_roots_distinctly() {
        let (data_dir, project_dir, roots) = setup();
        assert_ne!(roots.data_root, roots.project_root);
        assert!(roots
            .data_root
            .starts_with(data_dir.path().canonicalize().unwrap()));
        assert!(roots
            .project_root
            .starts_with(project_dir.path().canonicalize().unwrap()));
    }

    #[test]
    fn test_workspace_roots_root_for() {
        let (_data_dir, _project_dir, roots) = setup();
        assert_eq!(roots.root_for(SandboxRoot::Data), &roots.data_root);
        assert_eq!(roots.root_for(SandboxRoot::Project), &roots.project_root);
    }

    #[test]
    fn test_resolve_sandbox_path_normalizes_correctly() {
        let (data_dir, _project_dir, roots) = setup();

        // Create a file inside data_root
        let test_file = data_dir.path().join("test.txt");
        std::fs::write(&test_file, "hello").expect("write");

        let resolved =
            resolve_sandbox_path(&roots, "test.txt", SandboxRoot::Data).expect("resolve");
        assert!(resolved.ends_with("test.txt"));
        assert!(resolved.starts_with(&roots.data_root));
    }

    #[test]
    fn test_resolve_sandbox_path_subdirectory() {
        let (data_dir, _project_dir, roots) = setup();

        // Create a subdirectory with a file
        let sub_dir = data_dir.path().join("sessions");
        std::fs::create_dir_all(&sub_dir).expect("mkdir");
        let test_file = sub_dir.join("session1.json");
        std::fs::write(&test_file, "{}").expect("write");

        let resolved = resolve_sandbox_path(&roots, "sessions/session1.json", SandboxRoot::Data)
            .expect("resolve");
        assert!(resolved.ends_with("session1.json"));
        assert!(resolved.starts_with(&roots.data_root));
    }

    #[test]
    fn test_resolve_sandbox_path_rejects_dot_dot() {
        let (_data_dir, _project_dir, roots) = setup();

        let result = resolve_sandbox_path(&roots, "../etc/passwd", SandboxRoot::Data);
        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
            assert!(msg.contains("'..' components are not allowed"));
        } else {
            panic!("Expected Validation error for '..' path");
        }
    }

    #[test]
    fn test_resolve_sandbox_path_rejects_embedded_dot_dot() {
        let (_data_dir, _project_dir, roots) = setup();

        let result = resolve_sandbox_path(&roots, "foo/../../etc/passwd", SandboxRoot::Data);
        assert!(result.is_err());
        if let Err(Error::Validation(msg)) = result {
            assert!(msg.contains("'..' components are not allowed"));
        } else {
            panic!("Expected Validation error for embedded '..' path");
        }
    }

    #[test]
    fn test_resolve_sandbox_path_rejects_symlink_escape() {
        let (data_dir, _project_dir, roots) = setup();

        // Create a symlink inside data_root that points outside
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let link_path = data_dir.path().join("escape_link");
            symlink("/tmp", &link_path).expect("symlink");
        }

        #[cfg(windows)]
        {
            // On Windows, create a directory symlink pointing to a temp location outside.
            // We use a stable temp dir (not TempDir which drops too early).
            let outside_dir = TempDir::new().expect("outside tempdir");
            let link_path = data_dir.path().join("escape_link");
            // Use cmd /c mklink /D for directory symlinks on Windows
            let link_str = link_path.to_str().unwrap();
            let target_str = outside_dir.path().to_str().unwrap();
            let output = std::process::Command::new("cmd")
                .args(["/c", "mklink", "/D", link_str, target_str])
                .output()
                .expect("mklink");
            if !output.status.success() {
                // If we can't create symlinks (e.g., no admin privileges), skip this test.
                // This is acceptable: the defense still works — canonicalize will resolve
                // any symlink that does exist and the starts_with check will catch escapes.
                eprintln!(
                    "Skipping symlink test: mklink failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                );
                return;
            }
            // Don't drop outside_dir until after the assertion — its path must remain valid
            // for canonicalize to resolve the symlink target.
            let result = resolve_sandbox_path(&roots, "escape_link", SandboxRoot::Data);
            assert!(
                result.is_err(),
                "Expected sandbox escape rejection for symlink"
            );
            if let Err(Error::Validation(msg)) = result {
                assert!(
                    msg.contains("outside the workspace root"),
                    "Unexpected error: {msg}"
                );
            }
            drop(outside_dir);
        }

        #[cfg(unix)]
        {
            // The symlink itself exists, so canonicalize will resolve it
            // to a path outside data_root
            let result = resolve_sandbox_path(&roots, "escape_link", SandboxRoot::Data);
            assert!(
                result.is_err(),
                "Expected sandbox escape rejection for symlink"
            );
            if let Err(Error::Validation(msg)) = result {
                assert!(
                    msg.contains("outside the workspace root"),
                    "Unexpected error: {msg}"
                );
            }
        }
    }

    #[test]
    fn test_resolve_sandbox_path_nonexistent_file() {
        let (_data_dir, _project_dir, roots) = setup();

        let result = resolve_sandbox_path(&roots, "nonexistent.txt", SandboxRoot::Data);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolve_sandbox_path_project_root() {
        let (_data_dir, project_dir, roots) = setup();

        // Create a file inside project_root
        let test_file = project_dir.path().join("main.rs");
        std::fs::write(&test_file, "fn main() {}").expect("write");

        let resolved =
            resolve_sandbox_path(&roots, "main.rs", SandboxRoot::Project).expect("resolve");
        assert!(resolved.ends_with("main.rs"));
        assert!(resolved.starts_with(&roots.project_root));
    }

    #[test]
    fn test_workspace_roots_nonexistent_data_root() {
        let project_dir = TempDir::new().expect("project tempdir");
        let result = WorkspaceRoots::new(Path::new("/nonexistent/path/abc"), project_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_workspace_roots_nonexistent_project_root() {
        let data_dir = TempDir::new().expect("data tempdir");
        let result = WorkspaceRoots::new(data_dir.path(), Path::new("/nonexistent/path/abc"));
        assert!(result.is_err());
    }
}
