use crate::config_loader;
use agent_diva_core::audit::{audit_log_file_name_for_date, AuditEvent};
use agent_diva_core::config::Config;
use chrono::{Local, NaiveDate};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditRecord {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub event: AuditEvent,
}

#[derive(Debug, Deserialize)]
struct AuditLogLine {
    #[serde(default)]
    timestamp: String,
    #[serde(default)]
    level: String,
    #[serde(default)]
    target: String,
    #[serde(default)]
    fields: Map<String, Value>,
}

pub fn parse_audit_date(date: Option<&str>) -> Result<NaiveDate, String> {
    match date.map(str::trim) {
        Some("") | None => Ok(Local::now().date_naive()),
        Some(value) => NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .map_err(|error| format!("invalid audit date '{value}': {error}")),
    }
}

fn resolve_log_directory(config: &Config) -> PathBuf {
    let loader = config_loader();
    crate::resolve_configured_path(&config.logging.dir, loader.config_dir())
}

fn audit_log_path(log_dir: &Path, date: NaiveDate) -> PathBuf {
    log_dir.join(audit_log_file_name_for_date(date))
}

fn parse_event(fields: &Map<String, Value>) -> Result<AuditEvent, String> {
    let event_type = field_as_str(fields, "event_type")?;
    match event_type {
        "tool_invoked" => Ok(AuditEvent::ToolInvoked {
            tool: field_as_str(fields, "tool")?.to_string(),
            args_hash: field_as_str(fields, "args_hash")?.to_string(),
            duration_ms: field_as_u64(fields, "duration_ms")?,
        }),
        "tool_denied" => Ok(AuditEvent::ToolDenied {
            tool: field_as_str(fields, "tool")?.to_string(),
            reason: field_as_str(fields, "reason")?.to_string(),
        }),
        "decision_point" => Ok(AuditEvent::DecisionPoint {
            phase: field_as_str(fields, "phase")?.to_string(),
            llm_decision: field_as_str(fields, "llm_decision")?.to_string(),
        }),
        "injection_detected" => Ok(AuditEvent::InjectionDetected {
            pattern: field_as_str(fields, "pattern")?.to_string(),
            severity: field_as_str(fields, "severity")?.to_string(),
        }),
        "pii_redacted" => Ok(AuditEvent::PiiRedacted {
            kind: field_as_str(fields, "kind")?.to_string(),
            count: field_as_u64(fields, "count")? as usize,
        }),
        "token_used" => Ok(AuditEvent::TokenUsed {
            prompt: field_as_i64(fields, "prompt")?,
            completion: field_as_i64(fields, "completion")?,
            total: field_as_i64(fields, "total")?,
            model: field_as_str(fields, "model")?.to_string(),
        }),
        "presence_changed" => Ok(AuditEvent::PresenceChanged {
            from: field_as_str(fields, "from")?.to_string(),
            to: field_as_str(fields, "to")?.to_string(),
        }),
        "heartbeat_triggered" => Ok(AuditEvent::HeartbeatTriggered {
            state: field_as_str(fields, "state")?.to_string(),
            tasks: field_as_str(fields, "tasks")?.to_string(),
        }),
        other => Err(format!("unsupported audit event type '{other}'")),
    }
}

fn field_as_str<'a>(fields: &'a Map<String, Value>, key: &str) -> Result<&'a str, String> {
    fields
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("audit field '{key}' is missing or not a string"))
}

fn field_as_u64(fields: &Map<String, Value>, key: &str) -> Result<u64, String> {
    fields
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("audit field '{key}' is missing or not a u64"))
}

fn field_as_i64(fields: &Map<String, Value>, key: &str) -> Result<i64, String> {
    fields
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("audit field '{key}' is missing or not an i64"))
}

pub fn read_audit_events_for_date(
    config: &Config,
    date: NaiveDate,
) -> Result<Vec<AuditRecord>, String> {
    let log_dir = resolve_log_directory(config);
    let path = audit_log_path(&log_dir, date);
    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read audit log {}: {}", path.display(), error))?;
    let mut records = Vec::new();

    for (index, line) in content.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed: AuditLogLine = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if parsed.target != "audit" {
            continue;
        }
        let event = parse_event(&parsed.fields).map_err(|error| {
            format!(
                "failed to parse audit event from {} line {}: {}",
                path.display(),
                index + 1,
                error
            )
        })?;
        records.push(AuditRecord {
            timestamp: parsed.timestamp,
            level: parsed.level,
            target: parsed.target,
            event,
        });
    }

    Ok(records)
}

pub fn read_audit_raw_log_for_date(config: &Config, date: NaiveDate) -> Result<String, String> {
    let log_dir = resolve_log_directory(config);
    let path = audit_log_path(&log_dir, date);
    if !path.exists() {
        return Ok(String::new());
    }

    std::fs::read_to_string(&path)
        .map_err(|error| format!("failed to read audit log {}: {}", path.display(), error))
}

#[tauri::command]
pub fn get_audit_events(date: Option<String>) -> Result<Vec<AuditRecord>, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|error| format!("failed to load config for audit logs: {}", error))?;
    let parsed_date = parse_audit_date(date.as_deref())?;
    read_audit_events_for_date(&config, parsed_date)
}

#[tauri::command]
pub fn get_audit_raw_log(date: Option<String>) -> Result<String, String> {
    let loader = config_loader();
    let config = loader
        .load()
        .map_err(|error| format!("failed to load config for audit logs: {}", error))?;
    let parsed_date = parse_audit_date(date.as_deref())?;
    read_audit_raw_log_for_date(&config, parsed_date)
}

#[cfg(test)]
mod tests {
    use super::{parse_audit_date, parse_event, AuditEvent, AuditLogLine, AuditRecord};
    use chrono::NaiveDate;
    use serde_json::json;

    #[test]
    fn empty_date_defaults_to_today_shape() {
        let parsed = parse_audit_date(Some("")).expect("date should parse");
        assert_eq!(parsed.format("%Y-%m-%d").to_string().len(), 10);
    }

    #[test]
    fn parse_event_reconstructs_structured_variant() {
        let fields = json!({
            "event_type": "decision_point",
            "phase": "provider_response",
            "llm_decision": "tool_use"
        });
        let map = fields.as_object().expect("object");
        let event = parse_event(map).expect("event should parse");
        assert_eq!(
            event,
            AuditEvent::DecisionPoint {
                phase: "provider_response".to_string(),
                llm_decision: "tool_use".to_string(),
            }
        );
    }

    #[test]
    fn audit_line_shape_deserializes() {
        let line = json!({
            "timestamp": "2026-06-25T12:00:00+08:00",
            "level": "INFO",
            "target": "audit",
            "fields": {
                "message": "audit",
                "event_type": "tool_denied",
                "tool": "shell",
                "reason": "policy",
            }
        });
        let parsed: AuditLogLine =
            serde_json::from_value(line).expect("audit log line should deserialize");
        let event = parse_event(&parsed.fields).expect("event should parse");
        let record = AuditRecord {
            timestamp: parsed.timestamp,
            level: parsed.level,
            target: parsed.target,
            event,
        };

        assert_eq!(record.timestamp, "2026-06-25T12:00:00+08:00");
        assert_eq!(
            record.event,
            AuditEvent::ToolDenied {
                tool: "shell".to_string(),
                reason: "policy".to_string(),
            }
        );
    }

    #[test]
    fn explicit_date_parses() {
        let parsed = parse_audit_date(Some("2026-06-25")).expect("date should parse");
        assert_eq!(
            parsed,
            NaiveDate::from_ymd_opt(2026, 6, 25).expect("valid date")
        );
    }
}
