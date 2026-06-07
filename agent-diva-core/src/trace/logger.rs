use super::TraceEvent;
use crate::{config::schema::LoggingConfig, redaction::redact_secrets};
use chrono::{Local, NaiveDate};
use parking_lot::Mutex;
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

const DEFAULT_SUMMARY_LIMIT: usize = 280;
const DEFAULT_METADATA_LIMIT: usize = 512;

pub fn default_runtime_summary_limit() -> usize {
    DEFAULT_SUMMARY_LIMIT
}

pub fn default_runtime_metadata_limit() -> usize {
    DEFAULT_METADATA_LIMIT
}

pub fn truncate_and_redact_text(input: &str, limit: usize) -> String {
    let redacted = redact_secrets(input);
    let count = redacted.chars().count();
    if count <= limit {
        return redacted;
    }

    let mut truncated: String = redacted.chars().take(limit).collect();
    truncated.push_str("...");
    truncated
}

pub fn redact_and_truncate_value(value: Value, limit: usize) -> Value {
    match value {
        Value::String(text) => Value::String(truncate_and_redact_text(&text, limit)),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| redact_and_truncate_value(item, limit))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| (key, redact_and_truncate_value(value, limit)))
                .collect(),
        ),
        other => other,
    }
}

#[derive(Debug)]
pub struct TraceLogger {
    enabled: bool,
    dir: PathBuf,
    retention_days: u64,
    summary_limit: usize,
    metadata_limit: usize,
    record_tool_output_summaries: bool,
    write_lock: Mutex<()>,
}

impl TraceLogger {
    pub fn from_logging_config(config: &LoggingConfig) -> Arc<Self> {
        Arc::new(Self::new(
            config.structured_runtime_logs_enabled,
            config.runtime_log_dir.as_deref().unwrap_or(&config.dir),
            config.retention_days,
            default_runtime_summary_limit(),
            default_runtime_metadata_limit(),
            config.record_tool_output_summaries,
        ))
    }

    pub fn new(
        enabled: bool,
        dir: impl Into<PathBuf>,
        retention_days: u64,
        summary_limit: usize,
        metadata_limit: usize,
        record_tool_output_summaries: bool,
    ) -> Self {
        Self {
            enabled,
            dir: dir.into(),
            retention_days,
            summary_limit,
            metadata_limit,
            record_tool_output_summaries,
            write_lock: Mutex::new(()),
        }
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn record_tool_output_summaries(&self) -> bool {
        self.record_tool_output_summaries
    }

    pub fn write_event(&self, event: &TraceEvent) -> crate::Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let _guard = self.write_lock.lock();
        fs::create_dir_all(&self.dir)?;
        self.cleanup_old_logs()?;

        let sanitized = self.sanitize_event(event.clone());
        let path = self.log_path_for_date(sanitized.ts.with_timezone(&Local).date_naive());
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &sanitized)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        Ok(())
    }

    fn sanitize_event(&self, mut event: TraceEvent) -> TraceEvent {
        event.summary = truncate_and_redact_text(&event.summary, self.summary_limit);
        event.metadata = redact_and_truncate_value(event.metadata, self.metadata_limit);
        event
    }

    fn cleanup_old_logs(&self) -> crate::Result<()> {
        let path = Path::new(&self.dir);
        if !path.exists() {
            return Ok(());
        }

        let now = SystemTime::now();
        let threshold = Duration::from_secs(self.retention_days.saturating_mul(24 * 3600));
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let file_path = entry.path();
            if !file_path.is_file() {
                continue;
            }
            let Some(name) = file_path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !name.starts_with("runtime-") || !name.ends_with(".jsonl") {
                continue;
            }
            let Ok(metadata) = entry.metadata() else {
                continue;
            };
            let Ok(modified) = metadata.modified() else {
                continue;
            };
            let Ok(age) = now.duration_since(modified) else {
                continue;
            };
            if age > threshold {
                let _ = fs::remove_file(file_path);
            }
        }
        Ok(())
    }

    fn log_path_for_date(&self, date: NaiveDate) -> PathBuf {
        self.dir
            .join(format!("runtime-{}.jsonl", date.format("%Y-%m-%d")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::{TraceEvent, TraceId};
    use serde_json::json;

    #[test]
    fn trace_id_round_trips_in_json() {
        let trace_id = TraceId::new();
        let encoded = serde_json::to_string(&trace_id).unwrap();
        let decoded: TraceId = serde_json::from_str(&encoded).unwrap();
        assert_eq!(trace_id, decoded);
        assert!(trace_id.as_str().starts_with("tr_"));
    }

    #[test]
    fn trace_event_serializes_required_fields() {
        let event = TraceEvent::new(
            "info",
            TraceId::from("tr_demo"),
            "cli:test",
            "cli",
            "agent_loop",
            "message_received",
            "received",
            json!({"tool":"shell"}),
        );
        let value = serde_json::to_value(&event).unwrap();
        for key in [
            "ts",
            "level",
            "trace_id",
            "session_id",
            "channel",
            "component",
            "event",
            "summary",
            "metadata",
        ] {
            assert!(value.get(key).is_some(), "missing field {key}");
        }
    }

    #[test]
    fn redact_and_truncate_string_values() {
        let value = json!({
            "authorization": "Bearer sk-secret",
            "nested": ["ghp_demo", "x".repeat(600)]
        });
        let sanitized = redact_and_truncate_value(value, 64);
        let text = sanitized.to_string();
        assert!(text.contains("***REDACTED***"));
        assert!(!text.contains("sk-secret"));
        assert!(!text.contains("ghp_demo"));
        assert!(text.contains("..."));
    }

    #[test]
    fn trace_logger_writes_jsonl_lines() {
        let temp_dir = tempfile::tempdir().unwrap();
        let logger = TraceLogger::new(true, temp_dir.path(), 7, 280, 64, true);
        let event = TraceEvent::new(
            "info",
            TraceId::from("tr_demo"),
            "cli:test",
            "cli",
            "agent_loop",
            "tool_call_completed",
            "Authorization: Bearer sk-secret",
            json!({"result":"ghp_demo","tool":"shell"}),
        );

        logger.write_event(&event).unwrap();

        let log_path = temp_dir
            .path()
            .join(format!("runtime-{}.jsonl", Local::now().format("%Y-%m-%d")));
        let content = fs::read_to_string(log_path).unwrap();
        let lines: Vec<_> = content.lines().collect();
        assert_eq!(lines.len(), 1);
        let parsed: Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(parsed["event"], "tool_call_completed");
        assert!(parsed["summary"]
            .as_str()
            .unwrap()
            .contains("***REDACTED***"));
        assert!(!lines[0].contains("sk-secret"));
        assert!(!lines[0].contains("ghp_demo"));
    }
}
