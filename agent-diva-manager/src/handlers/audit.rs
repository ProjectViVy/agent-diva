//! Audit log HTTP endpoints for the manager API.
//!
//! Provides read-only access to the gateway audit log written by
//! `agent-diva-core::logging` via `tracing_appender::rolling::daily`.

use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::state::AppState;

// ── DTOs ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventDto {
    pub event_type: String,
    pub data: serde_json::Value,
    pub timestamp: String,
}

impl From<agent_diva_core::audit_parse::AuditEventDto> for AuditEventDto {
    fn from(dto: agent_diva_core::audit_parse::AuditEventDto) -> Self {
        Self {
            event_type: dto.event_type,
            data: dto.data,
            timestamp: dto.timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogResponse {
    pub date: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEventsResponse {
    pub date: String,
    pub events: Vec<AuditEventDto>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AuditQuery {
    /// Date in YYYY-MM-DD format. Defaults to today (UTC).
    pub date: Option<String>,
    /// Maximum number of lines/events to return (default 200).
    #[serde(default)]
    pub max_lines: Option<u64>,
}

// ── Helpers ───────────────────────────────────────────────────────────

/// Resolve the log directory from config, mirroring the logic in
/// `agent-diva-gui`'s `resolve_configured_path`.
fn resolve_log_dir() -> PathBuf {
    let loader = agent_diva_core::config::ConfigLoader::new();
    let config = loader.load().unwrap_or_default();
    let dir_str = &config.logging.dir;
    let expanded = expand_user_path(dir_str);
    if expanded.is_absolute() {
        expanded
    } else {
        loader.config_dir().join(expanded)
    }
}

/// Expand `~` at the start of a path to the home directory.
fn expand_user_path(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix('~') {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest.trim_start_matches(std::path::MAIN_SEPARATOR));
        }
    }
    PathBuf::from(path)
}

/// Today's date in YYYY-MM-DD format (UTC).
fn today_utc() -> String {
    chrono::Utc::now().format("%Y-%m-%d").to_string()
}

/// Read raw log lines from a daily-rolled gateway.log file.
fn read_log_lines(date: &str, max_lines: u64) -> Result<Vec<String>, String> {
    let log_dir = resolve_log_dir();
    // tracing_appender::rolling::daily produces: gateway.log.YYYY-MM-DD
    let log_path = log_dir.join(format!("gateway.log.{}", date));

    if !log_path.exists() {
        // Also try the base file (today's current log before rotation)
        let base_path = log_dir.join("gateway.log");
        if base_path.exists() && date == today_utc().as_str() {
            return read_lines_from_file(&base_path, max_lines);
        }
        return Err(format!("No audit log found for date {}", date));
    }

    read_lines_from_file(&log_path, max_lines)
}

fn read_lines_from_file(path: &PathBuf, max_lines: u64) -> Result<Vec<String>, String> {
    let content =
        std::fs::read_to_string(path).map_err(|e| format!("failed to read log file: {}", e))?;

    let mut all_lines: Vec<String> = content.lines().map(ToString::to_string).collect();
    let keep = max_lines.max(1) as usize;
    if all_lines.len() > keep {
        all_lines = all_lines.split_off(all_lines.len().saturating_sub(keep));
    }
    Ok(all_lines)
}

// ── Handlers ──────────────────────────────────────────────────────────

/// GET /api/audit/log?date=YYYY-MM-DD&max_lines=200
///
/// Returns raw log lines for the given date.
pub async fn get_audit_log_handler(
    State(_state): State<AppState>,
    Query(query): Query<AuditQuery>,
) -> Json<serde_json::Value> {
    let date = query.date.unwrap_or_else(today_utc);
    let max_lines = query.max_lines.unwrap_or(200);

    match read_log_lines(&date, max_lines) {
        Ok(lines) => Json(serde_json::json!({
            "status": "ok",
            "date": date,
            "lines": lines,
        })),
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e,
        })),
    }
}

/// GET /api/audit/events?date=YYYY-MM-DD&max_lines=200
///
/// Returns parsed audit events for the given date.
pub async fn get_audit_events_handler(
    State(_state): State<AppState>,
    Query(query): Query<AuditQuery>,
) -> Json<serde_json::Value> {
    let date = query.date.unwrap_or_else(today_utc);
    let max_lines = query.max_lines.unwrap_or(200);

    match read_log_lines(&date, max_lines) {
        Ok(lines) => {
            let events: Vec<AuditEventDto> = lines
                .iter()
                .filter_map(|line| {
                    agent_diva_core::audit_parse::parse_audit_event_from_json_line(line)
                        .map(AuditEventDto::from)
                })
                .collect();
            Json(serde_json::json!({
                "status": "ok",
                "date": date,
                "events": events,
            }))
        }
        Err(e) => Json(serde_json::json!({
            "status": "error",
            "message": e,
        })),
    }
}

#[cfg(test)]
mod tests {
    use agent_diva_core::audit_parse;

    #[test]
    fn rust_variant_to_snake_case_converts_correctly() {
        assert_eq!(
            audit_parse::rust_variant_to_snake_case("ToolInvoked"),
            "tool_invoked"
        );
        assert_eq!(
            audit_parse::rust_variant_to_snake_case("SessionCreated"),
            "session_created"
        );
        assert_eq!(audit_parse::rust_variant_to_snake_case("Error"), "error");
        assert_eq!(
            audit_parse::rust_variant_to_snake_case("lowercase"),
            "lowercase"
        );
    }

    #[test]
    fn parse_structured_tracing_json() {
        let line = r#"{"timestamp":"2026-07-01T12:00:00Z","level":"INFO","target":"audit","fields":{"event":"ToolInvoked","tool":"shell"}}"#;
        let evt = audit_parse::parse_audit_event_from_json_line(line).unwrap();
        assert_eq!(evt.event_type, "tool_invoked");
        assert_eq!(evt.timestamp, "2026-07-01T12:00:00Z");
    }

    #[test]
    fn parse_direct_format() {
        let line =
            r#"{"timestamp":"2026-07-01T12:00:00Z","type":"tool_invoked","data":{"tool":"shell"}}"#;
        let evt = audit_parse::parse_audit_event_from_json_line(line).unwrap();
        assert_eq!(evt.event_type, "tool_invoked");
        assert_eq!(evt.data["tool"], "shell");
    }

    #[test]
    fn parse_non_json_returns_none() {
        assert!(audit_parse::parse_audit_event_from_json_line("not json").is_none());
    }
}
