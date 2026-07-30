use std::{fs, fs::OpenOptions, io::Write, path::Path};

use agent_diva_core::evolution::{
    validate_governance_evidence, AuditEvent, AuditEventKind, AutoDreamRunRecord, EvidenceRef,
    EvolutionProposal, ProposalState, ProposalType, RiskLevel,
};
use agent_diva_laputa::{LaputaError, LaputaService};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{atomic::atomic_write_json, AutoDreamError, AutoDreamStorage, Result};

const AUTODREAM_RUN_SCHEMA_VERSION: &str = "1.0.0";
const AUTODREAM_ACTOR: &str = "autodream";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamArtifactSummary {
    pub headline: String,
    pub details: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamProposalCandidateDraft {
    pub proposal_type: String,
    pub proposed_patch: String,
    pub risk_level: RiskLevel,
    pub evidence_refs: Vec<EvidenceRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmittedProposalCandidate {
    pub proposal_id: String,
    pub proposal_type: ProposalType,
    pub target_section: agent_diva_core::evolution::LaputaSectionName,
    pub risk_level: RiskLevel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamRunArtifact {
    pub schema_version: String,
    pub run_id: String,
    pub generated_at: DateTime<Utc>,
    pub review_required: bool,
    pub confidence: u8,
    pub evidence_refs: Vec<EvidenceRef>,
    pub output_summary: AutoDreamArtifactSummary,
    pub proposal_candidates: Vec<EmittedProposalCandidate>,
    pub proposal_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AutoDreamOutputEventKind {
    ProposalCreated,
    OutputsPersisted,
    OutputsFailed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoDreamOutputEvent {
    pub id: String,
    pub run_id: String,
    pub kind: AutoDreamOutputEventKind,
    pub proposal_id: Option<String>,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct AutoDreamOutputRequest {
    pub run: AutoDreamRunRecord,
    pub generated_at: DateTime<Utc>,
    pub confidence: u8,
    pub evidence_refs: Vec<EvidenceRef>,
    pub output_summary: AutoDreamArtifactSummary,
    pub proposal_candidates: Vec<AutoDreamProposalCandidateDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmitOutputsResult {
    pub run: AutoDreamRunRecord,
    pub artifact: AutoDreamRunArtifact,
    pub events: Vec<AutoDreamOutputEvent>,
    pub proposals: Vec<EvolutionProposal>,
}

#[derive(Debug, Clone)]
pub struct AutoDreamOutputEmitter {
    storage: AutoDreamStorage,
    laputa: LaputaService,
}

impl AutoDreamOutputEmitter {
    pub fn new(storage: AutoDreamStorage, laputa: LaputaService) -> Self {
        Self { storage, laputa }
    }

    pub fn emit_outputs(&self, request: AutoDreamOutputRequest) -> Result<EmitOutputsResult> {
        validate_request(&request)?;
        let artifact_path = self.storage.paths().run_artifact_file(&request.run.id);
        if artifact_path.exists() {
            let artifact: AutoDreamRunArtifact = serde_json::from_slice(
                &fs::read(&artifact_path)
                    .map_err(|source| AutoDreamError::io(&artifact_path, source))?,
            )?;
            if artifact.run_id != request.run.id {
                return Err(AutoDreamError::InvalidState(
                    "AutoDream artifact run identity mismatch".to_string(),
                ));
            }
            let proposals = artifact
                .proposal_ids
                .iter()
                .map(|id| self.laputa.get_proposal(id).map_err(AutoDreamError::from))
                .collect::<Result<Vec<_>>>()?;
            let mut run = request.run;
            run.proposal_ids = artifact.proposal_ids.clone();
            run.summary = Some(render_run_summary(&artifact));
            self.write_run_record(&run)?;
            return Ok(EmitOutputsResult {
                run,
                artifact,
                events: Vec::new(),
                proposals,
            });
        }

        let mut persisted = Vec::with_capacity(request.proposal_candidates.len());
        let mut proposal_ids = Vec::with_capacity(request.proposal_candidates.len());
        let mut events = Vec::new();

        for (index, candidate) in request.proposal_candidates.iter().enumerate() {
            let proposal_type = candidate
                .proposal_type
                .parse::<ProposalType>()
                .map_err(|err| AutoDreamError::InvalidState(err.to_string()))?;
            let proposal = EvolutionProposal {
                id: format!("proposal-{}-{index}", request.run.id),
                created_at: request.generated_at,
                updated_at: request.generated_at,
                created_by: AUTODREAM_ACTOR.to_string(),
                proposal_type: proposal_type.clone(),
                target_section: proposal_type.target_section(),
                evidence_refs: candidate.evidence_refs.clone(),
                proposed_patch: candidate.proposed_patch.clone(),
                risk_level: candidate.risk_level.clone(),
                state: ProposalState::PendingReview,
                source_run_id: Some(request.run.id.clone()),
            };
            let proposal = match self.laputa.create_proposal(proposal.clone()) {
                Ok(created) => {
                    events.push(AutoDreamOutputEvent {
                        id: format!("evt-{}", Uuid::new_v4()),
                        run_id: request.run.id.clone(),
                        kind: AutoDreamOutputEventKind::ProposalCreated,
                        proposal_id: Some(created.id.clone()),
                        message: format!("proposal {} created for autodream run", created.id),
                        created_at: request.generated_at,
                    });
                    created
                }
                Err(LaputaError::ProposalAlreadyExists { .. }) => {
                    let existing = self.laputa.get_proposal(&proposal.id)?;
                    if !same_proposal_content(&existing, &proposal) {
                        return Err(AutoDreamError::InvalidState(format!(
                            "deterministic proposal {} conflicts with persisted content",
                            proposal.id
                        )));
                    }
                    existing
                }
                Err(error) => return Err(error.into()),
            };
            proposal_ids.push(proposal.id.clone());
            persisted.push(proposal);
        }

        let artifact = AutoDreamRunArtifact {
            schema_version: AUTODREAM_RUN_SCHEMA_VERSION.to_string(),
            run_id: request.run.id.clone(),
            generated_at: request.generated_at,
            review_required: true,
            confidence: request.confidence,
            evidence_refs: request.evidence_refs.clone(),
            output_summary: request.output_summary,
            proposal_candidates: persisted
                .iter()
                .map(|proposal| EmittedProposalCandidate {
                    proposal_id: proposal.id.clone(),
                    proposal_type: proposal.proposal_type.clone(),
                    target_section: proposal.target_section.clone(),
                    risk_level: proposal.risk_level.clone(),
                })
                .collect(),
            proposal_ids: proposal_ids.clone(),
        };
        self.write_run_artifact(&request.run.id, &artifact)?;

        let mut run = request.run;
        run.proposal_ids = proposal_ids.clone();
        run.summary = Some(render_run_summary(&artifact));

        events.push(AutoDreamOutputEvent {
            id: format!("evt-{}", Uuid::new_v4()),
            run_id: run.id.clone(),
            kind: AutoDreamOutputEventKind::OutputsPersisted,
            proposal_id: None,
            message: format!(
                "persisted autodream outputs with {} proposals",
                run.proposal_ids.len()
            ),
            created_at: artifact.generated_at,
        });
        self.write_run_record(&run)?;
        self.append_events(&events)?;

        Ok(EmitOutputsResult {
            run,
            artifact,
            events,
            proposals: persisted,
        })
    }

    fn write_run_record(&self, run: &AutoDreamRunRecord) -> Result<()> {
        atomic_write_json(&self.storage.paths().run_record_file(&run.id), run)
    }

    fn write_run_artifact(&self, run_id: &str, artifact: &AutoDreamRunArtifact) -> Result<()> {
        let path = self.storage.paths().run_artifact_file(run_id);
        let parent = path
            .parent()
            .ok_or_else(|| AutoDreamError::InvalidState("path has no parent".to_string()))?;
        fs::create_dir_all(parent).map_err(|source| AutoDreamError::io(parent, source))?;
        let payload = serde_json::to_vec(artifact)?;
        crate::atomic::atomic_write(&path, &payload)
    }

    fn append_events(&self, events: &[AutoDreamOutputEvent]) -> Result<()> {
        let path = self.storage.paths().events_jsonl();
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|source| AutoDreamError::io(&path, source))?;
        for event in events {
            let line = serde_json::to_string(event)?;
            writeln!(file, "{line}").map_err(|source| AutoDreamError::io(&path, source))?;
        }
        file.sync_all()
            .map_err(|source| AutoDreamError::io(&path, source))?;
        Ok(())
    }
}

fn same_proposal_content(left: &EvolutionProposal, right: &EvolutionProposal) -> bool {
    left.id == right.id
        && left.created_by == right.created_by
        && left.proposal_type == right.proposal_type
        && left.target_section == right.target_section
        && left.evidence_refs == right.evidence_refs
        && left.proposed_patch == right.proposed_patch
        && left.risk_level == right.risk_level
        && left.source_run_id == right.source_run_id
}

fn validate_request(request: &AutoDreamOutputRequest) -> Result<()> {
    if request.evidence_refs.is_empty() {
        return Err(AutoDreamError::InvalidState(
            "output artifacts require at least one evidence ref".to_string(),
        ));
    }
    validate_governance_evidence(&request.evidence_refs).map_err(AutoDreamError::InvalidState)?;
    if request.proposal_candidates.is_empty() {
        return Err(AutoDreamError::InvalidState(
            "output artifacts require at least one proposal candidate".to_string(),
        ));
    }
    if request.confidence > 100 {
        return Err(AutoDreamError::InvalidState(
            "confidence must be within 0..=100".to_string(),
        ));
    }
    for candidate in &request.proposal_candidates {
        validate_governance_evidence(&candidate.evidence_refs)
            .map_err(AutoDreamError::InvalidState)?;
    }
    Ok(())
}

fn render_run_summary(artifact: &AutoDreamRunArtifact) -> String {
    format!(
        "{} (confidence: {}%, proposals: {})",
        artifact.output_summary.headline,
        artifact.confidence,
        artifact.proposal_ids.len()
    )
}

#[allow(dead_code)]
fn _audit_event_from_output(event: &AutoDreamOutputEvent) -> AuditEvent {
    AuditEvent {
        id: event.id.clone(),
        kind: match event.kind {
            AutoDreamOutputEventKind::ProposalCreated => AuditEventKind::ProposalCreated,
            AutoDreamOutputEventKind::OutputsPersisted => AuditEventKind::NeedsAttention,
            AutoDreamOutputEventKind::OutputsFailed => AuditEventKind::WriteFailed,
        },
        actor: AUTODREAM_ACTOR.to_string(),
        proposal_id: event.proposal_id.clone(),
        target_section: None,
        message: event.message.clone(),
        created_at: event.created_at,
    }
}

#[allow(dead_code)]
fn _path_exists(path: &Path) -> bool {
    path.exists()
}
