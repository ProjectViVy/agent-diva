//! Debug-mode logging for explicit foreground gateway runs.

use chrono::{DateTime, Local, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use uuid::Uuid;

/// Explicit debug run metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugRun {
    pub run_id: String,
    pub dir: PathBuf,
    pub created_at: DateTime<Utc>,
    pub raw_payloads: bool,
}

impl DebugRun {
    pub fn new(config_dir: &Path) -> Self {
        let now = Utc::now();
        let short_id = Uuid::new_v4()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>();
        let run_id = format!(
            "debug-run-{}-{}",
            now.with_timezone(&Local).format("%Y%m%d-%H%M%S"),
            short_id
        );
        let dir = config_dir.join("debug-runs").join(&run_id);
        Self {
            run_id,
            dir,
            created_at: now,
            raw_payloads: true,
        }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.dir.join("manifest.json")
    }
}

/// Raw debug event written only during explicit debug gateway runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugEvent {
    pub ts: DateTime<Utc>,
    pub trace_id: Option<String>,
    pub session_id: Option<String>,
    pub component: String,
    pub event: String,
    pub payload: Value,
}

impl DebugEvent {
    pub fn new(
        trace_id: Option<String>,
        session_id: Option<String>,
        component: impl Into<String>,
        event: impl Into<String>,
        payload: Value,
    ) -> Self {
        Self {
            ts: Utc::now(),
            trace_id,
            session_id,
            component: component.into(),
            event: event.into(),
            payload,
        }
    }
}

/// Append-only debug event writer. This intentionally does not redact or truncate payloads.
#[derive(Debug)]
pub struct DebugEventLogger {
    run: DebugRun,
    write_lock: Mutex<()>,
}

impl DebugEventLogger {
    pub fn new(run: DebugRun) -> crate::Result<Arc<Self>> {
        fs::create_dir_all(&run.dir)?;
        let logger = Arc::new(Self {
            run,
            write_lock: Mutex::new(()),
        });
        logger.write_manifest()?;
        Ok(logger)
    }

    pub fn run(&self) -> &DebugRun {
        &self.run
    }

    pub fn write_event(&self, event: DebugEvent) -> crate::Result<()> {
        self.write_jsonl("events.jsonl", &event)
    }

    pub fn write_raw(&self, event: DebugEvent) -> crate::Result<()> {
        self.write_jsonl("raw.jsonl", &event)
    }

    fn write_jsonl<T: Serialize>(&self, file_name: &str, value: &T) -> crate::Result<()> {
        let _guard = self.write_lock.lock();
        fs::create_dir_all(&self.run.dir)?;
        let line = serde_json::to_vec(value)?;
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.run.dir.join(file_name))?;
        let mut writer = BufWriter::new(file);
        writer.write_all(&line)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        tracing::trace!(
            target: "agent_diva_debug",
            debug_file = file_name,
            "{}",
            String::from_utf8_lossy(&line)
        );
        Ok(())
    }

    fn write_manifest(&self) -> crate::Result<()> {
        let manifest = serde_json::json!({
            "run_id": self.run.run_id,
            "created_at": self.run.created_at,
            "raw_payloads": self.run.raw_payloads,
            "warning": "Debug mode writes raw provider payloads, tool output, MCP I/O, channel messages, and may include secrets."
        });
        fs::write(
            self.run.manifest_path(),
            serde_json::to_vec_pretty(&manifest)?,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_logger_writes_raw_jsonl_without_redaction() {
        let temp_dir = tempfile::tempdir().unwrap();
        let run = DebugRun {
            run_id: "debug-run-test".to_string(),
            dir: temp_dir.path().join("debug-run-test"),
            created_at: Utc::now(),
            raw_payloads: true,
        };
        let logger = DebugEventLogger::new(run).unwrap();
        logger
            .write_raw(DebugEvent::new(
                Some("tr_demo".to_string()),
                Some("cli:test".to_string()),
                "provider",
                "provider_request",
                serde_json::json!({"api_key":"sk-secret","output":"full result"}),
            ))
            .unwrap();

        let raw = fs::read_to_string(logger.run().dir.join("raw.jsonl")).unwrap();
        assert!(raw.contains("sk-secret"));
        assert!(raw.contains("full result"));
        let parsed: Value = serde_json::from_str(raw.lines().next().unwrap()).unwrap();
        assert_eq!(parsed["event"], "provider_request");
    }

    #[test]
    fn debug_run_id_uses_expected_prefix() {
        let temp_dir = tempfile::tempdir().unwrap();
        let run = DebugRun::new(temp_dir.path());
        assert!(run.run_id.starts_with("debug-run-"));
        assert!(run.dir.ends_with(&run.run_id));
    }
}
