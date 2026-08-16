use agent_diva_laputa::{LaputaStorage, RecallFeedbackStore};

#[test]
fn recall_feedback_store_reads_recent_events_from_workspace_journal() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let store = RecallFeedbackStore::new(storage);
    assert!(store.recent(10).unwrap().is_empty());
}
