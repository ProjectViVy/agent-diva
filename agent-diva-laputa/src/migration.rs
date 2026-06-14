use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use agent_diva_core::evolution::LaputaSectionName;
use chrono::{DateTime, Utc};
use serde_json::json;

use crate::{atomic_write_json, LaputaError, LaputaStorage, Result};

const MIGRATED_SCHEMA_VERSION: &str = "1.1.0";

/// Explicit Laputa legacy migration entrypoint.
#[derive(Debug, Clone)]
pub struct LaputaMigration {
    storage: LaputaStorage,
}

impl LaputaMigration {
    pub fn new(storage: LaputaStorage) -> Self {
        Self { storage }
    }

    pub fn run(&self, options: LaputaMigrationOptions) -> Result<LaputaMigrationOutcome> {
        let test_failure = options.test_failure;
        let sources = match options.sources {
            Some(sources) => sources,
            None => discover_legacy_sources(self.storage.paths().workspace_root())?,
        };
        let migration_time = Utc::now();
        let timestamp = windows_safe_timestamp(migration_time);
        let staging_dir = self.storage.paths().staging_dir();
        let backup_dir = self.storage.paths().legacy_backup_dir(&timestamp);

        cleanup_staging_dir(&staging_dir)?;
        fs::create_dir_all(&staging_dir).map_err(|source| LaputaError::io(&staging_dir, source))?;
        fs::create_dir_all(&backup_dir).map_err(|source| LaputaError::io(&backup_dir, source))?;

        let result = self.stage_and_commit(
            &sources,
            &staging_dir,
            &backup_dir,
            migration_time,
            test_failure,
        );

        cleanup_staging_dir(&staging_dir)?;
        result
    }

