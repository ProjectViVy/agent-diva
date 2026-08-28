//! Bounded FIFO admission for serializing turns within one logical session.
//!
//! The kernel is transport-agnostic. It owns only admission state and never
//! executes provider, tool, persistence, or memory work.

use parking_lot::Mutex;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::oneshot;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

/// Runtime limits used by [`SessionAdmissionKernel`].
///
/// These limits are deliberately independent from the serialized application
/// configuration. Configuration wiring is owned by the runtime integration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionAdmissionLimits {
    /// Maximum number of waiting turns. The running turn is not counted.
    pub max_queue_depth: usize,
    /// Maximum time a queued turn may wait for a lease.
    pub wait_timeout: Duration,
    /// Time an empty slot may remain before explicit idle eviction.
    pub idle_ttl: Duration,
}

impl Default for SessionAdmissionLimits {
    fn default() -> Self {
        Self {
            max_queue_depth: 2,
            wait_timeout: Duration::from_secs(30),
            idle_ttl: Duration::from_secs(10 * 60),
        }
    }
}

/// Why queued turns were explicitly cancelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAdmissionCancelReason {
    /// The individual turn was cancelled before it acquired a lease.
    TurnCancelled,
    /// The logical session was reset or deleted.
    SessionReset,
    /// The session worker became unavailable.
    WorkerUnavailable,
}

/// A typed admission failure suitable for later projection to stable wire codes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SessionAdmissionError {
    /// The bounded waiting queue was already full.
    #[error("session admission queue is full ({waiting_depth}/{max_queue_depth} waiting turns)")]
    QueueFull {
        /// Configured waiting capacity.
        max_queue_depth: usize,
        /// Waiting depth observed at rejection.
        waiting_depth: usize,
    },
    /// The turn did not acquire a lease before its deadline.
    #[error("session admission wait timed out after {timeout:?}")]
    WaitTimeout {
        /// Configured wait timeout.
        timeout: Duration,
    },
    /// The queued turn was explicitly cancelled.
    #[error("session admission was cancelled: {reason:?}")]
    Cancelled {
        /// Cancellation authority responsible for the result.
        reason: SessionAdmissionCancelReason,
    },
    /// The kernel no longer accepts new turns.
    #[error("session admission is unavailable")]
    Unavailable,
}

/// Read-only state for one session admission slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionAdmissionSnapshot {
    /// Whether one turn currently owns the session lease.
    pub running: bool,
    /// Number of accepted turns waiting in FIFO order.
    pub waiting: usize,
    /// Whether the kernel accepts new turns.
    pub accepting: bool,
    /// Elapsed idle time when the slot has neither a runner nor waiters.
    pub idle_for: Option<Duration>,
}

/// Clock boundary used for timeout and idle-eviction decisions.
pub trait SessionAdmissionClock: Send + Sync + 'static {
    /// Return the current monotonic time.
    fn now(&self) -> Instant;

    /// Sleep until a monotonic deadline.
    fn sleep_until(&self, deadline: Instant) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
}

/// Tokio-backed production clock.
#[derive(Debug, Default)]
pub struct TokioSessionAdmissionClock;

impl SessionAdmissionClock for TokioSessionAdmissionClock {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn sleep_until(&self, deadline: Instant) -> Pin<Box<dyn Future<Output = ()> + Send + 'static>> {
        Box::pin(tokio::time::sleep_until(deadline))
    }
}

/// Process-local bounded admission state keyed by canonical session key.
#[derive(Clone)]
pub struct SessionAdmissionKernel {
    inner: Arc<KernelInner>,
}

impl fmt::Debug for SessionAdmissionKernel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionAdmissionKernel")
            .field("limits", &self.inner.limits)
            .field("accepting", &self.inner.state.lock().accepting)
            .finish_non_exhaustive()
    }
}

impl SessionAdmissionKernel {
    /// Create a kernel backed by Tokio's monotonic clock.
    pub fn new(limits: SessionAdmissionLimits) -> Self {
        Self::with_clock(limits, Arc::new(TokioSessionAdmissionClock))
    }

    /// Create a kernel with an explicit clock implementation.
    pub fn with_clock(
        limits: SessionAdmissionLimits,
        clock: Arc<dyn SessionAdmissionClock>,
    ) -> Self {
        Self {
            inner: Arc::new(KernelInner {
                limits,
                clock,
                state: Mutex::new(KernelState {
                    accepting: true,
                    next_sequence: 1,
                    slots: HashMap::new(),
                }),
            }),
        }
    }

