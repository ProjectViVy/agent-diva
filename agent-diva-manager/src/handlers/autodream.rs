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
    let status = match payload.trigger.as_deref() {
        Some("notebook-daily" | "notebook-weekly" | "notebook-monthly") => state
            .autodream
            .execute_report_trigger(&status.run.id)
            .await
            .map_err(autodream_error_response)?,
        _ => {
            let service = state.autodream.clone();
            let run_id = status.run.id.clone();
            tokio::task::spawn_blocking(move || service.execute_reflection_worker(&run_id))
                .await
                .map_err(|error| {
                    internal_error_response(
                        "autodream_worker_join_failed",
                        format!("AutoDream worker task failed: {error}"),
                    )
                })?
                .map_err(autodream_error_response)?;
            state
                .autodream
                .get_run_status(&status.run.id)
                .map_err(autodream_error_response)?
        }
    };
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

fn internal_error_response(
    code: &'static str,
    message: String,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "status": "error",
            "code": code,
            "message": message,
        })),
    )
}

fn ok(value: serde_json::Value) -> JsonResult {
    Ok(Json(value))
}
