use std::fs;

use agent_diva_core::evolution::{
    EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState, ProposalType,
    RiskLevel,
};
use agent_diva_laputa::{
    ChangelogFilter, LaputaEventKind, LaputaService, ProposalFilter, RollbackChangelogRequest,
    SectionStatus,
};
use chrono::{DateTime, Utc};

fn ts(seconds: u32) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&format!("2026-06-14T00:02:{seconds:02}Z"))
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

fn proposal(id: &str) -> EvolutionProposal {
    EvolutionProposal {
        id: id.to_string(),
        created_at: ts(2),
        updated_at: ts(2),
        created_by: "autodream".to_string(),
        proposal_type: ProposalType::MemoryPatch,
        target_section: LaputaSectionName::MemoryMd,
        evidence_refs: vec![evidence("ev-1")],
        proposed_patch: r#"{"items":["new"]}"#.to_string(),
        risk_level: RiskLevel::Medium,
        state: ProposalState::PendingReview,
        source_run_id: Some("run-1".to_string()),
    }
}

#[test]
fn snapshot_lists_all_sections_and_marks_tbd_sections() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();

    let snapshot = service.read_snapshot(None).unwrap();

    assert_eq!(snapshot.sections.len(), 14);
    assert_eq!(
        snapshot.sections["journal_reflective"].status,
        SectionStatus::Tbd
    );
    assert_eq!(snapshot.sections["memory_md"].status, SectionStatus::Owned);
}

#[test]
fn section_reads_return_explicit_tbd_status_without_file() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();

    let section = service
        .read_section(LaputaSectionName::ProposalInbox)
        .unwrap();

    assert_eq!(section.status, SectionStatus::Tbd);
    assert!(section.content.is_null());
}

#[test]
fn apply_changelog_rollback_and_polling_events_are_available() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();
    fs::write(
        temp.path().join(".laputa/sections/memory_md.json"),
        r#"{"items":["old"]}"#,
    )
    .unwrap();

    service.create_proposal(proposal("proposal-1")).unwrap();
    service
        .transition_proposal("proposal-1", ProposalState::Approved, ts(3))
        .unwrap();
    let outcome = service
        .apply_proposal("proposal-1", "reviewer", ts(4))
        .unwrap();

    let proposals = service.list_proposals(ProposalFilter::default()).unwrap();
    assert_eq!(proposals[0].state, ProposalState::Applied);

    let changelog = service.get_changelog(&outcome.changelog.id).unwrap();
    assert_eq!(changelog.before, r#"{"items":["old"]}"#);
    assert_eq!(
        service
            .list_changelog(ChangelogFilter::default())
            .unwrap()
            .total,
        1
    );

    let proposal_events = service
        .poll_events(Some(LaputaEventKind::Proposal), Some(ts(2)))
        .unwrap();
    assert!(proposal_events
        .iter()
        .any(|event| event.proposal_id.as_deref() == Some("proposal-1")));
    let changelog_events = service
        .poll_events(Some(LaputaEventKind::Changelog), Some(ts(4)))
        .unwrap();
    assert_eq!(changelog_events.len(), 1);

    let rollback = service
        .rollback_changelog(
            &outcome.changelog.id,
            RollbackChangelogRequest {
                reason: "undo".to_string(),
                expected_current: None,
            },
            "reviewer",
            ts(5),
        )
        .unwrap();

    assert_eq!(
        rollback.changelog.action,
        agent_diva_core::evolution::ChangelogAction::Rollback
    );
    assert_eq!(
        fs::read_to_string(temp.path().join(".laputa/sections/memory_md.json")).unwrap(),
        r#"{"items":["old"]}"#
    );
}