    /// Acquire exclusive turn ownership for a session.
    pub async fn acquire(
        &self,
        session_key: impl Into<Arc<str>>,
    ) -> Result<SessionAdmissionLease, SessionAdmissionError> {
        self.acquire_inner(session_key.into(), None).await
    }

    /// Acquire a lease while observing an individual cancellation token.
    pub async fn acquire_cancellable(
        &self,
        session_key: impl Into<Arc<str>>,
        cancellation: CancellationToken,
    ) -> Result<SessionAdmissionLease, SessionAdmissionError> {
        self.acquire_inner(session_key.into(), Some(cancellation))
            .await
    }

    async fn acquire_inner(
        &self,
        session_key: Arc<str>,
        cancellation: Option<CancellationToken>,
    ) -> Result<SessionAdmissionLease, SessionAdmissionError> {
        let now = self.inner.clock.now();
        let mut expired = Vec::new();

        let admission = {
            let mut state = self.inner.state.lock();
            if !state.accepting {
                return Err(SessionAdmissionError::Unavailable);
            }

            let sequence = state.next_sequence;
            state.next_sequence = state.next_sequence.saturating_add(1);
            let slot = state
                .slots
                .entry(Arc::clone(&session_key))
                .or_insert_with(|| SessionSlot::new(now));

            remove_expired_waiters(slot, now, &mut expired);

            if !slot.running && slot.waiters.is_empty() {
                slot.running = true;
                Admission::Immediate(Grant {
                    sequence,
                    waited: Duration::ZERO,
                    waiting_depth_at_enqueue: 0,
                })
            } else if slot.waiters.len() >= self.inner.limits.max_queue_depth {
                Admission::Rejected(SessionAdmissionError::QueueFull {
                    max_queue_depth: self.inner.limits.max_queue_depth,
                    waiting_depth: slot.waiters.len(),
                })
            } else {
                let (sender, receiver) = oneshot::channel();
                let waiting_depth_at_enqueue = slot.waiters.len() + 1;
                let deadline = now + self.inner.limits.wait_timeout;
                slot.waiters.push_back(Waiter {
                    sequence,
                    enqueued_at: now,
                    deadline,
                    waiting_depth_at_enqueue,
                    sender,
                });
                Admission::Queued {
                    sequence,
                    deadline,
                    receiver,
                }
            }
        };

        notify_expired(expired, self.inner.limits.wait_timeout);

        match admission {
            Admission::Immediate(grant) => Ok(self.lease(session_key, grant)),
            Admission::Rejected(error) => Err(error),
            Admission::Queued {
                sequence,
                deadline,
                receiver,
            } => {
                self.await_queued(session_key, sequence, deadline, receiver, cancellation)
                    .await
            }
        }
    }

    async fn await_queued(
        &self,
        session_key: Arc<str>,
        sequence: u64,
        deadline: Instant,
        mut receiver: oneshot::Receiver<Result<Grant, SessionAdmissionError>>,
        cancellation: Option<CancellationToken>,
    ) -> Result<SessionAdmissionLease, SessionAdmissionError> {
        let _registration = WaiterRegistration {
            inner: Arc::clone(&self.inner),
            session_key: Arc::clone(&session_key),
            sequence,
        };
        let timeout = self.inner.clock.sleep_until(deadline);

        if let Some(cancellation) = cancellation {
            tokio::select! {
                biased;
                result = &mut receiver => self.finish_delivery(session_key, result),
                _ = cancellation.cancelled() => {
                    if self.remove_waiter(&session_key, sequence) {
                        Err(SessionAdmissionError::Cancelled {
                            reason: SessionAdmissionCancelReason::TurnCancelled,
                        })
                    } else {
                        self.finish_delivery(session_key, receiver.await)
                    }
                }
                _ = timeout => {
                    if self.remove_waiter(&session_key, sequence) {
                        Err(SessionAdmissionError::WaitTimeout {
                            timeout: self.inner.limits.wait_timeout,
                        })
                    } else {
                        self.finish_delivery(session_key, receiver.await)
                    }
                }
            }
        } else {
            tokio::select! {
                biased;
                result = &mut receiver => self.finish_delivery(session_key, result),
                _ = timeout => {
                    if self.remove_waiter(&session_key, sequence) {
                        Err(SessionAdmissionError::WaitTimeout {
                            timeout: self.inner.limits.wait_timeout,
                        })
                    } else {
                        self.finish_delivery(session_key, receiver.await)
                    }
                }
            }
        }
    }

