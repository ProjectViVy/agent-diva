//! Independent characterization of AutoDream versus the S3/S4 product
//! contract after the cognitive clean break.
//!
//! Product: AutoDream organizes the shared ACTMEM Work register and may only
//! emit Skill review requests. It never reads legacy Laputa sections, never
//! emits EvolutionProposal/MemoryPatch, and requires the machine-wide
//! MemoryHome authority.

use agent_diva_autodream::{AutoDreamInputCollector, AutoDreamStorage, AutoDreamWorker};
use agent_diva_core::session::SessionManager;

#[test]
fn current_inputs_have_no_legacy_laputa_or_stm_source() {
    let temp = tempfile::tempdir().unwrap();
    let mut sessions = SessionManager::new(temp.path());
    let session = sessions.get_or_create("chat:contract");
    session.add_message("user", "recent session evidence for contract test");
    let saved = session.clone();
    sessions.save(&saved).unwrap();

    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let collected = AutoDreamInputCollector::new(storage)
        .collect("run-contract")
        .unwrap();

    let sources: Vec<&str> = collected
        .items
        .iter()
        .map(|item| item.source.as_str())
        .collect();
    assert!(
        sources
            .iter()
            .all(|source| *source != "laputa" && *source != "stm"),
        "clean-break collector must not read legacy sections or STM: {sources:?}"
    );
    assert!(!collected.items.is_empty());
}

#[tokio::test]
async fn worker_without_memory_home_is_a_configuration_error() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let worker = AutoDreamWorker::new(storage);
    let error = worker.execute("run-contract").await.unwrap_err();
    assert!(
        error
            .to_string()
            .contains("requires the machine-wide MemoryHome authority"),
        "unexpected error: {error}"
    );
}
