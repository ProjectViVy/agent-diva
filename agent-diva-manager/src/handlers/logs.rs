//! Log query HTTP endpoint for the manager API.
//!
//! Provides `GET /api/logs` that scans `audit-*.jsonl` files and returns
//! filtered, paginated audit events.

use axum::{
    extract::{Query, State},
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::warn;

use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

/// Query parameters for `GET /api/logs`.
#[derive(Debug, Clone, Deserialize)]
pub struct LogQuery {
    /// Filter by event type (matches the `type` field in JSON).
    pub event_type: Option<String>,
    /// Relative time window: `1h`, `6h`, `24h`, `7d`. Defaults to `24h`.
    #[serde(default = "default_range")]
    pub range: String,
    /// Maximum events to return. Defaults to 100, capped at 1000.
    #[serde(default = "default_limit")]
    pub limit: usize,
    /// Opaque pagination cursor.
    pub cursor: Option<String>,
}

fn default_range() -> String {
    "24h".to_string()
}

fn default_limit() -> usize {
    100
}

/// Response body for `GET /api/logs`.
#[derive(Debug, Clone, Serialize)]
pub struct LogQueryResponse {
    pub events: Vec<serde_json::Value>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LogCursor {
    last_timestamp: String,
    skipped: usize,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ScanStats {
    malformed_lines: usize,
    unknown_events: usize,
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Resolve the data root where `audit-*.jsonl` files are stored.
fn resolve_data_root(state: &AppState) -> PathBuf {
    state.audit_root.clone()
}

/// Parse a range string into a `Duration`.
fn parse_range(range: &str) -> Option<Duration> {
    match range {
        "1h" => Some(Duration::hours(1)),
        "6h" => Some(Duration::hours(6)),
        "24h" => Some(Duration::hours(24)),
        "7d" => Some(Duration::days(7)),
        _ => None,
    }
}

/// Build a cursor from the last event's timestamp and the number of events skipped.
fn build_cursor(last_ts: &str, skipped: usize) -> String {
    use base64::{engine::general_purpose, Engine};
    let cursor = LogCursor {
        last_timestamp: last_ts.to_string(),
        skipped,
    };
    general_purpose::URL_SAFE_NO_PAD.encode(serde_json::to_vec(&cursor).unwrap_or_default())
}

/// Parse a cursor into (last_timestamp, skipped_count).
fn parse_cursor(cursor: &str) -> Result<LogCursor, String> {
    use base64::{engine::general_purpose, Engine};
    let decoded = general_purpose::URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| "invalid cursor encoding".to_string())?;
    serde_json::from_slice(&decoded).map_err(|_| "invalid cursor payload".to_string())
}

/// Extract timestamp from a JSON event value.
///
/// Tries `event_time`, then `timestamp`, then falls back to the current time.
fn extract_timestamp(value: &serde_json::Value) -> Option<DateTime<Utc>> {
    if let Some(ts_str) = value.get("event_time").and_then(|v| v.as_str()) {
        return DateTime::parse_from_rfc3339(ts_str)
            .ok()
            .map(|dt| dt.with_timezone(&Utc));
    }
    if let Some(ts_str) = value.get("timestamp").and_then(|v| v.as_str()) {
        return DateTime::parse_from_rfc3339(ts_str)
            .ok()
            .map(|dt| dt.with_timezone(&Utc));
    }
    None
}

/// Scan `audit-*.jsonl` files and return filtered events.
fn scan_audit_files(
    data_root: &std::path::Path,
    event_type_filter: Option<&str>,
    since: DateTime<Utc>,
    limit: usize,
    cursor: Option<&str>,
) -> Result<(Vec<serde_json::Value>, Option<String>, ScanStats), String> {
    if !data_root.exists() {
        return Ok((Vec::new(), None, ScanStats::default()));
    }

    let mut files: Vec<std::fs::DirEntry> = std::fs::read_dir(data_root)
        .map_err(|e| format!("failed to read data root: {}", e))?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name();
            let name = name.to_string_lossy();
            name.starts_with("audit-") && name.ends_with(".jsonl")
        })
        .collect();

    // Sort by filename descending (newest first: audit-YYYY-MM-DD.jsonl)
    files.sort_by(|a, b| {
        b.file_name()
            .to_string_lossy()
            .cmp(&a.file_name().to_string_lossy())
    });

    let mut events = Vec::new();
    let mut total_skipped = 0usize;
    let mut stats = ScanStats::default();

    // If cursor provided, parse it to know how many to skip
    let skip_count = match cursor {
        Some(cursor) => parse_cursor(cursor)?.skipped,
        None => 0,
    };

    for entry in files {
        let path = entry.path();
        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(_) => continue,
        };
        let reader = std::io::BufReader::new(file);

        for line_result in std::io::BufRead::lines(reader) {
            let line = match line_result {
                Ok(l) => l,
                Err(_) => {
                    stats.malformed_lines += 1;
                    continue;
                }
            };

            let value: serde_json::Value = match serde_json::from_str(&line) {
                Ok(v) => v,
                Err(_) => {
                    stats.malformed_lines += 1;
                    continue;
                }
            };

            // Filter by event_type if provided
            if let Some(filter) = event_type_filter {
                let matches = value
                    .get("type")
                    .and_then(|t| t.as_str())
                    .map(|t| t == filter)
                    .unwrap_or(false);
                if !matches {
                    continue;
                }
            }

            // Filter by timestamp
            let ts = extract_timestamp(&value).unwrap_or_else(Utc::now);
            if ts < since {
                continue;
            }

            if value.get("type").and_then(|t| t.as_str()).is_none()
                && value
                    .get("fields")
                    .and_then(|fields| fields.get("event").or_else(|| fields.get("message")))
                    .and_then(|event| event.as_str())
                    .is_none()
            {
                stats.unknown_events += 1;
            }

            // Handle cursor-based skip
            if total_skipped < skip_count {
                total_skipped += 1;
                continue;
            }

            events.push(value);

            if events.len() >= limit {
                break;
            }
        }

        if events.len() >= limit {
            break;
        }
    }

    // Build next_cursor if we may have more events
    let next_cursor = if events.len() >= limit {
        // Use the last event's timestamp as the cursor anchor
        let last_ts = events
            .last()
            .and_then(extract_timestamp)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| Utc::now().to_rfc3339());
        let new_skipped = skip_count + events.len();
        Some(build_cursor(&last_ts, new_skipped))
    } else {
        None
    };

    Ok((events, next_cursor, stats))
}

