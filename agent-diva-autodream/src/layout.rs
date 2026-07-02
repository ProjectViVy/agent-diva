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
        self.autodream_dir.join("reports")
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
        self.workspace_root.join("reports").join("monthly")
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
