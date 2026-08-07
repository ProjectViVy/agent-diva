use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::Serialize;

use crate::{atomic::atomic_write_json, AutoDreamError, Result};

const CHECKPOINT_SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoDreamPaths {
    workspace_root: PathBuf,
    autodream_dir: PathBuf,
}

impl AutoDreamPaths {
    pub fn new(workspace_root: impl Into<PathBuf>) -> Self {
        let workspace_root = workspace_root.into();
        let autodream_dir = workspace_root.join(".agent-diva").join("autodream");
        Self {
            workspace_root,
            autodream_dir,
        }
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn autodream_dir(&self) -> &Path {
        &self.autodream_dir
    }

    pub fn lock_file(&self) -> PathBuf {
        self.autodream_dir.join("lock")
    }

    pub fn checkpoint_file(&self) -> PathBuf {
        self.autodream_dir.join("checkpoint")
    }

    pub fn events_jsonl(&self) -> PathBuf {
        self.autodream_dir.join("events.jsonl")
    }

    pub fn runs_dir(&self) -> PathBuf {
        self.autodream_dir.join("runs")
    }

    pub fn run_dir(&self, run_id: &str) -> PathBuf {
        self.runs_dir().join(run_id)
    }

    pub fn run_record_file(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("record.json")
    }

    pub fn run_artifact_file(&self, run_id: &str) -> PathBuf {
        self.run_dir(run_id).join("autodream_run.json")
    }

    pub fn reports_dir(&self) -> PathBuf {
        self.workspace_root.join(".laputa").join("reports")
    }

    pub fn daily_reports_dir(&self) -> PathBuf {
        self.reports_dir().join("daily")
    }

    pub fn weekly_reports_dir(&self) -> PathBuf {
        self.reports_dir().join("weekly")
    }

    pub fn daily_report_file(&self, date: &str) -> PathBuf {
        self.daily_reports_dir().join(format!("{date}.md"))
    }

    pub fn weekly_report_file(&self, week: &str) -> PathBuf {
        self.weekly_reports_dir().join(format!("{week}.md"))
    }

    pub fn monthly_reports_dir(&self) -> PathBuf {
        self.reports_dir().join("monthly")
    }

    pub fn monthly_report_file(&self, month: &str) -> PathBuf {
        self.monthly_reports_dir().join(format!("{month}.md"))
    }

    pub fn monthly_error_marker_file(&self, month: &str) -> PathBuf {
        self.monthly_reports_dir()
            .join(format!("{month}.error.json"))
    }

    pub fn compact_dir(&self) -> PathBuf {
        self.workspace_root.join(".agent-diva").join("compact")
    }

    pub fn compact_capsules_dir(&self) -> PathBuf {
        self.compact_dir().join("capsules")
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.workspace_root.join("sessions")
    }

    fn directories(&self) -> [PathBuf; 2] {
        [self.autodream_dir.clone(), self.runs_dir()]
    }
}

#[derive(Debug, Clone)]
pub struct AutoDreamStorage {
    paths: AutoDreamPaths,
}

impl AutoDreamStorage {
    pub fn open(workspace_root: impl Into<PathBuf>) -> Result<Self> {
        let paths = AutoDreamPaths::new(workspace_root);
        for dir in paths.directories() {
            fs::create_dir_all(&dir).map_err(|source| AutoDreamError::io(dir, source))?;
        }

        migrate_legacy_report_artifacts(&paths)?;

        let checkpoint = paths.checkpoint_file();
        if !checkpoint.exists() {
            atomic_write_json(&checkpoint, &InitialCheckpoint::default())?;
        }

        let events = paths.events_jsonl();
        if !events.exists() {
            fs::File::create(&events).map_err(|source| AutoDreamError::io(&events, source))?;
        }

        Ok(Self { paths })
    }

