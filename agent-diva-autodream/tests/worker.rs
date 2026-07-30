use std::{fs, time::Duration};

use agent_diva_autodream::{
    AutoDreamReflectionStage, AutoDreamRestrictedAction, AutoDreamRestrictedProfile,
    AutoDreamService, AutoDreamStorage, AutoDreamWorker, AutoDreamWorkerConfig,
    AutoDreamWorkerOutcome, AutoDreamWorkerStageStatus, ManualRunTriggerRequest,
};
use agent_diva_core::evolution::{AutoDreamRunRecord, AutoDreamRunState, LaputaSectionName};
use agent_diva_laputa::{atomic_write_json, LaputaService, LaputaStorage};
use serde_json::Value;

#[test]
fn worker_executes_four_stages_in_order_and_completes_run() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    seed_session(
        temp.path(),
        "chat:1",
        "user wants concise iteration summaries",
    );
    seed_laputa(temp.path(), LaputaSectionName::MemoryMd, "memory evidence");
    let status = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap();

    let report = service.execute_reflection_worker(&status.run.id).unwrap();

    assert_eq!(report.outcome, AutoDreamWorkerOutcome::Success);
    assert_eq!(
        report
            .stages
            .iter()
            .map(|stage| stage.stage)
            .collect::<Vec<_>>(),
        vec![
            AutoDreamReflectionStage::Orient,
            AutoDreamReflectionStage::Gather,
            AutoDreamReflectionStage::Consolidate,
            AutoDreamReflectionStage::Propose,
        ]
    );
    assert!(report
        .stages
        .iter()
        .all(|stage| stage.status == AutoDreamWorkerStageStatus::Succeeded));
    assert_eq!(report.proposal_ids.len(), 1);

    let record = read_run(temp.path(), &status.run.id);
    assert_eq!(record.state, AutoDreamRunState::Completed);
    assert_eq!(record.proposal_ids, report.proposal_ids);
    assert!(record.completed_at.is_some());
    let checkpoint =
        fs::read_to_string(temp.path().join(".agent-diva/autodream/checkpoint")).unwrap();
    assert!(checkpoint.contains(&status.run.id));
}

#[test]
fn restricted_profile_denies_shell_and_direct_authority_writes() {
    let profile = AutoDreamRestrictedProfile::default();

    assert!(profile.is_allowed(AutoDreamRestrictedAction::ReadSessions));
    assert!(profile.is_allowed(AutoDreamRestrictedAction::ReadLaputaApi));
    assert!(profile.is_allowed(AutoDreamRestrictedAction::WriteAutoDreamOutput));
    assert!(profile.is_allowed(AutoDreamRestrictedAction::CreateLaputaProposalApi));
    assert!(!profile.is_allowed(AutoDreamRestrictedAction::ArbitraryShell));
    assert!(!profile.is_allowed(AutoDreamRestrictedAction::WriteExternalAuthority));
    assert!(!profile.is_allowed(AutoDreamRestrictedAction::DirectLaputaAuthorityWrite));
    assert!(!profile.is_allowed(AutoDreamRestrictedAction::WriteMonthlyReport));
}

#[test]
fn worker_timeout_records_failure_and_does_not_update_checkpoint() {
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
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let worker = AutoDreamWorker::new(storage, LaputaService::open(temp.path()).unwrap())
        .with_config(AutoDreamWorkerConfig {
            timeout: Some(Duration::ZERO),
            ..AutoDreamWorkerConfig::default()
        });

    let report = worker.execute(&status.run.id).unwrap();

    assert_eq!(report.outcome, AutoDreamWorkerOutcome::Timeout);
    assert_eq!(
        report.stages[0].status,
        AutoDreamWorkerStageStatus::TimedOut
    );
    let record = read_run(temp.path(), &status.run.id);
    assert_eq!(record.state, AutoDreamRunState::Failed);
    assert!(record
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("timed out"));
    let checkpoint =
        fs::read_to_string(temp.path().join(".agent-diva/autodream/checkpoint")).unwrap();
    assert!(checkpoint.contains("\"last_completed_run_id\": null"));
}

