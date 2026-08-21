//! Token ledger aggregation endpoints for the desktop statistics panel.

use agent_diva_core::token_ledger::{JsonlTokenLedger, TokenLedgerEntry, UsageFilters};
use axum::{
    extract::{Query, State},
    Json, Router,
};
use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PeriodQuery {
    #[serde(default = "default_period")]
    period: String,
    tz_offset: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SummaryQuery {
    #[serde(flatten)]
    period: PeriodQuery,
    #[serde(default = "default_group_by")]
    group_by: String,
}

#[derive(Debug, Deserialize)]
pub struct TimelineQuery {
    #[serde(flatten)]
    period: PeriodQuery,
    interval: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SessionsQuery {
    #[serde(flatten)]
    period: PeriodQuery,
    #[serde(default = "default_limit")]
    limit: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
struct UsageTotal {
    total_input: i64,
    total_output: i64,
    total_tokens: i64,
    total_cache_creation: i64,
    total_cache_read: i64,
    request_count: u64,
    total_cost: f64,
}

#[derive(Debug, Clone, Serialize)]
struct UsageSummary {
    group_key: String,
    #[serde(flatten)]
    usage: UsageTotal,
}

#[derive(Debug, Clone, Serialize)]
struct TimelinePoint {
    time_bucket: String,
    total_input: i64,
    total_output: i64,
    total_tokens: i64,
    request_count: u64,
}

#[derive(Debug, Clone, Serialize)]
struct SessionUsage {
    session_id: String,
    total_input: i64,
    total_output: i64,
    total_tokens: i64,
    request_count: u64,
    total_cost: f64,
    primary_model: String,
    channel: Option<String>,
    last_activity: String,
}

#[derive(Debug, Clone, Serialize)]
struct ModelDistribution {
    model: String,
    percentage: f64,
    total_tokens: i64,
}

fn default_period() -> String {
    "1d".to_string()
}
fn default_group_by() -> String {
    "endpoint".to_string()
}
fn default_limit() -> usize {
    20
}

fn since_for_period(period: &str, tz_offset: Option<String>) -> Result<DateTime<Utc>, String> {
    let tz_offset: i32 = tz_offset.and_then(|s| s.parse().ok()).unwrap_or(0);
    let now = Utc::now();
    let local_now = now - Duration::minutes(tz_offset as i64);
    let local_midnight = local_now
        .date_naive()
        .and_hms_opt(0, 0, 0)
        .expect("valid midnight");
    let utc_midnight =
        (local_midnight.and_utc() + Duration::minutes(tz_offset as i64)).with_timezone(&Utc);

    let duration_from_midnight = match period {
        "1d" => Duration::days(0),
        "3d" => Duration::days(2),
        "1w" => Duration::days(6),
        "1m" => Duration::days(29),
        "6m" => Duration::days(182),
        "1y" => Duration::days(364),
        _ => return Err(format!("invalid period: {period}")),
    };
    Ok(utc_midnight - duration_from_midnight)
}

fn read_entries(
    state: &AppState,
    since: Option<DateTime<Utc>>,
) -> Result<Vec<TokenLedgerEntry>, String> {
    let ledger = JsonlTokenLedger::new(&state.workspace_root.join(".agent-diva"))
        .map_err(|error| format!("failed to open token ledger: {error}"))?;
    ledger
        .read(&UsageFilters {
            since,
            ..Default::default()
        })
        .map_err(|error| format!("failed to read token ledger: {error}"))
}

fn add_usage(total: &mut UsageTotal, entry: &TokenLedgerEntry) {
    total.total_input += i64::from(entry.input_tokens);
    total.total_output += i64::from(entry.output_tokens);
    total.total_tokens += i64::from(entry.total_tokens);
    total.request_count += 1;
    total.total_cost += entry.cost_estimate.unwrap_or(0.0);
}

fn usage_total(entries: &[TokenLedgerEntry]) -> UsageTotal {
    let mut total = UsageTotal::default();
    for entry in entries {
        add_usage(&mut total, entry);
    }
    total
}

fn channel_for_session(session_id: &str) -> Option<String> {
    session_id
        .split_once(':')
        .map(|(channel, _)| channel.to_string())
}

fn endpoint_for_model(model: &str) -> String {
    model
        .split_once('/')
        .map(|(provider, _)| provider.to_string())
        .unwrap_or_else(|| model.to_string())
}

fn summary_key(entry: &TokenLedgerEntry, group_by: &str) -> Result<String, String> {
    match group_by {
        "endpoint" => Ok(endpoint_for_model(&entry.model)),
        "model" => Ok(entry.model.clone()),
        "session" => Ok(entry.session_id.clone()),
        "channel" => {
            Ok(channel_for_session(&entry.session_id).unwrap_or_else(|| "unknown".to_string()))
        }
        "operation_type" => Ok("llm_completion".to_string()),
        _ => Err(format!("invalid group_by: {group_by}")),
    }
}

fn ok<T: Serialize>(data: T) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "data": data }))
}

