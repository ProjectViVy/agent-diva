use agent_diva_core::{
    evolution::{
        EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState,
        ProposalType, RiskLevel,
    },
    governance::AuditCorrelation,
    memory::{
        memory_content_digest, MemoryProvenance, MemoryProvenanceSource, MemoryProvider,
        MemoryRecord, MemoryRecordKind, MemoryScope, MemorySensitivity, MemoryTrust,
        PrefetchRequest, RecallOutcomeRequest, RecallTurnOutcome,
    },
};
use agent_diva_laputa::{
    adapt_governed_proposal, LaputaStorage, MemoryAdapterContext, RecallFeedbackStore,
    RecallTaskOutcome, TypedLaputaMemoryProvider, TypedMemoryStore,
};
use chrono::Utc;

fn record(content: &str) -> MemoryRecord {
    let now = Utc::now();
    MemoryRecord {
        id: "record-1".to_string(),
        kind: MemoryRecordKind::LongTerm,
        content: content.to_string(),
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::AutoDream,
            source_id: "run-1".to_string(),
            content_digest: memory_content_digest(content.as_bytes()),
            captured_at: now,
            correlation: AuditCorrelation {
                request_id: "request-1".to_string(),
                turn_id: "turn-1".to_string(),
                session_id: "session-1".to_string(),
                trace_id: Some("run-1".to_string()),
            },
        },
        evidence_refs: Vec::new(),
        confidence_bps: 8_000,
        sensitivity: MemorySensitivity::Private,
        trust: MemoryTrust::AppliedAuthority,
        scope: MemoryScope {
            tenant_id: "local".to_string(),
            workspace_id: "workspace-1".to_string(),
            session_id: None,
        },
        created_at: now,
        effective_at: now,
        expires_at: None,
        supersedes: Vec::new(),
        tombstone: None,
    }
}

#[test]
fn governed_autodream_proposal_preserves_run_and_evidence_provenance() {
    let now = Utc::now();
    let proposal = EvolutionProposal {
        id: "proposal-1".to_string(),
        created_at: now,
        updated_at: now,
        created_by: "autodream".to_string(),
        proposal_type: ProposalType::LearningNote,
        target_section: LaputaSectionName::Preferences,
        evidence_refs: vec![EvidenceRef {
            id: "experience-1".to_string(),
            source: EvidenceSource::ExperienceJournal,
            uri: "experience://experience-1".to_string(),
            excerpt: None,
            hash: Some("sha256:evidence".to_string()),
            created_at: now,
        }],
        proposed_patch: "The user prefers concise release summaries.".to_string(),
        risk_level: RiskLevel::Low,
        state: ProposalState::Approved,
        source_run_id: Some("run-1".to_string()),
    };
    let record = adapt_governed_proposal(
        &proposal,
        &MemoryAdapterContext {
            tenant_id: "local".to_string(),
            workspace_id: "workspace-1".to_string(),
            session_id: None,
            correlation: AuditCorrelation {
                request_id: "request-1".to_string(),
                turn_id: "turn-1".to_string(),
                session_id: "session-1".to_string(),
                trace_id: Some("run-1".to_string()),
            },
            captured_at: now,
        },
    );

    assert_eq!(record.provenance.source, MemoryProvenanceSource::AutoDream);
    assert_eq!(record.provenance.source_id, "run-1");
    assert_eq!(record.evidence_refs, proposal.evidence_refs);
}