    fn finish_delivery(
        &self,
        session_key: Arc<str>,
        delivery: Result<Result<Grant, SessionAdmissionError>, oneshot::error::RecvError>,
    ) -> Result<SessionAdmissionLease, SessionAdmissionError> {
        match delivery {
            Ok(Ok(grant)) => Ok(self.lease(session_key, grant)),
            Ok(Err(error)) => Err(error),
            Err(_) => Err(SessionAdmissionError::Unavailable),
        }
    }

    fn lease(&self, session_key: Arc<str>, grant: Grant) -> SessionAdmissionLease {
        SessionAdmissionLease {
            inner: Some(Arc::clone(&self.inner)),
            session_key,
            sequence: grant.sequence,
            waited: grant.waited,
            waiting_depth_at_enqueue: grant.waiting_depth_at_enqueue,
        }
    }

    fn remove_waiter(&self, session_key: &str, sequence: u64) -> bool {
        remove_waiter(&self.inner, session_key, sequence)
    }

    /// Cancel all queued turns for one session without affecting its running lease.
    pub fn cancel_waiters(&self, session_key: &str, reason: SessionAdmissionCancelReason) -> usize {
        let now = self.inner.clock.now();
        let waiters = {
            let mut state = self.inner.state.lock();
            let Some(slot) = state.slots.get_mut(session_key) else {
                return 0;
            };
            let waiters: Vec<_> = slot.waiters.drain(..).collect();
            if !slot.running {
                slot.last_idle_at = now;
            }
            waiters
        };
        let count = waiters.len();
        for waiter in waiters {
            let _ = waiter
                .sender
                .send(Err(SessionAdmissionError::Cancelled { reason }));
        }
        count
    }

    /// Stop accepting new turns while allowing accepted turns to drain.
    pub fn close(&self) {
        self.inner.state.lock().accepting = false;
    }

    /// Return the current state for one session slot.
    pub fn snapshot(&self, session_key: &str) -> Option<SessionAdmissionSnapshot> {
        let now = self.inner.clock.now();
        let state = self.inner.state.lock();
        let slot = state.slots.get(session_key)?;
        Some(SessionAdmissionSnapshot {
            running: slot.running,
            waiting: slot.waiters.len(),
            accepting: state.accepting,
            idle_for: (!slot.running && slot.waiters.is_empty())
                .then(|| now.duration_since(slot.last_idle_at)),
        })
    }

    /// Evict slots that have remained fully idle for at least the configured TTL.
    pub fn evict_idle(&self) -> Vec<Arc<str>> {
        let now = self.inner.clock.now();
        let mut state = self.inner.state.lock();
        let mut evicted = Vec::new();
        state.slots.retain(|session_key, slot| {
            let should_evict = !slot.running
                && slot.waiters.is_empty()
                && now.duration_since(slot.last_idle_at) >= self.inner.limits.idle_ttl;
            if should_evict {
                evicted.push(Arc::clone(session_key));
            }
            !should_evict
        });
        evicted.sort_unstable_by(|left, right| left.as_ref().cmp(right.as_ref()));
        evicted
    }
}

/// Exclusive ownership of one session turn.
///
/// Dropping the lease synchronously releases the slot and promotes the oldest
/// eligible waiter without holding a lock across asynchronous work.
pub struct SessionAdmissionLease {
    inner: Option<Arc<KernelInner>>,
    session_key: Arc<str>,
    sequence: u64,
    waited: Duration,
    waiting_depth_at_enqueue: usize,
}

impl fmt::Debug for SessionAdmissionLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SessionAdmissionLease")
            .field("session_key", &self.session_key)
            .field("sequence", &self.sequence)
            .field("waited", &self.waited)
            .field("waiting_depth_at_enqueue", &self.waiting_depth_at_enqueue)
            .finish()
    }
}

impl SessionAdmissionLease {
    /// Canonical session key protected by this lease.
    pub fn session_key(&self) -> &str {
        &self.session_key
    }

    /// Monotonic admission sequence assigned at acceptance.
    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    /// Time spent waiting before this lease was granted.
    pub fn waited(&self) -> Duration {
        self.waited
    }

