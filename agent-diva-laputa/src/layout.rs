use std::{
    fs,
    path::{Path, PathBuf},
};

use agent_diva_core::evolution::LaputaSectionName;
use serde::Serialize;

use crate::{atomic_write_json, LaputaError, Result};

const STATE_SCHEMA_VERSION: &str = "1.0.0";

/// Typed path helpers for all file-first Laputa authority locations.
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

    pub fn proposals_dir(&self) -> PathBuf {
        self.laputa_dir.join("proposals")
    }

    pub fn changelog_dir(&self) -> PathBuf {
        self.laputa_dir.join("changelog")
    }

    pub fn audit_dir(&self) -> PathBuf {
        self.laputa_dir.join("audit")
    }

    pub fn rollback_dir(&self) -> PathBuf {
        self.laputa_dir.join("rollback")
    }

    pub fn migrations_dir(&self) -> PathBuf {
        self.laputa_dir.join("migrations")
    }

    pub fn locks_dir(&self) -> PathBuf {
        self.laputa_dir.join("locks")
    }

    pub fn legacy_dir(&self) -> PathBuf {
        self.laputa_dir.join("legacy")
    }

    pub fn lock_file(&self, name: &str) -> PathBuf {
        self.locks_dir().join(format!("{name}.lock"))
    }

    pub fn section_file(&self, section: LaputaSectionName) -> PathBuf {
        self.laputa_dir
            .join("sections")
            .join(format!("{}.json", section_file_stem(section)))
    }

    fn directories(&self) -> [PathBuf; 9] {
        [
            self.laputa_dir.clone(),
            self.proposals_dir(),
            self.changelog_dir(),
            self.audit_dir(),
            self.rollback_dir(),
            self.migrations_dir(),
            self.locks_dir(),
            self.legacy_dir(),
            self.laputa_dir.join("sections"),
        ]
    }
}

/// Initialized Laputa file-first storage boundary.
#[derive(Debug, Clone)]
pub struct LaputaStorage {
    paths: LaputaPaths,
}

impl LaputaStorage {
    /// Create the `.laputa/` layout if needed and return typed path helpers.
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

fn section_file_stem(section: LaputaSectionName) -> &'static str {
    match section {
        LaputaSectionName::Identity => "identity",
        LaputaSectionName::Relationship => "relationship",
        LaputaSectionName::Commitment => "commitment",
        LaputaSectionName::Preferences => "preferences",
        LaputaSectionName::MemoryMd => "memory_md",
        LaputaSectionName::HistoryMd => "history_md",
        LaputaSectionName::Daily => "daily",
        LaputaSectionName::Weekly => "weekly",
        LaputaSectionName::Monthly => "monthly",
        LaputaSectionName::JournalReflective => "journal_reflective",
        LaputaSectionName::ProposalInbox => "proposal_inbox",
        LaputaSectionName::Changelog => "changelog",
        LaputaSectionName::ReportIndexes => "report_indexes",
        LaputaSectionName::AaakSummaries => "aaak_summaries",
    }
}
