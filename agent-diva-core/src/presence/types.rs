//! Presence state types shared across runtime modules.

use serde::{Deserialize, Serialize};

/// Four-state presence model used by runtime modules.
///
/// State transitions:
/// - `Active` → `Distracted` after `active_timeout_s` (default 5 min)
/// - `Distracted` → `Gone` after `distracted_timeout_s` (default 30 min)
/// - `Gone` → `Away` after `gone_timeout_s` (default 2 h)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PresenceState {
    /// User is actively interacting with the system.
    #[default]
    Active,
    /// User has been idle for a short period.
    Distracted,
    /// User has been idle long enough for background-only behavior.
    Gone,
    /// User has been absent for an extended period; system enters deep-idle.
    Away,
}

/// Presence transition thresholds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresenceConfig {
    /// Seconds before `Active` transitions to `Distracted`.
    pub active_timeout_s: u64,
    /// Seconds before `Distracted` transitions to `Gone`.
    pub distracted_timeout_s: u64,
    /// Seconds before `Gone` transitions to `Away`.
    pub gone_timeout_s: u64,
    /// Multiplier applied to heartbeat cadence while distracted.
    pub distracted_heartbeat_multiplier: f64,
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            active_timeout_s: 300,      // 5 min
            distracted_timeout_s: 1800, // 30 min
            gone_timeout_s: 7200,       // 2 h
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
        assert_eq!(config.gone_timeout_s, 7200);
        assert_eq!(config.distracted_heartbeat_multiplier, 2.0);
    }
}
