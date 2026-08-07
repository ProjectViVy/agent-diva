use std::{
    fs,
    path::{Path, PathBuf},
};

use agent_diva_core::evolution::{
    validate_governance_evidence, AuditEvent, AuditEventKind, ChangelogAction, ChangelogRecord,
    EvidenceRef, EvolutionProposal, LaputaSectionName, ProposalState, ProposalType, RiskLevel,
    RollbackRequest,
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

/// Optional apply behavior used by tests and failure-injection validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApplyOptions {
    pub failure_point: Option<ApplyFailurePoint>,
    /// Whether the legacy section projection is the authority write target.
    pub write_authority: bool,
}

impl Default for ApplyOptions {
    fn default() -> Self {
        Self {
            failure_point: None,
            write_authority: true,
        }
    }
}

/// Deterministic failure points for apply transaction tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyFailurePoint {
    AfterSectionWriteBeforeChangelog,
    AfterChangelogBeforeAudit,
}

/// Durable result produced by a successful proposal apply.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApplyOutcome {
    pub proposal: EvolutionProposal,
    pub changelog: ChangelogRecord,
    pub audit_event: AuditEvent,
    pub rollback_request: RollbackRequest,
}

struct ApplyFailureRollback<'a> {
    wrote_section: bool,
    section_path: &'a Path,
    before: &'a str,
    rollback_request: Option<&'a RollbackRequest>,
    changelog: Option<&'a ChangelogRecord>,
    audit_event: Option<&'a AuditEvent>,
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
            validate_governance_evidence(&evidence_refs)
                .map_err(|reason| invalid_proposal(&proposal.id, &reason))?;
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

    pub fn apply_proposal(
        &self,
        id: &str,
        actor: impl Into<String>,
        applied_at: DateTime<Utc>,
    ) -> Result<ApplyOutcome> {
        self.apply_proposal_with_options(id, actor, applied_at, ApplyOptions::default())
    }

    pub fn apply_proposal_with_options(
        &self,
        id: &str,
        actor: impl Into<String>,
        applied_at: DateTime<Utc>,
        options: ApplyOptions,
    ) -> Result<ApplyOutcome> {
        let _guard = self.acquire_lock()?;
        let actor = actor.into();
        let mut proposal = self.get_proposal(id)?;

        ensure_transition_allowed(&proposal.state, &ProposalState::Applied)?;
        if let Err(error) = validate_apply_contract(&proposal, options.write_authority) {
            if matches!(error, LaputaError::UnresolvedConflict { .. }) {
                proposal.state = ProposalState::NeedsAttention;
                proposal.updated_at = applied_at;
                self.write_proposal(&proposal)?;
            }
            return Err(error);
        }

        let changelog_id = format!("changelog-{}-{}", proposal.id, applied_at.timestamp());
        let audit_event_id = format!("audit-{}-{}", proposal.id, applied_at.timestamp());
        let section_path = self
            .storage
            .paths()
            .section_file(proposal.target_section.clone());
        let before = fs::read_to_string(&section_path).unwrap_or_default();
        let rollback_request = RollbackRequest {
            changelog_id: changelog_id.clone(),
            requested_by: actor.clone(),
            reason: format!("staged before applying proposal {}", proposal.id),
            requested_at: applied_at,
        };
        let changelog = ChangelogRecord {
            id: changelog_id.clone(),
            action: ChangelogAction::Apply,
            target_section: proposal.target_section.clone(),
            before: before.clone(),
            after: proposal.proposed_patch.clone(),
            diff: unified_diff(&before, &proposal.proposed_patch),
            proposal_id: Some(proposal.id.clone()),
            audit_event_id: Some(audit_event_id.clone()),
            reverted: false,
            stale: false,
            created_at: applied_at,
            applied_by: actor.clone(),
        };
        let audit_event = AuditEvent {
            id: audit_event_id,
            kind: AuditEventKind::ProposalApplied,
            actor,
            proposal_id: Some(proposal.id.clone()),
            target_section: Some(proposal.target_section.clone()),
            message: format!("applied proposal {}", proposal.id),
            created_at: applied_at,
        };

        self.write_rollback_request(&rollback_request)?;

        let wrote_section =
            options.write_authority && proposal.proposal_type != ProposalType::Deprecation;
        if wrote_section {
            atomic_write_json(&section_path, &parse_json_patch(&proposal)?)?;
        }

        if options.failure_point == Some(ApplyFailurePoint::AfterSectionWriteBeforeChangelog) {
            let failure = LaputaError::InjectedApplyFailure {
                point: ApplyFailurePoint::AfterSectionWriteBeforeChangelog,
            };
            self.rollback_apply_failure(
                &mut proposal,
                ApplyFailureRollback {
                    wrote_section,
                    section_path: &section_path,
                    before: &before,
                    rollback_request: Some(&rollback_request),
                    changelog: None,
                    audit_event: None,
                },
                failure,
            )?;
        }

        self.write_changelog_record(&changelog).map_err(|error| {
            self.rollback_apply_failure(
                &mut proposal,
                ApplyFailureRollback {
                    wrote_section,
                    section_path: &section_path,
                    before: &before,
                    rollback_request: Some(&rollback_request),
                    changelog: None,
                    audit_event: None,
                },
                error,
            )
            .unwrap_err()
        })?;

        if options.failure_point == Some(ApplyFailurePoint::AfterChangelogBeforeAudit) {
            let failure = LaputaError::InjectedApplyFailure {
                point: ApplyFailurePoint::AfterChangelogBeforeAudit,
            };
            self.rollback_apply_failure(
                &mut proposal,
                ApplyFailureRollback {
                    wrote_section,
                    section_path: &section_path,
                    before: &before,
                    rollback_request: Some(&rollback_request),
                    changelog: Some(&changelog),
                    audit_event: None,
                },
                failure,
            )?;
        }

        self.write_audit_event(&audit_event).map_err(|error| {
            self.rollback_apply_failure(
                &mut proposal,
                ApplyFailureRollback {
                    wrote_section,
                    section_path: &section_path,
                    before: &before,
                    rollback_request: Some(&rollback_request),
                    changelog: Some(&changelog),
                    audit_event: None,
                },
                error,
            )
            .unwrap_err()
        })?;

        proposal.state = ProposalState::Applied;
        proposal.updated_at = applied_at;
        self.write_proposal(&proposal)?;

        Ok(ApplyOutcome {
            proposal,
            changelog,
            audit_event,
            rollback_request,
        })
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
                    .unwrap_or("<invalid>")
                    .to_string();
                // Read-only history tolerance: proposals persisted against
                // retired sections or older schemas are skipped instead of
                // breaking the whole listing.
                match read_proposal(&path, &id) {
                    Ok(proposal) => proposals.push(proposal),
                    Err(error) => {
                        tracing::warn!(
                            proposal_id = %id,
                            %error,
                            "skipping unreadable persisted proposal"
                        );
                    }
                }
            }
        }

        Ok(proposals)
    }

    fn write_proposal(&self, proposal: &EvolutionProposal) -> Result<()> {
        atomic_write_json(self.proposal_path(&proposal.id), proposal)
    }

    fn write_changelog_record(&self, record: &ChangelogRecord) -> Result<()> {
        atomic_write_json(
            self.storage
                .paths()
                .changelog_dir()
                .join(format!("{}.json", record.id)),
            record,
        )
    }

    fn write_audit_event(&self, event: &AuditEvent) -> Result<()> {
        atomic_write_json(
            self.storage
                .paths()
                .audit_dir()
                .join(format!("{}.json", event.id)),
            event,
        )
    }

    fn write_rollback_request(&self, request: &RollbackRequest) -> Result<()> {
        atomic_write_json(
            self.storage
                .paths()
                .rollback_dir()
                .join(format!("{}.json", request.changelog_id)),
            request,
        )
    }

    pub(crate) fn transition_proposal_without_lock(
        &self,
        id: &str,
        to: ProposalState,
        updated_at: DateTime<Utc>,
    ) -> Result<EvolutionProposal> {
        let mut proposal = self.get_proposal(id)?;
        ensure_transition_allowed(&proposal.state, &to)?;

        proposal.state = to;
        proposal.updated_at = updated_at;
        self.write_proposal(&proposal)?;
        Ok(proposal)
    }

    pub(crate) fn acquire_write_lock(&self) -> Result<LaputaLock> {
        self.acquire_lock()
    }

    fn rollback_apply_failure(
        &self,
        proposal: &mut EvolutionProposal,
        rollback: ApplyFailureRollback<'_>,
        failure: LaputaError,
    ) -> Result<ApplyOutcome> {
        if rollback.wrote_section {
            let rollback_result = if rollback.before.is_empty() {
                match fs::remove_file(rollback.section_path) {
                    Ok(()) => Ok(()),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                    Err(source) => Err(LaputaError::io(rollback.section_path, source)),
                }
            } else {
                crate::atomic_write(rollback.section_path, rollback.before.as_bytes())
            };

            if let Err(source) = rollback_result {
                return Err(LaputaError::RollbackFailed {
                    id: proposal.id.clone(),
                    source: Box::new(source),
                });
            }
        }

        if let Some(request) = rollback.rollback_request {
            let _ = fs::remove_file(
                self.storage
                    .paths()
                    .rollback_dir()
                    .join(format!("{}.json", request.changelog_id)),
            );
        }
        if let Some(record) = rollback.changelog {
            let _ = fs::remove_file(
                self.storage
                    .paths()
                    .changelog_dir()
                    .join(format!("{}.json", record.id)),
            );
        }
        if let Some(event) = rollback.audit_event {
            let _ = fs::remove_file(
                self.storage
                    .paths()
                    .audit_dir()
                    .join(format!("{}.json", event.id)),
            );
        }

        proposal.state = ProposalState::NeedsAttention;
        self.write_proposal(proposal)?;
        Err(failure)
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
    validate_governance_evidence(&proposal.evidence_refs)
        .map_err(|reason| invalid_proposal(&proposal.id, &reason))?;
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

fn validate_apply_contract(
    proposal: &EvolutionProposal,
    legacy_projection_write: bool,
) -> Result<()> {
    if proposal.target_section != proposal.proposal_type.target_section() {
        return Err(LaputaError::UnauthorizedTarget {
            id: proposal.id.clone(),
            proposal_type: proposal.proposal_type.clone(),
            target_section: proposal.target_section.clone(),
        });
    }

    if !is_writable_apply_target(&proposal.proposal_type, &proposal.target_section) {
        return Err(LaputaError::UnauthorizedTarget {
            id: proposal.id.clone(),
            proposal_type: proposal.proposal_type.clone(),
            target_section: proposal.target_section.clone(),
        });
    }

    // JSON is a legacy section-projection storage contract, not a typed Memory
    // content contract. Typed authority stores a validated MemoryRecord and may
    // legitimately contain durable plain text.
    if legacy_projection_write {
        parse_json_patch(proposal)?;
        reject_unresolved_conflicts(proposal)?;
    }

    Ok(())
}

fn is_writable_apply_target(
    proposal_type: &ProposalType,
    target_section: &LaputaSectionName,
) -> bool {
    match proposal_type {
        ProposalType::Deprecation => *target_section == LaputaSectionName::Changelog,
        _ => matches!(
            target_section,
            LaputaSectionName::Identity
                | LaputaSectionName::Relationship
                | LaputaSectionName::Commitment
                | LaputaSectionName::Preferences
                | LaputaSectionName::MemoryMd
                | LaputaSectionName::Daily
                | LaputaSectionName::Weekly
                | LaputaSectionName::Monthly
        ),
    }
}

fn parse_json_patch(proposal: &EvolutionProposal) -> Result<serde_json::Value> {
    serde_json::from_str(&proposal.proposed_patch).map_err(|source| {
        LaputaError::SchemaIncompatible {
            id: proposal.id.clone(),
            reason: source.to_string(),
        }
    })
}

fn reject_unresolved_conflicts(proposal: &EvolutionProposal) -> Result<()> {
    let value = parse_json_patch(proposal)?;
    let has_conflict_flag = value
        .get("unresolved_conflict")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
        || value
            .get("unresolved_conflicts")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false);
    let has_conflict_list = value
        .get("conflicts")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|conflicts| !conflicts.is_empty());

    if has_conflict_flag || has_conflict_list {
        Err(LaputaError::UnresolvedConflict {
            id: proposal.id.clone(),
            reason: "proposal contains unresolved conflict markers".to_string(),
        })
    } else {
        Ok(())
    }
}

fn ensure_transition_allowed(from: &ProposalState, to: &ProposalState) -> Result<()> {
    let allowed = match from {
        ProposalState::PendingReview => matches!(
            to,
            ProposalState::Approved
                | ProposalState::Rejected
                | ProposalState::Edited
                | ProposalState::Deferred
                | ProposalState::NeedsAttention
                | ProposalState::Superseded
        ),
        ProposalState::Edited => matches!(
            to,
            ProposalState::Approved
                | ProposalState::Rejected
                | ProposalState::Edited
                | ProposalState::Deferred
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
                | ProposalState::Deferred
                | ProposalState::Superseded
        ),
        ProposalState::Deferred => matches!(
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

pub(crate) fn unified_diff(before: &str, after: &str) -> String {
    let mut diff = String::from("--- before\n+++ after\n@@ -1 +1 @@\n");
    for line in before.lines() {
        diff.push('-');
        diff.push_str(line);
        diff.push('\n');
    }
    for line in after.lines() {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    if before.is_empty() {
        diff.push_str("-\n");
    }
    if after.is_empty() {
        diff.push_str("+\n");
    }
    diff
}
