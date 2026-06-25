//! Presence state types shared across runtime modules.

use serde::{Deserialize, Serialize};

/// Three-state presence model used by runtime modules.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PresenceState {
    /// User is actively interacting with the system.
    #[default]
    Active,
    /// User has been idle for a short period.
    Distracted,
    /// User has been idle long enough for background-only behavior.
    Gone,
}

/// Presence transition thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresenceConfig {
    /// Seconds before `Active` transitions to `Distracted`.
    pub active_timeout_s: u64,
    /// Seconds before `Distracted` transitions to `Gone`.
    pub distracted_timeout_s: u64,
    /// Multiplier applied to heartbeat cadence while distracted.
    pub distracted_heartbeat_multiplier: f64,
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            active_timeout_s: 300,
            distracted_timeout_s: 1800,
            distracted_heartbeat_multiplier: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_presence_state_is_active() {
        assert_eq!(PresenceState::default(), PresenceState::Active);
    }

    #[test]
    fn default_presence_thresholds_match_architecture_doc() {
        let config = PresenceConfig::default();
        assert_eq!(config.active_timeout_s, 300);
        assert_eq!(config.distracted_timeout_s, 1800);
        assert_eq!(config.distracted_heartbeat_multiplier, 2.0);
    }
}
