use std::fs;

use agent_diva_core::evolution::{
    EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState, ProposalType,
    RiskLevel,
};
use agent_diva_laputa::{LaputaEventKind, LaputaService, RollbackChangelogRequest};
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

fn assert_no_mentle_state(workspace: &std::path::Path) {
    assert!(!workspace.join("memory/palace.db").exists());
    assert!(!workspace.join(".mentle").exists());
}

#[test]
fn proposal_apply_audit_and_rollback_do_not_create_mentle_state() {
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
    let audit_events = service
        .poll_events(Some(LaputaEventKind::Changelog), Some(ts(4)))
        .unwrap();
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

    assert!(!audit_events.is_empty());
    assert_eq!(
        rollback.changelog.action,
        agent_diva_core::evolution::ChangelogAction::Rollback
    );
    assert_no_mentle_state(temp.path());
}

#[test]
fn laputa_governance_crate_does_not_depend_on_mentle() {
    let manifest = include_str!("../Cargo.toml");

    assert!(!manifest.contains("mentle"));
    assert!(!manifest.contains("memtle"));
}
