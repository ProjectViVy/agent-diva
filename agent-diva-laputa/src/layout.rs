use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{atomic_write_json, LaputaError, Result};

const STATE_SCHEMA_VERSION: &str = "1.0.0";

/// Typed path helpers for the surviving workspace-local Laputa stores.
///
/// The cognitive clean break removed the section/proposal/changelog/cognitive
/// layout families; what remains is the state marker, the workspace-local
/// typed memory database (offline migration source), lock files, and the
/// payload-free recall-feedback journal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaputaPaths {
    workspace_root: PathBuf,
    laputa_dir: PathBuf,
}

impl LaputaPaths {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let laputa_dir = workspace_root.join(".laputa");

        Self {
            workspace_root,
            laputa_dir,
        }
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn laputa_dir(&self) -> &Path {
        &self.laputa_dir
    }

    pub fn state_json(&self) -> PathBuf {
        self.laputa_dir.join("state.json")
    }

    pub fn locks_dir(&self) -> PathBuf {
        self.laputa_dir.join("locks")
    }

    pub fn recall_feedback_json(&self) -> PathBuf {
        self.laputa_dir.join("recall-feedback.json")
    }

    /// Legacy workspace typed memory database (offline migration source only).
    pub fn memory_database(&self) -> PathBuf {
        self.laputa_dir.join("memory.sqlite3")
    }

    pub fn lock_file(&self, name: &str) -> PathBuf {
        self.locks_dir().join(format!("{name}.lock"))
    }

    fn directories(&self) -> [PathBuf; 2] {
        [self.laputa_dir.clone(), self.locks_dir()]
    }
}

/// Initialized Laputa file-first storage boundary.
#[derive(Debug, Clone)]
pub struct LaputaStorage {
    paths: LaputaPaths,
}

impl LaputaStorage {
    /// Create the surviving `.laputa/` layout if needed.
    pub fn open(workspace_root: impl Into<PathBuf>) -> Result<Self> {
        let paths = LaputaPaths::new(workspace_root);
        for directory in paths.directories() {
            fs::create_dir_all(&directory).map_err(|source| LaputaError::io(directory, source))?;
        }

        let state_path = paths.state_json();
        if !state_path.exists() {
            atomic_write_json(&state_path, &InitialState::default())?;
        }

        Ok(Self { paths })
    }

    pub fn paths(&self) -> &LaputaPaths {
        &self.paths
    }
}

#[derive(Debug, Serialize)]
struct InitialState {
    schema_version: &'static str,
}

impl Default for InitialState {
    fn default() -> Self {
        Self {
            schema_version: STATE_SCHEMA_VERSION,
        }
    }
}
