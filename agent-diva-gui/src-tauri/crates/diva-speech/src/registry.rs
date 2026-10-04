//! Admission registry: trusted window context (session + monotonic
//! generation), one STT and one TTS slot, cancel tokens, and owned join
//! handles for abort/join teardown. The registry mutex is never held
//! across keyring or HTTP work.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;

use crate::{SpeechCode, SpeechError, SpeechResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    Stt,
    Tts,
}

impl RequestKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Stt => "stt",
            Self::Tts => "tts",
        }
    }
}

/// Trusted window context: the session the speech lane is bound to and
/// the caller's monotonic generation counter.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SpeechContext {
    pub session_id: String,
    pub generation: u64,
}

/// Cross-task abort signal: flag + notify, so a waiter created after the
/// cancel still observes it (enable-before-check pattern).
#[derive(Debug)]
pub struct CancelToken {
    flag: AtomicBool,
    notify: tokio::sync::Notify,
}

impl CancelToken {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            flag: AtomicBool::new(false),
            notify: tokio::sync::Notify::new(),
        })
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::Release);
        self.notify.notify_waiters();
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::Acquire)
    }

    /// Resolves once cancelled, without a check-then-wait hole.
    pub async fn cancelled(&self) {
        let notified = self.notify.notified();
        tokio::pin!(notified);
        notified.as_mut().enable();
        loop {
            if self.is_cancelled() {
                return;
            }
            notified.as_mut().await;
        }
    }
}

struct Inflight {
    session_id: String,
    generation: u64,
    token: Arc<CancelToken>,
}

#[derive(Default)]
struct Inner {
    context: Option<SpeechContext>,
    quitting: bool,
    stt_holder: Option<String>,
    tts_holder: Option<String>,
    inflight: HashMap<String, Inflight>,
    handles: Vec<JoinHandle<()>>,
}

#[derive(Default)]
pub struct Registry {
    inner: Mutex<Inner>,
}

fn poison() -> SpeechError {
    SpeechError::new(SpeechCode::Busy, "speech registry lock poisoned")
}

impl Registry {
    /// `speech_context_set`: identical tuple is idempotent; a strictly
    /// newer generation accepts and aborts every in-flight request of the
    /// old context; an older or conflicting tuple is rejected.
    pub fn set_context(&self, session_id: &str, generation: u64) -> SpeechResult<SpeechContext> {
        let mut inner = self.inner.lock().map_err(|_| poison())?;
        if inner.quitting {
            return Err(SpeechError::new(
                SpeechCode::Cancelled,
                "speech lane is shutting down",
            ));
        }
        if let Some(current) = &inner.context {
            if current.session_id == session_id && current.generation == generation {
                return Ok(current.clone());
            }
            if generation <= current.generation {
                return Err(SpeechError::new(
                    SpeechCode::StaleContext,
                    "context tuple is older than or conflicts with the current context",
                ));
            }
        }
        let next = SpeechContext {
            session_id: session_id.to_string(),
            generation,
        };
        inner.context = Some(next.clone());
        inner.abort_all_locked();
        Ok(next)
    }

    pub fn context(&self) -> Option<SpeechContext> {
        self.inner.lock().ok().and_then(|i| i.context.clone())
    }

