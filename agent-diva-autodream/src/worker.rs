use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::Arc,
    time::Duration,
};

use agent_diva_core::evolution::{
    AutoDreamFailureCode, AutoDreamOrchestrationPhase, AutoDreamRunRecord, AutoDreamRunState,
    CreateSkillProposal, SkillEvidence, SkillHome, SkillHomeError, SkillProposalSource,
};
use agent_diva_laputa::{actmem::ActmemPatch, MemoryHome};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    atomic::atomic_write_json, AutoDreamCheckpoint, AutoDreamCollectedInputs, AutoDreamError,
    AutoDreamEvent, AutoDreamLockRecord, AutoDreamStorage, ReflectionEvidence, Result,
    SkillReflectionEngine, SkillReflectionIndex, SkillReflectionInput,
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

#[derive(Clone)]
pub struct AutoDreamWorker {
    storage: AutoDreamStorage,
    config: AutoDreamWorkerConfig,
    memory_home: Option<MemoryHome>,
    skill_home: Option<SkillHome>,
    skill_reflection_engine: Option<Arc<dyn SkillReflectionEngine>>,
}

impl AutoDreamWorker {
    pub fn new(storage: AutoDreamStorage) -> Self {
        Self {
            storage,
            config: AutoDreamWorkerConfig::default(),
            memory_home: None,
            skill_home: None,
            skill_reflection_engine: None,
        }
    }

    pub fn with_config(mut self, config: AutoDreamWorkerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_memory_home(mut self, memory_home: Option<MemoryHome>) -> Self {
        self.memory_home = memory_home;
        self
    }

    pub fn with_skill_home(mut self, skill_home: Option<SkillHome>) -> Self {
        self.skill_home = skill_home;
        self
    }

    pub fn with_skill_reflection_engine(
        mut self,
        engine: Option<Arc<dyn SkillReflectionEngine>>,
    ) -> Self {
        self.skill_reflection_engine = engine;
        self
    }

    pub fn restricted_profile(&self) -> &AutoDreamRestrictedProfile {
        &self.config.profile
    }

    pub async fn execute(&self, run_id: &str) -> Result<AutoDreamWorkerReport> {
        if self.memory_home.is_none() {
            return Err(AutoDreamError::InvalidState(
                "AutoDream requires the machine-wide MemoryHome authority".to_string(),
            ));
        }
        self.execute_actmem_work(run_id).await
    }

    /// S3 production path: organize the shared ACTMEM Work register directly.
    /// It never emits governed memory proposals or writes BML directly.
    async fn execute_actmem_work(&self, run_id: &str) -> Result<AutoDreamWorkerReport> {
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
        self.start_attempt(run_id)?;
        let mut diagnostics = Vec::new();

        stages[0].status = AutoDreamWorkerStageStatus::Running;
        stages[0].started_at = Some(Utc::now());
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::ReadSessions)?;
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::WriteAutoDreamOutput)?;
        stages[0].status = AutoDreamWorkerStageStatus::Succeeded;
        stages[0].completed_at = Some(Utc::now());

        self.transition_phase(run_id, AutoDreamOrchestrationPhase::Gathering)?;
        stages[1].status = AutoDreamWorkerStageStatus::Running;
        stages[1].started_at = Some(Utc::now());
        let collected = crate::AutoDreamInputCollector::new(self.storage.clone()).collect(run_id);
        let collected = match collected {
            Ok(collected) => collected,
            Err(error) => {
                let message = error.to_string();
                stages[1].status = AutoDreamWorkerStageStatus::Failed;
                stages[1].completed_at = Some(Utc::now());
                stages[1].diagnostic = Some(message.clone());
                diagnostics.push(message);
                return self.finish_failure(
                    run_id,
                    AutoDreamWorkerOutcome::Failure,
                    stages,
                    diagnostics,
                );
            }
        };
        let mut run = self.read_run(run_id)?;
        run.input_summary = Some(collected.summary.clone());
        run.summary = Some(format!(
            "collected {} inputs for ACTMEM Work organization",
            collected.summary.total_items
        ));
        self.write_run(&run)?;
        stages[1].status = AutoDreamWorkerStageStatus::Succeeded;
        stages[1].completed_at = Some(Utc::now());

