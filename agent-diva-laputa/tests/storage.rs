use std::{
    fs,
    time::{Duration, Instant},
};

use agent_diva_laputa::{
    atomic_write, atomic_write_json, LaputaError, LaputaLock, LaputaStorage, LockOptions,
};
use serde_json::json;

#[test]
fn open_creates_surviving_layout_idempotently_without_legacy_families() {
    let temp = tempfile::tempdir().unwrap();

    let storage = LaputaStorage::open(temp.path()).unwrap();
    let paths = storage.paths();

    assert!(paths.state_json().is_file());
    assert!(paths.locks_dir().is_dir());
    for legacy in [
        "proposals",
        "changelog",
        "audit",
        "rollback",
        "sections",
        "cognitive",
    ] {
        assert!(
            !temp.path().join(".laputa").join(legacy).exists(),
            "{legacy} layout family must stay removed"
        );
    }

    fs::write(paths.recall_feedback_json(), "{}").unwrap();
    let state_before = fs::read_to_string(paths.state_json()).unwrap();
    let reopened = LaputaStorage::open(temp.path()).unwrap();
    let state_after = fs::read_to_string(reopened.paths().state_json()).unwrap();

    assert_eq!(state_before, state_after);
    assert!(state_after.contains("\"schema_version\": \"1.0.0\""));
    assert_eq!(
        fs::read_to_string(reopened.paths().recall_feedback_json()).unwrap(),
        "{}"
    );
}

#[test]
fn atomic_write_replaces_target_without_leaving_temp_files() {
    let temp = tempfile::tempdir().unwrap();
    let target = temp.path().join(".laputa/state.json");

    atomic_write(&target, br#"{"schema_version":"1.0.0"}"#).unwrap();
    atomic_write_json(&target, &json!({ "schema_version": "1.0.1" })).unwrap();

    let saved = fs::read_to_string(&target).unwrap();
    assert!(saved.contains("\"schema_version\": \"1.0.1\""));

    let temp_files = fs::read_dir(target.parent().unwrap())
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(temp_files, 0);
}

#[test]
fn atomic_write_reports_failure_for_invalid_target_parent() {
    let temp = tempfile::tempdir().unwrap();
    let parent_file = temp.path().join("not-a-directory");
    fs::write(&parent_file, "occupied").unwrap();
    let target = parent_file.join("state.json");

    let error = atomic_write(&target, b"{}").unwrap_err();

    assert!(matches!(error, LaputaError::Io { .. }));
}

#[test]
fn lock_times_out_when_existing_lock_is_fresh() {
    let temp = tempfile::tempdir().unwrap();
    let lock_path = temp.path().join(".laputa/locks/state.lock");
    let _guard = LaputaLock::acquire(&lock_path, LockOptions::default()).unwrap();

    let start = Instant::now();
    let error = LaputaLock::acquire(
        &lock_path,
        LockOptions {
            timeout: Duration::from_millis(30),
            stale_after: Duration::from_secs(60),
            retry_interval: Duration::from_millis(5),
        },
    )
    .unwrap_err();

    assert!(start.elapsed() >= Duration::from_millis(30));
    assert!(matches!(error, LaputaError::LockTimeout { .. }));
}

#[test]
fn storage_atomic_write_and_lock_support_windows_safe_paths() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp
        .path()
        .join("Windows Safe Workspace")
        .join("nested.dir");
    fs::create_dir_all(&workspace).unwrap();
    let storage = LaputaStorage::open(&workspace).unwrap();
    let journal_path = storage.paths().recall_feedback_json();

    atomic_write_json(&journal_path, &json!({ "events": ["safe path"] })).unwrap();
    let saved = fs::read_to_string(&journal_path).unwrap();
    assert!(saved.contains("safe path"));

    let lock_path = storage.paths().locks_dir().join("state.lock");
    let guard = LaputaLock::acquire(&lock_path, LockOptions::default()).unwrap();
    assert_eq!(guard.path(), lock_path.as_path());
}
