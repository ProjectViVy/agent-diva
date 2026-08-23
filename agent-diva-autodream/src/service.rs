use std::{
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::Path,
    sync::{Arc, OnceLock},
    time::{Duration, SystemTime},
};

use agent_diva_core::config::LlmCurationConfig;
use agent_diva_core::evolution::{
    AutoDreamFailureCode, AutoDreamOrchestrationPhase, AutoDreamOrchestrationRecord,
    AutoDreamRunRecord, AutoDreamRunState, SkillHome,
};
use agent_diva_core::reports::ReportNarrativeGenerator;
use agent_diva_laputa::MemoryHome;
use chrono::{DateTime, Datelike, NaiveDate, Utc, Weekday};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    atomic::atomic_write_json,
    metrics::{AutoDreamMetrics, AutoDreamMetricsSnapshot},
    AutoDreamCollectedInputs, AutoDreamError, AutoDreamInputCollector,
    AutoDreamMonthlyReportGenerator, AutoDreamRhythmReportGenerator, AutoDreamStorage,
    AutoDreamWorker, AutoDreamWorkerReport, MonthlyReportErrorMarker, Result,
    SkillReflectionEngine,
};

const DEFAULT_STALE_LOCK_SECS: u64 = 60 * 5;
const REPORT_TRIGGER_MAX_ATTEMPTS: u32 = 3;
const MONTHLY_REPORT_TRIGGER: &str = "notebook-monthly";
static AUTODREAM_METRICS: OnceLock<AutoDreamMetrics> = OnceLock::new();

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proposal_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure_code: Option<String>,
}