        self.transition_phase(run_id, AutoDreamOrchestrationPhase::Reflecting)?;
        stages[2].status = AutoDreamWorkerStageStatus::Running;
        stages[2].started_at = Some(Utc::now());
        if let Err(error) = self.organize_actmem_work(run_id, &collected).await {
            let message = error.to_string();
            stages[2].status = AutoDreamWorkerStageStatus::Failed;
            stages[2].completed_at = Some(Utc::now());
            stages[2].diagnostic = Some(message.clone());
            diagnostics.push(message);
            return self.finish_failure(
                run_id,
                AutoDreamWorkerOutcome::Failure,
                stages,
                diagnostics,
            );
        }
        stages[2].status = AutoDreamWorkerStageStatus::Succeeded;
        stages[2].completed_at = Some(Utc::now());

        self.transition_phase(run_id, AutoDreamOrchestrationPhase::Publishing)?;
        stages[3].status = AutoDreamWorkerStageStatus::Running;
        stages[3].started_at = Some(Utc::now());
        let proposal_ids = match self.reflect_skill_requests(run_id, &collected).await {
            Ok(ids) => ids,
            Err(error) => {
                let diagnostic = format!("skill_reflection_degraded: {error}");
                diagnostics.push(diagnostic.clone());
                stages[3].diagnostic = Some(diagnostic.clone());
                self.append_event(run_id, "skill_reflection_degraded", &diagnostic)?;
                Vec::new()
            }
        };
        let mut run = self.read_run(run_id)?;
        run.proposal_ids = proposal_ids.clone();
        self.write_run(&run)?;
        stages[3].status = AutoDreamWorkerStageStatus::Succeeded;
        stages[3].completed_at = Some(Utc::now());
        self.append_event(
            run_id,
            "actmem_work_organized",
            &format!(
                "ACTMEM Work organized; {} Skill review request(s) emitted and zero memory proposals",
                proposal_ids.len()
            ),
        )?;
        self.finish_success(run_id, stages, diagnostics)
    }

    async fn reflect_skill_requests(
        &self,
        run_id: &str,
        collected: &AutoDreamCollectedInputs,
    ) -> Result<Vec<String>> {
        let engine = self.skill_reflection_engine.as_ref().ok_or_else(|| {
            AutoDreamError::InvalidState("skill reflection provider unavailable".into())
        })?;
        let memory_home = self
            .memory_home
            .as_ref()
            .ok_or_else(|| AutoDreamError::InvalidState("MemoryHome is unavailable".into()))?;
        let skill_home = self
            .skill_home
            .as_ref()
            .ok_or_else(|| AutoDreamError::InvalidState("SkillHome is unavailable".into()))?;
        let actmem = memory_home
            .actmem()
            .read()
            .map_err(|error| AutoDreamError::InvalidState(error.to_string()))?;
        let memrules = memory_home
            .read_memrules()
            .map_err(|error| AutoDreamError::InvalidState(error.to_string()))?;
        let skills = skill_home
            .list()
            .map_err(|error| AutoDreamError::InvalidState(error.to_string()))?
            .into_iter()
            .map(|skill| SkillReflectionIndex {
                slug: skill.slug,
                content_hash: skill.content_hash,
            })
            .collect::<Vec<_>>();
        let evidence = collected
            .items
            .iter()
            .take(16)
            .map(|item| {
                let redacted = agent_diva_core::security::redact_pii(
                    &item.excerpt,
                    &agent_diva_core::security::PiiConfig::default(),
                )
                .redacted;
                let mut evidence = item.evidence.clone();
                evidence.excerpt = None;
                ReflectionEvidence {
                    evidence,
                    summary: truncate_chars(&redacted, 500),
                }
            })
            .collect();
        let output = engine
            .reflect_skills(SkillReflectionInput {
                schema_version: 1,
                run_id: run_id.to_string(),
                organized_work: truncate_chars(&actmem.work, 4_000),
                pulse: truncate_chars(&actmem.pulse, 1_000),
                recap: truncate_chars(&actmem.recap, 1_000),
                evidence,
                memrules: memrules.content,
                skills,
                max_candidates: 8,
            })
            .await
            .map_err(|error| {
                AutoDreamError::InvalidState(format!("skill reflection failed: {error}"))
            })?;
        if output.schema_version != 1 || output.candidates.len() > 8 {
            return Err(AutoDreamError::InvalidState(
                "skill reflection returned invalid schema".into(),
            ));
        }
        for code in output.diagnostic_codes {
            self.append_event(run_id, "skill_reflection_diagnostic", &code)?;
        }

        let mut ids = Vec::new();
        for candidate in output.candidates {
            let base_hash = skill_home
                .read(&candidate.slug)
                .map(|document| document.summary.content_hash)
                .unwrap_or_else(|_| "0".into());
            let result = skill_home.create_request(CreateSkillProposal {
                slug: candidate.slug.clone(),
                title: candidate.title,
                proposed_markdown: candidate.proposed_markdown,
                evidence: vec![SkillEvidence {
                    session_key: None,
                    actmem_pointer: Some("ACTMEM.MD#Work".into()),
                    autodream_run_id: Some(run_id.to_string()),
                    tool: None,
                    artifact: collected
                        .items
                        .first()
                        .map(|item| item.evidence.uri.clone()),
                }],
                attestation: None,
                base_hash,
                source: SkillProposalSource::Autodream,
                reason: candidate.reason,
            });
            match result {
                Ok(request) => ids.push(request.id),
                Err(SkillHomeError::RequestExists(_)) => self.append_event(
                    run_id,
                    "skill_request_skipped",
                    &format!("pending request already exists for {}", candidate.slug),
                )?,
                Err(error) => self.append_event(
                    run_id,
                    "skill_candidate_rejected",
                    &format!("{}: {}", error.code(), candidate.slug),
                )?,
            }
        }
        Ok(ids)
    }

    async fn organize_actmem_work(
        &self,
        run_id: &str,
        collected: &AutoDreamCollectedInputs,
    ) -> Result<()> {
        let home = self
            .memory_home
            .as_ref()
            .ok_or_else(|| AutoDreamError::InvalidState("MemoryHome is unavailable".to_string()))?;
        let rules = home
            .read_memrules()
            .map_err(|error| AutoDreamError::InvalidState(error.to_string()))?;
        let mut snapshot = home
            .actmem()
            .read()
            .map_err(|error| AutoDreamError::InvalidState(error.to_string()))?;

        for attempt in 0..=1 {
            let work = render_organized_work(&snapshot, collected, &rules);
            match home
                .actmem()
                .put(ActmemPatch {
                    work: Some(work),
                    base_revision: snapshot.revision,
                    ..Default::default()
                })
                .await
            {
                Ok(_) => return Ok(()),
                Err(agent_diva_laputa::ActmemError::RevisionConflict { .. }) if attempt == 0 => {
                    let current = home
                        .actmem()
                        .read()
                        .map_err(|error| AutoDreamError::InvalidState(error.to_string()))?;
                    ensure_pulse_recap_only_conflict(&snapshot, &current)?;
                    snapshot = current;
                    self.append_event(
                        run_id,
                        "actmem_work_retry",
                        "retrying once after Pulse/Recap-only ACTMEM conflict",
                    )?;
                }
                Err(error) => {
                    return Err(AutoDreamError::InvalidState(format!(
                        "ACTMEM Work commit failed: {error}"
                    )))
                }
            }
        }
        Err(AutoDreamError::InvalidState(
            "ACTMEM Work commit retry exhausted".to_string(),
        ))
    }

    fn start_attempt(&self, run_id: &str) -> Result<()> {
        let mut run = self.read_run(run_id)?;
        if matches!(
            run.state,
            AutoDreamRunState::Completed | AutoDreamRunState::Failed | AutoDreamRunState::Cancelled
        ) {
            return Err(AutoDreamError::InvalidState(format!(
                "cannot execute terminal run {run_id}"
            )));
        }
        run.state = AutoDreamRunState::Running;
        if let Some(orchestration) = run.orchestration.as_mut() {
            orchestration.attempt = orchestration.attempt.saturating_add(1);
            orchestration.deadline_at = Utc::now() + chrono::Duration::seconds(60);
            orchestration.updated_at = Utc::now();
        }
        self.write_run(&run)
    }

    fn transition_phase(&self, run_id: &str, phase: AutoDreamOrchestrationPhase) -> Result<()> {
        let mut run = self.read_run(run_id)?;
        let orchestration = run.orchestration.as_mut().ok_or_else(|| {
            AutoDreamError::InvalidState("run lacks orchestration record".to_string())
        })?;
        orchestration.phase = phase;
        orchestration.updated_at = Utc::now();
        self.write_run(&run)
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
        run.summary = Some(if run.proposal_ids.is_empty() {
            "AutoDream reflection completed with no eligible candidates".to_string()
        } else {
            "AutoDream restricted reflection completed".to_string()
        });
        run.error = None;
        run.failure_code = None;
        if let Some(orchestration) = run.orchestration.as_mut() {
            orchestration.phase = AutoDreamOrchestrationPhase::Completed;
            orchestration.updated_at = now;
        }
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
        if let Some(orchestration) = run.orchestration.as_mut() {
            orchestration.phase = if outcome == AutoDreamWorkerOutcome::Cancelled {
                AutoDreamOrchestrationPhase::Cancelled
            } else {
                AutoDreamOrchestrationPhase::Failed
            };
            orchestration.updated_at = now;
        }
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
        AutoDreamWorkerOutcome::Failure
            if diagnostics
                .iter()
                .any(|item| item.contains("reflection provider unavailable")) =>
        {
            AutoDreamFailureCode::ProviderUnavailable
        }
        AutoDreamWorkerOutcome::Failure
            if diagnostics
                .iter()
                .any(|item| item.contains("provider timed out")) =>
        {
            AutoDreamFailureCode::ProviderTimeout
        }
        AutoDreamWorkerOutcome::Failure
            if diagnostics
                .iter()
                .any(|item| item.contains("reflection provider failed")) =>
        {
            AutoDreamFailureCode::ProviderFailed
        }
        AutoDreamWorkerOutcome::Failure
            if diagnostics
                .iter()
                .any(|item| item.contains("invalid schema") || item.contains("candidate gate")) =>
        {
            AutoDreamFailureCode::InvalidCandidate
        }
        AutoDreamWorkerOutcome::Failure | AutoDreamWorkerOutcome::Success => {
            AutoDreamFailureCode::WorkerFailed
        }
    }
}

