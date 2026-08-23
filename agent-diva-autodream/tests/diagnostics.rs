//! Phase-level structured diagnostic contract for the S3 AutoDream worker.
//!
//! Required fields: run_id, phase, input summary, gate rejection, proposal_id,
//! failure code. This suite does not write REDLINE/DREAM/user prefs, restore
//! MemoryPatch/SopCreate/Governance, or convert STM into proposals.

use std::sync::Arc;

use agent_diva_autodream::{
    AutoDreamEvent, AutoDreamService, AutoDreamStorage, AutoDreamWorker, AutoDreamWorkerOutcome,
    ManualRunTriggerRequest, SkillReflectionCandidate, SkillReflectionEngine, SkillReflectionInput,
    SkillReflectionOutput,
};
use agent_diva_core::{evolution::SkillHome, session::SessionManager};
use agent_diva_laputa::MemoryHome;

struct StaticSkillReflection {
    output: Result<SkillReflectionOutput, agent_diva_autodream::ReflectionError>,
}

#[async_trait::async_trait]
impl SkillReflectionEngine for StaticSkillReflection {
    async fn reflect_skills(
        &self,
        _input: SkillReflectionInput,
    ) -> Result<SkillReflectionOutput, agent_diva_autodream::ReflectionError> {
        self.output.clone()
    }
}

fn skill_markdown(slug: &str) -> String {
    format!("---\nname: {slug}\ndescription: generated skill\nalways: true\n---\nsteps\n")
}

fn seed_session(root: &std::path::Path) {
    let mut sessions = SessionManager::new(root);
    let session = sessions.get_or_create("gui:chat");
    session.add_message("user", "Keep the implementation evidence bounded");
    let session = session.clone();
    sessions.save(&session).unwrap();
}

fn event_by_kind<'a>(events: &'a [AutoDreamEvent], kind: &str) -> &'a AutoDreamEvent {
    events
        .iter()
        .find(|event| event.kind == kind)
        .unwrap_or_else(|| panic!("missing diagnostic event {kind}"))
}

#[tokio::test]
async fn success_path_emits_phase_and_input_summary() {
    let dir = tempfile::tempdir().unwrap();
    let memory_home = MemoryHome::new(dir.path().join("machine-home"));
    memory_home
        .actmem()
        .append_pulse("gui:chat", "Ship the ACTMEM workspace")
        .await
        .unwrap();
    seed_session(dir.path());

    let service = AutoDreamService::open(dir.path())
        .unwrap()
        .with_memory_home(memory_home.clone());
    let run = service
        .trigger_manual_run(ManualRunTriggerRequest::default())
        .unwrap()
        .run;
    let report = service.execute_reflection_worker(&run.id).await.unwrap();
    assert_eq!(report.outcome, AutoDreamWorkerOutcome::Success);

    let events = service.list_run_events(&run.id, 200).unwrap();
    for (kind, phase) in [
        ("phase_started", "orient"),
        ("phase_started", "gather"),
        ("phase_started", "consolidate"),
        ("phase_started", "propose"),
    ] {
        assert!(
            events.iter().any(|event| event.kind == kind
                && event.phase.as_deref() == Some(phase)
                && event.run_id.as_deref() == Some(run.id.as_str())),
            "missing {kind} phase={phase} in {events:?}"
        );
    }

    let collected = event_by_kind(&events, "input_collected");
    assert_eq!(collected.phase.as_deref(), Some("gather"));
    let summary = collected.input_summary.as_deref().expect("input summary");
    assert!(summary.contains("items="), "{summary}");
    assert!(summary.contains("sources="), "{summary}");
    assert!(event_by_kind(&events, "worker_succeeded").phase.as_deref() == Some("completed"));
    assert!(!memory_home.database_path().exists());
}

