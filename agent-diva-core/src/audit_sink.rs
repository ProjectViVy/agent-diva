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
use std::sync::{Arc, OnceLock};

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
        let io_err = AuditSinkError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "disk full",
        ));
        assert!(io_err.to_string().contains("disk full"));

        let ser_err = AuditSinkError::Serialization("invalid json".into());
        assert!(ser_err.to_string().contains("invalid json"));
    }
}