// ── Handlers ──────────────────────────────────────────────────────────

/// GET /api/logs
///
/// Query audit events from `audit-*.jsonl` files with optional filtering
/// and pagination.
pub async fn query_logs_handler(
    State(state): State<AppState>,
    Query(params): Query<LogQuery>,
) -> Json<serde_json::Value> {
    let data_root = resolve_data_root(&state);

    // Validate and cap limit
    let limit = params.limit.clamp(1, 1000);

    // Parse range
    let range_duration = match parse_range(&params.range) {
        Some(d) => d,
        None => {
            return Json(serde_json::json!({
                "status": "error",
                "message": format!("invalid range: {}", params.range),
            }));
        }
    };
    let since = Utc::now() - range_duration;

    let event_type = params.event_type.as_deref();
    let cursor = params.cursor.as_deref();

    match scan_audit_files(&data_root, event_type, since, limit, cursor) {
        Ok((events, next_cursor, stats)) => {
            if stats.malformed_lines > 0 || stats.unknown_events > 0 {
                warn!(
                    malformed_lines = stats.malformed_lines,
                    unknown_events = stats.unknown_events,
                    "scan_audit_files skipped malformed audit lines or observed schema drift"
                );
            }
            Json(serde_json::json!({
            "status": "ok",
            "events": events,
            "next_cursor": next_cursor,
            }))
        }
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e,
        })),
    }
}

