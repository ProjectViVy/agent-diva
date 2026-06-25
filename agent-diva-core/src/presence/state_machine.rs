//! Presence state machine for tracking user activity.
//!
//! The [`PresenceManager`] tracks the last user activity timestamp and
//! automatically transitions through the presence states:
//!
//! ```text
//! Active ──(5 min)──> Distracted ──(30 min)──> Gone ──(2 h)──> Away
//! ```
//!
//! Any user activity resets the state back to `Active`.

use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use super::types::{PresenceConfig, PresenceState};

/// Result of a presence refresh operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresenceTransition {
    /// No state change occurred.
    None,
    /// State changed from one variant to another.
    Changed {
        from: PresenceState,
        to: PresenceState,
    },
}

/// Thread-safe presence state machine.
///
/// Tracks user activity and automatically transitions between presence states
/// based on configurable timeouts. Designed to be shared across threads via
/// `Arc<PresenceManager>`.
#[derive(Debug)]
pub struct PresenceManager {
    state: Arc<RwLock<PresenceState>>,
    last_activity: Arc<RwLock<Instant>>,
    config: PresenceConfig,
}

impl PresenceManager {
    /// Create a new `PresenceManager` with the given config.
    pub fn new(config: PresenceConfig) -> Self {
        Self {
            state: Arc::new(RwLock::new(PresenceState::Active)),
            last_activity: Arc::new(RwLock::new(Instant::now())),
            config,
        }
    }

    /// Create a new `PresenceManager` with default config.
    pub fn with_defaults() -> Self {
        Self::new(PresenceConfig::default())
    }

    /// Simulate elapsed time since last activity (for testing).
    ///
    /// This sets the internal `last_activity` timestamp to `Instant::now() - elapsed`.
    pub fn simulate_elapsed(&self, elapsed: Duration) {
        let mut last = self
            .last_activity
            .write()
            .expect("presence last_activity lock poisoned");
        *last = Instant::now() - elapsed;
    }

    /// Get the current presence state.
    pub fn state(&self) -> PresenceState {
        *self
            .state
            .read()
            .expect("presence state lock poisoned")
    }

    /// Record user activity and reset state to `Active`.
    ///
    /// Returns a `PresenceTransition` indicating whether the state changed.
    pub fn record_activity(&self) -> PresenceTransition {
        {
            let mut last = self
                .last_activity
                .write()
                .expect("presence last_activity lock poisoned");
            *last = Instant::now();
        }

        let mut state = self
            .state
            .write()
            .expect("presence state lock poisoned");
        if *state != PresenceState::Active {
            let from = *state;
            *state = PresenceState::Active;
            PresenceTransition::Changed {
                from,
                to: PresenceState::Active,
            }
        } else {
            PresenceTransition::None
        }
    }

    /// Re-evaluate the current state based on elapsed time since last activity.
    ///
    /// Returns a `PresenceTransition` indicating whether the state changed.
    pub fn refresh(&self) -> PresenceTransition {
        let elapsed = {
            let last = self
                .last_activity
                .read()
                .expect("presence last_activity lock poisoned");
            last.elapsed()
        };

        let next = self.compute_state(elapsed);

        let mut state = self
            .state
            .write()
            .expect("presence state lock poisoned");
        if *state != next {
            let from = *state;
            *state = next;
            PresenceTransition::Changed { from, to: next }
        } else {
            PresenceTransition::None
        }
    }

    /// Compute the expected state for a given elapsed duration.
    fn compute_state(&self, elapsed: Duration) -> PresenceState {
        let secs = elapsed.as_secs();
        if secs >= self.config.gone_timeout_s {
            PresenceState::Away
        } else if secs >= self.config.distracted_timeout_s {
            PresenceState::Gone
        } else if secs >= self.config.active_timeout_s {
            PresenceState::Distracted
        } else {
            PresenceState::Active
        }
    }
}

impl Default for PresenceManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

impl Clone for PresenceManager {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            last_activity: Arc::clone(&self.last_activity),
            config: self.config.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_is_active() {
        let pm = PresenceManager::with_defaults();
        assert_eq!(pm.state(), PresenceState::Active);
    }

    #[test]
    fn record_activity_resets_to_active() {
        let pm = PresenceManager::with_defaults();
        // Simulate idle by faking last_activity
        {
            let mut last = pm.last_activity.write().unwrap();
            *last = Instant::now() - Duration::from_secs(600);
        }
        // Refresh to move to Distracted
        pm.refresh();
        assert_eq!(pm.state(), PresenceState::Distracted);

        // Record activity should reset
        let transition = pm.record_activity();
        assert_eq!(
            transition,
            PresenceTransition::Changed {
                from: PresenceState::Distracted,
                to: PresenceState::Active,
            }
        );
        assert_eq!(pm.state(), PresenceState::Active);
    }

    #[test]
    fn record_activity_when_already_active_is_none() {
        let pm = PresenceManager::with_defaults();
        let transition = pm.record_activity();
        assert_eq!(transition, PresenceTransition::None);
    }

    #[test]
    fn transition_active_to_distracted() {
        let pm = PresenceManager::new(PresenceConfig {
            active_timeout_s: 5,
            distracted_timeout_s: 30,
            gone_timeout_s: 120,
            distracted_heartbeat_multiplier: 2.0,
        });
        {
            let mut last = pm.last_activity.write().unwrap();
            *last = Instant::now() - Duration::from_secs(6);
        }
        let transition = pm.refresh();
        assert_eq!(
            transition,
            PresenceTransition::Changed {
                from: PresenceState::Active,
                to: PresenceState::Distracted,
            }
        );
        assert_eq!(pm.state(), PresenceState::Distracted);
    }

    #[test]
    fn transition_distracted_to_gone() {
        let pm = PresenceManager::new(PresenceConfig {
            active_timeout_s: 5,
            distracted_timeout_s: 30,
            gone_timeout_s: 120,
            distracted_heartbeat_multiplier: 2.0,
        });
        {
            let mut last = pm.last_activity.write().unwrap();
            *last = Instant::now() - Duration::from_secs(31);
        }
        let transition = pm.refresh();
        assert_eq!(
            transition,
            PresenceTransition::Changed {
                from: PresenceState::Active,
                to: PresenceState::Gone,
            }
        );
        assert_eq!(pm.state(), PresenceState::Gone);
    }

    #[test]
    fn transition_gone_to_away() {
        let pm = PresenceManager::new(PresenceConfig {
            active_timeout_s: 5,
            distracted_timeout_s: 30,
            gone_timeout_s: 120,
            distracted_heartbeat_multiplier: 2.0,
        });
        {
            let mut last = pm.last_activity.write().unwrap();
            *last = Instant::now() - Duration::from_secs(121);
        }
        let transition = pm.refresh();
        assert_eq!(
            transition,
            PresenceTransition::Changed {
                from: PresenceState::Active,
                to: PresenceState::Away,
            }
        );
        assert_eq!(pm.state(), PresenceState::Away);
    }

    #[test]
    fn no_transition_within_active_threshold() {
        let pm = PresenceManager::with_defaults();
        let transition = pm.refresh();
        assert_eq!(transition, PresenceTransition::None);
        assert_eq!(pm.state(), PresenceState::Active);
    }

    #[test]
    fn clone_shares_state() {
        let pm1 = PresenceManager::with_defaults();
        let pm2 = pm1.clone();

        {
            let mut last = pm1.last_activity.write().unwrap();
            *last = Instant::now() - Duration::from_secs(600);
        }
        pm1.refresh();

        // pm2 should see the same state
        assert_eq!(pm2.state(), PresenceState::Distracted);
    }
}
