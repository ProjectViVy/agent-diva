//! Presence state types for user activity tracking.
//!
//! Defines the three-state user presence model (Active / Distracted / Gone),
//! configuration thresholds, and status snapshots.

use serde::{Deserialize, Serialize};

/// Default idle time (seconds) before Active → Distracted transition.
pub const DEFAULT_ACTIVE_TIMEOUT_S: i64 = 300; // 5 minutes

/// Default idle time (seconds) before Distracted → Gone transition.
pub const DEFAULT_DISTRACTED_TIMEOUT_S: i64 = 1800; // 30 minutes

/// Interval (milliseconds) for the periodic idle-check loop.
pub const IDLE_CHECK_INTERVAL_MS: u64 = 10_000; // 10 seconds

/// Three-state user presence model.
///
/// Mirrors the state machine used across the system:
/// - `Active`: user is actively interacting (< `active_timeout_s` since last message)
/// - `Distracted`: user is present but not focused (between `active_timeout_s` and `distracted_timeout_s`)
/// - `Gone`: user has been idle long enough to be considered away (> `distracted_timeout_s`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PresenceState {
    /// User is actively interacting.
    #[default]
    Active,
    /// User is present but not focused on the agent.
    Distracted,
    /// User has been idle long enough to be considered away.
    Gone,
}

impl PresenceState {
    /// Returns the heartbeat interval multiplier for this state.
    ///
    /// - Active: 1x (normal interval)
    /// - Distracted: 2x (slower heartbeat)
    /// - Gone: 0x (SOUL consolidation — heartbeat is paused or minimal)
    pub fn heartbeat_multiplier(&self) -> i64 {
        match self {
            PresenceState::Active => 1,
            PresenceState::Distracted => 2,
            PresenceState::Gone => 0,
        }
    }

    /// Returns `true` if this state allows SOUL consolidation.
    pub fn allows_consolidation(&self) -> bool {
        matches!(self, PresenceState::Gone)
    }
}

/// Presence detector configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceConfig {
    /// Seconds of inactivity before Active → Distracted transition.
    #[serde(default = "default_active_timeout")]
    pub active_timeout_s: i64,

    /// Seconds of inactivity before Distracted → Gone transition.
    #[serde(default = "default_distracted_timeout")]
    pub distracted_timeout_s: i64,

    /// Whether presence detection is enabled.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl Default for PresenceConfig {
    fn default() -> Self {
        Self {
            active_timeout_s: DEFAULT_ACTIVE_TIMEOUT_S,
            distracted_timeout_s: DEFAULT_DISTRACTED_TIMEOUT_S,
            enabled: true,
        }
    }
}

fn default_active_timeout() -> i64 {
    DEFAULT_ACTIVE_TIMEOUT_S
}

fn default_distracted_timeout() -> i64 {
    DEFAULT_DISTRACTED_TIMEOUT_S
}

fn default_enabled() -> bool {
    true
}

/// Current presence status snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresenceStatus {
    /// Current state.
    pub state: PresenceState,
    /// Seconds since last user activity.
    pub idle_secs: i64,
    /// Last activity timestamp (ISO 8601), if available.
    pub last_activity: Option<String>,
    /// Sender ID of the last active user, if any.
    pub last_sender_id: Option<String>,
}