#[test]
fn worker_observes_cancellation_and_does_not_mark_completed() {
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
    service.cancel_run(&status.run.id).unwrap();

    let report = service.execute_reflection_worker(&status.run.id).unwrap();

    assert_eq!(report.outcome, AutoDreamWorkerOutcome::Cancelled);
    assert_eq!(
        report.stages[0].status,
        AutoDreamWorkerStageStatus::Cancelled
    );
    let record = read_run(temp.path(), &status.run.id);
    assert_eq!(record.state, AutoDreamRunState::Cancelled);
    let checkpoint =
        fs::read_to_string(temp.path().join(".agent-diva/autodream/checkpoint")).unwrap();
    assert!(checkpoint.contains("\"last_completed_run_id\": null"));
}

#[test]
fn worker_failure_records_user_visible_diagnostics_and_keeps_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    let status = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap();

    let report = service.execute_reflection_worker(&status.run.id).unwrap();

    assert_eq!(report.outcome, AutoDreamWorkerOutcome::Failure);
    assert!(report
        .diagnostics
        .iter()
        .any(|item| item.contains("all mandatory inputs omitted")));
    let record = read_run(temp.path(), &status.run.id);
    assert_eq!(record.state, AutoDreamRunState::Failed);
    assert!(record
        .error
        .as_deref()
        .unwrap_or_default()
        .contains("all mandatory inputs omitted"));
    let events =
        fs::read_to_string(temp.path().join(".agent-diva/autodream/events.jsonl")).unwrap();
    assert!(events.contains("worker_failed"));
    let checkpoint =
        fs::read_to_string(temp.path().join(".agent-diva/autodream/checkpoint")).unwrap();
    assert!(checkpoint.contains("\"last_completed_run_id\": null"));
}

#[test]
fn worker_creates_proposals_through_laputa_api_without_direct_authority_writes() {
    let temp = tempfile::tempdir().unwrap();
    let service = AutoDreamService::open(temp.path()).unwrap();
    seed_session(temp.path(), "chat:1", "session evidence");
    seed_laputa(
        temp.path(),
        LaputaSectionName::MemoryMd,
        "authority evidence",
    );
    let authority_path = LaputaStorage::open(temp.path())
        .unwrap()
        .paths()
        .section_file(LaputaSectionName::MemoryMd);
    let before = fs::read_to_string(&authority_path).unwrap();
    let status = service
        .trigger_manual_run(ManualRunTriggerRequest { trigger: None })
        .unwrap();

    let report = service.execute_reflection_worker(&status.run.id).unwrap();

    assert_eq!(report.outcome, AutoDreamWorkerOutcome::Success);
    assert_eq!(report.proposal_ids.len(), 1);
    assert_eq!(fs::read_to_string(&authority_path).unwrap(), before);
    assert!(temp.path().join(".laputa/proposals").exists());
}

fn read_run(workspace: &std::path::Path, run_id: &str) -> AutoDreamRunRecord {
    let raw = fs::read_to_string(
        workspace
            .join(".agent-diva/autodream/runs")
            .join(run_id)
            .join("record.json"),
    )
    .unwrap();
    serde_json::from_str(&raw).unwrap()
}

fn seed_session(workspace: &std::path::Path, key: &str, content: &str) {
    let mut manager = agent_diva_core::session::SessionManager::new(workspace);
    let session = manager.get_or_create(key);
    session.add_message("user", content);
    let cloned = session.clone();
    manager.save(&cloned).unwrap();
}

fn seed_laputa(workspace: &std::path::Path, section: LaputaSectionName, content: &str) {
    let path = LaputaStorage::open(workspace)
        .unwrap()
        .paths()
        .section_file(section);
    atomic_write_json(&path, &Value::String(content.to_string())).unwrap();
}
