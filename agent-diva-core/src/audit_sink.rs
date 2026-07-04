//! Pluggable audit event sink - global dispatch layer.
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
//! Sink errors are **swallowed** by the `emit()` path - audit failure
//! must never interrupt business logic.

use std::fmt;
use std::fs::OpenOptions;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

use chrono::{DateTime, Local, NaiveDate, Utc};

use crate::audit::AuditEvent;

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

/// A backend that receives structured audit events.
pub trait AuditSink: Send + Sync {
    /// Consume one audit event.
    fn emit(&self, event: &AuditEvent) -> Result<(), AuditSinkError>;
}

static GLOBAL_SINK: OnceLock<Arc<dyn AuditSink>> = OnceLock::new();

/// Register the global audit sink.
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

trait AuditClock: Send + Sync {
    fn now_utc(&self) -> DateTime<Utc>;
    fn today_local(&self) -> NaiveDate;
}

#[derive(Default)]
struct SystemAuditClock;

impl AuditClock for SystemAuditClock {
    fn now_utc(&self) -> DateTime<Utc> {
        Utc::now()
    }

    fn today_local(&self) -> NaiveDate {
        Local::now().date_naive()
    }
}

trait AuditWriter: Write + Send {}

impl<T: Write + Send> AuditWriter for T {}

trait AuditWriterFactory: Send + Sync {
    fn open(&self, path: &Path) -> Result<Box<dyn AuditWriter>, AuditSinkError>;
}

#[derive(Default)]
struct FileAuditWriterFactory;

impl AuditWriterFactory for FileAuditWriterFactory {
    fn open(&self, path: &Path) -> Result<Box<dyn AuditWriter>, AuditSinkError> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(AuditSinkError::Io)?;
        Ok(Box::new(BufWriter::new(file)))
    }
}

/// A file-based audit sink that writes JSON Lines with daily rolling.
///
/// Each day gets its own file: `{dir}/audit-{YYYY-MM-DD}.jsonl`.
/// Events are appended as one JSON object per line and flushed before
/// returning so same-process readers can observe the write immediately.
pub struct JsonlAuditSink {
    writer: Mutex<Box<dyn AuditWriter>>,
    current_date: Mutex<String>,
    dir: PathBuf,
    clock: Arc<dyn AuditClock>,
    writer_factory: Arc<dyn AuditWriterFactory>,
}

impl JsonlAuditSink {
    /// Create a new `JsonlAuditSink` that writes to `dir`.
    pub fn new(dir: &Path) -> Result<Self, AuditSinkError> {
        Self::with_clock_and_factory(
            dir,
            Arc::new(SystemAuditClock),
            Arc::new(FileAuditWriterFactory),
        )
    }

    #[cfg(test)]
    fn with_clock(dir: &Path, clock: Arc<dyn AuditClock>) -> Result<Self, AuditSinkError> {
        Self::with_clock_and_factory(dir, clock, Arc::new(FileAuditWriterFactory))
    }

    fn with_clock_and_factory(
        dir: &Path,
        clock: Arc<dyn AuditClock>,
        writer_factory: Arc<dyn AuditWriterFactory>,
    ) -> Result<Self, AuditSinkError> {
        std::fs::create_dir_all(dir).map_err(AuditSinkError::Io)?;

        let today = clock.today_local().format("%Y-%m-%d").to_string();
        let writer = writer_factory.open(&Self::file_path(dir, &today))?;

        Ok(Self {
            writer: Mutex::new(writer),
            current_date: Mutex::new(today),
            dir: dir.to_path_buf(),
            clock,
            writer_factory,
        })
    }

    fn file_path(dir: &Path, date: &str) -> PathBuf {
        dir.join(format!("audit-{date}.jsonl"))
    }

    fn serialize_event_line(&self, event: &AuditEvent) -> Result<String, AuditSinkError> {
        let mut value = serde_json::to_value(event)
            .map_err(|e| AuditSinkError::Serialization(e.to_string()))?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| AuditSinkError::Serialization("audit event was not an object".into()))?;
        object.insert(
            "timestamp".to_string(),
            serde_json::Value::String(self.clock.now_utc().to_rfc3339()),
        );
        serde_json::to_string(&value).map_err(|e| AuditSinkError::Serialization(e.to_string()))
    }

    fn roll_if_needed(&self) -> Result<(), AuditSinkError> {
        let today = self.clock.today_local().format("%Y-%m-%d").to_string();
        let mut current = lock_mutex(&self.current_date, "current_date")?;
        if *current == today {
            return Ok(());
        }

        let mut writer = lock_mutex(&self.writer, "writer")?;
        writer.flush().map_err(AuditSinkError::Io)?;
        *writer = self
            .writer_factory
            .open(&Self::file_path(&self.dir, &today))?;
        *current = today;
        Ok(())
    }
}