fn error(message: String) -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "error", "message": message }))
}

pub async fn total_handler(
    State(state): State<AppState>,
    Query(query): Query<PeriodQuery>,
) -> Json<serde_json::Value> {
    match since_for_period(&query.period, query.tz_offset.clone())
        .and_then(|since| read_entries(&state, Some(since)))
    {
        Ok(entries) => ok(usage_total(&entries)),
        Err(message) => error(message),
    }
}

pub async fn summary_handler(
    State(state): State<AppState>,
    Query(query): Query<SummaryQuery>,
) -> Json<serde_json::Value> {
    let result = since_for_period(&query.period.period, query.period.tz_offset.clone())
        .and_then(|since| read_entries(&state, Some(since)))
        .and_then(|entries| {
            let mut grouped: HashMap<String, UsageTotal> = HashMap::new();
            for entry in &entries {
                let key = summary_key(entry, &query.group_by)?;
                add_usage(grouped.entry(key).or_default(), entry);
            }
            let mut summaries: Vec<_> = grouped
                .into_iter()
                .map(|(group_key, usage)| UsageSummary { group_key, usage })
                .collect();
            summaries.sort_by(|left, right| {
                right
                    .usage
                    .total_tokens
                    .cmp(&left.usage.total_tokens)
                    .then_with(|| left.group_key.cmp(&right.group_key))
            });
            Ok(summaries)
        });
    match result {
        Ok(data) => ok(data),
        Err(message) => error(message),
    }
}

pub async fn timeline_handler(
    State(state): State<AppState>,
    Query(query): Query<TimelineQuery>,
) -> Json<serde_json::Value> {
    let interval = query.interval.unwrap_or_else(|| {
        if query.period.period == "1d" {
            "half_hour".to_string()
        } else if query.period.period == "3d" {
            "hour".to_string()
        } else {
            "day".to_string()
        }
    });
    if interval != "half_hour" && interval != "hour" && interval != "day" {
        return error(format!("invalid interval: {interval}"));
    }
    let result = since_for_period(&query.period.period, query.period.tz_offset.clone())
        .and_then(|since| read_entries(&state, Some(since)).map(|entries| (since, entries)))
        .map(|(since, entries)| {
            let mut buckets: HashMap<String, UsageTotal> = HashMap::new();

            // Pre-fill buckets from `since` to `now`
            let now = Utc::now();
            let mut current = if interval == "half_hour" {
                since
                    .with_minute(if since.minute() >= 30 { 30 } else { 0 })
                    .and_then(|value| value.with_second(0))
                    .and_then(|value| value.with_nanosecond(0))
                    .expect("valid UTC half hour")
            } else if interval == "hour" {
                since
                    .with_minute(0)
                    .and_then(|value| value.with_second(0))
                    .and_then(|value| value.with_nanosecond(0))
                    .expect("valid UTC hour")
            } else {
                since
            };

            while current <= now {
                buckets.insert(current.to_rfc3339(), UsageTotal::default());
                current = if interval == "half_hour" {
                    current + Duration::minutes(30)
                } else if interval == "hour" {
                    current + Duration::hours(1)
                } else {
                    current + Duration::days(1)
                };
            }

            for entry in &entries {
                let bucket = if interval == "half_hour" {
                    entry
                        .timestamp
                        .with_minute(if entry.timestamp.minute() >= 30 {
                            30
                        } else {
                            0
                        })
                        .and_then(|value| value.with_second(0))
                        .and_then(|value| value.with_nanosecond(0))
                        .expect("valid UTC half hour")
                } else if interval == "hour" {
                    entry
                        .timestamp
                        .with_minute(0)
                        .and_then(|value| value.with_second(0))
                        .and_then(|value| value.with_nanosecond(0))
                        .expect("valid UTC hour")
                } else {
                    let tz_offset: i32 = query
                        .period
                        .tz_offset
                        .as_deref()
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let local_ts = entry.timestamp - Duration::minutes(tz_offset as i64);
                    let local_midnight = local_ts
                        .date_naive()
                        .and_hms_opt(0, 0, 0)
                        .expect("valid midnight");
                    (local_midnight.and_utc() + Duration::minutes(tz_offset as i64))
                        .with_timezone(&Utc)
                };
                add_usage(buckets.entry(bucket.to_rfc3339()).or_default(), entry);
            }
            let mut points: Vec<_> = buckets
                .into_iter()
                .map(|(time_bucket, usage)| TimelinePoint {
                    time_bucket,
                    total_input: usage.total_input,
                    total_output: usage.total_output,
                    total_tokens: usage.total_tokens,
                    request_count: usage.request_count,
                })
                .collect();
            points.sort_by_key(|point| point.time_bucket.clone());
            points
        });
    match result {
        Ok(data) => ok(data),
        Err(message) => error(message),
    }
}

