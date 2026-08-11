//! Sliding-window circuit breaker for model/provider rejection storms.
//!
//! Unlike the sandbox's approval-based [`GuardianRejectionCircuitBreaker`]
//! (which trips on command-approval denials), this breaker counts provider
//! rejection failures (rate-limit, context-length, auth, etc.). When rejections
//! within a sliding window exceed a threshold, the loop refuses to start a new
//! model iteration until the window slides clear — a dead-loop / rejection-storm
//! safety valve, not a budget-management feature.

use std::sync::Arc;

use super::rate_limit::ActionTracker;

/// Counts model/provider rejection failures over a sliding window.
#[derive(Clone, Debug)]
pub struct RejectionCircuitBreaker {
    tracker: Arc<ActionTracker>,
    /// Maximum rejections allowed within the window before the circuit trips.
    threshold: u32,
}

impl RejectionCircuitBreaker {
    /// Create a breaker with the given window and rejection threshold.
    pub fn new(window_secs: u64, threshold: u32) -> Self {
        Self {
            tracker: Arc::new(ActionTracker::with_window(window_secs)),
            threshold,
        }
    }

    /// Record one rejection failure and return the current count in the window.
    pub fn record_rejection(&self) -> usize {
        self.tracker.record()
    }

    /// Whether the number of rejections in the sliding window has reached the
    /// threshold and the circuit should refuse a new model iteration.
    pub fn is_triggered(&self) -> bool {
        self.tracker.is_rate_limited(self.threshold)
    }

    /// Current rejection count within the window (without recording).
    pub fn rejection_count(&self) -> usize {
        self.tracker.count()
    }

    /// The configured rejection threshold.
    pub fn threshold(&self) -> u32 {
        self.threshold
    }

    /// Force-clear all tracked rejections (e.g. operator reset).
    pub fn reset(&self) {
        self.tracker.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn threshold_breaker() -> RejectionCircuitBreaker {
        RejectionCircuitBreaker::new(1, 3)
    }

    #[test]
    fn trips_only_when_threshold_met() {
        let breaker = threshold_breaker();
        assert!(!breaker.is_triggered(), "fresh breaker must not be triggered");
        breaker.record_rejection();
        breaker.record_rejection();
        assert!(
            !breaker.is_triggered(),
            "below threshold must not trip the circuit"
        );
        breaker.record_rejection();
        assert!(breaker.is_triggered(), "threshold hit must trip the circuit");
        assert_eq!(breaker.rejection_count(), 3);
    }

    #[test]
    fn recovers_after_window_slides() {
        let breaker = threshold_breaker();
        for _ in 0..3 {
            breaker.record_rejection();
        }
        assert!(breaker.is_triggered());
        // Wait for the 1-second window to clear.
        thread::sleep(Duration::from_secs(2));
        assert_eq!(breaker.rejection_count(), 0);
        assert!(
            !breaker.is_triggered(),
            "circuit must recover once the window slides clear"
        );
    }

    #[test]
    fn reset_clears_tripped_state() {
        let breaker = threshold_breaker();
        for _ in 0..3 {
            breaker.record_rejection();
        }
        assert!(breaker.is_triggered());
        breaker.reset();
        assert!(!breaker.is_triggered());
        assert_eq!(breaker.threshold(), 3);
    }

    #[test]
    fn clones_share_the_same_window() {
        let breaker = threshold_breaker();
        let cloned = breaker.clone();
        breaker.record_rejection();
        breaker.record_rejection();
        breaker.record_rejection();
        assert!(
            cloned.is_triggered(),
            "clones must share the underlying rejection window"
        );
    }
}