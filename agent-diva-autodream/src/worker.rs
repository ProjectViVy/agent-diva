use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
    sync::Arc,
    time::{Duration, Instant},
};

use agent_diva_core::evolution::{
    AutoDreamFailureCode, AutoDreamOrchestrationPhase, AutoDreamRunRecord, AutoDreamRunState,
    CreateSkillProposal, EvolutionProposal, MemoryCandidate, RiskLevel, SkillEvidence, SkillHome,
    SkillHomeError, SkillProposalSource,
};
use agent_diva_core::experience::ExperienceJournal;
use agent_diva_laputa::{actmem::ActmemPatch, LaputaService, MemoryHome};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    atomic::atomic_write_json, AutoDreamArtifactSummary, AutoDreamCheckpoint,
    AutoDreamCollectedInputs, AutoDreamError, AutoDreamEvent, AutoDreamLockRecord,
    AutoDreamOutputEmitter, AutoDreamOutputRequest, AutoDreamProposalCandidateDraft,
    AutoDreamStorage, BoundedReflectionInput, CandidateGate, ReflectionEngine, ReflectionEvidence,
    Result, SkillReflectionEngine, SkillReflectionIndex, SkillReflectionInput,
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

#[async_trait::async_trait]
pub trait AutoDreamProposalGovernance: Send + Sync {
    async fn register_proposals(
        &self,
        proposals: &[EvolutionProposal],
    ) -> std::result::Result<(), String>;
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
    laputa: LaputaService,
    config: AutoDreamWorkerConfig,
    reflection_engine: Option<Arc<dyn ReflectionEngine>>,
    proposal_governance: Option<Arc<dyn AutoDreamProposalGovernance>>,
    memory_home: Option<MemoryHome>,
    skill_home: Option<SkillHome>,
    skill_reflection_engine: Option<Arc<dyn SkillReflectionEngine>>,
}

impl AutoDreamWorker {
    pub fn new(storage: AutoDreamStorage, laputa: LaputaService) -> Self {
        Self {
            storage,
            laputa,
            config: AutoDreamWorkerConfig::default(),
            reflection_engine: None,
            proposal_governance: None,
            memory_home: None,
            skill_home: None,
            skill_reflection_engine: None,
        }
    }

