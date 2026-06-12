//! NAG mechanism for the planning subsystem.
//!
//! [`NagTracker`] tracks consecutive agent turns without planning tool calls
//! and fires a nag message when the threshold is exceeded.

/// Tracks consecutive turns without a planning tool call.
///
/// When the agent ignores its TodoList for too many turns, [`NagTracker`]
/// produces a nag message that should be injected into the conversation.
#[derive(Debug, Clone)]
pub struct NagTracker {
    /// Consecutive turns without a planning tool call.
    turns_without_planning: u32,
    /// Number of turns before nagging fires.
    threshold: u32,
}

impl NagTracker {
    /// Create a new tracker with threshold=3 and counter at zero.
    pub fn new() -> Self {
        Self {
            turns_without_planning: 0,
            threshold: 3,
        }
    }

    /// Record the outcome of one agent turn.
    ///
    /// If the turn included a planning tool call (`todo_write` / `todo_show`),
    /// the counter resets to zero.  Otherwise the counter increments.
    pub fn record_turn(&mut self, had_planning_call: bool) {
        if had_planning_call {
            self.turns_without_planning = 0;
        } else {
            self.turns_without_planning += 1;
        }
    }

    /// Returns `true` when the counter has reached the nag threshold.
    pub fn should_nag(&self) -> bool {
        self.turns_without_planning >= self.threshold
    }

    /// The standard nag message to inject when the agent is off-track.
    pub fn nag_message(&self) -> &str {
        "You have pending TodoList items. Pick up the next one now."
    }

    /// Reset the counter to zero (e.g. when the agent switches plans).
    pub fn reset(&mut self) {
        self.turns_without_planning = 0;
    }
}

impl Default for NagTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nag_new_starts_at_zero() {
        let tracker = NagTracker::new();
        assert_eq!(tracker.turns_without_planning, 0);
        assert!(!tracker.should_nag());
    }

    #[test]
    fn test_nag_increments_without_planning_call() {
        let mut tracker = NagTracker::new();
        tracker.record_turn(false);
        assert_eq!(tracker.turns_without_planning, 1);
        tracker.record_turn(false);
        assert_eq!(tracker.turns_without_planning, 2);
    }

    #[test]
    fn test_nag_resets_on_planning_call() {
        let mut tracker = NagTracker::new();
        tracker.record_turn(false);
        tracker.record_turn(false);
        assert_eq!(tracker.turns_without_planning, 2);
        tracker.record_turn(true);
        assert_eq!(tracker.turns_without_planning, 0);
    }

    #[test]
    fn test_nag_fires_at_threshold() {
        let mut tracker = NagTracker::new();
        for _ in 0..3 {
            tracker.record_turn(false);
        }
        assert!(tracker.should_nag());
        assert_eq!(
            tracker.nag_message(),
            "You have pending TodoList items. Pick up the next one now."
        );
    }

    #[test]
    fn test_nag_does_not_fire_below_threshold() {
        let mut tracker = NagTracker::new();
        tracker.record_turn(false);
        tracker.record_turn(false);
        assert!(!tracker.should_nag());
    }
}