#[tokio::test]
async fn propose_path_logs_gate_rejection_and_proposal_id() {
    let dir = tempfile::tempdir().unwrap();
    let machine_home = dir.path().join("machine-home");
    let memory_home = MemoryHome::new(&machine_home);
    memory_home
        .actmem()
        .append_pulse("gui:chat", "Extract a reusable review routine")
        .await
        .unwrap();
    seed_session(dir.path());
    let skill_home = SkillHome::new(&machine_home, dir.path().join("builtin"));
    let engine = StaticSkillReflection {
        output: Ok(SkillReflectionOutput {
            schema_version: 1,
            candidates: vec![
                SkillReflectionCandidate {
                    slug: "INVALID SLUG".into(),
                    title: "Rejected".into(),
                    description: "invalid".into(),
                    proposed_markdown: skill_markdown("invalid-slug"),
                    reason: "gate should reject".into(),
                },
                SkillReflectionCandidate {
                    slug: "new-review".into(),
                    title: "New".into(),
                    description: "new".into(),
                    proposed_markdown: skill_markdown("new-review"),
                    reason: "bounded evidence".into(),
                },
            ],
            diagnostic_codes: Vec::new(),
        }),
    };
    let service = AutoDreamService::open(dir.path())
        .unwrap()
        .with_memory_home(memory_home.clone())
        .with_skill_home(skill_home.clone())
        .with_skill_reflection_engine(Some(Arc::new(engine)));
    let run = service
        .trigger_manual_run(ManualRunTriggerRequest::default())
        .unwrap()
        .run;
    let report = service.execute_reflection_worker(&run.id).await.unwrap();
    assert_eq!(report.proposal_ids.len(), 1);

    let events = service.list_run_events(&run.id, 200).unwrap();
    let rejected = event_by_kind(&events, "skill_candidate_rejected");
    assert_eq!(rejected.phase.as_deref(), Some("propose"));
    assert_eq!(rejected.gate_code.as_deref(), Some("skill_slug_invalid"));
    let created = event_by_kind(&events, "skill_request_created");
    assert_eq!(
        created.proposal_id.as_deref(),
        Some(report.proposal_ids[0].as_str())
    );
    assert!(!memory_home.database_path().exists());
    assert!(skill_home.read("new-review").is_err());
}

#[tokio::test]
async fn gather_failure_logs_input_unavailable_failure_code() {
    let dir = tempfile::tempdir().unwrap();
    let memory_home = MemoryHome::new(dir.path().join("machine-home"));
    let service = AutoDreamService::open(dir.path())
        .unwrap()
        .with_memory_home(memory_home);
    let run = service
        .trigger_manual_run(ManualRunTriggerRequest::default())
        .unwrap()
        .run;
    let report = service.execute_reflection_worker(&run.id).await.unwrap();
    assert_eq!(report.outcome, AutoDreamWorkerOutcome::Failure);

    let events = service.list_run_events(&run.id, 200).unwrap();
    let failed = event_by_kind(&events, "input_collection_failed");
    assert_eq!(failed.phase.as_deref(), Some("gather"));
    assert_eq!(failed.failure_code.as_deref(), Some("input_unavailable"));
    let terminal = event_by_kind(&events, "worker_failed");
    assert_eq!(terminal.failure_code.as_deref(), Some("input_unavailable"));
    assert_eq!(terminal.phase.as_deref(), Some("failed"));
}

#[tokio::test]
async fn missing_memory_home_logs_configuration_failure() {
    let temp = tempfile::tempdir().unwrap();
    let storage = AutoDreamStorage::open(temp.path()).unwrap();
    let worker = AutoDreamWorker::new(storage.clone());
    let error = worker.execute("run-contract").await.unwrap_err();
    assert!(error
        .to_string()
        .contains("requires the machine-wide MemoryHome authority"));

    let raw = std::fs::read_to_string(storage.paths().events_jsonl()).unwrap();
    let events: Vec<AutoDreamEvent> = raw
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let event = event_by_kind(&events, "memory_home_missing");
    assert_eq!(event.run_id.as_deref(), Some("run-contract"));
    assert_eq!(event.phase.as_deref(), Some("orient"));
    assert_eq!(event.failure_code.as_deref(), Some("worker_failed"));
}