impl AutoDreamEvent {
    pub fn new(
        run_id: Option<String>,
        kind: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("evt-{}", Uuid::new_v4()),
            run_id,
            kind: kind.into(),
            message: message.into(),
            created_at: Utc::now(),
            phase: None,
            input_summary: None,
            gate_code: None,
            proposal_id: None,
            failure_code: None,
        }
    }

    pub fn with_failure_code(mut self, code: impl Into<String>) -> Self {
        self.failure_code = Some(code.into());
        self
    }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduledMonthlyReportOutcome {
    Triggered { run_id: String, month_key: String },
    Skipped { month_key: String, reason: String },
    AlreadyGenerated { month_key: String },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManualRunTriggerRequest {
    pub trigger: Option<String>,
}

#[derive(Clone)]
pub struct AutoDreamService {
    storage: AutoDreamStorage,
    stale_lock_after: Duration,
    narrative_generator: Option<Arc<dyn ReportNarrativeGenerator>>,
    skill_reflection_engine: Option<Arc<dyn SkillReflectionEngine>>,
    memory_home: Option<MemoryHome>,
    skill_home: Option<SkillHome>,
    llm_curation: LlmCurationConfig,
}

impl AutoDreamService {
    fn metrics() -> &'static AutoDreamMetrics {
        AUTODREAM_METRICS.get_or_init(AutoDreamMetrics::new)
    }

    #[must_use]
    pub fn metrics_snapshot() -> AutoDreamMetricsSnapshot {
        Self::metrics().snapshot()
    }

    #[doc(hidden)]
    pub fn reset_metrics_for_test() {
        Self::metrics().reset_for_test();
    }

    pub fn open(workspace_root: impl Into<std::path::PathBuf>) -> Result<Self> {
        let storage = AutoDreamStorage::open(workspace_root)?;
        Ok(Self::from_storage(storage))
    }

    pub fn from_storage(storage: AutoDreamStorage) -> Self {
        Self {
            storage,
            stale_lock_after: Duration::from_secs(DEFAULT_STALE_LOCK_SECS),
            narrative_generator: None,
            skill_reflection_engine: None,
            memory_home: None,
            skill_home: None,
            llm_curation: LlmCurationConfig::default(),
        }
    }

    pub fn with_stale_lock_after(mut self, stale_lock_after: Duration) -> Self {
        self.stale_lock_after = stale_lock_after;
        self
    }

    /// Inject optional LLM narrative generator and curation config.
    pub fn with_report_curation(
        mut self,
        narrative_generator: Option<Arc<dyn ReportNarrativeGenerator>>,
        llm_curation: LlmCurationConfig,
    ) -> Self {
        self.narrative_generator = narrative_generator;
        self.llm_curation = llm_curation;
        self
    }

    pub fn with_skill_reflection_engine(
        mut self,
        engine: Option<Arc<dyn SkillReflectionEngine>>,
    ) -> Self {
        self.skill_reflection_engine = engine;
        self
    }

    /// Attach the machine-wide Memory/ACTMEM authority used by S3 runs.
    pub fn with_memory_home(mut self, memory_home: MemoryHome) -> Self {
        self.memory_home = Some(memory_home);
        self
    }

    pub fn with_skill_home(mut self, skill_home: SkillHome) -> Self {
        self.skill_home = Some(skill_home);
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
            state: AutoDreamRunState::Pending,
            trigger: trigger.clone(),
            summary: Some("manual run started".to_string()),
            input_summary: None,
            proposal_ids: Vec::new(),
            orchestration: Some(AutoDreamOrchestrationRecord {
                schema_version: 1,
                phase: AutoDreamOrchestrationPhase::Queued,
                attempt: 0,
                deadline_at: now + chrono::Duration::minutes(5),
                updated_at: now,
            }),
            failure_code: None,
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
        Self::metrics().record_run();
        self.append_event(AutoDreamEvent::new(
            Some(run_id),
            "manual_run_started",
            "manual AutoDream run started",
        ))?;
        self.status_from_run(run, Some(lock))
    }

    pub fn get_run_status(&self, run_id: &str) -> Result<AutoDreamRunStatus> {
        self.recover_stale_lock_if_needed()?;
        let run = self.read_run(run_id)?;
        let lock = self.read_lock()?.filter(|lock| lock.run_id == run.id);
        self.status_from_run(run, lock)
    }

    /// Return a bounded, payload-free event timeline for one persisted run.
    pub fn list_run_events(&self, run_id: &str, limit: usize) -> Result<Vec<AutoDreamEvent>> {
        self.read_run(run_id)?;
        let path = self.storage.paths().events_jsonl();
        let file = match fs::File::open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(AutoDreamError::io(path, source)),
        };
        let mut events = Vec::new();
        for line in BufReader::new(file).lines() {
            let line = line.map_err(|source| AutoDreamError::io(&path, source))?;
            let Ok(event) = serde_json::from_str::<AutoDreamEvent>(&line) else {
                continue;
            };
            if event.run_id.as_deref() == Some(run_id) {
                events.push(event);
            }
        }
        events.sort_by_key(|event| event.created_at);
        let start = events.len().saturating_sub(limit.clamp(1, 200));
        Ok(events.split_off(start))
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
        run.failure_code = Some(AutoDreamFailureCode::Cancelled);
        if let Some(orchestration) = run.orchestration.as_mut() {
            orchestration.phase = AutoDreamOrchestrationPhase::Cancelled;
            orchestration.updated_at = now;
        }
        run.error = Some("cancelled".to_string());
        self.write_run(&run)?;

        let active_lock = self.read_lock()?;
        if active_lock.as_ref().map(|lock| lock.run_id.as_str()) == Some(run_id) {
            let path = self.storage.paths().lock_file();
            fs::remove_file(&path).map_err(|source| AutoDreamError::io(path, source))?;
        }

        self.append_event(
            AutoDreamEvent::new(
                Some(run.id.clone()),
                "manual_run_cancelled",
                "manual AutoDream run cancelled",
            )
            .with_failure_code("cancelled"),
        )?;
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

    pub fn resumable_runs(&self) -> Result<Vec<(String, String)>> {
        self.recover_stale_lock_if_needed()?;
        let active = self.read_lock()?.map(|lock| lock.run_id);
        let mut resumable = Vec::new();
        for mut run in self.read_all_runs()? {
            if !matches!(
                run.state,
                AutoDreamRunState::Pending | AutoDreamRunState::Running
            ) || active.as_deref() == Some(run.id.as_str())
            {
                continue;
            }
            if run.orchestration.is_none() {
                let now = Utc::now();
                run.state = AutoDreamRunState::Failed;
                run.completed_at = Some(now);
                run.failure_code = Some(AutoDreamFailureCode::LegacyIncomplete);
                run.error =
                    Some("legacy incomplete AutoDream run cannot be recovered safely".to_string());
                self.write_run(&run)?;
                Self::metrics().record_failure();
                self.append_event(
                    AutoDreamEvent::new(
                        Some(run.id.clone()),
                        "legacy_incomplete_run_rejected",
                        "legacy incomplete AutoDream run rejected during recovery",
                    )
                    .with_failure_code("legacy_incomplete"),
                )?;
                continue;
            }
            resumable.push((run.id, run.trigger));
        }
        Ok(resumable)
    }

    pub fn checkpoint(&self) -> Result<AutoDreamCheckpoint> {
        self.read_checkpoint()
    }

    pub fn collect_inputs(&self, run_id: &str) -> Result<AutoDreamCollectedInputs> {
        let mut run = self.read_run(run_id)?;
        let collector = AutoDreamInputCollector::new(self.storage.clone());
        let collected = collector.collect(run_id).inspect_err(|_| {
            Self::metrics().record_failure();
        })?;
        run.input_summary = Some(collected.summary.clone());
        run.summary = Some(format!(
            "collected {} inputs with {} omissions",
            collected.summary.total_items,
            collected.summary.omissions.len()
        ));
        self.write_run(&run)?;
        Ok(collected)
    }

    pub async fn execute_reflection_worker(&self, run_id: &str) -> Result<AutoDreamWorkerReport> {
        let worker = AutoDreamWorker::new(self.storage.clone())
            .with_skill_reflection_engine(self.skill_reflection_engine.clone())
            .with_memory_home(self.memory_home.clone())
            .with_skill_home(self.skill_home.clone());
        worker.execute(run_id).await.inspect_err(|_| {
            Self::metrics().record_failure();
        })
    }

    pub async fn execute_report_trigger(&self, run_id: &str) -> Result<AutoDreamRunStatus> {
        let mut run = self.read_run(run_id)?;
        begin_report_attempt(&mut run)?;
        self.write_run(&run)?;
        let max_attempts = if run.trigger == MONTHLY_REPORT_TRIGGER {
            REPORT_TRIGGER_MAX_ATTEMPTS
        } else {
            1
        };
        let mut last_error = None;
        for attempt in 1..=max_attempts {
            match self
                .generate_report_for_trigger(&run.trigger, attempt)
                .await
            {
                Ok(result) => {
                    let now = Utc::now();
                    run.state = AutoDreamRunState::Completed;
                    run.completed_at = Some(now);
                    run.summary = Some(format!(
                        "generated rhythm report at {}",
                        result.path.display()
                    ));
                    run.failure_code = None;
                    run.error = None;
                    mark_orchestration(&mut run, AutoDreamOrchestrationPhase::Completed, now);
                    self.write_run(&run)?;
                    self.write_checkpoint_success(&run, now)?;
                    self.remove_active_lock(run_id)?;
                    self.append_event(AutoDreamEvent::new(
                        Some(run.id.clone()),
                        "rhythm_report_generated",
                        format!("generated {}", result.path.display()),
                    ))?;
                    return self.status_from_run(run, None);
                }
                Err(error) => {
                    last_error = Some(error);
                    if attempt < max_attempts {
                        self.append_event(AutoDreamEvent::new(
                            Some(run.id.clone()),
                            "rhythm_report_retry",
                            format!(
                                "retrying {} after attempt {attempt} of {max_attempts}",
                                run.trigger
                            ),
                        ))?;
                    }
                }
            }
        }

        let error = last_error.unwrap_or_else(|| {
            AutoDreamError::InvalidState("rhythm report generation failed".to_string())
        });
        let now = Utc::now();
        run.state = AutoDreamRunState::Failed;
        run.completed_at = Some(now);
        run.summary = Some("rhythm report generation failed".to_string());
        run.failure_code = Some(AutoDreamFailureCode::ReportGenerationFailed);
        run.error = Some(error.to_string());
        mark_orchestration(&mut run, AutoDreamOrchestrationPhase::Failed, now);
        self.write_run(&run)?;
        self.remove_active_lock(run_id)?;
        Self::metrics().record_failure();
        self.append_event(
            AutoDreamEvent::new(
                Some(run.id.clone()),
                "rhythm_report_failed",
                error.to_string(),
            )
            .with_failure_code("report_generation_failed"),
        )?;
        Err(error)
    }

    pub async fn execute_scheduled_monthly_report(
        &self,
        date: NaiveDate,
    ) -> Result<ScheduledMonthlyReportOutcome> {
        let month_key = format!("{:04}-{:02}", date.year(), date.month());
        let monthly = self.monthly_generator();
        let report_path = self.storage.paths().monthly_report_file(&month_key);
        if report_path.exists() && monthly.read_error_marker(&month_key)?.is_none() {
            return Ok(ScheduledMonthlyReportOutcome::AlreadyGenerated { month_key });
        }

        let marker = monthly.read_error_marker(&month_key)?;
        if !should_attempt_scheduled_monthly_report(date, marker.as_ref()) {
            return Ok(ScheduledMonthlyReportOutcome::Skipped {
                month_key,
                reason: "outside monthly schedule window".to_string(),
            });
        }
        if self.read_lock()?.is_some() {
            return Ok(ScheduledMonthlyReportOutcome::Skipped {
                month_key,
                reason: "AutoDream run already active".to_string(),
            });
        }

        let status = self.trigger_manual_run(ManualRunTriggerRequest {
            trigger: Some(MONTHLY_REPORT_TRIGGER.to_string()),
        })?;
        let completed = self
            .execute_monthly_report_trigger_for_month(&status.run.id, date.year(), date.month())
            .await?;
        Ok(ScheduledMonthlyReportOutcome::Triggered {
            run_id: completed.run.id,
            month_key,
        })
    }

    async fn execute_monthly_report_trigger_for_month(
        &self,
        run_id: &str,
        year: i32,
        month: u32,
    ) -> Result<AutoDreamRunStatus> {
        let mut run = self.read_run(run_id)?;
        if run.trigger != MONTHLY_REPORT_TRIGGER {
            return Err(AutoDreamError::InvalidState(format!(
                "run {} is not a monthly report trigger",
                run.id
            )));
        }
        begin_report_attempt(&mut run)?;
        self.write_run(&run)?;

        let generator = self.monthly_generator();
        let mut last_error = None;
        for attempt in 1..=REPORT_TRIGGER_MAX_ATTEMPTS {
            match generator
                .generate_for_month(year, month, Utc::now(), attempt)
                .await
            {
                Ok(result) => {
                    let now = Utc::now();
                    run.state = AutoDreamRunState::Completed;
                    run.completed_at = Some(now);
                    run.summary = Some(format!(
                        "generated rhythm report at {}",
                        result.path.display()
                    ));
                    run.failure_code = None;
                    run.error = None;
                    mark_orchestration(&mut run, AutoDreamOrchestrationPhase::Completed, now);
                    self.write_run(&run)?;
                    self.write_checkpoint_success(&run, now)?;
                    self.remove_active_lock(run_id)?;
                    self.append_event(AutoDreamEvent::new(
                        Some(run.id.clone()),
                        "rhythm_report_generated",
                        format!("generated {}", result.path.display()),
                    ))?;
                    return self.status_from_run(run, None);
                }
                Err(error) => {
                    last_error = Some(error);
                    if attempt < REPORT_TRIGGER_MAX_ATTEMPTS {
                        self.append_event(AutoDreamEvent::new(
                            Some(run.id.clone()),
                            "rhythm_report_retry",
                            format!(
                                "retrying {} after attempt {attempt} of {}",
                                run.trigger, REPORT_TRIGGER_MAX_ATTEMPTS
                            ),
                        ))?;
                    }
                }
            }
        }

        let error = last_error.unwrap_or_else(|| {
            AutoDreamError::InvalidState("rhythm report generation failed".to_string())
        });
        let now = Utc::now();
        run.state = AutoDreamRunState::Failed;
        run.completed_at = Some(now);
        run.summary = Some("rhythm report generation failed".to_string());
        run.failure_code = Some(AutoDreamFailureCode::ReportGenerationFailed);
        run.error = Some(error.to_string());
        mark_orchestration(&mut run, AutoDreamOrchestrationPhase::Failed, now);
        self.write_run(&run)?;
        self.remove_active_lock(run_id)?;
        Self::metrics().record_failure();
        self.append_event(
            AutoDreamEvent::new(
                Some(run.id.clone()),
                "rhythm_report_failed",
                error.to_string(),
            )
            .with_failure_code("report_generation_failed"),
        )?;
        Err(error)
    }

    fn rhythm_generator(&self) -> AutoDreamRhythmReportGenerator {
        AutoDreamRhythmReportGenerator::with_narrative(
            self.storage.clone(),
            self.narrative_generator.clone(),
            self.llm_curation.clone(),
        )
    }

    fn monthly_generator(&self) -> AutoDreamMonthlyReportGenerator {
        AutoDreamMonthlyReportGenerator::with_narrative(
            self.storage.clone(),
            self.narrative_generator.clone(),
            self.llm_curation.clone(),
        )
    }

    async fn generate_report_for_trigger(
        &self,
        trigger: &str,
        attempt: u32,
    ) -> Result<crate::RhythmReportWriteResult> {
        if trigger == MONTHLY_REPORT_TRIGGER {
            return self
                .monthly_generator()
                .generate_current_month(attempt)
                .await;
        }
        self.rhythm_generator().generate_for_trigger(trigger).await
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
        let foreign_process = self
            .read_lock()?
            .and_then(|lock| lock.pid)
            .is_some_and(|pid| pid != std::process::id());
        if age < self.stale_lock_after && !foreign_process {
            return Ok(());
        }

        if let Some(lock) = self.read_lock()? {
            if let Ok(mut run) = self.read_run(&lock.run_id) {
                if matches!(
                    run.state,
                    AutoDreamRunState::Pending | AutoDreamRunState::Running
                ) {
                    let now = Utc::now();
                    run.state = AutoDreamRunState::Pending;
                    run.completed_at = None;
                    run.summary = Some("interrupted run queued for recovery".to_string());
                    run.failure_code = None;
                    run.error = None;
                    if let Some(orchestration) = run.orchestration.as_mut() {
                        orchestration.deadline_at = now + chrono::Duration::minutes(5);
                        orchestration.updated_at = now;
                    }
                    self.write_run(&run)?;
                    Self::metrics().record_failure();
                    self.append_event(AutoDreamEvent::new(
                        Some(run.id.clone()),
                        "interrupted_run_requeued",
                        "interrupted AutoDream run queued for recovery",
                    ))?;
                }
            }
        }

        fs::remove_file(&lock_path).map_err(|source| AutoDreamError::io(lock_path, source))?;
        Ok(())
    }

    fn read_checkpoint(&self) -> Result<AutoDreamCheckpoint> {
        read_json_file(self.storage.paths().checkpoint_file())
    }

    fn write_checkpoint_success(&self, run: &AutoDreamRunRecord, now: DateTime<Utc>) -> Result<()> {
        let mut checkpoint: AutoDreamCheckpoint =
            read_json_file(self.storage.paths().checkpoint_file())?;
        checkpoint.last_completed_run_id = Some(run.id.clone());
        checkpoint.last_completed_at = Some(now);
        atomic_write_json(&self.storage.paths().checkpoint_file(), &checkpoint)
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

    fn remove_active_lock(&self, run_id: &str) -> Result<()> {
        let path = self.storage.paths().lock_file();
        let lock = match fs::read_to_string(&path) {
            Ok(content) => Some(serde_json::from_str::<AutoDreamLockRecord>(&content)?),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(source) => return Err(AutoDreamError::io(&path, source)),
        };
        if lock.as_ref().map(|lock| lock.run_id.as_str()) == Some(run_id) {
            fs::remove_file(&path).map_err(|source| AutoDreamError::io(path, source))?;
        }
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

fn mark_orchestration(
    run: &mut AutoDreamRunRecord,
    phase: AutoDreamOrchestrationPhase,
    now: DateTime<Utc>,
) {
    if let Some(orchestration) = run.orchestration.as_mut() {
        orchestration.phase = phase;
        orchestration.updated_at = now;
    }
}

fn begin_report_attempt(run: &mut AutoDreamRunRecord) -> Result<()> {
    if matches!(
        run.state,
        AutoDreamRunState::Completed | AutoDreamRunState::Failed | AutoDreamRunState::Cancelled
    ) {
        return Err(AutoDreamError::InvalidState(format!(
            "cannot execute terminal run {}",
            run.id
        )));
    }
    let now = Utc::now();
    let orchestration = run.orchestration.as_mut().ok_or_else(|| {
        AutoDreamError::InvalidState(format!(
            "run {} has no recoverable orchestration record",
            run.id
        ))
    })?;
    run.state = AutoDreamRunState::Running;
    orchestration.phase = AutoDreamOrchestrationPhase::Publishing;
    orchestration.attempt = orchestration.attempt.saturating_add(1);
    orchestration.deadline_at = now + chrono::Duration::seconds(60);
    orchestration.updated_at = now;
    Ok(())
}

fn should_attempt_scheduled_monthly_report(
    date: NaiveDate,
    marker: Option<&MonthlyReportErrorMarker>,
) -> bool {
    let within_first_week = date.day() <= 7;
    let first_monday = within_first_week && date.weekday() == Weekday::Mon;
    let retryable_marker = marker
        .is_some_and(|marker| within_first_week && marker.attempts < REPORT_TRIGGER_MAX_ATTEMPTS);
    first_monday || retryable_marker
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|source| AutoDreamError::io(path, source))?;
    Ok(serde_json::from_str(&content)?)
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc};

    use agent_diva_core::{
        evolution::{
            CreateSkillProposal, SkillEvidence, SkillHome, SkillProposalSource, SkillProposalStatus,
        },
        session::SessionManager,
    };
    use agent_diva_laputa::MemoryHome;

    use super::*;

    struct StaticSkillReflection {
        output: std::result::Result<crate::SkillReflectionOutput, crate::ReflectionError>,
    }

    #[async_trait::async_trait]
    impl crate::SkillReflectionEngine for StaticSkillReflection {
        async fn reflect_skills(
            &self,
            input: crate::SkillReflectionInput,
        ) -> std::result::Result<crate::SkillReflectionOutput, crate::ReflectionError> {
            assert!(input.organized_work.contains("### Goal"));
            assert!(!input.memrules.is_empty());
            self.output.clone()
        }
    }

    fn skill_markdown(slug: &str) -> String {
        format!("---\nname: {slug}\ndescription: generated skill\nalways: true\n---\nsteps\n")
    }

    #[test]
    fn run_event_reader_skips_invalid_json_but_propagates_line_read_errors() {
        let dir = tempfile::tempdir().unwrap();
        let service = AutoDreamService::open(dir.path()).unwrap();
        let run = service
            .trigger_manual_run(ManualRunTriggerRequest::default())
            .unwrap()
            .run;
        let path = service.storage.paths().events_jsonl();

        fs::write(&path, b"not-json\n").unwrap();
        let mut event = AutoDreamEvent::new(Some(run.id.clone()), "test", "bounded event");
        event.id = "event-valid".into();
        service.append_event(event).unwrap();
        let events = service.list_run_events(&run.id, 10).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].id, "event-valid");

        fs::write(&path, [0xff, b'\n']).unwrap();
        assert!(matches!(
            service.list_run_events(&run.id, 10),
            Err(AutoDreamError::Io { source, .. })
                if source.kind() == std::io::ErrorKind::InvalidData
        ));
    }

    #[tokio::test]
    async fn s3_worker_organizes_work_without_memory_patch_or_bml_write() {
        let dir = tempfile::tempdir().unwrap();
        let memory_home = MemoryHome::new(dir.path().join("machine-home"));
        memory_home
            .actmem()
            .append_pulse("gui:chat", "Ship the ACTMEM workspace")
            .await
            .unwrap();
        memory_home
            .actmem()
            .append_recap("gui:chat", "The kernel and routes are ready")
            .await
            .unwrap();
        let mut sessions = SessionManager::new(dir.path());
        let session = sessions.get_or_create("gui:chat");
        session.add_message("user", "Keep the implementation evidence bounded");
        let session = session.clone();
        sessions.save(&session).unwrap();

        let service = AutoDreamService::open(dir.path())
            .unwrap()
            .with_memory_home(memory_home.clone());
        let run = service
            .trigger_manual_run(ManualRunTriggerRequest::default())
            .unwrap()
            .run;
        let report = service.execute_reflection_worker(&run.id).await.unwrap();

        assert_eq!(report.outcome, crate::AutoDreamWorkerOutcome::Success);
        assert!(report.proposal_ids.is_empty());
        let document = memory_home.actmem().read().unwrap();
        assert!(document.work.contains("### Goal"));
        assert!(document.work.contains("### Open"));
        assert!(document.work.contains("### Next"));
        assert!(document.work.contains("### Constraints"));
        assert!(document.work.contains("### Pointers"));
        assert!(document.work.contains("MEMRULES:Default"));
        assert!(!memory_home.database_path().exists());
        assert!(service
            .storage
            .paths()
            .workspace_root()
            .join(".laputa/proposals")
            .read_dir()
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(true));
        let events = service.list_run_events(&run.id, 200).unwrap();
        assert!(events.iter().any(|event| {
            event.kind == "input_collected"
                && event.phase.as_deref() == Some("gather")
                && event
                    .input_summary
                    .as_deref()
                    .is_some_and(|summary| summary.contains("items="))
        }));
        assert!(events
            .iter()
            .any(|event| event.kind == "worker_succeeded"
                && event.phase.as_deref() == Some("completed")));
    }

    #[tokio::test]
    async fn skill_reflection_runs_after_work_and_only_creates_review_requests() {
        let dir = tempfile::tempdir().unwrap();
        let machine_home = dir.path().join("machine-home");
        let builtin = dir.path().join("builtin");
        let memory_home = MemoryHome::new(&machine_home);
        memory_home
            .actmem()
            .append_pulse("gui:chat", "Extract a reusable review routine")
            .await
            .unwrap();
        let mut sessions = SessionManager::new(dir.path());
        let session = sessions.get_or_create("gui:chat");
        session.add_message("user", "Verified bounded skill evidence");
        let session = session.clone();
        sessions.save(&session).unwrap();

        let skill_home = SkillHome::new(&machine_home, &builtin);
        skill_home
            .create_request(CreateSkillProposal {
                slug: "existing-review".into(),
                title: "Existing".into(),
                proposed_markdown: skill_markdown("existing-review"),
                evidence: vec![SkillEvidence {
                    session_key: Some("gui:chat".into()),
                    actmem_pointer: None,
                    autodream_run_id: None,
                    tool: None,
                    artifact: None,
                }],
                attestation: None,
                base_hash: "0".into(),
                source: SkillProposalSource::Distill,
                reason: "already pending".into(),
            })
            .unwrap();
        let engine = StaticSkillReflection {
            output: Ok(crate::SkillReflectionOutput {
                schema_version: 1,
                candidates: vec![
                    crate::SkillReflectionCandidate {
                        slug: "existing-review".into(),
                        title: "Duplicate".into(),
                        description: "duplicate".into(),
                        proposed_markdown: skill_markdown("existing-review"),
                        reason: "duplicate pending".into(),
                    },
                    crate::SkillReflectionCandidate {
                        slug: "new-review".into(),
                        title: "New".into(),
                        description: "new".into(),
                        proposed_markdown: skill_markdown("new-review"),
                        reason: "bounded evidence".into(),
                    },
                    crate::SkillReflectionCandidate {
                        slug: "INVALID SLUG".into(),
                        title: "Rejected".into(),
                        description: "invalid".into(),
                        proposed_markdown: skill_markdown("invalid-slug"),
                        reason: "gate should reject".into(),
                    },
                ],
                diagnostic_codes: Vec::new(),
            }),
        };
        let service = AutoDreamService::open(dir.path())
            .unwrap()
            .with_memory_home(memory_home.clone())
            .with_skill_home(skill_home.clone())
            .with_skill_reflection_engine(Some(Arc::new(engine)));
        let run = service
            .trigger_manual_run(ManualRunTriggerRequest::default())
            .unwrap()
            .run;
        let report = service.execute_reflection_worker(&run.id).await.unwrap();

        assert_eq!(report.outcome, crate::AutoDreamWorkerOutcome::Success);
        assert_eq!(report.proposal_ids.len(), 1);
        let requests = skill_home.list_requests().unwrap();
        assert_eq!(requests.len(), 2);
        let generated = requests
            .iter()
            .find(|request| request.slug == "new-review")
            .unwrap();
        assert_eq!(generated.status, SkillProposalStatus::Pending);
        assert_eq!(generated.source, SkillProposalSource::Autodream);
        assert!(generated.evidence[0].autodream_run_id.is_some());
        assert!(skill_home.read("new-review").is_err());
        assert!(!memory_home.database_path().exists());
        let events = service.list_run_events(&run.id, 200).unwrap();
        assert!(events.iter().any(|event| {
            event.kind == "skill_request_created"
                && event.proposal_id.as_deref() == Some(report.proposal_ids[0].as_str())
        }));
        assert!(events.iter().any(|event| {
            event.kind == "skill_candidate_rejected"
                && event.gate_code.as_deref() == Some("skill_slug_invalid")
        }));
    }

    #[tokio::test]
    async fn skill_reflection_failure_keeps_organized_work_and_creates_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let machine_home = dir.path().join("machine-home");
        let memory_home = MemoryHome::new(&machine_home);
        memory_home
            .actmem()
            .append_pulse("gui:chat", "Organize before provider failure")
            .await
            .unwrap();
        let mut sessions = SessionManager::new(dir.path());
        let session = sessions.get_or_create("gui:chat");
        session.add_message("user", "Bounded evidence");
        let session = session.clone();
        sessions.save(&session).unwrap();
        let skill_home = SkillHome::new(&machine_home, dir.path().join("builtin"));
        let service = AutoDreamService::open(dir.path())
            .unwrap()
            .with_memory_home(memory_home.clone())
            .with_skill_home(skill_home.clone())
            .with_skill_reflection_engine(Some(Arc::new(StaticSkillReflection {
                output: Err(crate::ReflectionError::InvalidSchema),
            })));
        let run = service
            .trigger_manual_run(ManualRunTriggerRequest::default())
            .unwrap()
            .run;
        let report = service.execute_reflection_worker(&run.id).await.unwrap();

        assert_eq!(report.outcome, crate::AutoDreamWorkerOutcome::Success);
        assert!(report.proposal_ids.is_empty());
        assert!(report
            .diagnostics
            .iter()
            .any(|item| item.contains("skill_reflection_degraded")));
        assert!(memory_home
            .actmem()
            .read()
            .unwrap()
            .work
            .contains("### Goal"));
        assert!(skill_home.list_requests().unwrap().is_empty());
    }
}
