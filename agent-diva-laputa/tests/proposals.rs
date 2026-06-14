use chrono::{DateTime, Utc};

use agent_diva_core::evolution::{
    route_proposal_type, EvidenceRef, EvidenceSource, EvolutionError, EvolutionProposal,
    LaputaSectionName, ProposalState, ProposalType, RiskLevel,
};
use agent_diva_laputa::{
    LaputaError, LaputaStorage, ProposalEdit, ProposalFilter, ProposalRepository,
};

fn ts(seconds: u32) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&format!("2026-06-14T00:00:{seconds:02}Z"))
        .unwrap()
        .with_timezone(&Utc)
}

fn evidence(id: &str) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        source: EvidenceSource::Session,
        uri: format!("session://{id}"),
        excerpt: Some("bounded evidence".to_string()),
        hash: Some(format!("hash-{id}")),
        created_at: ts(1),
    }
}

fn proposal(id: &str, state: ProposalState) -> EvolutionProposal {
    EvolutionProposal {
        id: id.to_string(),
        created_at: ts(2),
        updated_at: ts(2),
        created_by: "autodream".to_string(),
        proposal_type: ProposalType::MemoryPatch,
        target_section: LaputaSectionName::MemoryMd,
        evidence_refs: vec![evidence("ev-1")],
        proposed_patch: "remember stable context".to_string(),
        risk_level: RiskLevel::Medium,
        state,
        source_run_id: Some("run-1".to_string()),
    }
}

#[test]
fn proposals_create_and_fetch_by_id() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage);

    let created = repo
        .create_proposal(proposal("proposal-1", ProposalState::PendingReview))
        .unwrap();
    let fetched = repo.get_proposal("proposal-1").unwrap();

    assert_eq!(created, fetched);
    assert_eq!(fetched.state, ProposalState::PendingReview);
}

#[test]
fn proposals_list_summaries_include_inbox_fields_and_evidence_count() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage);

    repo.create_proposal(proposal("proposal-1", ProposalState::PendingReview))
        .unwrap();
    repo.create_proposal(proposal("proposal-2", ProposalState::RunFailed))
        .unwrap();

    let summaries = repo.list_summaries(ProposalFilter::default()).unwrap();

    assert_eq!(summaries.len(), 2);
    assert_eq!(summaries[0].id, "proposal-1");
    assert_eq!(summaries[0].state, ProposalState::PendingReview);
    assert_eq!(summaries[0].proposal_type, ProposalType::MemoryPatch);
    assert_eq!(summaries[0].target_section, LaputaSectionName::MemoryMd);
    assert_eq!(summaries[0].source_run_id.as_deref(), Some("run-1"));
    assert_eq!(summaries[0].risk_level, RiskLevel::Medium);
    assert_eq!(summaries[0].evidence_count, 1);

    let failed = repo
        .list_summaries(ProposalFilter {
            state: Some(ProposalState::RunFailed),
            proposal_type: None,
            target_section: None,
            source_run_id: None,
            since: None,
        })
        .unwrap();

    assert_eq!(failed.len(), 1);
    assert_eq!(failed[0].id, "proposal-2");
}

#[test]
fn proposals_edit_updates_patch_timestamp_and_preserves_evidence_by_default() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage);

    repo.create_proposal(proposal("proposal-1", ProposalState::PendingReview))
        .unwrap();

    let edited = repo
        .edit_proposal(
            "proposal-1",
            ProposalEdit {
                proposed_patch: Some("updated patch".to_string()),
                evidence_refs: None,
                risk_level: Some(RiskLevel::High),
                updated_at: ts(9),
            },
        )
        .unwrap();

    assert_eq!(edited.state, ProposalState::Edited);
    assert_eq!(edited.proposed_patch, "updated patch");
    assert_eq!(edited.updated_at, ts(9));
    assert_eq!(edited.risk_level, RiskLevel::High);
    assert_eq!(edited.evidence_refs, vec![evidence("ev-1")]);
}

#[test]
fn proposals_valid_transitions_follow_architecture_state_machine() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage);

    repo.create_proposal(proposal("proposal-1", ProposalState::PendingReview))
        .unwrap();

    assert_eq!(
        repo.transition_proposal("proposal-1", ProposalState::Approved, ts(3))
            .unwrap()
            .state,
        ProposalState::Approved
    );
    assert_eq!(
        repo.transition_proposal("proposal-1", ProposalState::Applied, ts(4))
            .unwrap()
            .state,
        ProposalState::Applied
    );
    assert_eq!(
        repo.transition_proposal("proposal-1", ProposalState::Reverted, ts(5))
            .unwrap()
            .state,
        ProposalState::Reverted
    );
}

#[test]
fn proposals_defer_and_resume_through_durable_state_machine() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage);

    repo.create_proposal(proposal("proposal-1", ProposalState::PendingReview))
        .unwrap();

    let deferred = repo
        .transition_proposal("proposal-1", ProposalState::Deferred, ts(3))
        .unwrap();
    assert_eq!(deferred.state, ProposalState::Deferred);

    let resumed = repo
        .transition_proposal("proposal-1", ProposalState::PendingReview, ts(4))
        .unwrap();
    assert_eq!(resumed.state, ProposalState::PendingReview);

    let deferred_again = repo
        .transition_proposal("proposal-1", ProposalState::Deferred, ts(5))
        .unwrap();
    assert_eq!(deferred_again.state, ProposalState::Deferred);

    let rejected = repo
        .transition_proposal("proposal-1", ProposalState::Rejected, ts(6))
        .unwrap();
    assert_eq!(rejected.state, ProposalState::Rejected);
}

#[test]
fn proposals_invalid_transition_returns_typed_error_without_mutating_state() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage);

    repo.create_proposal(proposal("proposal-1", ProposalState::PendingReview))
        .unwrap();
    repo.transition_proposal("proposal-1", ProposalState::Rejected, ts(3))
        .unwrap();
    let before = repo.get_proposal("proposal-1").unwrap();

    let error = repo
        .transition_proposal("proposal-1", ProposalState::Approved, ts(9))
        .unwrap_err();
    let after = repo.get_proposal("proposal-1").unwrap();

    assert!(matches!(
        error,
        LaputaError::InvalidProposalTransition {
            from: ProposalState::Rejected,
            to: ProposalState::Approved,
        }
    ));
    assert_eq!(before, after);
}

#[test]
fn proposals_invalid_create_rejects_unsafe_initial_state_and_bad_target_route() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage);

    let unsafe_error = repo
        .create_proposal(proposal("proposal-1", ProposalState::Approved))
        .unwrap_err();
    assert!(matches!(unsafe_error, LaputaError::InvalidProposal { .. }));

    let mut bad_target = proposal("proposal-2", ProposalState::PendingReview);
    bad_target.target_section = LaputaSectionName::Identity;
    let target_error = repo.create_proposal(bad_target).unwrap_err();
    assert!(matches!(target_error, LaputaError::InvalidProposal { .. }));

    let unknown = route_proposal_type("not_a_v1_type").unwrap_err();
    assert_eq!(
        unknown,
        EvolutionError::UnknownProposalType("not_a_v1_type".to_string())
    );
}