impl AuditSink for JsonlAuditSink {
    fn emit(&self, event: &AuditEvent) -> Result<(), AuditSinkError> {
        if let Err(error) = self.roll_if_needed() {
            tracing::error!("audit sink roll failed: {}", error);
            return Ok(());
        }

        let json = match self.serialize_event_line(event) {
            Ok(json) => json,
            Err(error) => {
                tracing::error!("audit event serialization failed: {}", error);
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

        if let Err(error) = writeln!(writer, "{}", json) {
            tracing::error!("audit sink write failed: {}", error);
            return Ok(());
        }
        if let Err(error) = writer.flush() {
            tracing::error!("audit sink flush failed: {}", error);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::io;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

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

    #[test]
    fn lifecycle_register_emit_retrieve() {
        assert!(get_sink().is_none(), "no sink registered yet");

        let sink = Arc::new(CountSink::new());
        register_sink(sink.clone());

        assert!(get_sink().is_some(), "sink should be registered");

        let event = AuditEvent::HeartbeatTriggered { interval_secs: 10 };
        get_sink().unwrap().emit(&event).unwrap();
        assert_eq!(sink.count.load(Ordering::SeqCst), 1);

        let event2 = AuditEvent::ToolInvoked {
            tool_name: "bash".into(),
            args: serde_json::json!({}),
        };
        get_sink().unwrap().emit(&event2).unwrap();
        assert_eq!(sink.count.load(Ordering::SeqCst), 2);

        let last = sink.last.lock().unwrap().clone().unwrap();
        assert_eq!(last, event2);
    }

    #[test]
    fn emit_without_sink_does_not_panic() {
        let _ = get_sink();
    }

    #[test]
    fn sink_error_types_display() {
        let io_err = AuditSinkError::Io(std::io::Error::other("disk full"));
        assert!(io_err.to_string().contains("disk full"));

        let ser_err = AuditSinkError::Serialization("invalid json".into());
        assert!(ser_err.to_string().contains("invalid json"));
    }

    #[derive(Clone)]
    struct FakeClock {
        local_date: Arc<Mutex<NaiveDate>>,
        utc_now: Arc<Mutex<DateTime<Utc>>>,
    }

    impl FakeClock {
        fn new(local_date: NaiveDate, utc_now: DateTime<Utc>) -> Self {
            Self {
                local_date: Arc::new(Mutex::new(local_date)),
                utc_now: Arc::new(Mutex::new(utc_now)),
            }
        }

        fn set(&self, local_date: NaiveDate, utc_now: DateTime<Utc>) {
            *self.local_date.lock().unwrap() = local_date;
            *self.utc_now.lock().unwrap() = utc_now;
        }
    }

    impl AuditClock for FakeClock {
        fn now_utc(&self) -> DateTime<Utc> {
            self.utc_now.lock().unwrap().clone()
        }

        fn today_local(&self) -> NaiveDate {
            *self.local_date.lock().unwrap()
        }
    }

    #[test]
    fn jsonl_sink_writes_three_events() {
        let dir = tempfile::tempdir().unwrap();
        let sink = JsonlAuditSink::new(dir.path()).unwrap();

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

        let today = Local::now().format("%Y-%m-%d").to_string();
        let path = dir.path().join(format!("audit-{today}.jsonl"));
        let contents = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = contents.lines().collect();

        assert_eq!(lines.len(), 3, "expected 3 JSON lines");

        for line in &lines {
            let json: serde_json::Value = serde_json::from_str(line).unwrap();
            assert!(json.get("type").is_some());
            assert!(json
                .get("timestamp")
                .and_then(|value| value.as_str())
                .is_some());
        }

        let types: Vec<String> = lines
            .iter()
            .map(|line| {
                let json: serde_json::Value = serde_json::from_str(line).unwrap();
                json["type"].as_str().unwrap().to_string()
            })
            .collect();
        assert_eq!(
            types,
            vec!["heartbeat_triggered", "tool_invoked", "token_used"]
        );
    }

    #[test]
    fn jsonl_sink_emit_is_immediately_visible_to_readers() {
        let dir = tempfile::tempdir().unwrap();
        let sink = JsonlAuditSink::new(dir.path()).unwrap();

        sink.emit(&AuditEvent::HeartbeatTriggered { interval_secs: 7 })
            .unwrap();

        let today = Local::now().format("%Y-%m-%d").to_string();
        let path = dir.path().join(format!("audit-{today}.jsonl"));
        let contents = std::fs::read_to_string(&path).unwrap();
        assert_eq!(contents.lines().count(), 1);
    }

    #[test]
    fn jsonl_sink_rolls_on_date_change() {
        let dir = tempfile::tempdir().unwrap();
        let day1 = NaiveDate::from_ymd_opt(2026, 7, 4).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 7, 5).unwrap();
        let clock = Arc::new(FakeClock::new(
            day1,
            Utc.with_ymd_and_hms(2026, 7, 4, 8, 0, 0).unwrap(),
        ));
        let sink = JsonlAuditSink::with_clock(dir.path(), clock.clone()).unwrap();

        sink.emit(&AuditEvent::HeartbeatTriggered { interval_secs: 10 })
            .unwrap();
        clock.set(day2, Utc.with_ymd_and_hms(2026, 7, 5, 8, 0, 0).unwrap());
        sink.emit(&AuditEvent::ToolInvoked {
            tool_name: "test".into(),
            args: serde_json::json!({}),
        })
        .unwrap();

        let day1_path = dir.path().join("audit-2026-07-04.jsonl");
        let day2_path = dir.path().join("audit-2026-07-05.jsonl");
        assert_eq!(
            std::fs::read_to_string(day1_path).unwrap().lines().count(),
            1
        );
        assert_eq!(
            std::fs::read_to_string(day2_path).unwrap().lines().count(),
            1
        );
    }

    #[test]
    fn jsonl_sink_swallows_write_errors() {
        struct FailWriter;

        impl Write for FailWriter {
            fn write(&mut self, _buf: &[u8]) -> io::Result<usize> {
                Err(io::Error::other("write failed"))
            }

            fn flush(&mut self) -> io::Result<()> {
                Err(io::Error::other("flush failed"))
            }
        }

        struct FailFactory;

        impl AuditWriterFactory for FailFactory {
            fn open(&self, _path: &Path) -> Result<Box<dyn AuditWriter>, AuditSinkError> {
                Ok(Box::new(FailWriter))
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let clock = Arc::new(FakeClock::new(
            NaiveDate::from_ymd_opt(2026, 7, 5).unwrap(),
            Utc.with_ymd_and_hms(2026, 7, 5, 9, 0, 0).unwrap(),
        ));
        let sink = JsonlAuditSink::with_clock_and_factory(dir.path(), clock, Arc::new(FailFactory))
            .unwrap();

        let result = sink.emit(&AuditEvent::HeartbeatTriggered { interval_secs: 5 });
        assert!(result.is_ok(), "emit must return Ok even when write fails");
    }

    #[test]
    fn jsonl_sink_swallows_roll_errors() {
        struct FlakyFactory {
            opens: AtomicUsize,
        }

        impl AuditWriterFactory for FlakyFactory {
            fn open(&self, path: &Path) -> Result<Box<dyn AuditWriter>, AuditSinkError> {
                let count = self.opens.fetch_add(1, Ordering::SeqCst);
                if count == 0 {
                    let file = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(path)
                        .map_err(AuditSinkError::Io)?;
                    Ok(Box::new(BufWriter::new(file)))
                } else {
                    Err(AuditSinkError::Io(io::Error::other("roll open failed")))
                }
            }
        }

        let dir = tempfile::tempdir().unwrap();
        let day1 = NaiveDate::from_ymd_opt(2026, 7, 5).unwrap();
        let day2 = NaiveDate::from_ymd_opt(2026, 7, 6).unwrap();
        let clock = Arc::new(FakeClock::new(
            day1,
            Utc.with_ymd_and_hms(2026, 7, 5, 9, 0, 0).unwrap(),
        ));
        let sink = JsonlAuditSink::with_clock_and_factory(
            dir.path(),
            clock.clone(),
            Arc::new(FlakyFactory {
                opens: AtomicUsize::new(0),
            }),
        )
        .unwrap();

        sink.emit(&AuditEvent::HeartbeatTriggered { interval_secs: 5 })
            .unwrap();
        clock.set(day2, Utc.with_ymd_and_hms(2026, 7, 6, 9, 0, 0).unwrap());

        let result = sink.emit(&AuditEvent::HeartbeatTriggered { interval_secs: 6 });
        assert!(result.is_ok(), "emit must return Ok even when roll fails");
    }
}
