//! Session-aware turn dispatch built on the core bounded admission kernel.
//!
//! HQ-02 deliberately keeps transport consumption serialized until request
//! correlation reaches every streaming consumer in HQ-03.  This dispatcher is
//! nevertheless concurrency-safe: callers that own independent turn workers
//! can submit concurrently, while one canonical session remains FIFO/serial.

use agent_diva_core::session::{
    SessionAdmissionCancelReason, SessionAdmissionError, SessionAdmissionKernel,
    SessionAdmissionLimits,
};
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

#[derive(Clone, Debug)]
pub(crate) struct SessionDispatcher {
    kernel: SessionAdmissionKernel,
    running: Arc<Mutex<HashMap<String, RunningTurn>>>,
}

#[derive(Clone, Debug)]
struct RunningTurn {
    sequence: u64,
    cancellation: CancellationToken,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum SessionDispatchError<E> {
    #[error(transparent)]
    Admission(#[from] SessionAdmissionError),
    #[error("turn execution failed: {0}")]
    Turn(E),
}

impl Default for SessionDispatcher {
    fn default() -> Self {
        Self::new(SessionAdmissionLimits::default())
    }
}

impl SessionDispatcher {
    pub(crate) fn new(limits: SessionAdmissionLimits) -> Self {
        Self {
            kernel: SessionAdmissionKernel::new(limits),
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Acquire the per-session lease before constructing or polling the turn.
    /// This ordering is the zero-side-effect boundary for queue rejection.
    pub(crate) async fn dispatch<T, E, F, Fut>(
        &self,
        session_key: String,
        execute: F,
    ) -> Result<T, SessionDispatchError<E>>
    where
        F: FnOnce(CancellationToken) -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let request_cancellation = CancellationToken::new();
        let lease = self
            .kernel
            .acquire_cancellable(session_key.clone(), request_cancellation.clone())
            .await?;
        let sequence = lease.sequence();
        let turn_cancellation = request_cancellation.child_token();
        {
            let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
            running.insert(
                session_key.clone(),
                RunningTurn {
                    sequence,
                    cancellation: turn_cancellation.clone(),
                },
            );
        }
        let _running_guard = RunningGuard {
            dispatcher: self.clone(),
            session_key,
            sequence,
        };
        let _lease = lease;
        execute(turn_cancellation)
            .await
            .map_err(SessionDispatchError::Turn)
    }

    /// Stop affects only the running turn. Queued leases retain FIFO position.
    pub(crate) fn stop_running(&self, session_key: &str) -> bool {
        let running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        let Some(turn) = running.get(session_key) else {
            return false;
        };
        turn.cancellation.cancel();
        true
    }

    /// Reset/delete cancellation affects the running turn and every waiter.
    pub(crate) fn reset_session(&self, session_key: &str) -> usize {
        self.stop_running(session_key);
        self.kernel
            .cancel_waiters(session_key, SessionAdmissionCancelReason::SessionReset)
    }

    pub(crate) fn close(&self) {
        self.kernel.close();
    }

    pub(crate) fn evict_idle(&self) -> Vec<Arc<str>> {
        self.kernel.evict_idle()
    }

    fn clear_running(&self, session_key: &str, sequence: u64) {
        let mut running = self.running.lock().unwrap_or_else(|e| e.into_inner());
        if running.get(session_key).map(|turn| turn.sequence) == Some(sequence) {
            running.remove(session_key);
        }
    }
}

struct RunningGuard {
    dispatcher: SessionDispatcher,
    session_key: String,
    sequence: u64,
}

impl Drop for RunningGuard {
    fn drop(&mut self) {
        self.dispatcher
            .clear_running(&self.session_key, self.sequence);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use tokio::sync::{mpsc, Barrier, Semaphore};

    fn limits(depth: usize, timeout: Duration) -> SessionAdmissionLimits {
        SessionAdmissionLimits {
            max_queue_depth: depth,
            wait_timeout: timeout,
            idle_ttl: Duration::from_secs(600),
        }
    }

    #[tokio::test]
    async fn same_session_never_reenters_and_keeps_fifo_order() {
        let dispatcher = SessionDispatcher::new(limits(2, Duration::from_secs(2)));
        let gate = Arc::new(Semaphore::new(0));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let (tx, mut rx) = mpsc::unbounded_channel();

        let mut handles = Vec::new();
        for ordinal in 0..3usize {
            let dispatcher = dispatcher.clone();
            let gate = gate.clone();
            let active = active.clone();
            let max_active = max_active.clone();
            let tx = tx.clone();
            handles.push(tokio::spawn(async move {
                dispatcher
                    .dispatch("gui:same".to_string(), move |_| async move {
                        let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(now, Ordering::SeqCst);
                        tx.send(ordinal).unwrap();
                        gate.acquire().await.unwrap().forget();
                        active.fetch_sub(1, Ordering::SeqCst);
                        Ok::<_, ()>(())
                    })
                    .await
                    .unwrap();
            }));
            tokio::task::yield_now().await;
        }

        assert_eq!(rx.recv().await, Some(0));
        gate.add_permits(1);
        assert_eq!(rx.recv().await, Some(1));
        gate.add_permits(1);
        assert_eq!(rx.recv().await, Some(2));
        gate.add_permits(1);
        for handle in handles {
            handle.await.unwrap();
        }
        assert_eq!(max_active.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn different_sessions_execute_concurrently() {
        let dispatcher = SessionDispatcher::default();
        let barrier = Arc::new(Barrier::new(3));
        let active = Arc::new(AtomicUsize::new(0));
        let max_active = Arc::new(AtomicUsize::new(0));
        let mut handles = Vec::new();

        for key in ["gui:a", "gui:b"] {
            let dispatcher = dispatcher.clone();
            let barrier = barrier.clone();
            let active = active.clone();
            let max_active = max_active.clone();
            handles.push(tokio::spawn(async move {
                dispatcher
                    .dispatch(key.to_string(), move |_| async move {
                        let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                        max_active.fetch_max(now, Ordering::SeqCst);
                        barrier.wait().await;
                        active.fetch_sub(1, Ordering::SeqCst);
                        Ok::<_, ()>(())
                    })
                    .await
                    .unwrap();
            }));
        }
        barrier.wait().await;
        for handle in handles {
            handle.await.unwrap();
        }
        assert_eq!(max_active.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn queue_full_is_rejected_before_execute_is_polled() {
        let dispatcher = SessionDispatcher::new(limits(1, Duration::from_secs(5)));
        let gate = Arc::new(Semaphore::new(0));
        let effects = Arc::new(AtomicUsize::new(0));

        let first = {
            let dispatcher = dispatcher.clone();
            let gate = gate.clone();
            tokio::spawn(async move {
                dispatcher
                    .dispatch("gui:full".to_string(), move |_| async move {
                        gate.acquire().await.unwrap().forget();
                        Ok::<_, ()>(())
                    })
                    .await
            })
        };
        tokio::task::yield_now().await;
        let second = {
            let dispatcher = dispatcher.clone();
            let gate = gate.clone();
            tokio::spawn(async move {
                dispatcher
                    .dispatch("gui:full".to_string(), move |_| async move {
                        gate.acquire().await.unwrap().forget();
                        Ok::<_, ()>(())
                    })
                    .await
            })
        };
        tokio::task::yield_now().await;

        let rejected = dispatcher
            .dispatch("gui:full".to_string(), {
                let effects = effects.clone();
                move |_| async move {
                    effects.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ()>(())
                }
            })
            .await;
        assert!(matches!(
            rejected,
            Err(SessionDispatchError::Admission(
                SessionAdmissionError::QueueFull { .. }
            ))
        ));
        assert_eq!(effects.load(Ordering::SeqCst), 0);

        gate.add_permits(2);
        first.await.unwrap().unwrap();
        second.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn wait_timeout_is_rejected_before_execute_is_polled() {
        let dispatcher = SessionDispatcher::new(limits(1, Duration::from_millis(20)));
        let gate = Arc::new(Semaphore::new(0));
        let effects = Arc::new(AtomicUsize::new(0));
        let first = {
            let dispatcher = dispatcher.clone();
            let gate = gate.clone();
            tokio::spawn(async move {
                dispatcher
                    .dispatch("gui:timeout".to_string(), move |_| async move {
                        gate.acquire().await.unwrap().forget();
                        Ok::<_, ()>(())
                    })
                    .await
            })
        };
        tokio::task::yield_now().await;

        let timed_out = dispatcher
            .dispatch("gui:timeout".to_string(), {
                let effects = effects.clone();
                move |_| async move {
                    effects.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, ()>(())
                }
            })
            .await;
        assert!(matches!(
            timed_out,
            Err(SessionDispatchError::Admission(
                SessionAdmissionError::WaitTimeout { .. }
            ))
        ));
        assert_eq!(effects.load(Ordering::SeqCst), 0);
        gate.add_permits(1);
        first.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn stop_cancels_running_but_not_waiter() {
        let dispatcher = SessionDispatcher::default();
        let running_started = Arc::new(Barrier::new(2));
        let first = {
            let dispatcher = dispatcher.clone();
            let started = running_started.clone();
            tokio::spawn(async move {
                dispatcher
                    .dispatch("gui:stop".to_string(), move |cancel| async move {
                        started.wait().await;
                        cancel.cancelled().await;
                        Ok::<_, ()>(())
                    })
                    .await
            })
        };
        running_started.wait().await;
        let waiter = {
            let dispatcher = dispatcher.clone();
            tokio::spawn(async move {
                dispatcher
                    .dispatch("gui:stop".to_string(), |_| async { Ok::<_, ()>(7) })
                    .await
            })
        };
        tokio::task::yield_now().await;
        assert!(dispatcher.stop_running("gui:stop"));
        first.await.unwrap().unwrap();
        assert_eq!(waiter.await.unwrap().unwrap(), 7);
    }

    #[tokio::test]
    async fn reset_cancels_running_and_all_waiters() {
        let dispatcher = SessionDispatcher::default();
        let running_started = Arc::new(Barrier::new(2));
        let first = {
            let dispatcher = dispatcher.clone();
            let started = running_started.clone();
            tokio::spawn(async move {
                dispatcher
                    .dispatch("gui:reset".to_string(), move |cancel| async move {
                        started.wait().await;
                        cancel.cancelled().await;
                        Ok::<_, ()>(())
                    })
                    .await
            })
        };
        running_started.wait().await;
        let waiter = {
            let dispatcher = dispatcher.clone();
            tokio::spawn(async move {
                dispatcher
                    .dispatch("gui:reset".to_string(), |_| async { Ok::<_, ()>(()) })
                    .await
            })
        };
        tokio::task::yield_now().await;
        assert_eq!(dispatcher.reset_session("gui:reset"), 1);
        first.await.unwrap().unwrap();
        assert!(matches!(
            waiter.await.unwrap(),
            Err(SessionDispatchError::Admission(
                SessionAdmissionError::Cancelled {
                    reason: SessionAdmissionCancelReason::SessionReset
                }
            ))
        ));
    }
}
