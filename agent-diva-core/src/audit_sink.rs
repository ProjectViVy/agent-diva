//! Pluggable audit event sink — global dispatch layer.
//!
//! The `AuditSink` trait allows consumers to register a single global
//! backend (file, in-memory buffer, network transport) that receives
//! every audit event emitted by the system.
//!
//! Registration is via [`register_sink`]; the registered sink is
//! stored in a static [`OnceLock`] and is called **after** the
//! `tracing::info!` dispatch in [`crate::audit::emit`].
//!
//! # Thread safety
//!
//! `AuditSink` requires `Send + Sync`. The global registration is
//! one-shot (once set it cannot be replaced). Callers should register
//! before any audit events are emitted (typically at startup).
//!
//! # Error handling
//!
//! Sink errors are **swallowed** by the `emit()` path — audit failure
//! must never interrupt business logic.

use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use chrono::Local;

use crate::audit::AuditEvent;

// ── Error type ────────────────────────────────────────────────────

/// Errors that can occur inside an [`AuditSink`] implementation.
#[derive(Debug)]
pub enum AuditSinkError {
    /// An I/O error occurred (file write, flush, etc.).
    Io(std::io::Error),
    /// Serialisation failed (JSON, message-pack, etc.).
    Serialization(String),
}

impl fmt::Display for AuditSinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "audit sink IO error: {e}"),
            Self::Serialization(msg) => write!(f, "audit sink serialization error: {msg}"),
        }
    }
}

impl std::error::Error for AuditSinkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Serialization(_) => None,
        }
    }
}

// ── Trait ─────────────────────────────────────────────────────────

/// A backend that receives structured audit events.
///
/// Implementors write events to a file, buffer, database, or
/// network transport. Only one implementation can be registered
/// globally via [`register_sink`].
pub trait AuditSink: Send + Sync {
    /// Consume one audit event.
    ///
    /// Implementations should be non-blocking and fast (<1 ms is
    /// ideal). Errors are logged but never propagated to the caller
    /// of [`crate::audit::emit`].
    fn emit(&self, event: &AuditEvent) -> Result<(), AuditSinkError>;
}

// ── Global registration ───────────────────────────────────────────

/// The single registered audit sink.
static GLOBAL_SINK: OnceLock<Arc<dyn AuditSink>> = OnceLock::new();

/// Register the global audit sink.
///
/// # Panics
///
/// Panics if a sink has already been registered. Registration is
/// one-shot by design.
pub fn register_sink(sink: Arc<dyn AuditSink>) {
    GLOBAL_SINK
        .set(sink)
        .map_err(|_| "GLOBAL_SINK is already registered")
        .expect("GLOBAL_SINK can only be registered once");
}

/// Retrieve the registered sink, if any.
pub fn get_sink() -> Option<&'static Arc<dyn AuditSink>> {
    GLOBAL_SINK.get()
}

/// Resolve the workspace-local audit directory.
pub fn workspace_audit_dir(workspace_root: &Path) -> PathBuf {
    workspace_root.join(".agent-diva").join("audit")
}

/// Register a workspace-local JSONL sink if no global sink exists yet.
pub fn ensure_workspace_jsonl_sink(workspace_root: &Path) -> Result<(), AuditSinkError> {
    if get_sink().is_some() {
        return Ok(());
    }

    let sink = Arc::new(JsonlAuditSink::new(&workspace_audit_dir(workspace_root))?);
    let _ = GLOBAL_SINK.set(sink);
    Ok(())
}

fn lock_mutex<'a, T>(
    mutex: &'a Mutex<T>,
    label: &str,
) -> Result<MutexGuard<'a, T>, AuditSinkError> {
    mutex
        .lock()
        .map_err(|_| AuditSinkError::Serialization(format!("{label} mutex poisoned")))
}

// ── JsonlAuditSink ────────────────────────────────────────────────

/// A file-based audit sink that writes JSON Lines with daily rolling.
///
/// Each day gets its own file: `{dir}/audit-{YYYY-MM-DD}.jsonl`.
/// Events are appended as one JSON object per line.
pub struct JsonlAuditSink {
    /// Buffered writer for the current day's file.
    writer: Mutex<BufWriter<File>>,
    /// Current date string in `YYYY-MM-DD` format.
    current_date: Mutex<String>,
    /// Directory where audit files are written.
    dir: PathBuf,
}

impl JsonlAuditSink {
    /// Create a new `JsonlAuditSink` that writes to `dir`.
    ///
    /// The directory is created if it does not exist.
    pub fn new(dir: &Path) -> Result<Self, AuditSinkError> {
        std::fs::create_dir_all(dir).map_err(AuditSinkError::Io)?;

        let today = Local::now().format("%Y-%m-%d").to_string();
        let path = Self::file_path(dir, &today);
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(AuditSinkError::Io)?;

        Ok(Self {
            writer: Mutex::new(BufWriter::new(file)),
            current_date: Mutex::new(today),
            dir: dir.to_path_buf(),
        })
    }

