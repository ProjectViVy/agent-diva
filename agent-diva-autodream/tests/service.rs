use std::time::Duration;

use agent_diva_autodream::{AutoDreamService, ManualRunTriggerRequest};
use agent_diva_core::evolution::AutoDreamRunState;

#[test]
fn manual_run_creation_persists_record_and_lock() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();

    let status = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap();

    assert_eq!(status.run.state, AutoDreamRunState::Running);
    assert_eq!(status.run.trigger, "manual");
    assert!(status.lock.is_some());
    assert!(!status.auto_mode_enabled);
    assert!(!status.session_threshold_enabled);
    assert!(temp
        .path()
        .join(".agent-diva/autodream/runs")
        .join(&status.run.id)
        .join("record.json")
        .exists());
    assert!(temp.path().join(".agent-diva/autodream/lock").exists());
}

#[test]
fn duplicate_active_lock_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    let first = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap();

    let error = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap_err();

    assert!(error.to_string().contains(&first.run.id));
}

#[test]
fn cancellation_updates_terminal_state_and_removes_lock() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    let status = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap();

    let cancelled = service.cancel_run(&status.run.id).unwrap();

    assert_eq!(cancelled.run.state, AutoDreamRunState::Cancelled);
    assert!(cancelled.run.completed_at.is_some());
    assert!(cancelled.lock.is_none());
    assert!(!temp.path().join(".agent-diva/autodream/lock").exists());
}

#[test]
fn stale_lock_recovery_marks_previous_run_failed_and_allows_new_run() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path())
        .unwrap()
        .with_stale_lock_after(Duration::ZERO);
    let first = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap();

    let second = service
        .trigger_manual_run(ManualRunTriggerRequest {
            trigger: Some("manual".to_string()),
        })
        .unwrap();
    let previous = service.get_run_status(&first.run.id).unwrap();

    assert_eq!(previous.run.state, AutoDreamRunState::Failed);
    assert_eq!(second.run.state, AutoDreamRunState::Running);
    assert_ne!(first.run.id, second.run.id);
}

#[test]
fn checkpoint_defaults_keep_auto_modes_off() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();

    let checkpoint = service.checkpoint().unwrap();
    let raw =
        std::fs::read_to_string(temp.path().join(".agent-diva/autodream/checkpoint")).unwrap();

    assert!(!checkpoint.auto_mode_enabled);
    assert!(!checkpoint.session_threshold_enabled);
    assert!(raw.contains("\"auto_mode_enabled\": false"));
    assert!(raw.contains("\"session_threshold_enabled\": false"));
}
