//! S3 vertical contract for AutoDream and the machine-wide ACTMEM authority.
//!
//! S3 deliberately retires the old AutoDream -> MemoryPatch -> Laputa approval
//! flow. Evolution/Skill proposal production belongs to S4.

use std::{path::Path, time::Duration};

use agent_diva_core::{
    bus::MessageBus,
    config::schema::MemoryAuthorityMode,
    evolution::AutoDreamRunState,
    experience::{ExperienceJournal, OutcomeKind},
    session::SessionManager,
};
use agent_diva_laputa::ActmemPatch;
use agent_diva_manager::{build_router, AppState};
use agent_diva_sandbox::CommandApprovalCoordinator;
use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::sync::mpsc;
use tower::ServiceExt;

async fn typed_state(root: &Path) -> AppState {
    let (api_tx, _api_rx) = mpsc::channel(8);
    let mut state = AppState::new_with_runtime_memory(
        api_tx,
        MessageBus::new(),
        root,
        CommandApprovalCoordinator::default(),
        agent_diva_core::ask_user::AskUserCoordinator::default(),
        MemoryAuthorityMode::Typed,
    )
    .unwrap();
    // Keep the S3/S4 vertical deterministic: no provider-backed skill engine.
    state.autodream = state.autodream.clone().with_skill_reflection_engine(None);
    state
}

fn seed_evidence(root: &Path, session_key: &str, content: &str) {
    let mut sessions = SessionManager::new(root);
    let session = sessions.get_or_create(session_key);
    session.add_message("user", content);
    let saved = session.clone();
    sessions.save(&saved).unwrap();

    let journal = ExperienceJournal::open(root);
    journal
        .append(&journal.tool_evidence(
            session_key,
            "trace-e2e",
            "tool-e2e",
            "exec",
            OutcomeKind::Succeeded,
        ))
        .unwrap();
}

async fn json_request(
    app: &Router,
    method: Method,
    uri: impl Into<String>,
    payload: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri.into())
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body).unwrap();
    (status, value)
}

async fn get_json(app: &Router, uri: impl Into<String>) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri(uri.into())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body).unwrap();
    (status, value)
}

async fn wait_terminal(state: &AppState, run_id: &str) -> agent_diva_autodream::AutoDreamRunStatus {
    tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let status = state.autodream.get_run_status(run_id).unwrap();
            if matches!(
                status.run.state,
                AutoDreamRunState::Completed
                    | AutoDreamRunState::Failed
                    | AutoDreamRunState::Cancelled
            ) {
                return status;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("AutoDream run did not reach a terminal state")
}

async fn trigger_and_wait(
    app: &Router,
    state: &AppState,
) -> (String, agent_diva_autodream::AutoDreamRunStatus) {
    let (status, body) = json_request(
        app,
        Method::POST,
        "/api/autodream/runs",
        json!({ "trigger": "manual" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "trigger response: {body}");
    let run_id = body["run"]["id"].as_str().unwrap().to_string();
    let terminal = wait_terminal(state, &run_id).await;
    (run_id, terminal)
}

async fn seed_actmem(state: &AppState) -> u64 {
    state
        .memory_home
        .actmem()
        .put(ActmemPatch {
            pulse: Some("- durable preference: concise release summaries".into()),
            recap: Some("- Evidence collection completed.".into()),
            work: Some(
                "### Goal\n- Prepare release\n\n### Open\n\n### Next\n\n### Constraints\n- Keep changes focused\n\n### Pointers"
                    .into(),
            ),
            base_revision: 0,
        })
        .await
        .unwrap()
        .revision
}

#[tokio::test]
async fn e2e_s3_organizes_work_without_memory_or_governance_writes() {
    let temp = TempDir::new().unwrap();
    seed_evidence(temp.path(), "chat:e2e", "verified release evidence");
    let state = typed_state(temp.path()).await;
    let initial_revision = seed_actmem(&state).await;
    let app = build_router(state.clone());

    let (run_id, terminal) = trigger_and_wait(&app, &state).await;
    assert_eq!(terminal.run.state, AutoDreamRunState::Completed);
    assert!(terminal.run.proposal_ids.is_empty());
    assert!(std::fs::read_dir(temp.path().join(".laputa/proposals"))
        .map(|entries| entries.count() == 0)
        .unwrap_or(true));
    assert!(!state.memory_home.database_path().exists());

    let document = state.memory_home.actmem().read().unwrap();
    assert!(document.revision > initial_revision);
    for section in ["Goal", "Open", "Next", "Constraints", "Pointers"] {
        assert!(document.work.contains(&format!("### {section}")));
    }
    assert!(document.work.contains("MEMRULES:Default"));
    assert!(document.work.contains("evidence:"));

    let (_, events) = get_json(&app, format!("/api/autodream/runs/{run_id}/events")).await;
    assert!(events["events"]
        .as_array()
        .unwrap()
        .iter()
        .any(|event| event["kind"] == "actmem_work_organized"));
}

#[tokio::test]
async fn e2e_s3_does_not_require_a_reflection_provider() {
    let temp = TempDir::new().unwrap();
    seed_evidence(
        temp.path(),
        "chat:provider",
        "provider-independent evidence",
    );
    let state = typed_state(temp.path()).await;
    seed_actmem(&state).await;
    let app = build_router(state.clone());

    let (_, terminal) = trigger_and_wait(&app, &state).await;
    assert_eq!(terminal.run.state, AutoDreamRunState::Completed);
    assert_eq!(terminal.run.failure_code, None);
    assert!(terminal.run.proposal_ids.is_empty());
}

#[tokio::test]
async fn e2e_s3_never_writes_legacy_proposal_surfaces() {
    let temp = TempDir::new().unwrap();
    seed_evidence(temp.path(), "chat:no-reflect", "verified task evidence");
    let state = typed_state(temp.path()).await;
    seed_actmem(&state).await;
    let app = build_router(state.clone());

    let (_, terminal) = trigger_and_wait(&app, &state).await;
    assert_eq!(terminal.run.state, AutoDreamRunState::Completed);
    assert!(terminal.run.proposal_ids.is_empty());
    assert!(std::fs::read_dir(temp.path().join(".laputa/proposals"))
        .map(|entries| entries.count() == 0)
        .unwrap_or(true));
    assert!(!temp.path().join(".laputa/sections/memory_md.json").exists());
    assert!(!temp.path().join(".laputa/governance.sqlite3").exists());
}

#[tokio::test]
async fn e2e_repeated_s3_organization_is_a_revision_stable_noop() {
    let temp = TempDir::new().unwrap();
    seed_evidence(temp.path(), "chat:noop", "stable evidence");
    let state = typed_state(temp.path()).await;
    seed_actmem(&state).await;
    let app = build_router(state.clone());

    let (_, first) = trigger_and_wait(&app, &state).await;
    assert_eq!(first.run.state, AutoDreamRunState::Completed);
    let first_document = state.memory_home.actmem().read().unwrap();
    let (_, second) = trigger_and_wait(&app, &state).await;
    assert_eq!(second.run.state, AutoDreamRunState::Completed);
    let second_document = state.memory_home.actmem().read().unwrap();

    assert_eq!(second_document.work, first_document.work);
    assert_eq!(second_document.revision, first_document.revision);
    assert!(second.run.proposal_ids.is_empty());
}
