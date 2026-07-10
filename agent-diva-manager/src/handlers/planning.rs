//! REST handlers for the planning API.
//!
//! These handlers translate HTTP requests into [`ManagerCommand`] variants
//! and return JSON responses, following the same pattern as other handlers
//! in this crate.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use tokio::sync::oneshot;

use crate::planning_service::{CreatePlanRequest, UpdatePlanRequest};
use crate::state::{AppState, ManagerCommand};

/// GET /api/plans
pub async fn list_plans_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state.api_tx.send(ManagerCommand::ListPlans(tx)).await {
        tracing::error!("Failed to send ListPlans request: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    match rx.await {
        Ok(Ok(plans)) => Ok(Json(serde_json::json!({ "status": "ok", "plans": plans }))),
        Ok(Err(e)) => {
            tracing::error!("ListPlans failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(e) => {
            tracing::error!("Failed to receive ListPlans response: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// POST /api/plans
pub async fn create_plan_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreatePlanRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)> {
    if payload.title.trim().is_empty() || payload.goal.trim().is_empty() {
        return Err((
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(
                serde_json::json!({ "status": "error", "message": "title and goal are required" }),
            ),
        ));
    }

    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::CreatePlan(payload, tx))
        .await
    {
        tracing::error!("Failed to send CreatePlan request: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        ));
    }
    match rx.await {
        Ok(Ok(plan)) => Ok((
            StatusCode::CREATED,
            Json(serde_json::json!({ "status": "ok", "plan": plan })),
        )),
        Ok(Err(e)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        )),
    }
}

/// GET /api/plans/:plan_id
pub async fn get_plan_handler(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GetPlan(plan_id, tx))
        .await
    {
        tracing::error!("Failed to send GetPlan request: {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    match rx.await {
        Ok(Ok(Some(detail))) => Ok(Json(serde_json::json!({ "status": "ok", "plan": detail }))),
        Ok(Ok(None)) => Err(StatusCode::NOT_FOUND),
        Ok(Err(e)) => {
            tracing::error!("GetPlan failed: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
        Err(e) => {
            tracing::error!("Failed to receive GetPlan response: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// PUT /api/plans/:plan_id
pub async fn update_plan_handler(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
    Json(payload): Json<UpdatePlanRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::UpdatePlan(plan_id, payload, tx))
        .await
    {
        tracing::error!("Failed to send UpdatePlan request: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        ));
    }
    match rx.await {
        Ok(Ok(plan)) => Ok(Json(serde_json::json!({ "status": "ok", "plan": plan }))),
        Ok(Err(e)) => {
            if e.contains("not found") || e.contains("NotFound") {
                Err((
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({ "status": "error", "message": e })),
                ))
            } else {
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({ "status": "error", "message": e })),
                ))
            }
        }
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        )),
    }
}

/// DELETE /api/plans/:plan_id
pub async fn delete_plan_handler(
    State(state): State<AppState>,
    Path(plan_id): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::DeletePlan(plan_id, tx))
        .await
    {
        tracing::error!("Failed to send DeletePlan request: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        ));
    }
    match rx.await {
        Ok(Ok(())) => Ok(StatusCode::NO_CONTENT),
        Ok(Err(e)) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        )),
    }
}

async fn set_plan_todo_handler(
    state: State<AppState>,
    Path((plan_id, todo_id)): Path<(String, String)>,
    restore: bool,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    let (tx, rx) = oneshot::channel();
    let command = if restore {
        ManagerCommand::RestorePlanTodo(plan_id, todo_id, tx)
    } else {
        ManagerCommand::DeletePlanTodo(plan_id, todo_id, tx)
    };
    state.api_tx.send(command).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status":"error","message":e.to_string()})),
        )
    })?;
    match rx.await {
        Ok(Ok(())) => Ok(StatusCode::NO_CONTENT),
        Ok(Err(e)) => Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"status":"error","message":e})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status":"error","message":e.to_string()})),
        )),
    }
}

pub async fn delete_plan_todo_handler(
    state: State<AppState>,
    path: Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    set_plan_todo_handler(state, path, false).await
}

pub async fn restore_plan_todo_handler(
    state: State<AppState>,
    path: Path<(String, String)>,
) -> Result<StatusCode, (StatusCode, Json<serde_json::Value>)> {
    set_plan_todo_handler(state, path, true).await
}

/// POST /api/plans/active/approve-execute
pub async fn approve_active_plan_handler(
    State(state): State<AppState>,
    Json(request): Json<agent_diva_core::planning::ApprovalRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tx, rx) = oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::ApproveActivePlan(request, tx))
        .await
    {
        tracing::error!("Failed to send ApproveActivePlan request: {}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        ));
    }
    match rx.await {
        Ok(Ok(result)) => Ok(Json(serde_json::json!({
            "status": "ok",
            "plan": result.plan,
            "receipt": result.receipt,
        }))),
        Ok(Err(e)) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "status": "error", "message": e })),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "status": "error", "message": e.to_string() })),
        )),
    }
}

pub async fn return_active_plan_to_draft_handler(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    let (tx, rx) = oneshot::channel();
    state
        .api_tx
        .send(ManagerCommand::ReturnActivePlanToDraft(tx))
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"status":"error","message":e.to_string()})),
            )
        })?;
    match rx.await {
        Ok(Ok(plan)) => Ok(Json(serde_json::json!({"status":"ok","plan":plan}))),
        Ok(Err(e)) => Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"status":"error","message":e})),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"status":"error","message":e.to_string()})),
        )),
    }
}