    pub fn with_config(mut self, config: AutoDreamWorkerConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_reflection_engine(mut self, engine: Option<Arc<dyn ReflectionEngine>>) -> Self {
        self.reflection_engine = engine;
        self
    }

    pub fn with_proposal_governance(
        mut self,
        governance: Option<Arc<dyn AutoDreamProposalGovernance>>,
    ) -> Self {
        self.proposal_governance = governance;
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
        if self.memory_home.is_some() {
            return self.execute_actmem_work(run_id).await;
        }
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
        let existing = self.read_run(run_id)?;
        if existing.state == AutoDreamRunState::Cancelled {
            let message = "AutoDream worker observed cancellation".to_string();
            stages[0].status = AutoDreamWorkerStageStatus::Cancelled;
            stages[0].started_at = Some(Utc::now());
            stages[0].completed_at = Some(Utc::now());
            stages[0].diagnostic = Some(message.clone());
            return Ok(AutoDreamWorkerReport {
                run_id: run_id.to_string(),
                outcome: AutoDreamWorkerOutcome::Cancelled,
                stages,
                diagnostics: vec![message],
                proposal_ids: existing.proposal_ids,
            });
        }
        self.start_attempt(run_id)?;
        let mut diagnostics = Vec::new();
        let mut collected = None;
        let mut candidates = None;

        for index in 0..stages.len() {
            if let Some(report) =
                self.check_interruption(run_id, started, &mut stages, index, &mut diagnostics)?
            {
                return Ok(report);
            }

            stages[index].status = AutoDreamWorkerStageStatus::Running;
            stages[index].started_at = Some(Utc::now());

            let stage_result = match stages[index].stage {
                AutoDreamReflectionStage::Orient => {
                    self.transition_phase(run_id, AutoDreamOrchestrationPhase::Gathering)?;
                    self.orient()
                }
                AutoDreamReflectionStage::Gather => self.gather(run_id).map(|inputs| {
                    collected = Some(inputs);
                }),
                AutoDreamReflectionStage::Consolidate => {
                    self.transition_phase(run_id, AutoDreamOrchestrationPhase::Reflecting)?;
                    let result = self.reflect(run_id, collected.as_ref()).await.map(|value| {
                        candidates = Some(value);
                    });
                    if result.is_ok() {
                        self.transition_phase(run_id, AutoDreamOrchestrationPhase::Validating)?;
                    }
                    result
                }
                AutoDreamReflectionStage::Propose => {
                    self.transition_phase(run_id, AutoDreamOrchestrationPhase::Publishing)?;
                    self.propose(run_id, collected.as_ref(), candidates.as_ref())
                        .await
                }
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

    /// S3 production path: organize the shared ACTMEM Work register directly.
    /// It never creates a MemoryPatch, writes BML, or registers governance.
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
        let mut input_config = crate::AutoDreamInputCollectorConfig::default();
        // Legacy Laputa Memory files are not an S3 production input.
        input_config.laputa_sections.clear();
        let collected =
            crate::AutoDreamInputCollector::new(self.storage.clone(), self.laputa.clone())
                .with_config(input_config)
                .collect(run_id);
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

    async fn reflect(
        &self,
        run_id: &str,
        collected: Option<&AutoDreamCollectedInputs>,
    ) -> Result<Vec<MemoryCandidate>> {
        let collected = collected.ok_or_else(|| {
            AutoDreamError::InvalidState("cannot consolidate before gather stage".to_string())
        })?;
        if collected.items.is_empty() {
            return Err(AutoDreamError::InvalidState(
                "cannot consolidate empty reflection inputs".to_string(),
            ));
        }
        let engine = self.reflection_engine.as_ref().ok_or_else(|| {
            AutoDreamError::InvalidState("reflection provider unavailable".to_string())
        })?;
        let workspace_id =
            ExperienceJournal::open(self.storage.paths().workspace_root()).workspace_id();
        let input = BoundedReflectionInput {
            schema_version: 1,
            workspace_id,
            run_id: run_id.to_string(),
            evidence: collected
                .items
                .iter()
                .map(|item| {
                    let is_existing_memory = item.source == "laputa";
                    let mut evidence = item.evidence.clone();
                    let summary = if is_existing_memory {
                        evidence.excerpt = None;
                        format!(
                            "existing_memory_digest:{}",
                            crate::content_digest(&item.excerpt)
                        )
                    } else {
                        let redacted = agent_diva_core::security::redact_pii(
                            &item.excerpt,
                            &agent_diva_core::security::PiiConfig::default(),
                        )
                        .redacted;
                        evidence.excerpt = Some(redacted.clone());
                        redacted
                    };
                    ReflectionEvidence { evidence, summary }
                })
                .collect(),
            existing_memory_digests: {
                let mut digests: std::collections::HashSet<String> = collected
                    .items
                    .iter()
                    .filter(|item| item.source == "laputa")
                    .map(|item| crate::content_digest(&item.excerpt))
                    .collect();
                match self.laputa.applied_authority_digests().await {
                    Ok(typed) => {
                        digests.extend(typed);
                    }
                    Err(error) => {
                        tracing::warn!(
                            error = %error,
                            "AutoDream dedup: typed authority digest unavailable; \
                             falling back to section-only dedup (G4 degraded)"
                        );
                    }
                }
                digests.into_iter().collect()
            },
            superseded_memory_digests: match self.laputa.superseded_authority_digests().await {
                Ok(digests) => digests,
                Err(error) => {
                    tracing::warn!(
                        error = %error,
                        "AutoDream superseded dedup: typed superseded digest unavailable; \
                         falling back to no superseded check (Wave 5 degraded)"
                    );
                    Vec::new()
                }
            },
            max_candidates: 8,
        };
        let output = engine
            .reflect(input.clone())
            .await
            .map_err(|error| AutoDreamError::InvalidState(format!("reflection failed: {error}")))?;
        let local_existing_memory = collected
            .items
            .iter()
            .filter(|item| item.source == "laputa")
            .map(|item| item.excerpt.clone())
            .collect::<Vec<_>>();
        let suppressed = self
            .laputa
            .active_candidate_suppression_digests(Utc::now())?;
        let gated = CandidateGate.evaluate(
            &input,
            output.candidates,
            &local_existing_memory,
            &suppressed,
        );
        if !gated.rejected.is_empty() {
            let rejection_summary = serde_json::to_string(&gated.rejected)
                .unwrap_or_else(|_| "candidate_gate_diagnostics_unavailable".to_string());
            self.append_event(run_id, "candidates_rejected", &rejection_summary)?;
        }
        Ok(gated.accepted)
    }

    async fn propose(
        &self,
        run_id: &str,
        collected: Option<&AutoDreamCollectedInputs>,
        candidates: Option<&Vec<MemoryCandidate>>,
    ) -> Result<()> {
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::WriteAutoDreamOutput)?;
        self.config
            .profile
            .validate_request(AutoDreamRestrictedAction::CreateLaputaProposalApi)?;
        let collected = collected.ok_or_else(|| {
            AutoDreamError::InvalidState("cannot propose before gather stage".to_string())
        })?;
        let candidates = candidates.ok_or_else(|| {
            AutoDreamError::InvalidState("cannot propose before reflection stage".to_string())
        })?;
        if candidates.is_empty() {
            self.append_event(
                run_id,
                "no_candidates",
                "reflection completed without eligible candidates",
            )?;
            return Ok(());
        }
        let run = self.read_run(run_id)?;
        let evidence_refs = candidates
            .iter()
            .flat_map(|candidate| candidate.evidence_refs.clone())
            .collect::<Vec<_>>();
        let summary = AutoDreamArtifactSummary {
            headline: format!(
                "AutoDream restricted reflection generated {} review candidates",
                candidates.len()
            ),
            details: vec![
                format!("inputs: {}", collected.summary.total_items),
                format!("omissions: {}", collected.summary.omissions.len()),
                "proposal persisted through Laputa proposal API".to_string(),
            ],
        };
        let drafts = candidates
            .iter()
            .map(|candidate| AutoDreamProposalCandidateDraft {
                proposal_type: candidate.proposal_type.to_string(),
                proposed_patch: candidate.content.clone(),
                risk_level: risk_for_candidate(candidate),
                evidence_refs: candidate.evidence_refs.clone(),
                metadata: Some(candidate.clone()),
            })
            .collect();
        let emitted = AutoDreamOutputEmitter::new(self.storage.clone(), self.laputa.clone())
            .emit_outputs(AutoDreamOutputRequest {
                run,
                generated_at: Utc::now(),
                confidence: candidates
                    .iter()
                    .map(|candidate| candidate.confidence)
                    .max()
                    .unwrap_or_default(),
                evidence_refs,
                output_summary: summary,
                proposal_candidates: drafts,
            })?;
        if let Some(governance) = &self.proposal_governance {
            governance
                .register_proposals(&emitted.proposals)
                .await
                .map_err(|error| {
                    AutoDreamError::InvalidState(format!(
                        "proposal governance registration failed: {error}"
                    ))
                })?;
        }
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
        if run
            .orchestration
            .as_ref()
            .is_some_and(|state| Utc::now() >= state.deadline_at)
        {
            let message = "AutoDream worker deadline expired before stage completion".to_string();
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

fn risk_for_candidate(candidate: &MemoryCandidate) -> RiskLevel {
    if candidate.sensitivity == agent_diva_core::memory::MemorySensitivity::Restricted {
        RiskLevel::High
    } else if candidate.confidence < 75 {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
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

#[cfg(test)]
mod wave4_tests {
    use super::ensure_pulse_recap_only_conflict;
    use crate::{
        content_digest, BoundedReflectionInput, CandidateGate, CandidateRejectionCode,
        ReflectionEvidence,
    };
    use agent_diva_core::evolution::{
        memory_candidate_content_digest, CandidateValue, EvidenceRef, EvidenceSource,
        MemoryCandidate, ProposalType,
    };
    use agent_diva_core::governance::AuditCorrelation;
    use agent_diva_core::memory::{
        memory_content_digest, MemoryAddRequest, MemoryCrudContext, MemoryCrudOutcome,
        MemoryProvenance, MemoryProvenanceSource, MemoryProvider, MemoryRecord, MemoryRecordKind,
        MemoryScope, MemorySensitivity, MemoryTombstone, MemoryTrust, MAX_CONFIDENCE_BPS,
    };
    use agent_diva_core::workspace_identity::canonical_workspace_id;
    use agent_diva_laputa::{LaputaService, TypedMemoryStore};
    use chrono::{TimeZone, Utc};

    fn evidence_marker() -> EvidenceRef {
        EvidenceRef {
            id: "wave4-primary".into(),
            source: EvidenceSource::ExperienceJournal,
            uri: "evidence://wave4-primary".into(),
            excerpt: Some("bounded verification".into()),
            hash: Some("sha256:wave4".into()),
            created_at: Utc.with_ymd_and_hms(2026, 8, 6, 0, 0, 0).unwrap(),
        }
    }

    fn make_candidate(evidence: EvidenceRef, content: &str) -> MemoryCandidate {
        MemoryCandidate {
            candidate_id: format!("candidate-{}", content_digest(content)),
            proposal_type: ProposalType::MemoryPatch,
            content: content.to_string(),
            evidence_refs: vec![evidence],
            confidence: 85,
            scope: MemoryScope {
                tenant_id: "local".into(),
                workspace_id: "workspace-wave4".into(),
                session_id: None,
            },
            sensitivity: MemorySensitivity::Private,
            expected_value: CandidateValue::Medium,
            invalidation_conditions: vec!["user correction".into()],
        }
    }

    fn make_input(existing_digests: Vec<String>, evidence: EvidenceRef) -> BoundedReflectionInput {
        BoundedReflectionInput {
            schema_version: 1,
            workspace_id: "workspace-wave4".into(),
            run_id: "run-wave4".into(),
            evidence: vec![ReflectionEvidence {
                evidence,
                summary: "bounded verification".into(),
            }],
            existing_memory_digests: existing_digests,
            superseded_memory_digests: Vec::new(),
            max_candidates: 8,
        }
    }

    async fn open_service_with_store(temp: &tempfile::TempDir) -> LaputaService {
        TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
        LaputaService::open(temp.path()).unwrap()
    }

    fn crud_context(temp: &tempfile::TempDir) -> MemoryCrudContext {
        MemoryCrudContext {
            workspace_root: temp.path().to_path_buf(),
        }
    }

    #[tokio::test]
    async fn candidate_duplicate_against_typed_authority_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let service = open_service_with_store(&temp).await;
        let provider = agent_diva_laputa::typed_provider::TypedLaputaMemoryProvider::open(
            temp.path(),
            canonical_workspace_id(temp.path()),
        )
        .await
        .unwrap();
        let authority_content = "kestrel prefers high ground at dawn";
        let outcome = provider
            .memory_add(
                &crud_context(&temp),
                MemoryAddRequest {
                    content: authority_content.into(),
                    evidence_refs: vec![],
                },
            )
            .await
            .unwrap();
        assert!(matches!(outcome, MemoryCrudOutcome::Applied { .. }));

        let digests = service.applied_authority_digests().await.unwrap();
        let expected_digest = memory_candidate_content_digest(authority_content);
        assert!(digests.contains(&expected_digest), "got {digests:?}");

        let evidence = evidence_marker();
        let input = make_input(digests, evidence.clone());
        let gated = CandidateGate.evaluate(
            &input,
            vec![make_candidate(evidence, authority_content)],
            &[],
            &[],
        );
        assert!(
            gated.accepted.is_empty(),
            "duplicate candidate must be rejected; accepted = {accepted:?}",
            accepted = gated.accepted
        );
        assert_eq!(gated.rejected.len(), 1);
        assert_eq!(gated.rejected[0].code, CandidateRejectionCode::Duplicate);
    }

    #[tokio::test]
    async fn candidate_fresh_against_typed_authority_is_accepted() {
        let temp = tempfile::tempdir().unwrap();
        let service = open_service_with_store(&temp).await;
        let provider = agent_diva_laputa::typed_provider::TypedLaputaMemoryProvider::open(
            temp.path(),
            canonical_workspace_id(temp.path()),
        )
        .await
        .unwrap();
        provider
            .memory_add(
                &crud_context(&temp),
                MemoryAddRequest {
                    content: "kestrel prefers high ground at dawn".into(),
                    evidence_refs: vec![],
                },
            )
            .await
            .unwrap();

        let digests = service.applied_authority_digests().await.unwrap();
        assert!(!digests.is_empty());

        let evidence = evidence_marker();
        let input = make_input(digests, evidence.clone());
        let gated = CandidateGate.evaluate(
            &input,
            vec![make_candidate(
                evidence,
                "completely novel observation about owls",
            )],
            &[],
            &[],
        );
        assert_eq!(gated.accepted.len(), 1, "fresh candidate must be accepted");
        assert!(gated.rejected.is_empty());
    }

    #[tokio::test]
    async fn superseded_authority_record_no_longer_blocks_duplicate_candidate() {
        let temp = tempfile::tempdir().unwrap();
        let service = open_service_with_store(&temp).await;
        let provider = agent_diva_laputa::typed_provider::TypedLaputaMemoryProvider::open(
            temp.path(),
            canonical_workspace_id(temp.path()),
        )
        .await
        .unwrap();
        let authority_content = "the staging manifest lives at /tmp/staging-xyz";
        let outcome = provider
            .memory_add(
                &crud_context(&temp),
                MemoryAddRequest {
                    content: authority_content.into(),
                    evidence_refs: vec![],
                },
            )
            .await
            .unwrap();
        let target_id = match outcome {
            MemoryCrudOutcome::Applied { entry, .. } => entry.expect("entry").id,
            other => panic!("expected Applied, got {other:?}"),
        };
        drop(provider);

        // Sanity: before the tombstone, the digest is visible.
        let before = service.applied_authority_digests().await.unwrap();
        assert!(before.contains(&memory_candidate_content_digest(authority_content)));

        // Write a supersedes tombstone directly via TypedMemoryStore (matching
        // the laputa wave3/wave4 test pattern — the worker does not care how
        // the tombstone arrived, only that the target digest is excluded).
        let store = TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
        let now = Utc::now();
        let metadata = store.metadata().await.unwrap();
        let tombstone = MemoryRecord {
            id: format!("tombstone-{}", now.timestamp_micros()),
            kind: MemoryRecordKind::LongTerm,
            content: String::new(),
            provenance: MemoryProvenance {
                source: MemoryProvenanceSource::AutoDream,
                source_id: "wave4-test".into(),
                content_digest: memory_content_digest(b""),
                captured_at: now,
                correlation: AuditCorrelation {
                    request_id: format!("tombstone-{target_id}"),
                    turn_id: "wave4".into(),
                    session_id: "global".into(),
                    trace_id: None,
                },
            },
            evidence_refs: vec![],
            confidence_bps: MAX_CONFIDENCE_BPS,
            sensitivity: MemorySensitivity::Internal,
            trust: MemoryTrust::AppliedAuthority,
            scope: MemoryScope {
                tenant_id: "local".into(),
                workspace_id: canonical_workspace_id(temp.path()),
                session_id: None,
            },
            created_at: now,
            effective_at: now,
            expires_at: None,
            supersedes: vec![target_id.clone()],
            tombstone: Some(MemoryTombstone {
                target_record_id: target_id.clone(),
                reason_digest: memory_content_digest(b"staging path retired"),
                actor_id: "wave4-test".into(),
                created_at: now,
            }),
        };
        store
            .put(tombstone, metadata.store_revision, None)
            .await
            .unwrap();
        drop(store);

        let after = service.applied_authority_digests().await.unwrap();
        assert!(
            !after.contains(&memory_candidate_content_digest(authority_content)),
            "superseded target digest must disappear; got {after:?}"
        );

        let evidence = evidence_marker();
        let input = make_input(after, evidence.clone());
        let gated = CandidateGate.evaluate(
            &input,
            vec![make_candidate(evidence, authority_content)],
            &[],
            &[],
        );
        assert_eq!(
            gated.accepted.len(),
            1,
            "after supersedes, same-content candidate must be acceptable again"
        );
        assert!(gated.rejected.is_empty());
    }

    #[test]
    fn autodream_retries_only_pulse_recap_conflicts() {
        let previous = agent_diva_laputa::ActmemDocument::empty();
        let mut pulse_changed = previous.clone();
        pulse_changed.revision = 1;
        pulse_changed.pulse = "- new pulse".into();
        assert!(ensure_pulse_recap_only_conflict(&previous, &pulse_changed).is_ok());

        let mut work_changed = pulse_changed;
        work_changed.work = "### Goal\n- concurrent edit\n\n### Open\n\n### Next\n\n### Constraints\n\n### Pointers".into();
        assert!(ensure_pulse_recap_only_conflict(&previous, &work_changed).is_err());
    }
}
