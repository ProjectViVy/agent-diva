use std::{fs, time::Duration};

use agent_diva_autodream::{
    AutoDreamService, ManualRunTriggerRequest, ScheduledMonthlyReportOutcome,
};
use agent_diva_core::evolution::{
    AutoDreamOrchestrationPhase, AutoDreamRunRecord, AutoDreamRunState, LaputaSectionName,
};
use agent_diva_laputa::{atomic_write_json, LaputaStorage};
use chrono::NaiveDate;
use serde_json::Value;

#[test]
fn manual_run_creation_persists_queued_record_and_lock() {
    let temp = tempfile::tempdir().unwrap();
    AutoDreamService::reset_metrics_for_test();
    let before = AutoDreamService::metrics_snapshot();
    let service = AutoDreamService::open(temp.path()).unwrap();

    let status = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap();

    assert_eq!(status.run.state, AutoDreamRunState::Pending);
    let orchestration = status.run.orchestration.as_ref().unwrap();
    assert_eq!(orchestration.phase, AutoDreamOrchestrationPhase::Queued);
    assert_eq!(orchestration.attempt, 0);
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
    let metrics = AutoDreamService::metrics_snapshot();
    assert!(metrics.autodream_runs_total > before.autodream_runs_total);
    assert!(metrics.autodream_failures_total >= before.autodream_failures_total);
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
fn stale_lock_recovery_requeues_previous_run_and_allows_new_run() {
    let temp = tempfile::tempdir().unwrap();
    AutoDreamService::reset_metrics_for_test();
    let before = AutoDreamService::metrics_snapshot();
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

    assert_eq!(previous.run.state, AutoDreamRunState::Pending);
    assert_eq!(
        previous
            .run
            .orchestration
            .as_ref()
            .map(|state| &state.phase),
        Some(&AutoDreamOrchestrationPhase::Queued)
    );
    assert_eq!(second.run.state, AutoDreamRunState::Pending);
    assert_ne!(first.run.id, second.run.id);
    let resumable = service
        .resumable_runs()
        .unwrap()
        .into_iter()
        .map(|run| run.0)
        .collect::<Vec<_>>();
    assert!(resumable.contains(&first.run.id));
    assert!(resumable.contains(&second.run.id));
    let metrics = AutoDreamService::metrics_snapshot();
    assert!(metrics.autodream_runs_total >= before.autodream_runs_total + 2);
    assert!(metrics.autodream_failures_total >= before.autodream_failures_total);
}

#[test]
fn legacy_incomplete_run_fails_closed_instead_of_being_resumed() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    let run = AutoDreamRunRecord {
        id: "legacy-incomplete".to_string(),
        started_at: chrono::Utc::now(),
        completed_at: None,
        state: AutoDreamRunState::Pending,
        trigger: "manual".to_string(),
        summary: None,
        input_summary: None,
        proposal_ids: Vec::new(),
        orchestration: None,
        failure_code: None,
        error: None,
    };
    let path = temp
        .path()
        .join(".agent-diva/autodream/runs/legacy-incomplete/record.json");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    atomic_write_json(&path, &run).unwrap();

    assert!(service.resumable_runs().unwrap().is_empty());
    let recovered = service.get_run_status(&run.id).unwrap();
    assert_eq!(recovered.run.state, AutoDreamRunState::Failed);
    assert_eq!(
        recovered.run.failure_code,
        Some(agent_diva_core::evolution::AutoDreamFailureCode::LegacyIncomplete)
    );
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

#[test]
fn collect_inputs_persists_summary_into_run_record() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    seed_session(temp.path(), "chat:1", "session evidence");
    seed_laputa(
        temp.path(),
        LaputaSectionName::MemoryMd,
        "authority evidence",
    );

    let status = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap();

    let collected = service.collect_inputs(&status.run.id).unwrap();
    let raw = std::fs::read_to_string(
        temp.path()
            .join(".agent-diva/autodream/runs")
            .join(&status.run.id)
            .join("record.json"),
    )
    .unwrap();
    let record: AutoDreamRunRecord = serde_json::from_str(&raw).unwrap();

    assert!(collected.summary.total_items >= 2);
    assert!(record.input_summary.is_some());
    assert!(record
        .summary
        .as_deref()
        .unwrap_or_default()
        .contains("collected"));
}

