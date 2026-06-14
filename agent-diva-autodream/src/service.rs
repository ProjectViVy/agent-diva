use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::{Duration, SystemTime},
};

use agent_diva_core::evolution::{AutoDreamRunRecord, AutoDreamRunState};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    atomic::atomic_write_json, AutoDreamCollectedInputs, AutoDreamError, AutoDreamInputCollector,
    AutoDreamStorage, AutoDreamWorker, AutoDreamWorkerReport, Result,
};

const DEFAULT_STALE_LOCK_SECS: u64 = 60 * 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDreamCheckpoint {
    pub schema_version: String,
    pub last_completed_run_id: Option<String>,
    pub last_completed_at: Option<DateTime<Utc>>,
    pub auto_mode_enabled: bool,
    pub session_threshold_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDreamLockRecord {
    pub run_id: String,
    pub pid: Option<u32>,
    pub trigger: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDreamEvent {
    pub id: String,
    pub run_id: Option<String>,
    pub kind: String,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDreamRunStatus {
    pub run: AutoDreamRunRecord,
    pub lock: Option<AutoDreamLockRecord>,
    pub auto_mode_enabled: bool,
    pub session_threshold_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDreamRunList {
    pub runs: Vec<AutoDreamRunRecord>,
    pub active_run_id: Option<String>,
    pub auto_mode_enabled: bool,
    pub session_threshold_enabled: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManualRunTriggerRequest {
    pub trigger: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AutoDreamService {
    storage: AutoDreamStorage,
    stale_lock_after: Duration,
}

impl AutoDreamService {
    pub fn open(workspace_root: impl Into<std::path::PathBuf>) -> Result<Self> {
        let storage = AutoDreamStorage::open(workspace_root)?;
        Ok(Self::from_storage(storage))
    }

    pub fn from_storage(storage: AutoDreamStorage) -> Self {
        Self {
            storage,
            stale_lock_after: Duration::from_secs(DEFAULT_STALE_LOCK_SECS),
        }
    }

    pub fn with_stale_lock_after(mut self, stale_lock_after: Duration) -> Self {
        self.stale_lock_after = stale_lock_after;
        self
    }

    pub fn trigger_manual_run(
        &self,
        request: ManualRunTriggerRequest,
    ) -> Result<AutoDreamRunStatus> {
        self.recover_stale_lock_if_needed()?;
        if let Some(lock) = self.read_lock()? {
            return Err(AutoDreamError::ActiveRunExists { id: lock.run_id });
        }

        let now = Utc::now();
        let trigger = request
            .trigger
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("manual")
            .to_string();
        let run_id = format!("run-{}", Uuid::new_v4());
        let run = AutoDreamRunRecord {
            id: run_id.clone(),
            started_at: now,
            completed_at: None,
            state: AutoDreamRunState::Running,
            trigger: trigger.clone(),
            summary: Some("manual run started".to_string()),
            input_summary: None,
            proposal_ids: Vec::new(),
            error: None,
        };
        self.write_run(&run)?;

        let lock = AutoDreamLockRecord {
            run_id: run_id.clone(),
            pid: Some(std::process::id()),
            trigger,
            started_at: now,
        };
        self.write_lock(&lock)?;
        self.append_event(AutoDreamEvent {
            id: format!("evt-{}", Uuid::new_v4()),
            run_id: Some(run_id),
            kind: "manual_run_started".to_string(),
            message: "manual AutoDream run started".to_string(),
            created_at: now,
        })?;
        self.status_from_run(run, Some(lock))
    }

    pub fn get_run_status(&self, run_id: &str) -> Result<AutoDreamRunStatus> {
        self.recover_stale_lock_if_needed()?;
        let run = self.read_run(run_id)?;
        let lock = self.read_lock()?.filter(|lock| lock.run_id == run.id);
        self.status_from_run(run, lock)
    }

    pub fn cancel_run(&self, run_id: &str) -> Result<AutoDreamRunStatus> {
        self.recover_stale_lock_if_needed()?;
        let mut run = self.read_run(run_id)?;
        if !matches!(
            run.state,
            AutoDreamRunState::Pending | AutoDreamRunState::Running
        ) {
            return Err(AutoDreamError::RunNotCancellable {
                id: run.id,
                state: run.state,
            });
        }

        let now = Utc::now();
        run.state = AutoDreamRunState::Cancelled;
        run.completed_at = Some(now);
        run.summary = Some("manual run cancelled".to_string());
        run.error = Some("cancelled".to_string());
        self.write_run(&run)?;

        let active_lock = self.read_lock()?;
        if active_lock.as_ref().map(|lock| lock.run_id.as_str()) == Some(run_id) {
            let path = self.storage.paths().lock_file();
            fs::remove_file(&path).map_err(|source| AutoDreamError::io(path, source))?;
        }

        self.append_event(AutoDreamEvent {
            id: format!("evt-{}", Uuid::new_v4()),
            run_id: Some(run.id.clone()),
            kind: "manual_run_cancelled".to_string(),
            message: "manual AutoDream run cancelled".to_string(),
            created_at: now,
        })?;
        self.status_from_run(run, None)
    }

    pub fn list_runs(&self) -> Result<AutoDreamRunList> {
        self.recover_stale_lock_if_needed()?;
        let mut runs = self.read_all_runs()?;
        runs.sort_by(|left, right| {
            right
                .started_at
                .cmp(&left.started_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        let checkpoint = self.read_checkpoint()?;
        let active_run_id = self.read_lock()?.map(|lock| lock.run_id);
        Ok(AutoDreamRunList {
            runs,
            active_run_id,
            auto_mode_enabled: checkpoint.auto_mode_enabled,
            session_threshold_enabled: checkpoint.session_threshold_enabled,
        })
    }

    pub fn checkpoint(&self) -> Result<AutoDreamCheckpoint> {
        self.read_checkpoint()
    }

    pub fn collect_inputs(&self, run_id: &str) -> Result<AutoDreamCollectedInputs> {
        let mut run = self.read_run(run_id)?;
        let collector = AutoDreamInputCollector::new(
            self.storage.clone(),
            agent_diva_laputa::LaputaService::open(self.storage.paths().workspace_root())
                .map_err(|error| AutoDreamError::InputCollection(error.to_string()))?,
        );
        let collected = collector.collect(run_id)?;
        run.input_summary = Some(collected.summary.clone());
        run.summary = Some(format!(
            "collected {} inputs with {} omissions",
            collected.summary.total_items,
            collected.summary.omissions.len()
        ));
        self.write_run(&run)?;
        Ok(collected)
    }

    pub fn execute_reflection_worker(&self, run_id: &str) -> Result<AutoDreamWorkerReport> {
        let worker = AutoDreamWorker::new(
            self.storage.clone(),
            agent_diva_laputa::LaputaService::open(self.storage.paths().workspace_root())
                .map_err(|error| AutoDreamError::InputCollection(error.to_string()))?,
        );
        worker.execute(run_id)
    }

    fn status_from_run(
        &self,
        run: AutoDreamRunRecord,
        lock: Option<AutoDreamLockRecord>,
    ) -> Result<AutoDreamRunStatus> {
        let checkpoint = self.read_checkpoint()?;
        Ok(AutoDreamRunStatus {
            run,
            lock,
            auto_mode_enabled: checkpoint.auto_mode_enabled,
            session_threshold_enabled: checkpoint.session_threshold_enabled,
        })
    }

    fn recover_stale_lock_if_needed(&self) -> Result<()> {
        let lock_path = self.storage.paths().lock_file();
        if !lock_path.exists() {
            return Ok(());
        }
        let metadata =
            fs::metadata(&lock_path).map_err(|source| AutoDreamError::io(&lock_path, source))?;
        let modified = metadata
            .modified()
            .map_err(|source| AutoDreamError::io(&lock_path, source))?;
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::ZERO);
        if age < self.stale_lock_after {
            return Ok(());
        }

        if let Some(lock) = self.read_lock()? {
            if let Ok(mut run) = self.read_run(&lock.run_id) {
                if matches!(
                    run.state,
                    AutoDreamRunState::Pending | AutoDreamRunState::Running
                ) {
                    let now = Utc::now();
                    run.state = AutoDreamRunState::Failed;
                    run.completed_at = Some(now);
                    run.summary = Some("stale lock recovered".to_string());
                    run.error = Some("stale lock recovered".to_string());
                    self.write_run(&run)?;
                    self.append_event(AutoDreamEvent {
                        id: format!("evt-{}", Uuid::new_v4()),
                        run_id: Some(run.id.clone()),
                        kind: "stale_lock_recovered".to_string(),
                        message: "stale AutoDream lock recovered".to_string(),
                        created_at: now,
                    })?;
                }
            }
        }

        fs::remove_file(&lock_path).map_err(|source| AutoDreamError::io(lock_path, source))?;
        Ok(())
    }

    fn read_checkpoint(&self) -> Result<AutoDreamCheckpoint> {
        read_json_file(self.storage.paths().checkpoint_file())
    }

    fn read_lock(&self) -> Result<Option<AutoDreamLockRecord>> {
        let path = self.storage.paths().lock_file();
        match fs::read_to_string(&path) {
            Ok(content) => Ok(Some(serde_json::from_str(&content)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(AutoDreamError::io(path, source)),
        }
    }

    fn write_lock(&self, lock: &AutoDreamLockRecord) -> Result<()> {
        let path = self.storage.paths().lock_file();
        let bytes = serde_json::to_vec_pretty(lock)?;
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&path)
            .map_err(|source| AutoDreamError::io(&path, source))?;
        file.write_all(&bytes)
            .map_err(|source| AutoDreamError::io(&path, source))?;
        file.sync_all()
            .map_err(|source| AutoDreamError::io(&path, source))?;
        Ok(())
    }

    fn read_run(&self, run_id: &str) -> Result<AutoDreamRunRecord> {
        let path = self.storage.paths().run_record_file(run_id);
        read_json_file(&path).map_err(|error| match error {
            AutoDreamError::Io { source, .. } if source.kind() == std::io::ErrorKind::NotFound => {
                AutoDreamError::RunNotFound {
                    id: run_id.to_string(),
                }
            }
            other => other,
        })
    }

    fn write_run(&self, run: &AutoDreamRunRecord) -> Result<()> {
        let dir = self.storage.paths().run_dir(&run.id);
        fs::create_dir_all(&dir).map_err(|source| AutoDreamError::io(&dir, source))?;
        atomic_write_json(&self.storage.paths().run_record_file(&run.id), run)
    }

    fn read_all_runs(&self) -> Result<Vec<AutoDreamRunRecord>> {
        let runs_dir = self.storage.paths().runs_dir();
        let mut runs = Vec::new();
        for entry in
            fs::read_dir(&runs_dir).map_err(|source| AutoDreamError::io(&runs_dir, source))?
        {
            let entry = entry.map_err(|source| AutoDreamError::io(&runs_dir, source))?;
            let record = entry.path().join("record.json");
            if record.exists() {
                runs.push(read_json_file(record)?);
            }
        }
        Ok(runs)
    }

    fn append_event(&self, event: AutoDreamEvent) -> Result<()> {
        let path = self.storage.paths().events_jsonl();
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|source| AutoDreamError::io(&path, source))?;
        let line = serde_json::to_string(&event)?;
        writeln!(file, "{line}").map_err(|source| AutoDreamError::io(&path, source))?;
        file.sync_all()
            .map_err(|source| AutoDreamError::io(&path, source))?;
        Ok(())
    }
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|source| AutoDreamError::io(path, source))?;
    Ok(serde_json::from_str(&content)?)
}
