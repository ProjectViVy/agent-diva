//! Sliding-window rate limiting for security actions

use parking_lot::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Tracks actions per session in a sliding window for rate limiting
#[derive(Debug)]
pub struct ActionTracker {
    /// Per-session action timestamps (within the window)
    sessions: Mutex<HashMap<String, Vec<Instant>>>,
    /// Window size in seconds (default: 3600 = 1 hour)
    window_secs: u64,
}

impl ActionTracker {
    /// Create a new action tracker with default 1-hour window
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            window_secs: 3600,
        }
    }

    /// Create a new action tracker with custom window size
    pub fn with_window(window_secs: u64) -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
            window_secs,
        }
    }

    /// Record an action for the given session key and return the current count
    pub fn record(&self, key: &str) -> usize {
        let mut sessions = self.sessions.lock();
        Self::cleanup_all(&mut sessions, self.window_secs);
        let actions = sessions.entry(key.to_string()).or_default();
        actions.push(Instant::now());
        actions.len()
    }

    /// Get the current action count for a session key without recording
    pub fn count(&self, key: &str) -> usize {
        let mut sessions = self.sessions.lock();
        Self::cleanup_all(&mut sessions, self.window_secs);
        sessions.get(key).map(|v| v.len()).unwrap_or(0)
    }

    /// Check if the action count for a session key exceeds the limit
    pub fn is_rate_limited(&self, key: &str, max_actions: u32) -> bool {
        self.count(key) >= max_actions as usize
    }

    /// Try to record an action for a session key, returning false if rate limited
    pub fn try_record(&self, key: &str, max_actions: u32) -> bool {
        let mut sessions = self.sessions.lock();
        Self::cleanup_all(&mut sessions, self.window_secs);

        let actions = sessions.entry(key.to_string()).or_default();
        if actions.len() >= max_actions as usize {
            false
        } else {
            actions.push(Instant::now());
            true
        }
    }

    /// Clean up expired actions across all sessions
    fn cleanup_all(sessions: &mut HashMap<String, Vec<Instant>>, window_secs: u64) {
        let Some(cutoff) = Instant::now().checked_sub(Duration::from_secs(window_secs)) else {
            return;
        };
        sessions.retain(|_, actions| {
            actions.retain(|t| *t > cutoff);
            !actions.is_empty()
        });
    }

    /// Get the window duration
    pub fn window_duration(&self) -> Duration {
        Duration::from_secs(self.window_secs)
    }

    /// Reset all tracked actions
    pub fn reset(&self) {
        let mut sessions = self.sessions.lock();
        sessions.clear();
    }

    /// Reset tracked actions for a specific session key
    pub fn reset_key(&self, key: &str) {
        let mut sessions = self.sessions.lock();
        sessions.remove(key);
    }
}

impl Default for ActionTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ActionTracker {
    fn clone(&self) -> Self {
        let sessions = self.sessions.lock();
        Self {
            sessions: Mutex::new(sessions.clone()),
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
        assert_eq!(tracker.record(""), 1);
        assert_eq!(tracker.record(""), 2);
        assert_eq!(tracker.record(""), 3);

        // Check count
        assert_eq!(tracker.count(""), 3);

        // Wait for window to expire
        thread::sleep(Duration::from_secs(2));

        // Actions should be cleaned up
        assert_eq!(tracker.count(""), 0);
    }

    #[test]
    fn test_rate_limiting() {
        let tracker = ActionTracker::with_window(3600);

        // Record actions
        for i in 0..5 {
            assert!(
                !tracker.is_rate_limited("", 5),
                "Should not be rate limited at action {}",
                i
            );
            tracker.record("");
        }

        // After 5 records with limit of 5, should be rate limited
        assert!(
            tracker.is_rate_limited("", 5),
            "Should be rate limited after 5 actions with limit of 5"
        );
        assert_eq!(tracker.count(""), 5, "Count should be 5");
        assert!(
            !tracker.try_record("", 5),
            "Should not be able to record when rate limited"
        );
        assert!(
            tracker.try_record("", 6),
            "Should be able to record when limit is 6"
        );
    }

    #[test]
    fn test_clone() {
        let tracker = ActionTracker::with_window(3600);
        tracker.record("");
        tracker.record("");

        assert_eq!(tracker.count(""), 2, "Original tracker should have 2 actions");

        let cloned = tracker.clone();
        assert_eq!(cloned.count(""), 2, "Cloned tracker should have 2 actions");

        // Recording on clone should not affect original
        cloned.record("");
        assert_eq!(
            cloned.count(""),
            3,
            "Cloned tracker should have 3 actions after record"
        );
        assert_eq!(
            tracker.count(""),
            2,
            "Original tracker should still have 2 actions"
        );
    }

    #[test]
    fn test_per_session_independent() {
        let tracker = ActionTracker::with_window(3600);

        // Saturate session "a" with 5 actions
        for _ in 0..5 {
            assert!(
                tracker.try_record("a", 5),
                "Should be able to record session a up to limit"
            );
        }

        // Session "a" should now be rate limited
        assert!(
            tracker.is_rate_limited("a", 5),
            "Session a should be rate limited after 5 actions"
        );
        assert!(
            !tracker.try_record("a", 5),
            "Session a should not be able to record when at limit"
        );

        // Session "b" should still allow actions (independent limit)
        assert!(
            !tracker.is_rate_limited("b", 5),
            "Session b should not be rate limited"
        );
        assert!(
            tracker.try_record("b", 5),
            "Session b should be able to record"
        );
        assert_eq!(tracker.count("b"), 1, "Session b should have 1 action");

        // Session "a" remains blocked
        assert!(
            tracker.is_rate_limited("a", 5),
            "Session a should still be rate limited"
        );
        assert_eq!(tracker.count("a"), 5, "Session a should still have 5 actions");
    }

    #[test]
    fn test_single_session_exceeded() {
        let tracker = ActionTracker::with_window(3600);

        // Saturate a specific session
        for _ in 0..5 {
            tracker.record("session_x");
        }

        // Session should be rate limited
        assert!(
            tracker.is_rate_limited("session_x", 5),
            "Session x should be rate limited after 5 actions"
        );
        assert!(
            !tracker.try_record("session_x", 5),
            "Session x should not be able to record when at limit"
        );

        // Verify count
        assert_eq!(tracker.count("session_x"), 5);
    }

    #[test]
    fn test_default_key_fallback() {
        let tracker = ActionTracker::with_window(3600);

        // Using "" as key should work as default key fallback
        assert_eq!(tracker.count(""), 0, "Default key should start at 0");
        tracker.record("");
        assert_eq!(tracker.count(""), 1, "Default key should have 1 action");

        // A different explicit key should have independent count
        assert_eq!(
            tracker.count("other"),
            0,
            "Other key should be independent of default key"
        );

        // Record under other key and verify independence
        tracker.record("other");
        assert_eq!(tracker.count("other"), 1);
        assert_eq!(tracker.count(""), 1, "Default key should still have 1 action");
    }
}