pub async fn sessions_handler(
    State(state): State<AppState>,
    Query(query): Query<SessionsQuery>,
) -> Json<serde_json::Value> {
    let result = since_for_period(&query.period.period, query.period.tz_offset.clone())
        .and_then(|since| read_entries(&state, Some(since)))
        .map(|entries| {
            let mut grouped: HashMap<String, Vec<TokenLedgerEntry>> = HashMap::new();
            for entry in entries {
                grouped
                    .entry(entry.session_id.clone())
                    .or_default()
                    .push(entry);
            }
            let mut sessions: Vec<_> = grouped
                .into_iter()
                .map(|(session_id, entries)| {
                    let usage = usage_total(&entries);
                    let mut models: HashMap<&str, i64> = HashMap::new();
                    for entry in &entries {
                        *models.entry(&entry.model).or_default() += i64::from(entry.total_tokens);
                    }
                    let primary_model = models
                        .into_iter()
                        .max_by(|left, right| {
                            left.1.cmp(&right.1).then_with(|| right.0.cmp(left.0))
                        })
                        .map(|(model, _)| model.to_string())
                        .unwrap_or_default();
                    let last_activity = entries
                        .iter()
                        .map(|entry| entry.timestamp)
                        .max()
                        .expect("non-empty session entries")
                        .to_rfc3339();
                    SessionUsage {
                        session_id: session_id.clone(),
                        total_input: usage.total_input,
                        total_output: usage.total_output,
                        total_tokens: usage.total_tokens,
                        request_count: usage.request_count,
                        total_cost: usage.total_cost,
                        primary_model,
                        channel: channel_for_session(&session_id),
                        last_activity,
                    }
                })
                .collect();
            sessions.sort_by(|left, right| {
                right
                    .last_activity
                    .cmp(&left.last_activity)
                    .then_with(|| right.total_tokens.cmp(&left.total_tokens))
            });
            sessions.truncate(query.limit.clamp(1, 100));
            sessions
        });
    match result {
        Ok(data) => ok(data),
        Err(message) => error(message),
    }
}

pub async fn models_handler(
    State(state): State<AppState>,
    Query(query): Query<PeriodQuery>,
) -> Json<serde_json::Value> {
    let result = since_for_period(&query.period, query.tz_offset.clone())
        .and_then(|since| read_entries(&state, Some(since)))
        .map(|entries| {
            let total = entries
                .iter()
                .map(|entry| i64::from(entry.total_tokens))
                .sum::<i64>();
            let mut grouped: HashMap<String, i64> = HashMap::new();
            for entry in entries {
                *grouped.entry(entry.model).or_default() += i64::from(entry.total_tokens);
            }
            let mut models: Vec<_> = grouped
                .into_iter()
                .map(|(model, total_tokens)| ModelDistribution {
                    model,
                    percentage: if total == 0 {
                        0.0
                    } else {
                        total_tokens as f64 * 100.0 / total as f64
                    },
                    total_tokens,
                })
                .collect();
            models.sort_by(|left, right| {
                right
                    .total_tokens
                    .cmp(&left.total_tokens)
                    .then_with(|| left.model.cmp(&right.model))
            });
            models
        });
    match result {
        Ok(data) => ok(data),
        Err(message) => error(message),
    }
}

