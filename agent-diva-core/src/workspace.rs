//! Workspace directory structure helpers.
//!
//! Provides utilities for creating the dual-root workspace directory
//! layout used by agent-diva instances.

use std::path::Path;

/// Subdirectories created under each dual-root workspace.
const DUAL_ROOT_SUBDIRS: &[&str] = &[
    "config",
    "sessions",
    "skills",
    "memory",
    "logs",
    "todos",
    "runs",
];

/// Creates the dual-root workspace directory structure under `base_path`.
///
/// This is idempotent — calling it multiple times on the same path
/// will not fail or recreate existing directories.
///
/// # Errors
///
/// Returns an I/O error if any directory cannot be created (e.g.,
/// insufficient permissions, path component is a file).
pub fn create_dual_root_dirs(base_path: impl AsRef<Path>) -> std::io::Result<()> {
    let base = base_path.as_ref();
    for sub in DUAL_ROOT_SUBDIRS {
        std::fs::create_dir_all(base.join(sub))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_all_seven_subdirs() {
        let dir = tempfile::tempdir().unwrap();
        create_dual_root_dirs(dir.path()).unwrap();

        for sub in DUAL_ROOT_SUBDIRS {
            let p = dir.path().join(sub);
            assert!(
                p.exists(),
                "expected directory to exist: {}",
                p.display()
            );
            assert!(
                p.is_dir(),
                "expected a directory, not a file: {}",
                p.display()
            );
        }
    }

    #[test]
    fn idempotent_second_call_no_error() {
        let dir = tempfile::tempdir().unwrap();
        create_dual_root_dirs(dir.path()).unwrap();
        // Second call must succeed without panicking or erroring.
        create_dual_root_dirs(dir.path()).unwrap();

        // All subdirs still exist.
        for sub in DUAL_ROOT_SUBDIRS {
            assert!(dir.path().join(sub).is_dir());
        }
    }

    #[test]
    fn non_existent_parent_creates_intermediates() {
        let dir = tempfile::tempdir().unwrap();
        let nested = dir.path().join("nested").join("workspace");
        create_dual_root_dirs(&nested).unwrap();

        for sub in DUAL_ROOT_SUBDIRS {
            assert!(nested.join(sub).is_dir());
        }
    }
}