#[tokio::test]
async fn typed_prefetch_feedback_commits_only_after_terminal_turn_outcome() {
    let temp = tempfile::tempdir().unwrap();
    let store = TypedMemoryStore::open(temp.path(), "workspace-1")
        .await
        .unwrap();
    let content = "The user prefers concise release summaries.";
    store.put(record(content), 0, None).await.unwrap();
    drop(store);
    let provider = TypedLaputaMemoryProvider::open(temp.path(), "workspace-1")
        .await
        .unwrap();
    let response = provider
        .prefetch(PrefetchRequest {
            workspace_root: temp.path().to_path_buf(),
            intent: "concise release summaries".to_string(),
            current_room: None,
            user_message: None,
        })
        .await
        .unwrap();
    assert!(response.prompt_block.is_some());
    let feedback = RecallFeedbackStore::new(LaputaStorage::open(temp.path()).unwrap());
    assert!(feedback.recent(10).unwrap().is_empty());

    provider
        .record_recall_outcome(RecallOutcomeRequest {
            workspace_root: temp.path().to_path_buf(),
            request_id: "turn-1".to_string(),
            outcome: RecallTurnOutcome::Succeeded,
            corrected: false,
        })
        .await
        .unwrap();

    let events = feedback.recent(10).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].record_id, "record-1");
    assert!(events[0].selected);
    assert!(events[0].injected);
    assert!(!events[0].corrected);
    assert_eq!(events[0].task_outcome, RecallTaskOutcome::Succeeded);
    let raw = std::fs::read_to_string(
        LaputaStorage::open(temp.path())
            .unwrap()
            .paths()
            .recall_feedback_json(),
    )
    .unwrap();
    assert!(!raw.contains(content));
}

#[tokio::test]
async fn failed_corrected_turn_is_payload_free_and_drives_tombstone_shape() {
    let temp = tempfile::tempdir().unwrap();
    let store = TypedMemoryStore::open(temp.path(), "workspace-1")
        .await
        .unwrap();
    let content = "A stale preference that must never enter feedback.";
    store.put(record(content), 0, None).await.unwrap();
    drop(store);
    let provider = TypedLaputaMemoryProvider::open(temp.path(), "workspace-1")
        .await
        .unwrap();
    provider
        .prefetch(PrefetchRequest {
            workspace_root: temp.path().to_path_buf(),
            intent: "stale preference".to_string(),
            current_room: None,
            user_message: None,
        })
        .await
        .unwrap();
    provider
        .record_recall_outcome(RecallOutcomeRequest {
            workspace_root: temp.path().to_path_buf(),
            request_id: "turn-failed".to_string(),
            outcome: RecallTurnOutcome::Failed,
            corrected: true,
        })
        .await
        .unwrap();

    let feedback = RecallFeedbackStore::new(LaputaStorage::open(temp.path()).unwrap());
    let events = feedback.recent(10).unwrap();
    assert_eq!(events.len(), 1);
    assert!(events[0].corrected);
    assert_eq!(events[0].task_outcome, RecallTaskOutcome::Failed);
    let raw = std::fs::read_to_string(
        LaputaStorage::open(temp.path())
            .unwrap()
            .paths()
            .recall_feedback_json(),
    )
    .unwrap();
    assert!(!raw.contains(content));
}

#[test]
fn governed_deprecation_adapts_to_content_free_tombstone() {
    let now = Utc::now();
    let proposal = EvolutionProposal {
        id: "proposal-deprecate-1".to_string(),
        created_at: now,
        updated_at: now,
        created_by: "autodream".to_string(),
        proposal_type: ProposalType::Deprecation,
        target_section: LaputaSectionName::Changelog,
        evidence_refs: vec![EvidenceRef {
            id: "feedback-1".to_string(),
            source: EvidenceSource::RecallFeedback,
            uri: "recall-feedback://feedback-1".to_string(),
            excerpt: None,
            hash: Some("sha256:evidence".to_string()),
            created_at: now,
        }],
        proposed_patch: serde_json::json!({
            "schema_version": 1,
            "target_record_id": "record-1",
            "reason": "user_correction"
        })
        .to_string(),
        risk_level: RiskLevel::Medium,
        state: ProposalState::Approved,
        source_run_id: Some("run-1".to_string()),
    };
    let record = adapt_governed_proposal(
        &proposal,
        &MemoryAdapterContext {
            tenant_id: "local".to_string(),
            workspace_id: "workspace-1".to_string(),
            session_id: None,
            correlation: AuditCorrelation {
                request_id: "request-1".to_string(),
                turn_id: "turn-1".to_string(),
                session_id: "session-1".to_string(),
                trace_id: Some("run-1".to_string()),
            },
            captured_at: now,
        },
    );
    assert!(record.content.is_empty());
    assert_eq!(record.supersedes, vec!["record-1"]);
    assert_eq!(
        record.tombstone.as_ref().unwrap().target_record_id,
        "record-1"
    );
}
