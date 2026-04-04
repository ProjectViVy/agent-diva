//! Sliding-window rate limiting for security actions

use parking_lot::Mutex;
use std::time::{Duration, Instant};

/// Tracks actions in a sliding window for rate limiting
#[derive(Debug)]
pub struct ActionTracker {
    /// Recent action timestamps (within the window)
    actions: Mutex<Vec<Instant>>,
    /// Window size in seconds (default: 3600 = 1 hour)
    window_secs: u64,
}

impl ActionTracker {
    /// Create a new action tracker with default 1-hour window
    pub fn new() -> Self {
        Self {
            actions: Mutex::new(Vec::new()),
            window_secs: 3600,
        }
    }

    /// Create a new action tracker with custom window size
    pub fn with_window(window_secs: u64) -> Self {
        Self {
            actions: Mutex::new(Vec::new()),
            window_secs,
        }
    }

    /// Record an action and return the current count in the window
    pub fn record(&self) -> usize {
        let mut actions = self.actions.lock();
        self.cleanup(&mut actions);
        actions.push(Instant::now());
        actions.len()
    }

    /// Get the current action count without recording
    pub fn count(&self) -> usize {
        let mut actions = self.actions.lock();
        self.cleanup(&mut actions);
        actions.len()
    }

    /// Check if the action count exceeds the limit
    pub fn is_rate_limited(&self, max_actions: u32) -> bool {
        self.count() >= max_actions as usize
    }

    /// Try to record an action, returning false if rate limited
    pub fn try_record(&self, max_actions: u32) -> bool {
        let mut actions = self.actions.lock();
        self.cleanup(&mut actions);

        if actions.len() >= max_actions as usize {
            false
        } else {
            actions.push(Instant::now());
            true
        }
    }

    /// Clean up expired actions
    fn cleanup(&self, actions: &mut Vec<Instant>) {
        // If we can't subtract (program running less than window), nothing is expired
        let Some(cutoff) = Instant::now().checked_sub(Duration::from_secs(self.window_secs)) else {
            return;
        };
        actions.retain(|t| *t > cutoff);
    }

    /// Get the window duration
    pub fn window_duration(&self) -> Duration {
        Duration::from_secs(self.window_secs)
    }

    /// Reset all tracked actions
    pub fn reset(&self) {
        let mut actions = self.actions.lock();
        actions.clear();
    }
}

impl Default for ActionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ActionTracker {
    fn clone(&self) -> Self {
        let actions = self.actions.lock();
        Self {
            actions: Mutex::new(actions.clone()),
            window_secs: self.window_secs,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_action_tracker_basic() {
        let tracker = ActionTracker::with_window(1); // 1 second window for testing

        // Record some actions
        assert_eq!(tracker.record(), 1);
        assert_eq!(tracker.record(), 2);
        assert_eq!(tracker.record(), 3);

        // Check count
        assert_eq!(tracker.count(), 3);

        // Wait for window to expire
        thread::sleep(Duration::from_secs(2));

        // Actions should be cleaned up
        assert_eq!(tracker.count(), 0);
    }

    #[test]
    fn test_rate_limiting() {
        let tracker = ActionTracker::with_window(3600);

        // Record actions
        for i in 0..5 {
            assert!(!tracker.is_rate_limited(5), "Should not be rate limited at action {}", i);
            tracker.record();
        }

        // After 5 records with limit of 5, should be rate limited
        assert!(tracker.is_rate_limited(5), "Should be rate limited after 5 actions with limit of 5");
        assert_eq!(tracker.count(), 5, "Count should be 5");
        assert!(!tracker.try_record(5), "Should not be able to record when rate limited");
        assert!(tracker.try_record(6), "Should be able to record when limit is 6");
    }

    #[test]
    fn test_clone() {
        let tracker = ActionTracker::with_window(3600);
        tracker.record();
        tracker.record();

        assert_eq!(tracker.count(), 2, "Original tracker should have 2 actions");

        let cloned = tracker.clone();
        assert_eq!(cloned.count(), 2, "Cloned tracker should have 2 actions");

        // Recording on clone should not affect original
        cloned.record();
        assert_eq!(cloned.count(), 3, "Cloned tracker should have 3 actions after record");
        assert_eq!(tracker.count(), 2, "Original tracker should still have 2 actions");
    }
}
