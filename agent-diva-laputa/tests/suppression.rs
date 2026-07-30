use agent_diva_core::evolution::{
    memory_candidate_content_digest, EvidenceRef, EvidenceSource, EvolutionProposal,
    LaputaSectionName, ProposalState, ProposalType, RiskLevel,
};
use agent_diva_laputa::{CandidateSuppressionStore, LaputaService, LaputaStorage};
use chrono::{Duration, TimeZone, Utc};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 31, 1, 0, 0).unwrap()
}

fn proposal(id: &str, content: &str, updated_at: chrono::DateTime<Utc>) -> EvolutionProposal {
    EvolutionProposal {
        id: id.to_string(),
        created_at: updated_at,
        updated_at,
        created_by: "autodream".to_string(),
        proposal_type: ProposalType::LearningNote,
        target_section: LaputaSectionName::Preferences,
        evidence_refs: vec![EvidenceRef {
            id: "evidence-1".to_string(),
            source: EvidenceSource::ExperienceJournal,
            uri: "experience://evidence-1".to_string(),
            excerpt: None,
            hash: Some("sha256:evidence".to_string()),
            created_at: updated_at,
        }],
        proposed_patch: content.to_string(),
        risk_level: RiskLevel::Low,
        state: ProposalState::PendingReview,
        source_run_id: Some("run-1".to_string()),
    }
}

#[test]
fn rejected_proposal_creates_payload_free_bounded_suppression() {
    let temp = tempfile::tempdir().unwrap();
    let service = LaputaService::open(temp.path()).unwrap();
    let content = "The user prefers concise release summaries.";
    let created = service
        .create_proposal(proposal("proposal-1", content, now()))
        .unwrap();
    service
        .transition_proposal(&created.id, ProposalState::Rejected, now())
        .unwrap();

    let reopened = LaputaService::open(temp.path()).unwrap();
    let digests = reopened
        .active_candidate_suppression_digests(now() + Duration::days(1))
        .unwrap();
    assert!(digests.contains(&memory_candidate_content_digest(content)));
    let suppression_path = LaputaStorage::open(temp.path())
        .unwrap()
        .paths()
        .suppression_json();
    let raw = std::fs::read_to_string(suppression_path).unwrap();
    assert!(!raw.contains(content));
    assert!(raw.contains("proposal-1"));
}

#[test]
fn suppression_is_exact_and_expires() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let store = CandidateSuppressionStore::new(storage);
    let content = "The user prefers concise release summaries.";
    store
        .record_rejection(&proposal("proposal-1", content, now()), now())
        .unwrap();

    assert!(store.is_suppressed(content, now()).unwrap());
    assert!(!store
        .is_suppressed(
            "The user prefers concise release summaries with verification IDs.",
            now(),
        )
        .unwrap());
    assert!(!store
        .is_suppressed(content, now() + Duration::days(91))
        .unwrap());
}

#[test]
fn corrupt_suppression_store_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    std::fs::write(storage.paths().suppression_json(), b"{broken").unwrap();
    let store = CandidateSuppressionStore::new(storage);

    assert!(store.active_digests(now()).is_err());
}