    /// Queue depth immediately after this turn was enqueued, or zero if immediate.
    pub fn waiting_depth_at_enqueue(&self) -> usize {
        self.waiting_depth_at_enqueue
    }
}

impl Drop for SessionAdmissionLease {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            release_and_promote(&inner, &self.session_key);
        }
    }
}

struct KernelInner {
    limits: SessionAdmissionLimits,
    clock: Arc<dyn SessionAdmissionClock>,
    state: Mutex<KernelState>,
}

struct KernelState {
    accepting: bool,
    next_sequence: u64,
    slots: HashMap<Arc<str>, SessionSlot>,
}

struct SessionSlot {
    running: bool,
    waiters: VecDeque<Waiter>,
    last_idle_at: Instant,
}

impl SessionSlot {
    fn new(now: Instant) -> Self {
        Self {
            running: false,
            waiters: VecDeque::new(),
            last_idle_at: now,
        }
    }
}

struct Waiter {
    sequence: u64,
    enqueued_at: Instant,
    deadline: Instant,
    waiting_depth_at_enqueue: usize,
    sender: oneshot::Sender<Result<Grant, SessionAdmissionError>>,
}

struct WaiterRegistration {
    inner: Arc<KernelInner>,
    session_key: Arc<str>,
    sequence: u64,
}

impl Drop for WaiterRegistration {
    fn drop(&mut self) {
        remove_waiter(&self.inner, &self.session_key, self.sequence);
    }
}

struct Grant {
    sequence: u64,
    waited: Duration,
    waiting_depth_at_enqueue: usize,
}

enum Admission {
    Immediate(Grant),
    Queued {
        sequence: u64,
        deadline: Instant,
        receiver: oneshot::Receiver<Result<Grant, SessionAdmissionError>>,
    },
    Rejected(SessionAdmissionError),
}

fn remove_expired_waiters(
    slot: &mut SessionSlot,
    now: Instant,
    expired: &mut Vec<oneshot::Sender<Result<Grant, SessionAdmissionError>>>,
) {
    let mut retained = VecDeque::with_capacity(slot.waiters.len());
    while let Some(waiter) = slot.waiters.pop_front() {
        if waiter.deadline <= now {
            expired.push(waiter.sender);
        } else {
            retained.push_back(waiter);
        }
    }
    slot.waiters = retained;
    if !slot.running && slot.waiters.is_empty() && !expired.is_empty() {
        slot.last_idle_at = now;
    }
}

fn remove_waiter(inner: &Arc<KernelInner>, session_key: &str, sequence: u64) -> bool {
    let now = inner.clock.now();
    let mut state = inner.state.lock();
    let Some(slot) = state.slots.get_mut(session_key) else {
        return false;
    };
    let Some(position) = slot
        .waiters
        .iter()
        .position(|waiter| waiter.sequence == sequence)
    else {
        return false;
    };
    slot.waiters.remove(position);
    if !slot.running && slot.waiters.is_empty() {
        slot.last_idle_at = now;
    }
    true
}

fn notify_expired(
    expired: Vec<oneshot::Sender<Result<Grant, SessionAdmissionError>>>,
    timeout: Duration,
) {
    for sender in expired {
        let _ = sender.send(Err(SessionAdmissionError::WaitTimeout { timeout }));
    }
}

fn release_and_promote(inner: &Arc<KernelInner>, session_key: &Arc<str>) {
    {
        let now = inner.clock.now();
        let mut state = inner.state.lock();
        let Some(slot) = state.slots.get_mut(session_key.as_ref()) else {
            return;
        };
        if !slot.running {
            return;
        }
        slot.running = false;
        slot.last_idle_at = now;
    }

    promote_next(inner, session_key);
}