    pub fn paths(&self) -> &AutoDreamPaths {
        &self.paths
    }
}

/// One-time D2 migration: report artifacts live under `.laputa/reports/`.
/// Legacy files are moved only (never rewritten); existing destinations win
/// so the legacy copy remains a rollback source. Idempotent.
fn migrate_legacy_report_artifacts(paths: &AutoDreamPaths) -> Result<()> {
    let legacy_sources = [
        paths.autodream_dir().join("reports"),
        paths.workspace_root().join("reports"),
    ];
    for legacy_root in legacy_sources {
        if !legacy_root.is_dir() {
            continue;
        }
        for period in ["daily", "weekly", "monthly"] {
            let legacy_dir = legacy_root.join(period);
            if !legacy_dir.is_dir() {
                continue;
            }
            let target_dir = paths.reports_dir().join(period);
            fs::create_dir_all(&target_dir)
                .map_err(|source| AutoDreamError::io(&target_dir, source))?;
            for entry in fs::read_dir(&legacy_dir)
                .map_err(|source| AutoDreamError::io(&legacy_dir, source))?
            {
                let entry = entry.map_err(|source| AutoDreamError::io(&legacy_dir, source))?;
                if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    continue;
                }
                let target = target_dir.join(entry.file_name());
                if target.exists() {
                    continue;
                }
                fs::rename(entry.path(), &target)
                    .map_err(|source| AutoDreamError::io(&target, source))?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Serialize)]
struct InitialCheckpoint {
    schema_version: &'static str,
    last_completed_run_id: Option<String>,
    last_completed_at: Option<String>,
    auto_mode_enabled: bool,
    session_threshold_enabled: bool,
}

impl Default for InitialCheckpoint {
    fn default() -> Self {
        Self {
            schema_version: CHECKPOINT_SCHEMA_VERSION,
            last_completed_run_id: None,
            last_completed_at: None,
            auto_mode_enabled: false,
            session_threshold_enabled: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn legacy_report_artifacts_are_moved_into_laputa_reports_on_open() {
        let temp = tempfile::tempdir().unwrap();
        seed(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-14.md"),
            "legacy daily",
        );
        seed(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/weekly/2026-W24.md"),
            "legacy weekly",
        );
        seed(
            &temp.path().join("reports/monthly/2026-06.md"),
            "legacy monthly",
        );

        let storage = AutoDreamStorage::open(temp.path()).unwrap();

        assert_eq!(
            fs::read_to_string(storage.paths().daily_report_file("2026-06-14")).unwrap(),
            "legacy daily"
        );
        assert_eq!(
            fs::read_to_string(storage.paths().weekly_report_file("2026-W24")).unwrap(),
            "legacy weekly"
        );
        assert_eq!(
            fs::read_to_string(storage.paths().monthly_report_file("2026-06")).unwrap(),
            "legacy monthly"
        );
        assert!(!temp
            .path()
            .join(".agent-diva/autodream/reports/daily/2026-06-14.md")
            .exists());
        assert!(!temp.path().join("reports/monthly/2026-06.md").exists());
    }

    #[test]
    fn report_migration_is_idempotent_and_keeps_conflicting_legacy_copies() {
        let temp = tempfile::tempdir().unwrap();
        seed(
            &temp
                .path()
                .join(".agent-diva/autodream/reports/daily/2026-06-14.md"),
            "legacy daily",
        );
        seed(
            &temp.path().join(".laputa/reports/daily/2026-06-14.md"),
            "existing destination",
        );

        AutoDreamStorage::open(temp.path()).unwrap();
        // Open again: migration must stay idempotent.
        let storage = AutoDreamStorage::open(temp.path()).unwrap();

        // Existing destinations win; the legacy copy remains as rollback source.
        assert_eq!(
            fs::read_to_string(storage.paths().daily_report_file("2026-06-14")).unwrap(),
            "existing destination"
        );
        assert_eq!(
            fs::read_to_string(
                temp.path()
                    .join(".agent-diva/autodream/reports/daily/2026-06-14.md")
            )
            .unwrap(),
            "legacy daily"
        );
    }
}
