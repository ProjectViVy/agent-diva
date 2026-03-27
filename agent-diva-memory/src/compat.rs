//! Compatibility helpers for the minimal MEMORY.md / HISTORY.md layer.

use std::path::{Path, PathBuf};

pub fn memory_dir_path(workspace: &Path) -> PathBuf {
    workspace.join("memory")
}

pub fn long_term_memory_file_path(workspace: &Path) -> PathBuf {
    memory_dir_path(workspace).join("MEMORY.md")
}

pub fn history_file_path(workspace: &Path) -> PathBuf {
    memory_dir_path(workspace).join("HISTORY.md")
}
