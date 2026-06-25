//! Heartbeat rhythm derived from user presence state.
//!
//! The heartbeat cadence adapts to user presence:
//! - **Normal**: user is active, heartbeat at base interval.
//! - **Slow**: user is distracted/gone, heartbeat at reduced frequency.
//! - **Suspended**: user is away, heartbeat paused.

use serde::{Deserialize, Serialize};

use super::types::PresenceState;

/// Heartbeat rhythm modes derived from presence state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum HeartbeatRhythm {
    /// Normal cadence — user is actively present.
    #[default]
    Normal,
    /// Reduced cadence — user is distracted or gone.
    Slow,
    /// Suspended — user is away; no heartbeat ticks.
    Suspended,
}

impl HeartbeatRhythm {
    /// Compute the rhythm for a given presence state.
    pub fn for_presence(state: PresenceState) -> Self {
        match state {
            PresenceState::Active => HeartbeatRhythm::Normal,
            PresenceState::Distracted | PresenceState::Gone => HeartbeatRhythm::Slow,
            PresenceState::Away => HeartbeatRhythm::Suspended,
        }
    }

    /// Whether the heartbeat should tick in this rhythm.
    pub fn is_active(&self) -> bool {
        matches!(self, HeartbeatRhythm::Normal | HeartbeatRhythm::Slow)
    }

    /// Interval multiplier relative to the base heartbeat interval.
    ///
    /// - `Normal` → 1.0 (base interval)
    /// - `Slow` → 2.0 (double the interval)
    /// - `Suspended` → ∞ (effectively disabled)
    pub fn interval_multiplier(&self) -> f64 {
        match self {
            HeartbeatRhythm::Normal => 1.0,
            HeartbeatRhythm::Slow => 2.0,
            HeartbeatRhythm::Suspended => f64::INFINITY,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_maps_to_normal() {
        assert_eq!(
            HeartbeatRhythm::for_presence(PresenceState::Active),
            HeartbeatRhythm::Normal
        );
    }

    #[test]
    fn distracted_maps_to_slow() {
        assert_eq!(
            HeartbeatRhythm::for_presence(PresenceState::Distracted),
            HeartbeatRhythm::Slow
        );
    }

    #[test]
    fn gone_maps_to_slow() {
        assert_eq!(
            HeartbeatRhythm::for_presence(PresenceState::Gone),
            HeartbeatRhythm::Slow
        );
    }

    #[test]
    fn away_maps_to_suspended() {
        assert_eq!(
            HeartbeatRhythm::for_presence(PresenceState::Away),
            HeartbeatRhythm::Suspended
        );
    }

    #[test]
    fn normal_and_slow_are_active() {
        assert!(HeartbeatRhythm::Normal.is_active());
        assert!(HeartbeatRhythm::Slow.is_active());
        assert!(!HeartbeatRhythm::Suspended.is_active());
    }

    #[test]
    fn interval_multipliers() {
        assert_eq!(HeartbeatRhythm::Normal.interval_multiplier(), 1.0);
        assert_eq!(HeartbeatRhythm::Slow.interval_multiplier(), 2.0);
        assert!(HeartbeatRhythm::Suspended.interval_multiplier().is_infinite());
    }

    #[test]
    fn default_is_normal() {
        assert_eq!(HeartbeatRhythm::default(), HeartbeatRhythm::Normal);
    }
}