pub async fn realtime_handler(State(state): State<AppState>) -> Json<serde_json::Value> {
    match read_entries(&state, None) {
        Ok(entries) => ok(usage_total(&entries)),
        Err(message) => error(message),
    }
}

pub fn token_stats_routes() -> Router<AppState> {
    Router::new()
        .route("/api/stats/tokens/total", axum::routing::get(total_handler))
        .route(
            "/api/stats/tokens/summary",
            axum::routing::get(summary_handler),
        )
        .route(
            "/api/stats/tokens/timeline",
            axum::routing::get(timeline_handler),
        )
        .route(
            "/api/stats/tokens/sessions",
            axum::routing::get(sessions_handler),
        )
        .route(
            "/api/stats/tokens/models",
            axum::routing::get(models_handler),
        )
        .route(
            "/api/stats/tokens/realtime",
            axum::routing::get(realtime_handler),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::bus::MessageBus;
    use axum::{body::to_bytes, body::Body, http::Request};
    use chrono::Duration;
    use tokio::sync::mpsc;
    use tower::util::ServiceExt;

    fn test_app() -> (tempfile::TempDir, Router) {
        let temp = tempfile::tempdir().unwrap();
        let (api_tx, _api_rx) = mpsc::channel(1);
        let state = AppState::new(api_tx, MessageBus::new(), temp.path()).unwrap();
        let ledger = JsonlTokenLedger::new(&temp.path().join(".agent-diva")).unwrap();

        let mut first = TokenLedgerEntry::new("gui:one", "openai/gpt-test", 10, 5).with_cost(0.02);
        first.timestamp = Utc::now() - Duration::minutes(20);
        ledger.append(first).unwrap();
        let mut second = TokenLedgerEntry::new("telegram:two", "deepseek-chat", 7, 3);
        second.timestamp = Utc::now() - Duration::minutes(10);
        ledger.append(second).unwrap();
        (temp, token_stats_routes().with_state(state))
    }

    async fn response_json(app: Router, uri: &str) -> serde_json::Value {
        let response = app
            .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
            .await
            .unwrap();
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
    }

    #[tokio::test]
    async fn token_stat_routes_aggregate_ledger_entries() {
        let (_temp, app) = test_app();
        let total = response_json(app.clone(), "/api/stats/tokens/total?period=1d").await;
        assert_eq!(total["status"], "ok");
        assert_eq!(total["data"]["total_input"], 17);
        assert_eq!(total["data"]["total_output"], 8);
        assert_eq!(total["data"]["total_tokens"], 25);
        assert_eq!(total["data"]["request_count"], 2);
        assert_eq!(total["data"]["total_cost"], 0.02);

        let summary = response_json(
            app.clone(),
            "/api/stats/tokens/summary?period=1d&group_by=channel",
        )
        .await;
        assert_eq!(summary["data"].as_array().unwrap().len(), 2);
        let models = response_json(app.clone(), "/api/stats/tokens/models?period=1d").await;
        assert_eq!(models["data"][0]["model"], "openai/gpt-test");
        let sessions =
            response_json(app.clone(), "/api/stats/tokens/sessions?period=1d&limit=1").await;
        assert_eq!(sessions["data"].as_array().unwrap().len(), 1);
        let timeline = response_json(
            app.clone(),
            "/api/stats/tokens/timeline?period=1d&interval=hour",
        )
        .await;
        assert!(!timeline["data"].as_array().unwrap().is_empty());
        let realtime = response_json(app, "/api/stats/tokens/realtime").await;
        assert_eq!(realtime["data"]["total_tokens"], 25);
    }

    #[tokio::test]
    async fn token_stat_routes_report_invalid_query_parameters() {
        let (_temp, app) = test_app();
        let invalid_period =
            response_json(app.clone(), "/api/stats/tokens/total?period=tomorrow").await;
        assert_eq!(invalid_period["status"], "error");
        let invalid_group = response_json(app, "/api/stats/tokens/summary?group_by=provider").await;
        assert_eq!(invalid_group["status"], "error");
    }
}
