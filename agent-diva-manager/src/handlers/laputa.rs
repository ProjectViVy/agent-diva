//! Payload-free recall-feedback listing for the memory surface.
//!
//! The legacy Laputa section/proposal governance HTTP surface was removed by
//! the cognitive clean break (S5). This module only keeps the recall-feedback
//! reader consumed by the desktop memory workspace.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

use crate::state::AppState;

type JsonResult = Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>;

#[derive(Debug, Deserialize)]
pub struct RecallFeedbackQuery {
    pub limit: Option<usize>,
}

pub async fn list_recall_feedback_handler(
    State(state): State<AppState>,
    Query(query): Query<RecallFeedbackQuery>,
) -> JsonResult {
    let storage = agent_diva_laputa::LaputaStorage::open(&state.workspace_root)
        .map_err(laputa_error_response)?;
    let events = agent_diva_laputa::RecallFeedbackStore::new(storage)
        .recent(query.limit.unwrap_or(50).min(200))
        .map_err(laputa_error_response)?;
    Ok(Json(serde_json::json!({
        "status": "ok",
        "feedback": events,
    })))
}

fn laputa_error_response(
    error: agent_diva_laputa::LaputaError,
) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({
            "status": "error",
            "code": error.code(),
            "message": error.to_string(),
        })),
    )
}
