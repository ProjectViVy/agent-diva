use agent_diva_autodream::{AutoDreamInputCollector, AutoDreamStorage};
use agent_diva_core::{
    evolution::EvidenceSource,
    governance::{ContentDigest, DigestAlgorithm},
};
use agent_diva_laputa::{
    LaputaService, LaputaStorage, PendingRecallFeedback, RecallFeedbackStore, RecallTaskOutcome,
};
use chrono::Utc;

#[test]
fn collector_prioritizes_payload_free_recall_feedback() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    RecallFeedbackStore::new(storage)
        .commit_pending(
            vec![PendingRecallFeedback {
                request_id: "recall-1".to_string(),
                selected: vec![(
                    "record-1".to_string(),
                    ContentDigest {
                        algorithm: DigestAlgorithm::Sha256,
                        value: "abc".to_string(),
                    },
                )],
                injected: true,
                selected_at: Utc::now(),
            }],
            RecallTaskOutcome::Succeeded,
            true,
            Utc::now(),
        )
        .unwrap();
    let collected = AutoDreamInputCollector::new(
        AutoDreamStorage::open(temp.path()).unwrap(),
        LaputaService::open(temp.path()).unwrap(),
    )
    .collect("run-1")
    .unwrap();

    let feedback = collected
        .items
        .iter()
        .find(|item| item.source == "recall_feedback")
        .unwrap();
    assert_eq!(feedback.evidence.source, EvidenceSource::RecallFeedback);
    assert!(feedback.excerpt.contains("corrected=true"));
    assert!(!feedback.excerpt.contains("Memory content"));
}
