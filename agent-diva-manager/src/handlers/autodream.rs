use agent_diva_autodream::{AutoDreamError, ManualRunTriggerRequest};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::state::AppState;

type JsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

pub async fn trigger_autodream_run_handler(
    State(state): State<AppState>,
    Json(payload): Json<ManualRunTriggerRequest>,
) -> JsonResult {
    let status = state
        .autodream
        .trigger_manual_run(payload.clone())
        .map_err(autodream_error_response)?;
    spawn_autodream_runs(
        state.autodream.clone(),
        vec![(status.run.id.clone(), status.run.trigger.clone())],
    );
    ok(
        serde_json::json!({ "status": "ok", "run": status.run, "lock": status.lock, "auto_mode_enabled": status.auto_mode_enabled, "session_threshold_enabled": status.session_threshold_enabled }),
    )
}

pub async fn get_autodream_run_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> JsonResult {
    let status = state
        .autodream
        .get_run_status(&id)
        .map_err(autodream_error_response)?;
    ok(
        serde_json::json!({ "status": "ok", "run": status.run, "lock": status.lock, "auto_mode_enabled": status.auto_mode_enabled, "session_threshold_enabled": status.session_threshold_enabled }),
    )
}

pub async fn list_autodream_run_events_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> JsonResult {
    let events = state
        .autodream
        .list_run_events(&id, 200)
        .map_err(autodream_error_response)?;
    ok(serde_json::json!({ "status": "ok", "events": events }))
}

pub async fn cancel_autodream_run_handler(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> JsonResult {
    let status = state
        .autodream
        .cancel_run(&id)
        .map_err(autodream_error_response)?;
    ok(
        serde_json::json!({ "status": "ok", "run": status.run, "lock": status.lock, "auto_mode_enabled": status.auto_mode_enabled, "session_threshold_enabled": status.session_threshold_enabled }),
    )
}

pub async fn list_autodream_runs_handler(State(state): State<AppState>) -> JsonResult {
    let list = state
        .autodream
        .list_runs()
        .map_err(autodream_error_response)?;
    ok(serde_json::json!({
        "status": "ok",
        "runs": list.runs,
        "active_run_id": list.active_run_id,
        "auto_mode_enabled": list.auto_mode_enabled,
        "session_threshold_enabled": list.session_threshold_enabled,
    }))
}

fn autodream_error_response(error: AutoDreamError) -> (StatusCode, Json<serde_json::Value>) {
    let (status, code) = match error {
        AutoDreamError::RunNotFound { .. } => (StatusCode::NOT_FOUND, "run_not_found"),
        AutoDreamError::ActiveRunExists { .. } => (StatusCode::CONFLICT, "active_run_exists"),
        AutoDreamError::RunNotCancellable { .. } => (StatusCode::CONFLICT, "run_not_cancellable"),
        AutoDreamError::InvalidState(_) => (StatusCode::BAD_REQUEST, "invalid_state"),
        AutoDreamError::Io { .. }
        | AutoDreamError::Json(_)
        | AutoDreamError::InputCollection(_)
        | AutoDreamError::ProposalPersistence(_) => {
            (StatusCode::INTERNAL_SERVER_ERROR, "autodream_storage_error")
        }
    };
    (
        status,
        Json(serde_json::json!({
            "status": "error",
            "code": code,
            "message": error.to_string(),
        })),
    )
}

pub(crate) fn spawn_autodream_runs(
    service: agent_diva_autodream::AutoDreamService,
    runs: Vec<(String, String)>,
) {
    tokio::spawn(async move {
        for (run_id, trigger) in runs {
            let result = if matches!(
                trigger.as_str(),
                "notebook-daily" | "notebook-weekly" | "notebook-monthly"
            ) {
                service.execute_report_trigger(&run_id).await.map(|_| ())
            } else {
                service.execute_reflection_worker(&run_id).await.map(|_| ())
            };
            if let Err(error) = result {
                tracing::error!(%run_id, %error, "AutoDream orchestrator run failed");
            }
        }
    });
}

fn ok(value: serde_json::Value) -> JsonResult {
    Ok(Json(value))
}
