use agent_diva_autodream::{
    BoundedReflectionInput, DeterministicReflectionEngine, ReflectionEngine, ReflectionEvidence,
};
use agent_diva_core::{
    evolution::{EvidenceRef, EvidenceSource, ProposalType},
    memory::memory_content_digest,
};
use chrono::Utc;

#[tokio::test]
async fn corrected_recall_feedback_proposes_governed_deprecation() {
    let evidence = EvidenceRef {
        id: "feedback-1".to_string(),
        source: EvidenceSource::RecallFeedback,
        uri: "recall-feedback://feedback-1".to_string(),
        excerpt: None,
        hash: Some(format!("sha256:{}", memory_content_digest(b"stale").value)),
        created_at: Utc::now(),
    };
    let output = DeterministicReflectionEngine::evidence_echo()
        .reflect(BoundedReflectionInput {
            schema_version: 1,
            workspace_id: "workspace-1".to_string(),
            run_id: "run-1".to_string(),
            evidence: vec![ReflectionEvidence {
                evidence: evidence.clone(),
                summary:
                    "record=record-1 selected=true injected=true corrected=true outcome=Succeeded"
                        .to_string(),
            }],
            existing_memory_digests: Vec::new(),
            max_candidates: 8,
        })
        .await
        .unwrap();

    assert_eq!(output.candidates.len(), 1);
    let candidate = &output.candidates[0];
    assert_eq!(candidate.proposal_type, ProposalType::Deprecation);
    assert_eq!(candidate.evidence_refs, vec![evidence]);
    let patch: serde_json::Value = serde_json::from_str(&candidate.content).unwrap();
    assert_eq!(patch["target_record_id"], "record-1");
    assert_eq!(patch["reason"], "user_correction");
}
