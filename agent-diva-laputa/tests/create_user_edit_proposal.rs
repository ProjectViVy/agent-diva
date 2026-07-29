use std::fs;

use agent_diva_core::evolution::{
    EvidenceSource, LaputaSectionName, ProposalState, ProposalType, RiskLevel,
};
use agent_diva_laputa::LaputaService;
use chrono::Utc;

#[test]
fn user_edit_creates_pending_proposal_without_mutating_authority() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();
    let section_path = temp.path().join(".laputa/sections/memory_md.json");
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    let now = Utc::now();
    let proposal = service
        .create_user_edit_proposal(
            LaputaSectionName::MemoryMd,
            r#"{"items":["new"]}"#,
            "user",
            Some("Remember the new item".to_string()),
            now,
        )
        .unwrap();

    assert_eq!(proposal.state, ProposalState::PendingReview);
    assert_eq!(proposal.proposal_type, ProposalType::MemoryPatch);
    assert_eq!(proposal.risk_level, RiskLevel::Medium);
    assert_eq!(proposal.created_by, "user");
    assert_eq!(proposal.evidence_refs.len(), 1);
    assert_eq!(proposal.evidence_refs[0].source, EvidenceSource::UserInput);
    assert_eq!(
        proposal.evidence_refs[0].excerpt.as_deref(),
        Some("Remember the new item")
    );
    assert_eq!(
        fs::read_to_string(&section_path).unwrap(),
        r#"{"items":["old"]}"#
    );

    assert!(fs::read_dir(temp.path().join(".laputa/changelog"))
        .unwrap()
        .next()
        .is_none());
    assert!(fs::read_dir(temp.path().join(".laputa/audit"))
        .unwrap()
        .next()
        .is_none());
    assert!(fs::read_dir(temp.path().join(".laputa/rollback"))
        .unwrap()
        .next()
        .is_none());
}

#[test]
fn user_edit_rejects_non_json_patch_for_json_section() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();
    let section_path = temp.path().join(".laputa/sections/memory_md.json");
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    let error = service
        .create_user_edit_proposal(
            LaputaSectionName::MemoryMd,
            "not-json",
            "user",
            None,
            Utc::now(),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        agent_diva_laputa::LaputaError::SchemaIncompatible { .. }
    ));
    assert_eq!(
        fs::read_to_string(&section_path).unwrap(),
        r#"{"items":["old"]}"#
    );
}

#[test]
fn user_edit_rejects_non_writable_section() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();

    let error = service
        .create_user_edit_proposal(
            LaputaSectionName::ProposalInbox,
            r#"{"items":["new"]}"#,
            "user",
            None,
            Utc::now(),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        agent_diva_laputa::LaputaError::UnauthorizedTarget { .. }
    ));
}

#[test]
fn user_edit_risk_mapping_is_fail_closed_by_section() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();
    let cases = [
        (
            LaputaSectionName::Identity,
            ProposalType::IdentityPatch,
            RiskLevel::High,
        ),
        (
            LaputaSectionName::Relationship,
            ProposalType::RelationshipUpdate,
            RiskLevel::High,
        ),
        (
            LaputaSectionName::Commitment,
            ProposalType::CommitmentSet,
            RiskLevel::High,
        ),
        (
            LaputaSectionName::Changelog,
            ProposalType::Deprecation,
            RiskLevel::Critical,
        ),
        (
            LaputaSectionName::Preferences,
            ProposalType::LearningNote,
            RiskLevel::Medium,
        ),
        (
            LaputaSectionName::HistoryMd,
            ProposalType::HistoryPatch,
            RiskLevel::Low,
        ),
    ];

    for (section, expected_type, expected_risk) in cases {
        let proposal = service
            .create_user_edit_proposal(section, "{}", "user", None, Utc::now())
            .unwrap();
        assert_eq!(proposal.proposal_type, expected_type);
        assert_eq!(proposal.risk_level, expected_risk);
        assert_eq!(proposal.state, ProposalState::PendingReview);
    }
}