    /// Compute the full file path for a given date.
    fn file_path(dir: &Path, date: &str) -> PathBuf {
        dir.join(format!("audit-{date}.jsonl"))
    }

    fn serialize_event_line(event: &AuditEvent) -> Result<String, AuditSinkError> {
        let mut value = serde_json::to_value(event)
            .map_err(|e| AuditSinkError::Serialization(e.to_string()))?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| AuditSinkError::Serialization("audit event was not an object".into()))?;
        object.insert(
            "timestamp".to_string(),
            serde_json::Value::String(chrono::Utc::now().to_rfc3339()),
        );
        serde_json::to_string(&value).map_err(|e| AuditSinkError::Serialization(e.to_string()))
    }

    /// Roll over to a new file if the date has changed.
    ///
    /// Must be called while holding the writer lock (or before acquiring it).
    fn roll_if_needed(&self) -> Result<(), AuditSinkError> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let mut current = lock_mutex(&self.current_date, "current_date")?;

        if *current != today {
            // Date changed — flush old writer and open new file.
            let mut writer = lock_mutex(&self.writer, "writer")?;
            writer.flush().map_err(AuditSinkError::Io)?;
            drop(writer);

            let path = Self::file_path(&self.dir, &today);
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .map_err(AuditSinkError::Io)?;

            *lock_mutex(&self.writer, "writer")? = BufWriter::new(file);
            *current = today;
        }
        Ok(())
    }
}

