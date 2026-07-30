use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    time::{Duration, Instant},
};

use agent_diva_core::evolution::{
    AutoDreamFailureCode, AutoDreamRunRecord, AutoDreamRunState, EvidenceRef, RiskLevel,
};
use agent_diva_laputa::LaputaService;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    atomic::atomic_write_json, AutoDreamArtifactSummary, AutoDreamCheckpoint,
    AutoDreamCollectedInputs, AutoDreamError, AutoDreamEvent, AutoDreamLockRecord,
    AutoDreamOutputEmitter, AutoDreamOutputRequest, AutoDreamProposalCandidateDraft,
    AutoDreamStorage, Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDreamReflectionStage {
    Orient,
    Gather,
    Consolidate,
    Propose,
}

impl AutoDreamReflectionStage {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Orient => "orient",
            Self::Gather => "gather",
            Self::Consolidate => "consolidate",
            Self::Propose => "propose",
        }
    }

    const fn all() -> [Self; 4] {
        [Self::Orient, Self::Gather, Self::Consolidate, Self::Propose]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDreamWorkerStageStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamReflectionStageRecord {
    pub stage: AutoDreamReflectionStage,
    pub status: AutoDreamWorkerStageStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDreamRestrictedAction {
    ReadSessions,
    ReadLaputaApi,
    WriteAutoDreamOutput,
    CreateLaputaProposalApi,
    ArbitraryShell,
    WriteExternalAuthority,
    DirectLaputaAuthorityWrite,
    WriteMonthlyReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamRestrictedProfile {
    allowed: Vec<AutoDreamRestrictedAction>,
}

impl Default for AutoDreamRestrictedProfile {
    fn default() -> Self {
        Self {
            allowed: vec![
                AutoDreamRestrictedAction::ReadSessions,
                AutoDreamRestrictedAction::ReadLaputaApi,
                AutoDreamRestrictedAction::WriteAutoDreamOutput,
                AutoDreamRestrictedAction::CreateLaputaProposalApi,
            ],
        }
    }
}

impl AutoDreamRestrictedProfile {
    pub fn is_allowed(&self, action: AutoDreamRestrictedAction) -> bool {
        self.allowed.contains(&action)
    }

    pub fn validate_request(&self, action: AutoDreamRestrictedAction) -> Result<()> {
        if self.is_allowed(action) {
            return Ok(());
        }
        Err(AutoDreamError::InvalidState(format!(
            "restricted AutoDream profile denies {action:?}"
        )))
    }

    pub fn allowed_actions(&self) -> &[AutoDreamRestrictedAction] {
        &self.allowed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDreamWorkerOutcome {
    Success,
    Failure,
    Cancelled,
    Timeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamWorkerReport {
    pub run_id: String,
    pub outcome: AutoDreamWorkerOutcome,
    pub stages: Vec<AutoDreamReflectionStageRecord>,
    pub diagnostics: Vec<String>,
    pub proposal_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AutoDreamWorkerConfig {
    pub timeout: Option<Duration>,
    pub profile: AutoDreamRestrictedProfile,
}

impl Default for AutoDreamWorkerConfig {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(60)),
            profile: AutoDreamRestrictedProfile::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutoDreamWorker {
    storage: AutoDreamStorage,
    laputa: LaputaService,
    config: AutoDreamWorkerConfig,
}

impl AutoDreamWorker {
    pub fn new(storage: AutoDreamStorage, laputa: LaputaService) -> Self {
        Self {
            storage,
            laputa,
            config: AutoDreamWorkerConfig::default(),
        }
    }

    pub fn with_config(mut self, config: AutoDreamWorkerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn restricted_profile(&self) -> &AutoDreamRestrictedProfile {
        &self.config.profile
    }

    pub fn execute(&self, run_id: &str) -> Result<AutoDreamWorkerReport> {
        let started = Instant::now();
        let mut stages = AutoDreamReflectionStage::all()
            .into_iter()
            .map(|stage| AutoDreamReflectionStageRecord {
                stage,
                status: AutoDreamWorkerStageStatus::Pending,
                started_at: None,
                completed_at: None,
                diagnostic: None,
            })
            .collect::<Vec<_>>();
        let mut diagnostics = Vec::new();
        let mut collected = None;

        for index in 0..stages.len() {
            if let Some(report) =
                self.check_interruption(run_id, started, &mut stages, index, &mut diagnostics)?
            {
                return Ok(report);
            }

            stages[index].status = AutoDreamWorkerStageStatus::Running;
            stages[index].started_at = Some(Utc::now());

            let stage_result = match stages[index].stage {
                AutoDreamReflectionStage::Orient => self.orient(),
                AutoDreamReflectionStage::Gather => self.gather(run_id).map(|inputs| {
                    collected = Some(inputs);
                }),
                AutoDreamReflectionStage::Consolidate => self.consolidate(collected.as_ref()),
                AutoDreamReflectionStage::Propose => self.propose(run_id, collected.as_ref()),
            };

            match stage_result {
                Ok(()) => {
                    stages[index].status = AutoDreamWorkerStageStatus::Succeeded;
                    stages[index].completed_at = Some(Utc::now());
                }
                Err(error) => {
                    let message = error.to_string();
                    stages[index].status = AutoDreamWorkerStageStatus::Failed;
                    stages[index].completed_at = Some(Utc::now());
                    stages[index].diagnostic = Some(message.clone());
                    diagnostics.push(message);
                    return self.finish_failure(
                        run_id,
                        AutoDreamWorkerOutcome::Failure,
                        stages,
                        diagnostics,
                    );
                }
            }
        }

        self.finish_success(run_id, stages, diagnostics)
    }

    fn orient(&self) -> Result<()> {
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::ReadSessions)?;
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::ReadLaputaApi)?;
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::WriteAutoDreamOutput)?;
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::CreateLaputaProposalApi)?;
        Ok(())
    }

    fn gather(&self, run_id: &str) -> Result<AutoDreamCollectedInputs> {
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::ReadSessions)?;
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::ReadLaputaApi)?;
        let collector =
            crate::AutoDreamInputCollector::new(self.storage.clone(), self.laputa.clone());
        let collected = collector.collect(run_id)?;
        let mut run = self.read_run(run_id)?;
        run.input_summary = Some(collected.summary.clone());
        run.summary = Some(format!(
            "collected {} inputs with {} omissions",
            collected.summary.total_items,
            collected.summary.omissions.len()
        ));
        self.write_run(&run)?;
        Ok(collected)
    }

    fn consolidate(&self, collected: Option<&AutoDreamCollectedInputs>) -> Result<()> {
        let collected = collected.ok_or_else(|| {
            AutoDreamError::InvalidState("cannot consolidate before gather stage".to_string())
        })?;
        if collected.items.is_empty() {
            return Err(AutoDreamError::InvalidState(
                "cannot consolidate empty reflection inputs".to_string(),
            ));
        }
        Ok(())
    }

    fn propose(&self, run_id: &str, collected: Option<&AutoDreamCollectedInputs>) -> Result<()> {
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::WriteAutoDreamOutput)?;
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::CreateLaputaProposalApi)?;
        let collected = collected.ok_or_else(|| {
            AutoDreamError::InvalidState("cannot propose before gather stage".to_string())
        })?;
        let run = self.read_run(run_id)?;
        let evidence_refs = collected
            .items
            .iter()
            .map(|item| item.evidence.clone())
            .collect::<Vec<EvidenceRef>>();
        let summary = AutoDreamArtifactSummary {
            headline: "AutoDream restricted reflection generated one review proposal".to_string(),
            details: vec![
                format!("inputs: {}", collected.summary.total_items),
                format!("omissions: {}", collected.summary.omissions.len()),
                "proposal persisted through Laputa proposal API".to_string(),
            ],
        };
        let draft = AutoDreamProposalCandidateDraft {
            proposal_type: "journal_note".to_string(),
            proposed_patch: render_proposed_patch(collected),
            risk_level: RiskLevel::Low,
            evidence_refs: evidence_refs.clone(),
        };
        AutoDreamOutputEmitter::new(self.storage.clone(), self.laputa.clone()).emit_outputs(
            AutoDreamOutputRequest {
                run,
                generated_at: Utc::now(),
                confidence: 60,
                evidence_refs,
                output_summary: summary,
                proposal_candidates: vec![draft],
            },
        )?;
        Ok(())
    }

    fn check_interruption(
        &self,
        run_id: &str,
        started: Instant,
        stages: &mut [AutoDreamReflectionStageRecord],
        index: usize,
        diagnostics: &mut Vec<String>,
    ) -> Result<Option<AutoDreamWorkerReport>> {
        if self
            .config
            .timeout
            .map(|timeout| started.elapsed() >= timeout)
            .unwrap_or(false)
        {
            let message = "AutoDream worker timed out before stage completion".to_string();
            stages[index].status = AutoDreamWorkerStageStatus::TimedOut;
            stages[index].started_at.get_or_insert_with(Utc::now);
            stages[index].completed_at = Some(Utc::now());
            stages[index].diagnostic = Some(message.clone());
            diagnostics.push(message);
            return self
                .finish_failure(
                    run_id,
                    AutoDreamWorkerOutcome::Timeout,
                    stages.to_vec(),
                    diagnostics.clone(),
                )
                .map(Some);
        }

        let run = self.read_run(run_id)?;
        if run.state == AutoDreamRunState::Cancelled {
            let message = "AutoDream worker observed cancellation".to_string();
            stages[index].status = AutoDreamWorkerStageStatus::Cancelled;
            stages[index].started_at.get_or_insert_with(Utc::now);
            stages[index].completed_at = Some(Utc::now());
            stages[index].diagnostic = Some(message.clone());
            diagnostics.push(message);
            return self
                .finish_failure(
                    run_id,
                    AutoDreamWorkerOutcome::Cancelled,
                    stages.to_vec(),
                    diagnostics.clone(),
                )
                .map(Some);
        }

        Ok(None)
    }

    fn finish_success(
        &self,
        run_id: &str,
        stages: Vec<AutoDreamReflectionStageRecord>,
        diagnostics: Vec<String>,
    ) -> Result<AutoDreamWorkerReport> {
        let now = Utc::now();
        let mut run = self.read_run(run_id)?;
        run.state = AutoDreamRunState::Completed;
        run.completed_at = Some(now);
        run.summary = Some("AutoDream restricted reflection completed".to_string());
        run.error = None;
        run.failure_code = None;
        let proposal_ids = run.proposal_ids.clone();
        self.write_run(&run)?;
        self.write_checkpoint_success(&run, now)?;
        self.remove_active_lock(run_id)?;
        self.append_event(run_id, "worker_succeeded", "AutoDream worker completed")?;
        Ok(AutoDreamWorkerReport {
            run_id: run_id.to_string(),
            outcome: AutoDreamWorkerOutcome::Success,
            stages,
            diagnostics,
            proposal_ids,
        })
    }

    fn finish_failure(
        &self,
        run_id: &str,
        outcome: AutoDreamWorkerOutcome,
        stages: Vec<AutoDreamReflectionStageRecord>,
        diagnostics: Vec<String>,
    ) -> Result<AutoDreamWorkerReport> {
        let now = Utc::now();
        let mut run = self.read_run(run_id)?;
        if outcome == AutoDreamWorkerOutcome::Cancelled {
            run.state = AutoDreamRunState::Cancelled;
        } else {
            run.state = AutoDreamRunState::Failed;
        }
        run.completed_at = Some(now);
        run.summary = Some(render_failure_summary(&outcome));
        run.error = Some(diagnostics.join("; "));
        run.failure_code = Some(worker_failure_code(&outcome, &diagnostics));
        let proposal_ids = run.proposal_ids.clone();
        self.write_run(&run)?;
        self.remove_active_lock(run_id)?;
        self.append_event(
            run_id,
            worker_event_kind(&outcome),
            &render_failure_summary(&outcome),
        )?;
        Ok(AutoDreamWorkerReport {
            run_id: run_id.to_string(),
            outcome,
            stages,
            diagnostics,
            proposal_ids,
        })
    }

    fn read_run(&self, run_id: &str) -> Result<AutoDreamRunRecord> {
        read_json_file(self.storage.paths().run_record_file(run_id)).map_err(|error| match error {
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

    fn write_checkpoint_success(&self, run: &AutoDreamRunRecord, now: DateTime<Utc>) -> Result<()> {
        let mut checkpoint: AutoDreamCheckpoint =
            read_json_file(self.storage.paths().checkpoint_file())?;
        checkpoint.last_completed_run_id = Some(run.id.clone());
        checkpoint.last_completed_at = Some(now);
        atomic_write_json(&self.storage.paths().checkpoint_file(), &checkpoint)
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

    fn append_event(&self, run_id: &str, kind: &str, message: &str) -> Result<()> {
        let path = self.storage.paths().events_jsonl();
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|source| AutoDreamError::io(&path, source))?;
        let event = AutoDreamEvent {
            id: format!("evt-{}", Uuid::new_v4()),
            run_id: Some(run_id.to_string()),
            kind: kind.to_string(),
            message: message.to_string(),
            created_at: Utc::now(),
        };
        let line = serde_json::to_string(&event)?;
        writeln!(file, "{line}").map_err(|source| AutoDreamError::io(&path, source))?;
        file.sync_all()
            .map_err(|source| AutoDreamError::io(&path, source))?;
        Ok(())
    }
}

fn render_proposed_patch(collected: &AutoDreamCollectedInputs) -> String {
    let evidence = collected
        .items
        .iter()
        .take(3)
        .map(|item| format!("- {}: {}", item.uri, item.excerpt))
        .collect::<Vec<_>>()
        .join("\n");
    format!("Restricted AutoDream reflection candidate:\n{evidence}")
}

fn worker_failure_code(
    outcome: &AutoDreamWorkerOutcome,
    diagnostics: &[String],
) -> AutoDreamFailureCode {
    match outcome {
        AutoDreamWorkerOutcome::Cancelled => AutoDreamFailureCode::Cancelled,
        AutoDreamWorkerOutcome::Timeout => AutoDreamFailureCode::WorkerTimeout,
        AutoDreamWorkerOutcome::Failure
            if diagnostics
                .iter()
                .any(|item| item.contains("all mandatory inputs omitted")) =>
        {
            AutoDreamFailureCode::InputUnavailable
        }
        AutoDreamWorkerOutcome::Failure | AutoDreamWorkerOutcome::Success => {
            AutoDreamFailureCode::WorkerFailed
        }
    }
}

fn render_failure_summary(outcome: &AutoDreamWorkerOutcome) -> String {
    match outcome {
        AutoDreamWorkerOutcome::Success => "AutoDream restricted reflection completed".to_string(),
        AutoDreamWorkerOutcome::Failure => {
            "AutoDream restricted reflection failed with diagnostics".to_string()
        }
        AutoDreamWorkerOutcome::Cancelled => {
            "AutoDream restricted reflection cancelled".to_string()
        }
        AutoDreamWorkerOutcome::Timeout => "AutoDream restricted reflection timed out".to_string(),
    }
}

fn worker_event_kind(outcome: &AutoDreamWorkerOutcome) -> &'static str {
    match outcome {
        AutoDreamWorkerOutcome::Success => "worker_succeeded",
        AutoDreamWorkerOutcome::Failure => "worker_failed",
        AutoDreamWorkerOutcome::Cancelled => "worker_cancelled",
        AutoDreamWorkerOutcome::Timeout => "worker_timed_out",
    }
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: impl AsRef<Path>) -> Result<T> {
    let path = path.as_ref();
    let content = fs::read_to_string(path).map_err(|source| AutoDreamError::io(path, source))?;
    Ok(serde_json::from_str(&content)?)
}
