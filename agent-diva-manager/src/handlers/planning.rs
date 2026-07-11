//! REST handlers for the planning API.
//!
//! These handlers translate HTTP requests into [`ManagerCommand`] variants
//! and return JSON responses, following the same pattern as other handlers
//! in this crate.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use tokio::sync::oneshot;

use crate::planning_service::{
    AppendPlanReportRevisionRequest, ApprovePlanReportRequest, CreatePlanReportRequest,
};
use crate::state::{AppState, ManagerCommand};

#[derive(Debug, Deserialize)]
pub struct ActiveExecutionQuery {
    pub session_key: String,
}

/// GET /api/plan-reports
pub async fn list_plan_reports_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (tx, rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::ListPlanReports(tx))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match rx.await {
        Ok(Ok(reports)) => Ok(Json(
            serde_json::json!({ "status": "ok", "reports": reports }),
        )),
        Ok(Err(error)) => {
            tracing::error!(%error, "ListPlanReports failed");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /api/plan-reports
pub async fn create_plan_report_handler(
    State(state): State<AppState>,
    Json(request): Json<CreatePlanReportRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    if request.session_key.trim().is_empty()
        || request.title.trim().is_empty()
        || request.markdown.trim().is_empty()
    {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(
                serde_json::json!({ "status": "error", "message": "session_key, title, and markdown are required" }),
            ),
        ));
    }
    let (tx, rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::CreatePlanReport(request, tx))
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "status": "error", "message": error.to_string() })),
            )
        })?;
    match rx.await {
        Ok(Ok(report)) => Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({ "status": "ok", "report": report })),
        )),
        Ok(Err(error)) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "status": "error", "message": error })),
        )),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": error.to_string() })),
        )),
    }
}

/// POST /api/plan-reports/:report_id/revisions
pub async fn append_plan_report_revision_handler(
    State(state): State<AppState>,
    Path(report_id): Path<String>,
    Json(request): Json<AppendPlanReportRevisionRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tx, rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::AppendPlanReportRevision(
            report_id, request, tx,
        ))
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "status": "error", "message": error.to_string() })),
            )
        })?;
    match rx.await {
        Ok(Ok(report)) => Ok(Json(
            serde_json::json!({ "status": "ok", "report": report }),
        )),
        Ok(Err(error)) => Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "status": "error", "message": error })),
        )),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": error.to_string() })),
        )),
    }
}

/// POST /api/plan-reports/:report_id/approve
pub async fn approve_plan_report_handler(
    State(state): State<AppState>,
    Path(report_id): Path<String>,
    Json(request): Json<ApprovePlanReportRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tx, rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::ApprovePlanReport(report_id, request, tx))
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "status": "error", "message": error.to_string() })),
            )
        })?;
    match rx.await {
        Ok(Ok(execution)) => Ok(Json(
            serde_json::json!({ "status": "ok", "execution": execution }),
        )),
        Ok(Err(error)) => Err((
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "status": "error", "message": error })),
        )),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": error.to_string() })),
        )),
    }
}

/// GET /api/plan-executions/active?session_key=channel:chat_id
pub async fn active_plan_execution_handler(
    State(state): State<AppState>,
    Query(query): Query<ActiveExecutionQuery>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if query.session_key.trim().is_empty() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({"status":"error","message":"session_key is required"})),
        ));
    }
    let (tx, rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::GetActivePlanExecution(
            query.session_key,
            tx,
        ))
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"status":"error","message":error.to_string()})),
            )
        })?;
    match rx.await {
        Ok(Ok(execution)) => Ok(Json(serde_json::json!({
            "status": "ok",
            "execution": execution,
        }))),
        Ok(Err(error)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status":"error","message":error})),
        )),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status":"error","message":error.to_string()})),
        )),
    }
}

/// GET /api/plan-executions/:execution_id/todos
pub async fn list_execution_todos_handler(
    State(state): State<AppState>,
    Path(execution_id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tx, rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::ListExecutionTodos(execution_id, tx))
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"status":"error","message":error.to_string()})),
            )
        })?;
    match rx.await {
        Ok(Ok(todos)) => Ok(Json(serde_json::json!({
            "status": "ok",
            "todos": todos,
        }))),
        Ok(Err(error)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status":"error","message":error})),
        )),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status":"error","message":error.to_string()})),
        )),
    }
}

/// PATCH /api/plan-executions/:execution_id/todos/:todo_id
pub async fn update_execution_todo_handler(
    State(state): State<AppState>,
    Path((execution_id, todo_id)): Path<(String, String)>,
    Json(request): Json<crate::planning_service::UpdateExecutionTodoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tx, rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::UpdateExecutionTodo(
            execution_id,
            todo_id,
            request,
            tx,
        ))
        .await
        .map_err(|error| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"status":"error","message":error.to_string()})),
            )
        })?;
    match rx.await {
        Ok(Ok(todo)) => Ok(Json(serde_json::json!({
            "status": "ok",
            "todo": todo,
        }))),
        Ok(Err(error)) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"status":"error","message":error})),
        )),
        Err(error) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status":"error","message":error.to_string()})),
        )),
    }
}