impl AuditSink for JsonlAuditSink {
    fn emit(&self, event: &AuditEvent) -> Result<(), AuditSinkError> {
        // Roll to a new file if the date boundary crossed.
        if let Err(e) = self.roll_if_needed() {
            tracing::error!("audit sink roll failed: {}", e);
            return Ok(());
        }

        let json = match Self::serialize_event_line(event) {
            Ok(j) => j,
            Err(e) => {
                tracing::error!("audit event serialization failed: {}", e);
                return Ok(());
            }
        };

        let mut writer = match lock_mutex(&self.writer, "writer") {
            Ok(writer) => writer,
            Err(error) => {
                tracing::error!("audit sink writer lock failed: {}", error);
                return Ok(());
            }
        };
        if let Err(e) = writeln!(writer, "{}", json) {
            tracing::error!("audit sink write failed: {}", e);
            // Swallow error — audit must never interrupt business logic.
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audit::AuditEvent;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    /// A test sink that counts calls and remembers the last event.
    struct CountSink {
        count: AtomicUsize,
        last: Mutex<Option<AuditEvent>>,
    }

    impl CountSink {
        fn new() -> Self {
            Self {
                count: AtomicUsize::new(0),
                last: Mutex::new(None),
            }
        }
    }

    impl AuditSink for CountSink {
        fn emit(&self, event: &AuditEvent) -> Result<(), AuditSinkError> {
            self.count.fetch_add(1, Ordering::SeqCst);
            *self.last.lock().unwrap() = Some(event.clone());
            Ok(())
        }
    }

    // ── NOTE ─────────────────────────────────────────────────────
    // These tests share a static OnceLock. Because `cargo test`
    // runs each test in its own thread within the same process,
    // registration in one test would poison the lock for the next.
    // We work around this by testing registration + emit + retrieval
    // inside a **single** test function. This is intentional and safe.
    // ──────────────────────────────────────────────────────────────

    #[test]
    fn lifecycle_register_emit_retrieve() {
        // 1. Before registration, get_sink() returns None.
        assert!(get_sink().is_none(), "no sink registered yet");

        // 2. Register a counting sink.
        let sink = Arc::new(CountSink::new());
        register_sink(sink.clone());

        // 3. After registration, get_sink() returns the registered sink.
        assert!(get_sink().is_some(), "sink should be registered");

        // 4. Emit an event through the sink directly — verify it counts.
        let event = AuditEvent::HeartbeatTriggered { interval_secs: 10 };
        get_sink().unwrap().emit(&event).unwrap();
        assert_eq!(sink.count.load(Ordering::SeqCst), 1);

        // 5. Emit a second event.
        let event2 = AuditEvent::ToolInvoked {
            tool_name: "bash".into(),
            args: serde_json::json!({}),
        };
        get_sink().unwrap().emit(&event2).unwrap();
        assert_eq!(sink.count.load(Ordering::SeqCst), 2);

        // 6. Last event captured correctly.
        let last = sink.last.lock().unwrap().clone().unwrap();
        assert_eq!(last, event2);
    }

    #[test]
    fn emit_without_sink_does_not_panic() {
        // If no sink is registered, calling get_sink() returns None
        // and the if-let in audit.rs simply skips the dispatch.
        //
        // Because GLOBAL_SINK is a static OnceLock and might already
        // be set by lifecycle_register_emit_retrieve (which runs
        // first in the same process), we can only test this reliably
        // by checking that get_sink() itself doesn't panic regardless
        // of state.
        //
        // The actual "no sink → no panic → tracing still works"
        // behaviour is verified by the existing `test_emit_does_not_panic`
        // test in audit.rs, which calls emit() before any sink is
        // registered.
        let _ = get_sink(); // Must not panic
    }

    #[test]
    fn sink_error_types_display() {
        let io_err =
            AuditSinkError::Io(std::io::Error::new(std::io::ErrorKind::Other, "disk full"));
        assert!(io_err.to_string().contains("disk full"));

        let ser_err = AuditSinkError::Serialization("invalid json".into());
        assert!(ser_err.to_string().contains("invalid json"));
    }

    // ── JsonlAuditSink tests ───────────────────────────────────────

    #[test]
    fn jsonl_sink_writes_three_events() {
        let dir = tempfile::tempdir().unwrap();
        let sink = JsonlAuditSink::new(dir.path()).unwrap();

        // Emit 3 distinct events.
        sink.emit(&AuditEvent::HeartbeatTriggered { interval_secs: 5 })
            .unwrap();
        sink.emit(&AuditEvent::ToolInvoked {
            tool_name: "bash".into(),
            args: serde_json::json!({"cmd": "ls"}),
        })
        .unwrap();
        sink.emit(&AuditEvent::TokenUsed {
            provider: "openai".into(),
            model: "gpt-4".into(),
            tokens: 42,
        })
        .unwrap();

        // Force flush so the file is fully written before we read it.
        {
            let mut w = sink.writer.lock().unwrap();
            w.flush().unwrap();
        }

        let today = Local::now().format("%Y-%m-%d").to_string();
        let path = dir.path().join(format!("audit-{today}.jsonl"));
        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();

        assert_eq!(lines.len(), 3, "expected 3 JSON lines");

        // Each line should be valid JSON and contain the `type` field.
        for line in &lines {
            let json: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(
                json.get("type").is_some(),
                "each line must have a `type` field"
            );
            assert!(
                json.get("timestamp").and_then(|value| value.as_str()).is_some(),
                "each line must include a timestamp"
            );
        }

        // Verify specific event types.
        let types: Vec<String> = lines
            .iter()
            .map(|l| {
                let json: serde_json::Value = serde_json::from_str(l).unwrap();
                json["type"].as_str().unwrap().to_string()
            })
            .collect();
        assert_eq!(
            types,
            vec!["heartbeat_triggered", "tool_invoked", "token_used"]
        );
    }

    #[test]
    fn jsonl_sink_rolls_on_date_change() {
        let dir = tempfile::tempdir().unwrap();
        let sink = JsonlAuditSink::new(dir.path()).unwrap();

        // Emit one event "today".
        sink.emit(&AuditEvent::HeartbeatTriggered { interval_secs: 10 })
            .unwrap();
        {
            let mut w = sink.writer.lock().unwrap();
            w.flush().unwrap();
        }

        // Simulate a date change by manually updating current_date to a
        // future date. When emit() runs, it sees the stored date differs
        // from the real current date (today), so it rolls over to a new
        // file for today.
        {
            let mut current = sink.current_date.lock().unwrap();
            *current = "2099-12-31".to_string();
        }

        // Emit another event — roll should create a new file for today.
        sink.emit(&AuditEvent::ToolInvoked {
            tool_name: "test".into(),
            args: serde_json::json!({}),
        })
        .unwrap();
        {
            let mut w = sink.writer.lock().unwrap();
            w.flush().unwrap();
        }

        let today = Local::now().format("%Y-%m-%d").to_string();
        let today_path = dir.path().join(format!("audit-{today}.jsonl"));
        let old_path = dir.path().join("audit-2099-12-31.jsonl");

        assert!(today_path.exists(), "today's file should exist after roll");
        assert!(
            !old_path.exists(),
            "old future-date file should NOT exist (roll goes to today)"
        );

        let today_lines = std::fs::read_to_string(&today_path).unwrap();

        // The original event was flushed to today's file, then after the
        // simulated date change the second event was also written to today's
        // file (because the real date is still today).
        assert_eq!(
            today_lines.lines().count(),
            2,
            "today's file has both events"
        );
    }

    #[test]
    fn jsonl_sink_swallows_write_errors() {
        // Create a temp dir, then remove write permissions so the sink
        // cannot create files.
        let dir = tempfile::tempdir().unwrap();

        // On Unix, chmod 555 removes write. On Windows this is trickier,
        // so we instead test by pointing at a file path that is not a dir.
        let file_as_dir = dir.path().join("not_a_dir");
        std::fs::write(&file_as_dir, "x").unwrap(); // create a regular file

        // Attempting to create JsonlAuditSink inside a file should fail,
        // but the emit() path swallows errors.
        // Instead, create a valid sink then make it fail by removing the dir.
        let sink = JsonlAuditSink::new(dir.path()).unwrap();

        // Remove the directory so subsequent writes fail.
        std::fs::remove_dir_all(dir.path()).unwrap();

        // emit() should return Ok even though the write will fail.
        let result = sink.emit(&AuditEvent::HeartbeatTriggered { interval_secs: 5 });
        assert!(result.is_ok(), "emit must return Ok even when write fails");
    }
}
