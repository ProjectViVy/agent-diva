use std::fs;

use agent_diva_core::evolution::LaputaSectionName;
use agent_diva_laputa::{
    LaputaMigration, LaputaMigrationOptions, LaputaMigrationOutcome, LaputaMigrationSource,
    LaputaMigrationSourceKind, LaputaMigrationTestFailure, LaputaStorage,
};
use serde_json::Value;

#[test]
fn migration_new_install_with_no_legacy_sources_is_noop_but_records_state() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();

    let outcome = LaputaMigration::new(storage.clone()).run(LaputaMigrationOptions::default());

    assert_eq!(outcome.unwrap(), LaputaMigrationOutcome::default());
    let state = fs::read_to_string(storage.paths().state_json()).unwrap();
    assert!(state.contains("\"schema_version\": \"1.1.0\""));
    assert!(state.contains("\"legacy_migration\""));
}

#[test]
fn migration_copies_legacy_sources_and_maps_supported_sections() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("memory")).unwrap();
    fs::write(
        temp.path().join("memory/MEMORY.md"),
        "remember the useful fact",
    )
    .unwrap();
    fs::write(temp.path().join("SOUL.md"), "soul identity").unwrap();
    fs::write(temp.path().join("IDENTITY.md"), "explicit identity").unwrap();
    fs::write(temp.path().join("BOOTSTRAP.md"), "bootstrap only").unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();

    let outcome = LaputaMigration::new(storage.clone())
        .run(LaputaMigrationOptions::default())
        .unwrap();

    assert_eq!(outcome.backed_up_sources.len(), 4);
    assert_eq!(outcome.written_sections.len(), 2);
    assert!(temp.path().join("memory/MEMORY.md").exists());
    assert!(temp.path().join("BOOTSTRAP.md").exists());
    for backup in &outcome.backed_up_sources {
        assert!(backup.backup_path.exists());
        assert!(backup
            .backup_path
            .strip_prefix(storage.paths().legacy_dir())
            .unwrap()
            .components()
            .next()
            .is_some());
    }

    let memory =
        fs::read_to_string(storage.paths().section_file(LaputaSectionName::MemoryMd)).unwrap();
    assert!(memory.contains("remember the useful fact"));
    assert!(memory.contains("\"status\": \"owned\""));

    let identity =
        fs::read_to_string(storage.paths().section_file(LaputaSectionName::Identity)).unwrap();
    assert!(identity.contains("soul identity"));
    assert!(identity.contains("explicit identity"));
    assert!(!identity.contains("bootstrap only"));
    assert!(!identity.contains("BOOTSTRAP.md"));
}

#[test]
fn migration_preserves_unsupported_sources_as_tbd_payloads() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("legacy")).unwrap();
    fs::write(
        temp.path().join("legacy/future.md"),
        "future owned material",
    )
    .unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let source = LaputaMigrationSource {
        path: temp.path().join("legacy/future.md"),
        kind: LaputaMigrationSourceKind::Unsupported {
            section: LaputaSectionName::AaakSummaries,
            reason: "owned by future story".to_string(),
        },
    };

    LaputaMigration::new(storage.clone())
        .run(LaputaMigrationOptions {
            sources: Some(vec![source]),
            ..LaputaMigrationOptions::default()
        })
        .unwrap();

    let section = fs::read_to_string(
        storage
            .paths()
            .section_file(LaputaSectionName::AaakSummaries),
    )
    .unwrap();
    let payload: Value = serde_json::from_str(&section).unwrap();
    assert_eq!(payload["status"], "tbd");
    assert_eq!(payload["metadata"]["reason"], "owned by future story");
    assert_eq!(payload["entries"][0]["content"], "future owned material");
}

#[test]
fn migration_discovers_legacy_relationship_history_and_tbd_task_templates() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("memory")).unwrap();
    fs::write(temp.path().join("memory/HISTORY.md"), "old history").unwrap();
    fs::write(temp.path().join("USER.md"), "known user").unwrap();
    fs::write(temp.path().join("PROFILE.md"), "known profile").unwrap();
    fs::write(temp.path().join("TASK.md"), "pending task").unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();

    let outcome = LaputaMigration::new(storage.clone())
        .run(LaputaMigrationOptions::default())
        .unwrap();

    assert_eq!(outcome.backed_up_sources.len(), 4);
    assert!(outcome
        .written_sections
        .contains(&"relationship".to_string()));
    assert!(outcome.written_sections.contains(&"history_md".to_string()));
    assert!(outcome
        .written_sections
        .contains(&"journal_reflective".to_string()));

    let relationship = fs::read_to_string(
        storage
            .paths()
            .section_file(LaputaSectionName::Relationship),
    )
    .unwrap();
    assert!(relationship.contains("known user"));
    assert!(relationship.contains("known profile"));
    let task = fs::read_to_string(
        storage
            .paths()
            .section_file(LaputaSectionName::JournalReflective),
    )
    .unwrap();
    let payload: Value = serde_json::from_str(&task).unwrap();
    assert_eq!(payload["status"], "tbd");
    assert_eq!(payload["metadata"]["content_type"], "tbd");
}

#[test]
fn migration_failure_before_commit_leaves_previous_state_and_sections_readable() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("MEMORY.md"), "new migration content").unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    let state_before = fs::read_to_string(storage.paths().state_json()).unwrap();

    let error = LaputaMigration::new(storage.clone())
        .run(LaputaMigrationOptions {
            test_failure: Some(LaputaMigrationTestFailure::AfterStagingBeforeCommit),
            ..LaputaMigrationOptions::default()
        })
        .unwrap_err();

    assert!(error.to_string().contains("injected migration failure"));
    let state_after = fs::read_to_string(storage.paths().state_json()).unwrap();
    assert_eq!(state_after, state_before);
    assert!(!storage
        .paths()
        .section_file(LaputaSectionName::MemoryMd)
        .exists());
    assert!(!storage.paths().staging_dir().exists());
}

#[test]
fn migration_rerun_is_idempotent_for_same_legacy_sources() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("memory")).unwrap();
    fs::write(temp.path().join("memory/MEMORY.md"), "stable memory").unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();

    let first = LaputaMigration::new(storage.clone())
        .run(LaputaMigrationOptions::default())
        .unwrap();
    let second = LaputaMigration::new(storage.clone())
        .run(LaputaMigrationOptions::default())
        .unwrap();

    assert_eq!(first.written_sections, second.written_sections);
    let memory =
        fs::read_to_string(storage.paths().section_file(LaputaSectionName::MemoryMd)).unwrap();
    let payload: Value = serde_json::from_str(&memory).unwrap();
    assert_eq!(payload["entries"].as_array().unwrap().len(), 1);
    assert_eq!(payload["entries"][0]["content"], "stable memory");
}