#[tokio::test]
async fn notebook_daily_trigger_generates_report_and_completes_run() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    seed_session(temp.path(), "chat:1", "daily report content");

    let status = service
        .trigger_manual_run(ManualRunTriggerRequest {
            trigger: Some("notebook-daily".to_string()),
        })
        .unwrap();
    let completed = service
        .execute_report_trigger(&status.run.id)
        .await
        .unwrap();

    assert_eq!(completed.run.state, AutoDreamRunState::Completed);
    assert!(completed.lock.is_none());
    assert!(temp.path().join(".laputa/reports/daily").exists());
}

#[tokio::test]
async fn notebook_weekly_trigger_generates_report_and_completes_run() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    seed_session(temp.path(), "chat:1", "weekly report content");

    let status = service
        .trigger_manual_run(ManualRunTriggerRequest {
            trigger: Some("notebook-weekly".to_string()),
        })
        .unwrap();
    let completed = service
        .execute_report_trigger(&status.run.id)
        .await
        .unwrap();

    assert_eq!(completed.run.state, AutoDreamRunState::Completed);
    assert!(completed.lock.is_none());
    assert!(temp.path().join(".laputa/reports/weekly").exists());
}

#[tokio::test]
async fn notebook_monthly_trigger_generates_report_and_completes_run() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    seed_session(temp.path(), "chat:1", "monthly report content");

    let status = service
        .trigger_manual_run(ManualRunTriggerRequest {
            trigger: Some("notebook-monthly".to_string()),
        })
        .unwrap();
    let completed = service
        .execute_report_trigger(&status.run.id)
        .await
        .unwrap();

    assert_eq!(completed.run.state, AutoDreamRunState::Completed);
    assert!(completed.lock.is_none());
    assert!(temp.path().join(".laputa/reports/monthly").exists());
}

#[tokio::test]
async fn scheduled_monthly_report_runs_on_first_monday() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    seed_session_at(
        temp.path(),
        "chat:1",
        "scheduled monthly report",
        "2026-06-01T01:02:03Z",
    );

    let outcome = service
        .execute_scheduled_monthly_report(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap())
        .await
        .unwrap();

    match outcome {
        ScheduledMonthlyReportOutcome::Triggered { run_id, month_key } => {
            assert!(!run_id.is_empty());
            assert_eq!(month_key, "2026-06");
        }
        other => panic!("expected triggered outcome, got {other:?}"),
    }
    assert!(temp
        .path()
        .join(".laputa/reports/monthly/2026-06.md")
        .exists());
}

#[tokio::test]
async fn scheduled_monthly_report_skips_outside_window_without_retry_marker() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();

    let outcome = service
        .execute_scheduled_monthly_report(NaiveDate::from_ymd_opt(2026, 6, 10).unwrap())
        .await
        .unwrap();

    assert_eq!(
        outcome,
        ScheduledMonthlyReportOutcome::Skipped {
            month_key: "2026-06".to_string(),
            reason: "outside monthly schedule window".to_string(),
        }
    );
}

fn seed_session(workspace: &std::path::Path, key: &str, content: &str) {
    let mut manager = agent_diva_core::session::SessionManager::new(workspace);
    let session = manager.get_or_create(key);
    session.add_message("user", content);
    let cloned = session.clone();
    manager.save(&cloned).unwrap();
}

fn seed_session_at(workspace: &std::path::Path, key: &str, content: &str, timestamp: &str) {
    let sessions_dir = workspace.join("sessions");
    fs::create_dir_all(&sessions_dir).unwrap();
    let safe_key = key.replace([':', '/', '\\'], "_");
    let path = sessions_dir.join(format!("{safe_key}.jsonl"));
    let content = serde_json::json!({
        "_type": "metadata",
        "key": key,
        "created_at": timestamp,
        "updated_at": timestamp,
        "metadata": {},
        "title": null,
        "last_consolidated": null,
        "last_compacted": 0,
        "compaction_history": [],
    })
    .to_string()
        + "\n"
        + &serde_json::json!({
            "role": "user",
            "content": content,
            "timestamp": timestamp,
            "metadata": {},
        })
        .to_string();
    fs::write(path, content).unwrap();
}

fn seed_laputa(workspace: &std::path::Path, section: LaputaSectionName, content: &str) {
    let path = LaputaStorage::open(workspace)
        .unwrap()
        .paths()
        .section_file(section);
    atomic_write_json(&path, &Value::String(content.to_string())).unwrap();
}
