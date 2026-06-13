use std::{
    fs,
    path::{Path, PathBuf},
};

use agent_diva_core::evolution::{
    EvidenceRef, EvolutionProposal, LaputaSectionName, ProposalState, ProposalType, RiskLevel,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::{atomic_write_json, LaputaError, LaputaLock, LaputaStorage, LockOptions, Result};

/// Filters for proposal list and summary reads.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProposalFilter {
    pub state: Option<ProposalState>,
    pub proposal_type: Option<ProposalType>,
    pub target_section: Option<LaputaSectionName>,
    pub source_run_id: Option<String>,
    pub since: Option<DateTime<Utc>>,
}

/// Compact proposal data required by later inbox and manager surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposalSummary {
    pub id: String,
    pub state: ProposalState,
    pub proposal_type: ProposalType,
    pub target_section: LaputaSectionName,
    pub source_run_id: Option<String>,
    pub risk_level: RiskLevel,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub evidence_count: usize,
}

/// Editable proposal fields. Evidence refs are preserved unless replaced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProposalEdit {
    pub proposed_patch: Option<String>,
    pub evidence_refs: Option<Vec<EvidenceRef>>,
    pub risk_level: Option<RiskLevel>,
    pub updated_at: DateTime<Utc>,
}

/// File-first repository for proposal persistence and lifecycle transitions.
#[derive(Debug, Clone)]
pub struct ProposalRepository {
    storage: LaputaStorage,
    lock_options: LockOptions,
}

impl ProposalRepository {
    pub fn new(storage: LaputaStorage) -> Self {
        Self {
            storage,
            lock_options: LockOptions::default(),
        }
    }

    pub fn with_lock_options(storage: LaputaStorage, lock_options: LockOptions) -> Self {
        Self {
            storage,
            lock_options,
        }
    }

    pub fn create_proposal(&self, proposal: EvolutionProposal) -> Result<EvolutionProposal> {
        validate_new_proposal(&proposal)?;
        let _guard = self.acquire_lock()?;
        let path = self.proposal_path(&proposal.id);

        if path.exists() {
            return Err(LaputaError::ProposalAlreadyExists {
                id: proposal.id.clone(),
            });
        }

        self.write_proposal(&proposal)?;
        Ok(proposal)
    }

    pub fn get_proposal(&self, id: &str) -> Result<EvolutionProposal> {
        read_proposal(&self.proposal_path(id), id)
    }

    pub fn list_proposals(&self, filter: ProposalFilter) -> Result<Vec<EvolutionProposal>> {
        let mut proposals = self.read_all_proposals()?;
        proposals.retain(|proposal| filter.matches(proposal));
        proposals.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then_with(|| left.id.cmp(&right.id))
        });
        Ok(proposals)
    }

    pub fn list_summaries(&self, filter: ProposalFilter) -> Result<Vec<ProposalSummary>> {
        self.list_proposals(filter)
            .map(|proposals| proposals.into_iter().map(ProposalSummary::from).collect())
    }

    pub fn edit_proposal(&self, id: &str, edit: ProposalEdit) -> Result<EvolutionProposal> {
        let _guard = self.acquire_lock()?;
        let mut proposal = self.get_proposal(id)?;
        ensure_transition_allowed(&proposal.state, &ProposalState::Edited)?;

        if let Some(patch) = edit.proposed_patch {
            proposal.proposed_patch = patch;
        }
        if let Some(evidence_refs) = edit.evidence_refs {
            if evidence_refs.is_empty() {
                return Err(invalid_proposal(
                    &proposal.id,
                    "evidence_refs must not be empty",
                ));
            }
            proposal.evidence_refs = evidence_refs;
        }
        if let Some(risk_level) = edit.risk_level {
            proposal.risk_level = risk_level;
        }
        proposal.state = ProposalState::Edited;
        proposal.updated_at = edit.updated_at;

        self.write_proposal(&proposal)?;
        Ok(proposal)
    }

    pub fn transition_proposal(
        &self,
        id: &str,
        to: ProposalState,
        updated_at: DateTime<Utc>,
    ) -> Result<EvolutionProposal> {
        let _guard = self.acquire_lock()?;
        let mut proposal = self.get_proposal(id)?;
        ensure_transition_allowed(&proposal.state, &to)?;

        proposal.state = to;
        proposal.updated_at = updated_at;
        self.write_proposal(&proposal)?;
        Ok(proposal)
    }

    fn read_all_proposals(&self) -> Result<Vec<EvolutionProposal>> {
        let proposals_dir = self.storage.paths().proposals_dir();
        let mut proposals = Vec::new();

        for entry in fs::read_dir(&proposals_dir)
            .map_err(|source| LaputaError::io(&proposals_dir, source))?
        {
            let entry = entry.map_err(|source| LaputaError::io(&proposals_dir, source))?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                let id = path
                    .file_stem()
                    .and_then(|stem| stem.to_str())
                    .unwrap_or("<invalid>");
                proposals.push(read_proposal(&path, id)?);
            }
        }

        Ok(proposals)
    }

    fn write_proposal(&self, proposal: &EvolutionProposal) -> Result<()> {
        atomic_write_json(self.proposal_path(&proposal.id), proposal)
    }

    fn proposal_path(&self, id: &str) -> PathBuf {
        self.storage
            .paths()
            .proposals_dir()
            .join(format!("{id}.json"))
    }

    fn acquire_lock(&self) -> Result<LaputaLock> {
        LaputaLock::acquire(
            self.storage.paths().lock_file("proposals"),
            self.lock_options,
        )
    }
}

