use std::{fs, time::Duration};

use agent_diva_core::evolution::{
    AuditEvent, AuditEventKind, ChangelogAction, ChangelogRecord, EvidenceRef, EvidenceSource,
    EvolutionProposal, LaputaSectionName, ProposalState, ProposalType, RiskLevel, RollbackRequest,
};
use agent_diva_laputa::{
    ApplyFailurePoint, ApplyOptions, LaputaError, LaputaLock, LaputaStorage, LockOptions,
    ProposalRepository,
};
use chrono::{DateTime, Utc};
use serde_json::json;

fn ts(seconds: u32) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&format!("2026-06-14T00:01:{seconds:02}Z"))
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

fn proposal(id: &str, proposal_type: ProposalType, state: ProposalState) -> EvolutionProposal {
    EvolutionProposal {
        id: id.to_string(),
        created_at: ts(2),
        updated_at: ts(2),
        created_by: "autodream".to_string(),
        target_section: proposal_type.target_section(),
        proposal_type,
        evidence_refs: vec![evidence("ev-1")],
        proposed_patch: r#"{"items":["remember stable context"]}"#.to_string(),
        risk_level: RiskLevel::Medium,
        state,
        source_run_id: Some("run-1".to_string()),
    }
}

#[test]
fn apply_success_writes_authority_changelog_audit_rollback_and_applied_status() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage.clone());
    let section_path = storage.paths().section_file(LaputaSectionName::MemoryMd);
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    repo.create_proposal(proposal(
        "proposal-1",
        ProposalType::MemoryPatch,
        ProposalState::PendingReview,
    ))
    .unwrap();
    repo.transition_proposal("proposal-1", ProposalState::Approved, ts(3))
        .unwrap();

    let outcome = repo
        .apply_proposal("proposal-1", "reviewer", ts(4))
        .unwrap();

    assert_eq!(outcome.proposal.state, ProposalState::Applied);
    let section_json: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(section_path).unwrap()).unwrap();
    assert_eq!(section_json, json!({"items": ["remember stable context"]}));
    assert!(storage
        .paths()
        .rollback_dir()
        .join(format!("{}.json", outcome.rollback_request.changelog_id))
        .exists());

    let changelog: ChangelogRecord = serde_json::from_slice(
        &fs::read(
            storage
                .paths()
                .changelog_dir()
                .join(format!("{}.json", outcome.changelog.id)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(changelog.action, ChangelogAction::Apply);
    assert_eq!(changelog.before, r#"{"items":["old"]}"#);
    assert_eq!(changelog.after, r#"{"items":["remember stable context"]}"#);
    assert_eq!(changelog.proposal_id.as_deref(), Some("proposal-1"));

    let audit: AuditEvent = serde_json::from_slice(
        &fs::read(
            storage
                .paths()
                .audit_dir()
                .join(format!("{}.json", outcome.audit_event.id)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(audit.kind, AuditEventKind::ProposalApplied);
    assert_eq!(audit.proposal_id.as_deref(), Some("proposal-1"));

    let rollback: RollbackRequest = serde_json::from_slice(
        &fs::read(
            storage
                .paths()
                .rollback_dir()
                .join(format!("{}.json", outcome.rollback_request.changelog_id)),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(rollback.changelog_id, outcome.changelog.id);
}

#[test]
fn apply_rejects_non_approved_proposal_without_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage.clone());
    let section_path = storage.paths().section_file(LaputaSectionName::MemoryMd);
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    repo.create_proposal(proposal(
        "proposal-1",
        ProposalType::MemoryPatch,
        ProposalState::PendingReview,
    ))
    .unwrap();

    let error = repo
        .apply_proposal("proposal-1", "reviewer", ts(4))
        .unwrap_err();

    assert!(matches!(
        error,
        LaputaError::InvalidProposalTransition {
            from: ProposalState::PendingReview,
            to: ProposalState::Applied
        }
    ));
    assert_eq!(
        fs::read_to_string(section_path).unwrap(),
        r#"{"items":["old"]}"#
    );
}

#[test]
fn apply_rejects_unauthorized_target_and_schema_mismatch_without_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage.clone());

    let mut unauthorized = proposal(
        "proposal-1",
        ProposalType::MemoryPatch,
        ProposalState::Approved,
    );
    unauthorized.target_section = LaputaSectionName::ProposalInbox;
    fs::write(
        storage.paths().proposals_dir().join("proposal-1.json"),
        serde_json::to_vec_pretty(&unauthorized).unwrap(),
    )
    .unwrap();
    let error = repo
        .apply_proposal("proposal-1", "reviewer", ts(4))
        .unwrap_err();
    assert!(matches!(error, LaputaError::UnauthorizedTarget { .. }));

    let mut schema_mismatch = proposal(
        "proposal-2",
        ProposalType::MemoryPatch,
        ProposalState::PendingReview,
    );
    schema_mismatch.proposed_patch = "not-json".to_string();
    repo.create_proposal(schema_mismatch).unwrap();
    repo.transition_proposal("proposal-2", ProposalState::Approved, ts(3))
        .unwrap();

    let error = repo
        .apply_proposal("proposal-2", "reviewer", ts(4))
        .unwrap_err();
    assert!(matches!(error, LaputaError::SchemaIncompatible { .. }));
    assert!(!storage
        .paths()
        .section_file(LaputaSectionName::MemoryMd)
        .exists());
}

#[test]
fn apply_rejects_unresolved_conflicts_with_recovery_status_without_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage.clone());
    let section_path = storage.paths().section_file(LaputaSectionName::MemoryMd);
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    let mut conflicted = proposal(
        "proposal-1",
        ProposalType::MemoryPatch,
        ProposalState::PendingReview,
    );
    conflicted.proposed_patch =
        r#"{"items":["new"],"conflicts":["memory_md changed since proposal"]}"#.to_string();
    repo.create_proposal(conflicted).unwrap();
    repo.transition_proposal("proposal-1", ProposalState::Approved, ts(3))
        .unwrap();

    let error = repo
        .apply_proposal("proposal-1", "reviewer", ts(4))
        .unwrap_err();

    assert!(matches!(error, LaputaError::UnresolvedConflict { .. }));
    assert_eq!(
        repo.get_proposal("proposal-1").unwrap().state,
        ProposalState::NeedsAttention
    );
    assert_eq!(
        fs::read_to_string(section_path).unwrap(),
        r#"{"items":["old"]}"#
    );
}

#[test]
fn apply_deprecation_appends_records_without_mutating_authority_section() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage.clone());
    let changelog_section = storage.paths().section_file(LaputaSectionName::Changelog);

    let mut proposal = proposal(
        "proposal-1",
        ProposalType::Deprecation,
        ProposalState::PendingReview,
    );
    proposal.proposed_patch = r#"{"deprecated":"old-memory"}"#.to_string();
    repo.create_proposal(proposal).unwrap();
    repo.transition_proposal("proposal-1", ProposalState::Approved, ts(3))
        .unwrap();

    let outcome = repo
        .apply_proposal("proposal-1", "reviewer", ts(4))
        .unwrap();

    assert_eq!(outcome.proposal.state, ProposalState::Applied);
    assert!(!changelog_section.exists());
    assert!(storage
        .paths()
        .changelog_dir()
        .join(format!("{}.json", outcome.changelog.id))
        .exists());
}

#[test]
fn apply_recovery_rolls_back_section_when_changelog_write_fails_after_section_write() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage.clone());
    let section_path = storage.paths().section_file(LaputaSectionName::MemoryMd);
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    repo.create_proposal(proposal(
        "proposal-1",
        ProposalType::MemoryPatch,
        ProposalState::PendingReview,
    ))
    .unwrap();
    repo.transition_proposal("proposal-1", ProposalState::Approved, ts(3))
        .unwrap();

    let error = repo
        .apply_proposal_with_options(
            "proposal-1",
            "reviewer",
            ts(4),
            ApplyOptions {
                failure_point: Some(ApplyFailurePoint::AfterSectionWriteBeforeChangelog),
            },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        LaputaError::InjectedApplyFailure {
            point: ApplyFailurePoint::AfterSectionWriteBeforeChangelog
        }
    ));
    assert_eq!(
        fs::read_to_string(section_path).unwrap(),
        r#"{"items":["old"]}"#
    );
    assert_eq!(
        repo.get_proposal("proposal-1").unwrap().state,
        ProposalState::NeedsAttention
    );
    assert_eq!(
        fs::read_dir(storage.paths().rollback_dir())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
            .count(),
        0
    );
    assert_eq!(
        fs::read_dir(storage.paths().changelog_dir())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
            .count(),
        0
    );
}