fn promote_next(inner: &Arc<KernelInner>, session_key: &Arc<str>) {
    loop {
        let now = inner.clock.now();
        let (expired, candidate) = {
            let mut state = inner.state.lock();
            let Some(slot) = state.slots.get_mut(session_key.as_ref()) else {
                return;
            };
            if slot.running {
                return;
            }

            let mut expired = Vec::new();
            while slot
                .waiters
                .front()
                .is_some_and(|waiter| waiter.deadline <= now)
            {
                if let Some(waiter) = slot.waiters.pop_front() {
                    expired.push(waiter.sender);
                }
            }

            let candidate = slot.waiters.pop_front().map(|waiter| {
                slot.running = true;
                let grant = Grant {
                    sequence: waiter.sequence,
                    waited: now.duration_since(waiter.enqueued_at),
                    waiting_depth_at_enqueue: waiter.waiting_depth_at_enqueue,
                };
                (waiter.sender, grant)
            });
            if candidate.is_none() {
                slot.last_idle_at = now;
            }
            (expired, candidate)
        };

        notify_expired(expired, inner.limits.wait_timeout);

        let Some((sender, grant)) = candidate else {
            return;
        };
        if sender.send(Ok(grant)).is_ok() {
            return;
        }

        let mut state = inner.state.lock();
        if let Some(slot) = state.slots.get_mut(session_key.as_ref()) {
            slot.running = false;
            slot.last_idle_at = inner.clock.now();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    fn limits() -> SessionAdmissionLimits {
        SessionAdmissionLimits {
            max_queue_depth: 2,
            wait_timeout: Duration::from_secs(30),
            idle_ttl: Duration::from_secs(600),
        }
    }

    async fn wait_for_depth(kernel: &SessionAdmissionKernel, session_key: &str, expected: usize) {
        for _ in 0..100 {
            if kernel.snapshot(session_key).map(|state| state.waiting) == Some(expected) {
                return;
            }
            tokio::task::yield_now().await;
        }
        panic!("session {session_key} did not reach waiting depth {expected}");
    }

    #[tokio::test]
    async fn first_turn_is_immediate_and_sessions_are_independent() {
        let kernel = SessionAdmissionKernel::new(limits());
        let first = kernel.acquire("gui:first").await.unwrap();
        let _second = kernel.acquire("gui:second").await.unwrap();

        assert_eq!(first.session_key(), "gui:first");
        assert_eq!(first.waited(), Duration::ZERO);
        assert_eq!(first.waiting_depth_at_enqueue(), 0);
        assert!(kernel.snapshot("gui:first").unwrap().running);
        assert!(kernel.snapshot("gui:second").unwrap().running);
    }

    #[tokio::test]
    async fn same_session_is_fifo_and_running_is_excluded_from_capacity() {
        let kernel = SessionAdmissionKernel::new(limits());
        let first = kernel.acquire("gui:fifo").await.unwrap();
        let (acquired_tx, mut acquired_rx) = mpsc::unbounded_channel();
        let (release_one_tx, release_one_rx) = oneshot::channel();

        let second_kernel = kernel.clone();
        let second_tx = acquired_tx.clone();
        let second = tokio::spawn(async move {
            let lease = second_kernel.acquire("gui:fifo").await.unwrap();
            second_tx.send(lease.sequence()).unwrap();
            let _ = release_one_rx.await;
            drop(lease);
        });
        wait_for_depth(&kernel, "gui:fifo", 1).await;

        let third_kernel = kernel.clone();
        let third = tokio::spawn(async move {
            let lease = third_kernel.acquire("gui:fifo").await.unwrap();
            acquired_tx.send(lease.sequence()).unwrap();
            lease
        });
        wait_for_depth(&kernel, "gui:fifo", 2).await;

        let error = kernel.acquire("gui:fifo").await.unwrap_err();
        assert_eq!(
            error,
            SessionAdmissionError::QueueFull {
                max_queue_depth: 2,
                waiting_depth: 2,
            }
        );

        drop(first);
        let second_sequence = acquired_rx.recv().await.unwrap();
        assert!(acquired_rx.try_recv().is_err());
        release_one_tx.send(()).unwrap();
        let third_sequence = acquired_rx.recv().await.unwrap();
        assert!(second_sequence < third_sequence);

        drop(third.await.unwrap());
        second.await.unwrap();
        assert_eq!(kernel.snapshot("gui:fifo").unwrap().waiting, 0);
        assert!(!kernel.snapshot("gui:fifo").unwrap().running);
    }

    #[tokio::test]
    async fn zero_depth_allows_a_runner_but_no_waiter() {
        let kernel = SessionAdmissionKernel::new(SessionAdmissionLimits {
            max_queue_depth: 0,
            ..limits()
        });
        let lease = kernel.acquire("gui:zero").await.unwrap();
        assert!(matches!(
            kernel.acquire("gui:zero").await,
            Err(SessionAdmissionError::QueueFull {
                max_queue_depth: 0,
                waiting_depth: 0
            })
        ));
        drop(lease);
    }

    #[tokio::test(start_paused = true)]
    async fn timeout_removes_waiter_and_releases_capacity() {
        let kernel = SessionAdmissionKernel::new(limits());
        let first = kernel.acquire("gui:timeout").await.unwrap();
        let waiting_kernel = kernel.clone();
        let waiting = tokio::spawn(async move { waiting_kernel.acquire("gui:timeout").await });
        wait_for_depth(&kernel, "gui:timeout", 1).await;

        tokio::time::advance(Duration::from_secs(30)).await;
        let error = waiting.await.unwrap().unwrap_err();
        assert_eq!(
            error,
            SessionAdmissionError::WaitTimeout {
                timeout: Duration::from_secs(30)
            }
        );
        assert_eq!(kernel.snapshot("gui:timeout").unwrap().waiting, 0);

        let replacement_kernel = kernel.clone();
        let replacement =
            tokio::spawn(async move { replacement_kernel.acquire("gui:timeout").await.unwrap() });
        wait_for_depth(&kernel, "gui:timeout", 1).await;
        drop(first);
        drop(replacement.await.unwrap());
    }

    #[tokio::test(start_paused = true)]
    async fn release_timeout_race_has_no_ghost_lease() {
        for attempt in 0..20 {
            let key = format!("gui:race-{attempt}");
            let kernel = SessionAdmissionKernel::new(limits());
            let first = kernel
                .acquire(Arc::<str>::from(key.as_str()))
                .await
                .unwrap();
            let waiting_kernel = kernel.clone();
            let waiting_key = Arc::<str>::from(key.as_str());
            let waiting = tokio::spawn(async move { waiting_kernel.acquire(waiting_key).await });
            wait_for_depth(&kernel, &key, 1).await;

            tokio::time::advance(Duration::from_secs(30)).await;
            drop(first);
            if let Ok(lease) = waiting.await.unwrap() {
                drop(lease);
            }
            tokio::task::yield_now().await;

            let snapshot = kernel.snapshot(&key).unwrap();
            assert!(!snapshot.running);
            assert_eq!(snapshot.waiting, 0);
        }
    }

    #[tokio::test]
    async fn individual_cancellation_removes_only_that_waiter() {
        let kernel = SessionAdmissionKernel::new(limits());
        let first = kernel.acquire("gui:cancel").await.unwrap();
        let cancellation = CancellationToken::new();
        let cancelled_kernel = kernel.clone();
        let cancelled_token = cancellation.clone();
        let cancelled = tokio::spawn(async move {
            cancelled_kernel
                .acquire_cancellable("gui:cancel", cancelled_token)
                .await
        });
        wait_for_depth(&kernel, "gui:cancel", 1).await;

        let surviving_kernel = kernel.clone();
        let surviving =
            tokio::spawn(async move { surviving_kernel.acquire("gui:cancel").await.unwrap() });
        wait_for_depth(&kernel, "gui:cancel", 2).await;
        cancellation.cancel();

        assert_eq!(
            cancelled.await.unwrap().unwrap_err(),
            SessionAdmissionError::Cancelled {
                reason: SessionAdmissionCancelReason::TurnCancelled
            }
        );
        assert_eq!(kernel.snapshot("gui:cancel").unwrap().waiting, 1);
        drop(first);
        drop(surviving.await.unwrap());
    }

    #[tokio::test]
    async fn session_cancellation_drains_waiters_but_keeps_runner() {
        let kernel = SessionAdmissionKernel::new(limits());
        let first = kernel.acquire("gui:reset").await.unwrap();
        let mut waiters = Vec::new();
        for expected_depth in 1..=2 {
            let waiter_kernel = kernel.clone();
            waiters.push(tokio::spawn(async move {
                waiter_kernel.acquire("gui:reset").await
            }));
            wait_for_depth(&kernel, "gui:reset", expected_depth).await;
        }

        assert_eq!(
            kernel.cancel_waiters("gui:reset", SessionAdmissionCancelReason::SessionReset),
            2
        );
        for waiter in waiters {
            assert_eq!(
                waiter.await.unwrap().unwrap_err(),
                SessionAdmissionError::Cancelled {
                    reason: SessionAdmissionCancelReason::SessionReset
                }
            );
        }
        let snapshot = kernel.snapshot("gui:reset").unwrap();
        assert!(snapshot.running);
        assert_eq!(snapshot.waiting, 0);
        drop(first);
    }

    #[tokio::test]
    async fn dropped_waiter_future_is_removed_without_blocking_fifo() {
        let kernel = SessionAdmissionKernel::new(limits());
        let first = kernel.acquire("gui:dropped").await.unwrap();
        let dropped_kernel = kernel.clone();
        let dropped =
            tokio::spawn(async move { dropped_kernel.acquire("gui:dropped").await.unwrap() });
        wait_for_depth(&kernel, "gui:dropped", 1).await;
        dropped.abort();
        let _ = dropped.await;
        wait_for_depth(&kernel, "gui:dropped", 0).await;

        let next_kernel = kernel.clone();
        let next = tokio::spawn(async move { next_kernel.acquire("gui:dropped").await.unwrap() });
        wait_for_depth(&kernel, "gui:dropped", 1).await;
        drop(first);
        drop(next.await.unwrap());
    }

    #[tokio::test]
    async fn aborting_lease_owner_promotes_the_next_waiter() {
        let kernel = SessionAdmissionKernel::new(limits());
        let owner_kernel = kernel.clone();
        let (ready_tx, ready_rx) = oneshot::channel();
        let owner = tokio::spawn(async move {
            let _lease = owner_kernel.acquire("gui:abort").await.unwrap();
            ready_tx.send(()).unwrap();
            std::future::pending::<()>().await;
        });
        ready_rx.await.unwrap();

        let next_kernel = kernel.clone();
        let next = tokio::spawn(async move { next_kernel.acquire("gui:abort").await.unwrap() });
        wait_for_depth(&kernel, "gui:abort", 1).await;
        owner.abort();
        let _ = owner.await;
        drop(next.await.unwrap());
    }

    #[tokio::test(start_paused = true)]
    async fn idle_eviction_respects_ttl_and_active_slots() {
        let kernel = SessionAdmissionKernel::new(SessionAdmissionLimits {
            wait_timeout: Duration::from_secs(1_200),
            ..limits()
        });
        let idle = kernel.acquire("gui:idle").await.unwrap();
        drop(idle);
        let running = kernel.acquire("gui:running").await.unwrap();
        let queued_kernel = kernel.clone();
        let queued =
            tokio::spawn(async move { queued_kernel.acquire("gui:running").await.unwrap() });
        wait_for_depth(&kernel, "gui:running", 1).await;

        tokio::time::advance(Duration::from_secs(599)).await;
        assert!(kernel.evict_idle().is_empty());
        tokio::time::advance(Duration::from_secs(1)).await;
        assert_eq!(kernel.evict_idle(), vec![Arc::<str>::from("gui:idle")]);
        assert!(kernel.snapshot("gui:idle").is_none());
        assert!(kernel.snapshot("gui:running").is_some());

        drop(running);
        drop(queued.await.unwrap());
    }

    #[tokio::test]
    async fn close_rejects_new_turns_and_drains_accepted_waiters() {
        let kernel = SessionAdmissionKernel::new(limits());
        let first = kernel.acquire("gui:close").await.unwrap();
        let waiting_kernel = kernel.clone();
        let waiting =
            tokio::spawn(async move { waiting_kernel.acquire("gui:close").await.unwrap() });
        wait_for_depth(&kernel, "gui:close", 1).await;

        kernel.close();
        assert_eq!(
            kernel.acquire("gui:new").await.unwrap_err(),
            SessionAdmissionError::Unavailable
        );
        assert!(!kernel.snapshot("gui:close").unwrap().accepting);

        drop(first);
        drop(waiting.await.unwrap());
        let snapshot = kernel.snapshot("gui:close").unwrap();
        assert!(!snapshot.running);
        assert_eq!(snapshot.waiting, 0);
    }

    #[test]
    fn defaults_and_clock_injection_are_stable() {
        assert_eq!(
            SessionAdmissionLimits::default(),
            SessionAdmissionLimits {
                max_queue_depth: 2,
                wait_timeout: Duration::from_secs(30),
                idle_ttl: Duration::from_secs(600),
            }
        );
        let kernel = SessionAdmissionKernel::with_clock(
            SessionAdmissionLimits::default(),
            Arc::new(TokioSessionAdmissionClock),
        );
        assert!(format!("{kernel:?}").contains("SessionAdmissionKernel"));
    }
}