    /// Admit a request under the current context: validates the identity
    /// against the trusted context, reserves `request_id` and the
    /// kind's single slot, and returns the request's cancel token.
    /// The lock is released before the caller does keyring/HTTP work.
    pub fn admit(
        &self,
        request_id: &str,
        session_id: &str,
        generation: u64,
        kind: RequestKind,
    ) -> SpeechResult<Arc<CancelToken>> {
        let mut inner = self.inner.lock().map_err(|_| poison())?;
        if inner.quitting {
            return Err(SpeechError::new(
                SpeechCode::Cancelled,
                "speech lane is shutting down",
            ));
        }
        let context = inner
            .context
            .clone()
            .ok_or_else(|| SpeechError::new(SpeechCode::NotConfigured, "no speech context set"))?;
        if context.session_id != session_id || context.generation != generation {
            return Err(SpeechError::new(
                SpeechCode::StaleContext,
                "request identity does not match the current context",
            ));
        }
        if inner.inflight.contains_key(request_id) {
            return Err(SpeechError::new(
                SpeechCode::InvalidInput,
                "request_id already admitted",
            ));
        }
        let holder = match kind {
            RequestKind::Stt => &mut inner.stt_holder,
            RequestKind::Tts => &mut inner.tts_holder,
        };
        if holder.is_some() {
            return Err(SpeechError::new(
                SpeechCode::Busy,
                format!("a {} request is already in flight", kind.as_str()),
            )
            .retryable(true));
        }
        *holder = Some(request_id.to_string());
        let token = CancelToken::new();
        inner.inflight.insert(
            request_id.to_string(),
            Inflight {
                session_id: session_id.to_string(),
                generation,
                token: token.clone(),
            },
        );
        Ok(token)
    }

    /// Settle an admitted request: frees its slot and reports whether the
    /// outcome is still deliverable (context unchanged, not cancelled,
    /// not quitting). A non-deliverable success MUST be discarded.
    pub fn settle(&self, request_id: &str) -> bool {
        let mut inner = match self.inner.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
        inner.handles.retain(|h| !h.is_finished());
        let Some(entry) = inner.inflight.remove(request_id) else {
            return false;
        };
        if inner.stt_holder.as_deref() == Some(request_id) {
            inner.stt_holder = None;
        }
        if inner.tts_holder.as_deref() == Some(request_id) {
            inner.tts_holder = None;
        }
        if inner.quitting || entry.token.is_cancelled() {
            return false;
        }
        match &inner.context {
            Some(ctx) => ctx.session_id == entry.session_id && ctx.generation == entry.generation,
            None => false,
        }
    }

    /// `speech_cancel`: window-scoped; idempotent. Returns "cancelled"
    /// when an in-flight request was aborted, else "settled" — local
    /// abort never claims a remote refund.
    pub fn cancel(&self, request_id: &str) -> &'static str {
        match self.inner.lock() {
            Ok(inner) => match inner.inflight.get(request_id) {
                Some(entry) => {
                    entry.token.cancel();
                    "cancelled"
                }
                None => "settled",
            },
            Err(_) => "settled",
        }
    }

    /// Drop the context and abort everything in flight (window hide).
    pub fn invalidate_context(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.context = None;
            inner.abort_all_locked();
        }
    }

    /// Quit: reject future admission and abort in-flight requests.
    pub fn begin_shutdown(&self) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.quitting = true;
            inner.context = None;
            inner.abort_all_locked();
        }
    }

    /// Register a spawned task handle for quit-time joining.
    pub fn track(&self, handle: JoinHandle<()>) {
        if let Ok(mut inner) = self.inner.lock() {
            inner.handles.retain(|h| !h.is_finished());
            inner.handles.push(handle);
        }
    }

    /// Detach every tracked handle so the caller can join them outside
    /// the lock. Returns (in-flight request count, handle count).
    pub fn take_handles(&self) -> (usize, Vec<JoinHandle<()>>) {
        match self.inner.lock() {
            Ok(mut inner) => {
                let inflight = inner.inflight.len();
                (inflight, std::mem::take(&mut inner.handles))
            }
            Err(_) => (0, Vec::new()),
        }
    }

    /// Test/diagnostic count of live slots and requests.
    pub fn active_count(&self) -> usize {
        self.inner
            .lock()
            .map(|i| i.inflight.len())
            .unwrap_or_default()
    }

    pub fn is_quitting(&self) -> bool {
        self.inner.lock().map(|i| i.quitting).unwrap_or(true)
    }
}

impl Inner {
    fn abort_all_locked(&mut self) {
        for entry in self.inflight.values() {
            entry.token.cancel();
        }
        self.stt_holder = None;
        self.tts_holder = None;
    }
}