    fn stage_and_commit(
        &self,
        sources: &[LaputaMigrationSource],
        staging_dir: &Path,
        backup_dir: &Path,
        migrated_at: DateTime<Utc>,
        test_failure: Option<LaputaMigrationTestFailure>,
    ) -> Result<LaputaMigrationOutcome> {
        let mut outcome = LaputaMigrationOutcome::default();
        let mut section_entries = BTreeMap::<String, (LaputaSectionName, Vec<SectionEntry>)>::new();
        let mut section_status = BTreeMap::<String, &'static str>::new();
        let mut section_reasons = BTreeMap::<String, String>::new();

        for source in sources {
            let content = fs::read_to_string(&source.path)
                .map_err(|error| LaputaError::io(&source.path, error))?;
            let backup_path = backup_path_for(backup_dir, &source.path);
            if let Some(parent) = backup_path.parent() {
                fs::create_dir_all(parent).map_err(|error| LaputaError::io(parent, error))?;
            }
            fs::copy(&source.path, &backup_path)
                .map_err(|error| LaputaError::io(&backup_path, error))?;
            outcome.backed_up_sources.push(LaputaMigrationBackup {
                source_path: source.path.clone(),
                backup_path,
            });

            if let Some(section) = source.kind.section() {
                let section_key = section.as_str().to_string();
                let status = source.kind.status();
                section_status.insert(section_key.clone(), status);
                if let LaputaMigrationSourceKind::Unsupported { reason, .. } = &source.kind {
                    section_reasons.insert(section_key.clone(), reason.clone());
                }
                section_entries
                    .entry(section_key)
                    .or_insert_with(|| (section, Vec::new()))
                    .1
                    .push(SectionEntry {
                        source_path: source.path.clone(),
                        content,
                        migrated_at,
                    });
            }
        }

        let sections = section_entries
            .into_iter()
            .map(|(section_key, (section, entries))| {
                section_payload_for(
                    section.clone(),
                    section_status.get(&section_key).copied().unwrap_or("owned"),
                    entries,
                    section_reasons.get(&section_key).cloned(),
                )
            })
            .collect::<Vec<_>>();

        for staged in &sections {
            let staged_path = staging_dir.join(format!("{}.json", staged.section.as_str()));
            atomic_write_json(staged_path, &staged.payload)?;
        }

        let staged_state = staging_dir.join("state.json");
        atomic_write_json(
            &staged_state,
            &json!({
                "schema_version": MIGRATED_SCHEMA_VERSION,
                "legacy_migration": {
                    "migrated_at": migrated_at,
                    "source_count": sources.len(),
                    "backup_dir": backup_dir,
                }
            }),
        )?;

        if test_failure == Some(LaputaMigrationTestFailure::AfterStagingBeforeCommit) {
            return Err(LaputaError::InjectedMigrationFailure {
                point: LaputaMigrationTestFailure::AfterStagingBeforeCommit,
            });
        }

        for staged in sections {
            let section_path = self.storage.paths().section_file(staged.section.clone());
            atomic_write_json(&section_path, &staged.payload)?;
            outcome
                .written_sections
                .push(staged.section.as_str().to_string());
        }
        atomic_write_json(
            self.storage.paths().state_json(),
            &json!({
                "schema_version": MIGRATED_SCHEMA_VERSION,
                "legacy_migration": {
                    "migrated_at": migrated_at,
                    "source_count": sources.len(),
                    "backup_dir": backup_dir,
                }
            }),
        )?;

        outcome.written_sections.sort();
        outcome.written_sections.dedup();
        Ok(outcome)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaputaMigrationOptions {
    pub sources: Option<Vec<LaputaMigrationSource>>,
    pub test_failure: Option<LaputaMigrationTestFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaputaMigrationSource {
    pub path: PathBuf,
    pub kind: LaputaMigrationSourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaputaMigrationSourceKind {
    Supported {
        section: LaputaSectionName,
    },
    Unsupported {
        section: LaputaSectionName,
        reason: String,
    },
    BootstrapOnly {
        reason: String,
    },
}

impl LaputaMigrationSourceKind {
    fn section(&self) -> Option<LaputaSectionName> {
        match self {
            Self::Supported { section } | Self::Unsupported { section, .. } => {
                Some(section.clone())
            }
            Self::BootstrapOnly { .. } => None,
        }
    }

    fn status(&self) -> &'static str {
        match self {
            Self::Supported { .. } => "owned",
            Self::Unsupported { .. } => "tbd",
            Self::BootstrapOnly { .. } => "bootstrap_only",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaputaMigrationTestFailure {
    AfterStagingBeforeCommit,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LaputaMigrationOutcome {
    pub backed_up_sources: Vec<LaputaMigrationBackup>,
    pub written_sections: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaputaMigrationBackup {
    pub source_path: PathBuf,
    pub backup_path: PathBuf,
}

#[derive(Debug)]
struct StagedSection {
    section: LaputaSectionName,
    payload: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
struct SectionEntry {
    source_path: PathBuf,
    content: String,
    migrated_at: DateTime<Utc>,
}

fn discover_legacy_sources(workspace_root: &Path) -> Result<Vec<LaputaMigrationSource>> {
    let candidates = [
        (
            "SOUL.md",
            LaputaMigrationSourceKind::Supported {
                section: LaputaSectionName::Identity,
            },
        ),
        (
            "IDENTITY.md",
            LaputaMigrationSourceKind::Supported {
                section: LaputaSectionName::Identity,
            },
        ),
        (
            "memory/MEMORY.md",
            LaputaMigrationSourceKind::Supported {
                section: LaputaSectionName::MemoryMd,
            },
        ),
        (
            "memory/HISTORY.md",
            LaputaMigrationSourceKind::Supported {
                section: LaputaSectionName::HistoryMd,
            },
        ),
        (
            "USER.md",
            LaputaMigrationSourceKind::Supported {
                section: LaputaSectionName::Relationship,
            },
        ),
        (
            "PROFILE.md",
            LaputaMigrationSourceKind::Supported {
                section: LaputaSectionName::Relationship,
            },
        ),
        (
            "TASK.md",
            LaputaMigrationSourceKind::Unsupported {
                section: LaputaSectionName::JournalReflective,
                reason: "legacy TASK.md maps to TBD journal_reflective schema".to_string(),
            },
        ),
        (
            "BOOTSTRAP.md",
            LaputaMigrationSourceKind::BootstrapOnly {
                reason: "BOOTSTRAP.md is one-time initialization input and is not persisted into Laputa sections"
                    .to_string(),
            },
        ),
    ];

    let mut sources = Vec::new();
    for (relative, kind) in candidates {
        let path = workspace_root.join(relative);
        if path.is_file() {
            sources.push(LaputaMigrationSource { path, kind });
        }
    }
    Ok(sources)
}

fn section_payload_for(
    section: LaputaSectionName,
    status: &'static str,
    entries: Vec<SectionEntry>,
    reason: Option<String>,
) -> StagedSection {
    let mut metadata = json!({
        "migration": "legacy_schema",
        "content_type": if status == "tbd" { "tbd" } else { "legacy_markdown" },
    });
    if let Some(reason) = reason {
        metadata["reason"] = json!(reason);
    }

    StagedSection {
        section,
        payload: json!({
            "status": status,
            "entries": entries,
            "metadata": metadata,
        }),
    }
}

fn backup_path_for(backup_dir: &Path, source_path: &Path) -> PathBuf {
    let file_name = source_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("legacy-source"));
    backup_dir.join(file_name)
}

fn cleanup_staging_dir(staging_dir: &Path) -> Result<()> {
    match fs::remove_dir_all(staging_dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(LaputaError::io(staging_dir, error)),
    }
}

fn windows_safe_timestamp(timestamp: DateTime<Utc>) -> String {
    timestamp.format("%Y%m%dT%H%M%SZ").to_string()
}