impl ProposalFilter {
    fn matches(&self, proposal: &EvolutionProposal) -> bool {
        self.state
            .as_ref()
            .map_or(true, |state| proposal.state == *state)
            && self.proposal_type.as_ref().map_or(true, |proposal_type| {
                proposal.proposal_type == *proposal_type
            })
            && self
                .target_section
                .as_ref()
                .map_or(true, |section| proposal.target_section == *section)
            && self.source_run_id.as_ref().map_or(true, |source_run_id| {
                proposal.source_run_id.as_ref() == Some(source_run_id)
            })
            && self
                .since
                .map_or(true, |since| proposal.updated_at >= since)
    }
}

impl From<EvolutionProposal> for ProposalSummary {
    fn from(proposal: EvolutionProposal) -> Self {
        Self {
            id: proposal.id,
            state: proposal.state,
            proposal_type: proposal.proposal_type,
            target_section: proposal.target_section,
            source_run_id: proposal.source_run_id,
            risk_level: proposal.risk_level,
            created_at: proposal.created_at,
            updated_at: proposal.updated_at,
            evidence_count: proposal.evidence_refs.len(),
        }
    }
}

fn read_proposal(path: &Path, id: &str) -> Result<EvolutionProposal> {
    if !path.exists() {
        return Err(LaputaError::ProposalNotFound { id: id.to_string() });
    }

    let bytes = fs::read(path).map_err(|source| LaputaError::io(path, source))?;
    serde_json::from_slice(&bytes).map_err(LaputaError::from)
}

fn validate_new_proposal(proposal: &EvolutionProposal) -> Result<()> {
    if proposal.id.trim().is_empty() {
        return Err(invalid_proposal(&proposal.id, "id must not be empty"));
    }
    if proposal.evidence_refs.is_empty() {
        return Err(invalid_proposal(
            &proposal.id,
            "evidence_refs must not be empty",
        ));
    }
    if proposal.target_section != proposal.proposal_type.target_section() {
        return Err(invalid_proposal(
            &proposal.id,
            "target_section must match proposal_type routing",
        ));
    }
    if !matches!(
        proposal.state,
        ProposalState::PendingReview | ProposalState::NeedsAttention | ProposalState::RunFailed
    ) {
        return Err(invalid_proposal(
            &proposal.id,
            "new proposals must start in pending_review or a diagnostic state",
        ));
    }

    Ok(())
}

fn ensure_transition_allowed(from: &ProposalState, to: &ProposalState) -> Result<()> {
    let allowed = match from {
        ProposalState::PendingReview => matches!(
            to,
            ProposalState::Approved
                | ProposalState::Rejected
                | ProposalState::Edited
                | ProposalState::NeedsAttention
                | ProposalState::Superseded
        ),
        ProposalState::Edited => matches!(
            to,
            ProposalState::Approved
                | ProposalState::Rejected
                | ProposalState::Edited
                | ProposalState::NeedsAttention
                | ProposalState::Superseded
        ),
        ProposalState::Approved => matches!(
            to,
            ProposalState::Applied | ProposalState::Rejected | ProposalState::NeedsAttention
        ),
        ProposalState::Applied => {
            matches!(to, ProposalState::Reverted | ProposalState::Superseded)
        }
        ProposalState::NeedsAttention => matches!(
            to,
            ProposalState::PendingReview
                | ProposalState::Rejected
                | ProposalState::Edited
                | ProposalState::Superseded
        ),
        ProposalState::RunFailed
        | ProposalState::Rejected
        | ProposalState::Reverted
        | ProposalState::Superseded => false,
    };

    if allowed {
        Ok(())
    } else {
        Err(LaputaError::InvalidProposalTransition {
            from: from.clone(),
            to: to.clone(),
        })
    }
}

fn invalid_proposal(id: &str, reason: impl Into<String>) -> LaputaError {
    LaputaError::InvalidProposal {
        id: id.to_string(),
        reason: reason.into(),
    }
}
