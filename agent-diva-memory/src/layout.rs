//! Path layout helpers for the enhanced memory subsystem.

use std::path::{Path, PathBuf};

pub fn diary_dir_path(workspace: &Path) -> PathBuf {
    workspace.join("memory").join("diary")
}

pub fn rational_diary_dir_path(workspace: &Path) -> PathBuf {
    diary_dir_path(workspace).join("rational")
}

pub fn emotional_diary_dir_path(workspace: &Path) -> PathBuf {
    diary_dir_path(workspace).join("emotional")
}

pub fn index_dir_path(workspace: &Path) -> PathBuf {
    workspace.join("memory").join("index")
}

pub fn brain_db_path(workspace: &Path) -> PathBuf {
    workspace.join("memory").join("brain.db")
}
