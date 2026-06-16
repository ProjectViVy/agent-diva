use std::fs;

use agent_diva_core::evolution::{
    AuditEvent, AuditEventKind, ChangelogAction, EvidenceRef, EvidenceSource, EvolutionProposal,
    LaputaSectionName, ProposalState, ProposalType, RiskLevel,
};
use agent_diva_laputa::{
    ChangelogFilter, LaputaEventKind, LaputaService, RollbackChangelogRequest, SectionStatus,
};
use chrono::{DateTime, Utc};

fn ts(seconds: u32) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&format!("2026-06-15T00:04:{seconds:02}Z"))
        .unwrap()
        .with_timezone(&Utc)
}

fn evidence(id: &str) -> EvidenceRef {
    EvidenceRef {
        id: id.to_string(),
        source: EvidenceSource::Session,
        uri: format!("session://{id}"),
        excerpt: Some("bounded governance evidence".to_string()),
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
        proposed_patch: r#"{"items":["remember governed authority only"]}"#.to_string(),
        risk_level: RiskLevel::Medium,
        state: ProposalState::PendingReview,
        source_run_id: Some("run-proof-loop".to_string()),
    }
}

#[test]
fn governance_proof_loop() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path();
    let service = LaputaService::open(workspace).unwrap();
    let section_path = workspace.join(".laputa/sections/memory_md.json");
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    let created = service.create_proposal(proposal("proposal-proof")).unwrap();
    assert_eq!(created.state, ProposalState::PendingReview);

    let approved = service
        .transition_proposal("proposal-proof", ProposalState::Approved, ts(3))
        .unwrap();
    assert_eq!(approved.state, ProposalState::Approved);

    let apply = service
        .apply_proposal("proposal-proof", "reviewer", ts(4))
        .unwrap();
    assert_eq!(apply.proposal.state, ProposalState::Applied);
    let applied_section: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&section_path).unwrap()).unwrap();
    assert_eq!(
        applied_section,
        serde_json::json!({"items": ["remember governed authority only"]})
    );
    assert_eq!(apply.changelog.action, ChangelogAction::Apply);

    let snapshot = service.read_snapshot(None).unwrap();
    let memory_section = &snapshot.sections["memory_md"];
    assert_eq!(memory_section.status, SectionStatus::Owned);
    assert_eq!(
        memory_section.content,
        serde_json::json!({"items": ["remember governed authority only"]})
    );

    let changelog_page = service
        .list_changelog(ChangelogFilter {
            proposal_id: Some("proposal-proof".to_string()),
            ..ChangelogFilter::default()
        })
        .unwrap();
    assert_eq!(changelog_page.total, 1);
    assert_eq!(changelog_page.items[0].id, apply.changelog.id);

    let apply_audit: AuditEvent = serde_json::from_slice(
        &fs::read(
            workspace
                .join(".laputa/audit")
                .join(format!("{}.json", apply.audit_event.id)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(apply_audit.kind, AuditEventKind::ProposalApplied);

    let rollback = service
        .rollback_changelog(
            &apply.changelog.id,
            RollbackChangelogRequest {
                reason: "prove rollback".to_string(),
                expected_current: Some(apply.changelog.after.clone()),
            },
            "reviewer",
            ts(5),
        )
        .unwrap();
    assert_eq!(rollback.changelog.action, ChangelogAction::Rollback);
    let rolled_back_section: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&section_path).unwrap()).unwrap();
    assert_eq!(rolled_back_section, serde_json::json!({"items": ["old"]}));

    let reverted = service.get_changelog(&apply.changelog.id).unwrap();
    assert!(reverted.reverted);

    let proposals = service.list_proposals(Default::default()).unwrap();
    assert_eq!(proposals.len(), 1);
    assert_eq!(proposals[0].state, ProposalState::Reverted);

    let changelog_events = service
        .poll_events(Some(LaputaEventKind::Changelog), None)
        .unwrap();
    assert!(changelog_events.iter().any(|event| {
        event.changelog_id.as_deref() == Some(apply.changelog.id.as_str())
            && event.action.as_deref() == Some("apply")
    }));
    assert!(changelog_events.iter().any(|event| {
        event.changelog_id.as_deref() == Some(rollback.changelog.id.as_str())
            && event.action.as_deref() == Some("rollback")
    }));

    let rollback_audit: AuditEvent = serde_json::from_slice(
        &fs::read(
            workspace
                .join(".laputa/audit")
                .join(format!("{}.json", rollback.audit_event.id)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(rollback_audit.kind, AuditEventKind::RollbackApplied);

    assert_no_direct_authority_writes_outside_laputa(workspace);
}

fn assert_no_direct_authority_writes_outside_laputa(workspace: &std::path::Path) {
    let forbidden = [
        workspace.join("MEMORY.md"),
        workspace.join("memory").join("MEMORY.md"),
        workspace.join("IDENTITY.md"),
        workspace.join("SOUL.md"),
        workspace.join("proposal_inbox.json"),
    ];

    for path in forbidden {
        assert!(
            !path.exists(),
            "proof loop must not write authority file outside Laputa: {}",
            path.display()
        );
    }
}
