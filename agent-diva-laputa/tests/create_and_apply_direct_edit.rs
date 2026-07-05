use std::fs;

use agent_diva_core::evolution::{
    ChangelogAction, EvidenceSource, LaputaSectionName, ProposalState, ProposalType,
};
use agent_diva_laputa::LaputaService;
use chrono::Utc;
use serde_json::json;

#[test]
fn direct_edit_success_applies_patch_and_writes_changelog_audit_rollback() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();
    let section_path = temp.path().join(".laputa/sections/memory_md.json");
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    let now = Utc::now();
    let outcome = service
        .create_and_apply_direct_edit(
            LaputaSectionName::MemoryMd,
            r#"{"items":["new"]}"#,
            "user",
            now,
        )
        .unwrap();

    assert_eq!(outcome.changelog.action, ChangelogAction::Apply);

    let section = service.read_section(LaputaSectionName::MemoryMd).unwrap();
    assert_eq!(section.content, json!({"items": ["new"]}));

    let proposal = service.get_proposal(&outcome.proposal.id).unwrap();
    assert_eq!(proposal.state, ProposalState::Applied);
    assert_eq!(proposal.proposal_type, ProposalType::MemoryPatch);
    assert_eq!(proposal.created_by, "user");
    assert_eq!(proposal.evidence_refs.len(), 1);
    assert_eq!(proposal.evidence_refs[0].source, EvidenceSource::UserInput);

    assert!(temp
        .path()
        .join(format!(".laputa/changelog/{}.json", outcome.changelog.id))
        .exists());
    assert!(temp
        .path()
        .join(format!(".laputa/audit/{}.json", outcome.audit_event.id))
        .exists());
    assert!(temp
        .path()
        .join(format!(
            ".laputa/rollback/{}.json",
            outcome.rollback_request.changelog_id
        ))
        .exists());
}

#[test]
fn direct_edit_rejects_non_json_patch_for_json_section() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();
    let section_path = temp.path().join(".laputa/sections/memory_md.json");
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    let error = service
        .create_and_apply_direct_edit(LaputaSectionName::MemoryMd, "not-json", "user", Utc::now())
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
fn direct_edit_rejects_non_writable_section() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();

    let error = service
        .create_and_apply_direct_edit(
            LaputaSectionName::ProposalInbox,
            r#"{"items":["new"]}"#,
            "user",
            Utc::now(),
        )
        .unwrap_err();

    assert!(matches!(
        error,
        agent_diva_laputa::LaputaError::UnauthorizedTarget { .. }
    ));
}
