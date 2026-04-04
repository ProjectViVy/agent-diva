//! Token usage statistics API handlers

use agent_diva_core::usage::{GroupBy, TimeInterval, TimeRange, UsageSummary};
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::state::{AppState, ManagerCommand};

/// Query parameters for summary endpoint
#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    /// Time range: 1d, 3d, 1w, 1m, 6m, 1y
    #[serde(default = "default_period")]
    pub period: String,
    /// Grouping dimension: endpoint, model, operation_type, session, channel
    #[serde(default = "default_group_by")]
    pub group_by: String,
    /// Optional start date (ISO 8601)
    pub start: Option<String>,
    /// Optional end date (ISO 8601)
    pub end: Option<String>,
}

fn default_period() -> String {
    "1d".to_string()
}

fn default_group_by() -> String {
    "endpoint".to_string()
}

/// Query parameters for timeline endpoint
#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    /// Time range
    #[serde(default = "default_period")]
    pub period: String,
    /// Time interval: hour, day
    #[serde(default)]
    pub interval: Option<String>,
}

/// Query parameters for sessions endpoint
#[derive(Debug, Deserialize)]
pub struct SessionsQuery {
    /// Time range
    #[serde(default = "default_period")]
    pub period: String,
    /// Maximum number of sessions to return
    #[serde(default = "default_limit")]
    pub limit: u64,
}

fn default_limit() -> u64 {
    20
}

/// Response wrapper for summary data
#[derive(Debug, Serialize)]
pub struct SummaryResponse {
    pub start: String,
    pub end: String,
    pub group_by: String,
    pub data: Vec<UsageSummary>,
}

/// Response wrapper for timeline data
#[derive(Debug, Serialize)]
pub struct TimelineResponse {
    pub start: String,
    pub end: String,
    pub interval: String,
    pub data: Vec<agent_diva_core::usage::types::TimelinePoint>,
}

/// Response for model distribution
#[derive(Debug, Serialize)]
pub struct ModelDistributionResponse {
    pub period: String,
    pub distribution: Vec<ModelDistributionEntry>,
}

#[derive(Debug, Serialize)]
pub struct ModelDistributionEntry {
    pub model: String,
    pub percentage: f64,
    pub total_tokens: i64,
}

/// In-memory stats response (real-time)
#[derive(Debug, Serialize)]
pub struct RealTimeStatsResponse {
    pub current_session: InMemoryStatsDto,
}

#[derive(Debug, Serialize)]
pub struct InMemoryStatsDto {
    pub total_tokens: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub request_count: u64,
    pub total_cost: f64,
}

/// Get total token usage statistics
pub async fn get_token_usage_total(
    State(state): State<AppState>,
    Query(query): Query<SummaryQuery>,
) -> Json<serde_json::Value> {
    let range = parse_time_range(&query.period, query.start.as_deref(), query.end.as_deref());

    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GetTokenUsageTotal(range, tx))
        .await
    {
        tracing::error!("Failed to send GetTokenUsageTotal request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(total)) => Json(serde_json::json!({ "status": "ok", "data": total })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GetTokenUsageTotal response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

/// Get token usage summary grouped by dimension
pub async fn get_token_usage_summary(
    State(state): State<AppState>,
    Query(query): Query<SummaryQuery>,
) -> Json<serde_json::Value> {
    let range = parse_time_range(&query.period, query.start.as_deref(), query.end.as_deref());
    let group_by = GroupBy::parse_dimension(&query.group_by).unwrap_or_default();

    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GetTokenUsageSummary(range, group_by, tx))
        .await
    {
        tracing::error!("Failed to send GetTokenUsageSummary request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(summary)) => Json(serde_json::json!({ "status": "ok", "data": summary })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GetTokenUsageSummary response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

/// Get token usage timeline for charting
pub async fn get_token_usage_timeline(
    State(state): State<AppState>,
    Query(query): Query<TimelineQuery>,
) -> Json<serde_json::Value> {
    let range = parse_time_range(&query.period, None, None);
    let interval = query
        .interval
        .as_deref()
        .and_then(|s| match s.to_lowercase().as_str() {
            "hour" => Some(TimeInterval::Hour),
            "day" => Some(TimeInterval::Day),
            _ => None,
        })
        .unwrap_or_else(|| range.timeline_interval());

    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GetTokenUsageTimeline(range, interval, tx))
        .await
    {
        tracing::error!("Failed to send GetTokenUsageTimeline request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(timeline)) => Json(serde_json::json!({ "status": "ok", "data": timeline })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GetTokenUsageTimeline response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

/// Get session-level token usage details
pub async fn get_token_usage_sessions(
    State(state): State<AppState>,
    Query(query): Query<SessionsQuery>,
) -> Json<serde_json::Value> {
    let range = parse_time_range(&query.period, None, None);

    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GetTokenUsageSessions(
            range,
            query.limit,
            tx,
        ))
        .await
    {
        tracing::error!("Failed to send GetTokenUsageSessions request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(sessions)) => Json(serde_json::json!({ "status": "ok", "data": sessions })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GetTokenUsageSessions response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

/// Get model distribution
pub async fn get_token_usage_models(
    State(state): State<AppState>,
    Query(query): Query<SummaryQuery>,
) -> Json<serde_json::Value> {
    let range = parse_time_range(&query.period, None, None);

    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GetTokenUsageModels(range, tx))
        .await
    {
        tracing::error!("Failed to send GetTokenUsageModels request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(distribution)) => {
            let entries: Vec<ModelDistributionEntry> = distribution
                .into_iter()
                .map(|(model, percentage)| ModelDistributionEntry {
                    model,
                    percentage,
                    total_tokens: 0, // Would need additional query
                })
                .collect();
            Json(serde_json::json!({ "status": "ok", "data": entries }))
        }
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GetTokenUsageModels response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

/// Get real-time in-memory statistics
pub async fn get_token_usage_realtime(State(state): State<AppState>) -> Json<serde_json::Value> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    if let Err(e) = state
        .api_tx
        .send(ManagerCommand::GetTokenUsageRealtime(tx))
        .await
    {
        tracing::error!("Failed to send GetTokenUsageRealtime request: {}", e);
        return Json(serde_json::json!({ "status": "error", "message": e.to_string() }));
    }

    match rx.await {
        Ok(Ok(stats)) => Json(serde_json::json!({ "status": "ok", "data": stats })),
        Ok(Err(e)) => Json(serde_json::json!({ "status": "error", "message": e })),
        Err(e) => {
            tracing::error!("Failed to receive GetTokenUsageRealtime response: {}", e);
            Json(serde_json::json!({ "status": "error", "message": e.to_string() }))
        }
    }
}

/// Parse time range from query parameters
fn parse_time_range(period: &str, start: Option<&str>, end: Option<&str>) -> TimeRange {
    // If both start and end are provided, use custom range
    if let (Some(start_str), Some(end_str)) = (start, end) {
        let start_dt = chrono::DateTime::parse_from_rfc3339(start_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok();
        let end_dt = chrono::DateTime::parse_from_rfc3339(end_str)
            .map(|dt| dt.with_timezone(&chrono::Utc))
            .ok();

        if let (Some(start), Some(end)) = (start_dt, end_dt) {
            return TimeRange::Custom { start, end };
        }
    }

    // Use period string
    TimeRange::parse_period(period).unwrap_or_default()
}