#[test]
fn apply_recovery_cleans_changelog_when_audit_fails_after_changelog_write() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage.clone());
    let section_path = storage.paths().section_file(LaputaSectionName::MemoryMd);
    fs::write(&section_path, r#"{"items":["old"]}"#).unwrap();

    repo.create_proposal(proposal(
        "proposal-1",
        ProposalType::MemoryPatch,
        ProposalState::PendingReview,
    ))
    .unwrap();
    repo.transition_proposal("proposal-1", ProposalState::Approved, ts(3))
        .unwrap();

    let error = repo
        .apply_proposal_with_options(
            "proposal-1",
            "reviewer",
            ts(4),
            ApplyOptions {
                failure_point: Some(ApplyFailurePoint::AfterChangelogBeforeAudit),
            },
        )
        .unwrap_err();

    assert!(matches!(
        error,
        LaputaError::InjectedApplyFailure {
            point: ApplyFailurePoint::AfterChangelogBeforeAudit
        }
    ));
    assert_eq!(
        fs::read_to_string(section_path).unwrap(),
        r#"{"items":["old"]}"#
    );
    assert_eq!(
        fs::read_dir(storage.paths().rollback_dir())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
            .count(),
        0
    );
    assert_eq!(
        fs::read_dir(storage.paths().changelog_dir())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
            .count(),
        0
    );
}

#[test]
fn apply_allows_raw_tbd_section_writes() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let repo = ProposalRepository::new(storage.clone());

    let mut proposal = proposal(
        "proposal-1",
        ProposalType::JournalNote,
        ProposalState::PendingReview,
    );
    proposal.proposed_patch = "raw reflective note".to_string();
    repo.create_proposal(proposal).unwrap();
    repo.transition_proposal("proposal-1", ProposalState::Approved, ts(3))
        .unwrap();

    repo.apply_proposal("proposal-1", "reviewer", ts(4))
        .unwrap();

    assert_eq!(
        fs::read_to_string(
            storage
                .paths()
                .section_file(LaputaSectionName::JournalReflective)
        )
        .unwrap(),
        "raw reflective note"
    );
}

#[test]
fn apply_honors_lock_acquisition_timeout() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let _guard = LaputaLock::acquire(
        storage.paths().lock_file("proposals"),
        LockOptions::default(),
    )
    .unwrap();
    let repo = ProposalRepository::with_lock_options(
        storage,
        LockOptions {
            timeout: Duration::from_millis(20),
            stale_after: Duration::from_secs(60),
            retry_interval: Duration::from_millis(5),
        },
    );

    let error = repo
        .apply_proposal("missing", "reviewer", ts(4))
        .unwrap_err();

    assert!(matches!(error, LaputaError::LockTimeout { .. }));
}