/// Build the router for log query routes.
pub fn logs_routes() -> Router<AppState> {
    Router::new().route("/api/logs", axum::routing::get(query_logs_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::bus::MessageBus;
    use axum::body::to_bytes;
    use axum::http::{Request, StatusCode};
    use axum::Router;
    use tokio::sync::mpsc;
    use tower::util::ServiceExt;

    fn test_app_with_dir(dir: &std::path::Path) -> Router {
        let (api_tx, _api_rx) = mpsc::channel(1);
        let state = AppState::new(api_tx, MessageBus::new(), dir).unwrap();
        Router::new()
            .route("/api/logs", axum::routing::get(query_logs_handler))
            .with_state(state)
    }

    #[tokio::test]
    async fn logs_empty_when_no_audit_files() {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_dir(temp.path());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/logs")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
        assert!(value["events"].as_array().unwrap().is_empty());
        assert!(value["next_cursor"].is_null());
    }

    #[test]
    fn scan_audit_files_counts_malformed_and_unknown_lines() {
        let temp = tempfile::tempdir().unwrap();
        let audit_dir = temp.path().join(".agent-diva").join("audit");
        std::fs::create_dir_all(&audit_dir).unwrap();
        let audit_file = audit_dir.join(format!("audit-{}.jsonl", Utc::now().format("%Y-%m-%d")));
        let valid_ts = (Utc::now() - Duration::minutes(1)).to_rfc3339();
        std::fs::write(
            &audit_file,
            format!(
                "{{\"type\":\"tool_invoked\",\"data\":{{}},\"timestamp\":\"{}\"}}\nnot-json\n{{\"timestamp\":\"{}\",\"level\":\"INFO\",\"message\":\"drifted\"}}\n",
                valid_ts, valid_ts
            ),
        )
        .unwrap();

        let (events, next_cursor, stats) =
            scan_audit_files(&audit_dir, None, Utc::now() - Duration::hours(1), 10, None).unwrap();

        assert_eq!(events.len(), 2);
        assert!(next_cursor.is_none());
        assert_eq!(stats.malformed_lines, 1);
        assert_eq!(stats.unknown_events, 1);
    }

    #[tokio::test]
    async fn logs_filter_by_event_type() {
        let temp = tempfile::tempdir().unwrap();
        let first_ts = (Utc::now() - Duration::minutes(10)).to_rfc3339();
        let second_ts = (Utc::now() - Duration::minutes(5)).to_rfc3339();
        // Write an mock audit file with two events
        let audit_file = temp
            .path()
            .join(".agent-diva")
            .join("audit")
            .join(format!("audit-{}.jsonl", Utc::now().format("%Y-%m-%d")));
        std::fs::create_dir_all(audit_file.parent().unwrap()).unwrap();
        std::fs::write(
            &audit_file,
            format!(
                "{{\"type\":\"tool_invoked\",\"data\":{{\"tool\":\"bash\"}},\"timestamp\":\"{}\"}}\n{{\"type\":\"heartbeat_triggered\",\"data\":{{\"interval_secs\":5}},\"timestamp\":\"{}\"}}\n",
                first_ts, second_ts
            ),
        )
        .unwrap();

        let app = test_app_with_dir(temp.path());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/logs?event_type=tool_invoked")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["events"].as_array().unwrap().len(), 1);
        assert_eq!(value["events"][0]["type"], "tool_invoked");
    }

    #[tokio::test]
    async fn logs_filter_by_range() {
        let temp = tempfile::tempdir().unwrap();
        // Write an event with a very old timestamp
        let audit_file = temp
            .path()
            .join(".agent-diva")
            .join("audit")
            .join(format!("audit-{}.jsonl", Utc::now().format("%Y-%m-%d")));
        std::fs::create_dir_all(audit_file.parent().unwrap()).unwrap();
        std::fs::write(
            &audit_file,
            r#"{"type":"tool_invoked","data":{"tool":"bash"},"timestamp":"2020-01-01T00:00:00Z"}
"#,
        )
        .unwrap();

        let app = test_app_with_dir(temp.path());

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/logs?range=1h")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // Old event should be filtered out by the 1h range
        assert!(value["events"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn logs_pagination_with_cursor() {
        let temp = tempfile::tempdir().unwrap();
        let ts1 = (Utc::now() - Duration::minutes(3)).to_rfc3339();
        let ts2 = (Utc::now() - Duration::minutes(2)).to_rfc3339();
        let ts3 = (Utc::now() - Duration::minutes(1)).to_rfc3339();
        let audit_file = temp
            .path()
            .join(".agent-diva")
            .join("audit")
            .join(format!("audit-{}.jsonl", Utc::now().format("%Y-%m-%d")));
        std::fs::create_dir_all(audit_file.parent().unwrap()).unwrap();
        // Write 3 events
        std::fs::write(
            &audit_file,
            format!(
                "{{\"type\":\"event1\",\"data\":{{}},\"timestamp\":\"{}\"}}\n{{\"type\":\"event2\",\"data\":{{}},\"timestamp\":\"{}\"}}\n{{\"type\":\"event3\",\"data\":{{}},\"timestamp\":\"{}\"}}\n",
                ts1, ts2, ts3
            ),
        )
        .unwrap();

        let app = test_app_with_dir(temp.path());

        // First page: limit=1
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/logs?limit=1")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["events"].as_array().unwrap().len(), 1);
        assert!(value["next_cursor"].is_string());

        // Second page: use cursor
        let cursor = value["next_cursor"].as_str().unwrap();
        let response2 = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(&format!("/api/logs?limit=1&cursor={}", cursor))
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response2.status(), StatusCode::OK);
        let body2 = to_bytes(response2.into_body(), usize::MAX).await.unwrap();
        let value2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
        assert_eq!(value2["events"].as_array().unwrap().len(), 1);
        assert_eq!(value2["events"][0]["type"], "event2");
    }

    #[tokio::test]
    async fn logs_returns_error_for_invalid_cursor_payload() {
        let temp = tempfile::tempdir().unwrap();
        let app = test_app_with_dir(temp.path());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/logs?cursor=not-base64")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["message"], "invalid cursor encoding");
    }

    #[tokio::test]
    async fn logs_preserve_unknown_schema_drift_events() {
        let temp = tempfile::tempdir().unwrap();
        let audit_file = temp
            .path()
            .join(".agent-diva")
            .join("audit")
            .join(format!("audit-{}.jsonl", Utc::now().format("%Y-%m-%d")));
        std::fs::create_dir_all(audit_file.parent().unwrap()).unwrap();
        let ts = (Utc::now() - Duration::minutes(2)).to_rfc3339();
        std::fs::write(
            &audit_file,
            format!(
                "{{\"timestamp\":\"{}\",\"level\":\"INFO\",\"message\":\"unexpected-shape\"}}\n",
                ts
            ),
        )
        .unwrap();

        let app = test_app_with_dir(temp.path());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/logs")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
        assert_eq!(value["events"].as_array().unwrap().len(), 1);
        assert_eq!(value["events"][0]["timestamp"], ts);
        assert!(value["events"][0]["type"].is_null());
    }
}
