use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Coordinates application shutdown across GUI-managed background tasks.
pub struct ShutdownManager {
    cancel_token: CancellationToken,
    exit_in_progress: Arc<AtomicBool>,
}

impl ShutdownManager {
    pub fn new() -> Self {
        Self {
            cancel_token: CancellationToken::new(),
            exit_in_progress: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Mark shutdown as started and notify background tasks exactly once.
    pub fn begin_shutdown(&self) -> bool {
        if self
            .exit_in_progress
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            self.cancel_token.cancel();
            true
        } else {
            false
        }
    }

    pub fn is_shutting_down(&self) -> bool {
        self.exit_in_progress.load(Ordering::SeqCst)
    }

    pub fn mark_exit_observed(&self) {
        self.exit_in_progress.store(true, Ordering::SeqCst);
        self.cancel_token.cancel();
    }

    pub fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone()
    }
}

impl Default for ShutdownManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::ShutdownManager;

    #[test]
    fn shutdown_is_only_started_once() {
        let manager = ShutdownManager::new();

        assert!(manager.begin_shutdown());
        assert!(manager.is_shutting_down());
        assert!(!manager.begin_shutdown());
    }

    #[tokio::test]
    async fn cancellation_token_is_cancelled_on_shutdown() {
        let manager = ShutdownManager::new();
        let token = manager.cancel_token();

        assert!(!token.is_cancelled());
        assert!(manager.begin_shutdown());
        token.cancelled().await;
        assert!(token.is_cancelled());
    }
}
