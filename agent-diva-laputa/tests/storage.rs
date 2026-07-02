use std::{
    fs, thread,
    time::{Duration, Instant},
};

use agent_diva_core::evolution::LaputaSectionName;
use agent_diva_laputa::{
    atomic_write, atomic_write_json, LaputaError, LaputaLock, LaputaStorage, LockOptions,
};
use serde_json::json;

#[test]
fn open_creates_file_first_layout_idempotently() {
    let temp = tempfile::tempdir().unwrap();

    let storage = LaputaStorage::open(temp.path()).unwrap();
    let paths = storage.paths();

    assert!(paths.state_json().is_file());
    assert!(paths.proposals_dir().is_dir());
    assert!(paths.changelog_dir().is_dir());
    assert!(paths.audit_dir().is_dir());
    assert!(paths.rollback_dir().is_dir());
    assert!(paths.migrations_dir().is_dir());
    assert!(paths.locks_dir().is_dir());
    assert!(paths.legacy_dir().is_dir());
    assert_eq!(
        paths.section_file(LaputaSectionName::MemoryMd),
        temp.path().join(".laputa/sections/memory_md.json")
    );

    let state_before = fs::read_to_string(paths.state_json()).unwrap();
    let reopened = LaputaStorage::open(temp.path()).unwrap();
    let state_after = fs::read_to_string(reopened.paths().state_json()).unwrap();

    assert_eq!(state_before, state_after);
    assert!(state_after.contains("\"schema_version\": \"1.0.0\""));
}

#[test]
fn service_reports_schema_version_from_state_json() {
    let temp = tempfile::tempdir().unwrap();
    let storage = LaputaStorage::open(temp.path()).unwrap();
    fs::write(
        storage.paths().state_json(),
        r#"{"schema_version":"1.1.0","extra":"kept"}"#,
    )
    .unwrap();
    let service = agent_diva_laputa::LaputaService::from_storage(storage);

    let snapshot = service.read_snapshot(None).unwrap();
    let section = service.read_section(LaputaSectionName::Identity).unwrap();

    assert_eq!(snapshot.schema_version, "1.1.0");
    assert_eq!(section.version, "1.1.0");
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
fn lock_recovers_stale_lock_file() {
    let temp = tempfile::tempdir().unwrap();
    let lock_path = temp.path().join(".laputa/locks/state.lock");
    fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
    fs::write(&lock_path, "stale").unwrap();
    thread::sleep(Duration::from_millis(20));

    let guard = LaputaLock::acquire(
        &lock_path,
        LockOptions {
            timeout: Duration::from_millis(300),
            stale_after: Duration::from_millis(10),
            retry_interval: Duration::from_millis(5),
        },
    )
    .unwrap();

    assert_eq!(guard.path(), lock_path.as_path());
    assert!(lock_path.exists());
}

#[test]
fn lock_does_not_recover_live_owner_lock_file() {
    let temp = tempfile::tempdir().unwrap();
    let lock_path = temp.path().join(".laputa/locks/state.lock");
    fs::create_dir_all(lock_path.parent().unwrap()).unwrap();
    fs::write(
        &lock_path,
        format!("pid={}\ncreated_at=stale", std::process::id()),
    )
    .unwrap();

    let error = LaputaLock::acquire(
        &lock_path,
        LockOptions {
            timeout: Duration::from_millis(20),
            stale_after: Duration::ZERO,
            retry_interval: Duration::from_millis(5),
        },
    )
    .unwrap_err();

    assert!(matches!(error, LaputaError::LockTimeout { .. }));
    assert!(lock_path.exists());
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
    let section_path = storage.paths().section_file(LaputaSectionName::MemoryMd);

    atomic_write_json(&section_path, &json!({ "items": ["safe path"] })).unwrap();
    let saved = fs::read_to_string(&section_path).unwrap();
    assert!(saved.contains("safe path"));

    let lock_path = storage.paths().locks_dir().join("state.lock");
    let guard = LaputaLock::acquire(&lock_path, LockOptions::default()).unwrap();
    assert_eq!(guard.path(), lock_path.as_path());
}