fn render_organized_work(
    actmem: &agent_diva_laputa::ActmemDocument,
    collected: &AutoDreamCollectedInputs,
    rules: &agent_diva_laputa::MemRulesDocument,
) -> String {
    let mut sections = std::collections::BTreeMap::<String, String>::new();
    let mut current = None::<String>;
    let mut body = Vec::new();
    for line in actmem.work.lines() {
        if let Some(name) = line.strip_prefix("### ") {
            if let Some(previous) = current.replace(name.trim().to_string()) {
                sections.insert(previous, body.join("\n").trim().to_string());
                body.clear();
            }
        } else if current.is_some() {
            body.push(line);
        }
    }
    if let Some(previous) = current {
        sections.insert(previous, body.join("\n").trim().to_string());
    }

    if sections.get("Goal").map_or(true, |value| value.is_empty()) {
        if let Some(pulse) = latest_visible_ring_line(&actmem.pulse) {
            sections.insert(
                "Goal".to_string(),
                format!("- {}", truncate_chars(pulse, 200)),
            );
        }
    }
    if sections.get("Next").map_or(true, |value| value.is_empty()) {
        if let Some(recap) = latest_visible_ring_line(&actmem.recap) {
            sections.insert(
                "Next".to_string(),
                format!("- {}", truncate_chars(recap, 200)),
            );
        }
    }

    let pointers = sections.entry("Pointers".to_string()).or_default();
    let rules_pointer = format!(
        "- MEMRULES:{:?} ({} chars)",
        rules.source,
        rules.content.chars().count()
    );
    append_unique_line(pointers, &rules_pointer);
    for item in collected.items.iter().take(3) {
        append_unique_line(pointers, &format!("- evidence:{}", item.evidence.id));
    }

    agent_diva_laputa::actmem::WORK_SECTIONS
        .iter()
        .map(|name| {
            let content = sections.get(*name).map(String::as_str).unwrap_or_default();
            if content.is_empty() {
                format!("### {name}")
            } else {
                format!("### {name}\n{content}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn ensure_pulse_recap_only_conflict(
    previous: &agent_diva_laputa::ActmemDocument,
    current: &agent_diva_laputa::ActmemDocument,
) -> Result<()> {
    if current.work == previous.work {
        Ok(())
    } else {
        Err(AutoDreamError::InvalidState(
            "ACTMEM Work changed during AutoDream organization".to_string(),
        ))
    }
}

fn latest_visible_ring_line(value: &str) -> Option<&str> {
    value
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("<!-- session:"))
}

fn truncate_chars(value: &str, cap: usize) -> String {
    value.chars().take(cap).collect()
}

fn append_unique_line(section: &mut String, line: &str) {
    if section.lines().any(|existing| existing.trim() == line) {
        return;
    }
    if !section.is_empty() {
        section.push('\n');
    }
    section.push_str(line);
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
