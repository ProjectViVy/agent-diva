//! Presence detector — subscribes to MessageBus, tracks user activity,
//! detects state transitions, and emits audit events.
//!
//! Design:
//! 1. Listens for `InboundMessage` traffic via a bus receiver.
//! 2. On each message, records `sender_id` + timestamp (monotonic clock).
//! 3. A periodic tick checks idle time and computes current `PresenceState`.
//! 4. On state change, emits both an `AuditEvent::PresenceChanged` and a
//!    `PresenceEvent` on a watch channel that the heartbeat service consumes.

use std::sync::Arc;
use std::time::Instant;

use tokio::sync::watch;
use tracing::{debug, info};

use crate::audit::{self, AuditEvent};
use crate::bus::InboundMessage;
use crate::presence::service::PresenceEvent;
use crate::presence::types::{PresenceConfig, PresenceState, PresenceStatus};

/// Detects user presence transitions by monitoring `InboundMessage` traffic.
pub struct PresenceDetector {
    config: PresenceConfig,
    /// Current presence state.
    state: Arc<tokio::sync::RwLock<PresenceState>>,
    /// Monotonic timestamp of last user activity.
    last_activity: Arc<tokio::sync::RwLock<Instant>>,
    /// Last sender ID who sent a message.
    last_sender: Arc<tokio::sync::RwLock<Option<String>>>,
    /// Notifier for presence state changes (consumed by `HeartbeatService`).
    event_tx: watch::Sender<PresenceEvent>,
}

impl PresenceDetector {
    /// Create a new presence detector with the given config.
    ///
    /// Initial state is `Active` and the activity clock starts at `Instant::now()`.
    pub fn new(config: PresenceConfig) -> Self {
        let (event_tx, _) = watch::channel(PresenceEvent::StateChanged {
            from: PresenceState::Active,
            to: PresenceState::Active,
        });

        Self {
            config,
            state: Arc::new(tokio::sync::RwLock::new(PresenceState::Active)),
            last_activity: Arc::new(tokio::sync::RwLock::new(Instant::now())),
            last_sender: Arc::new(tokio::sync::RwLock::new(None)),
            event_tx,
        }
    }

    /// Subscribe to presence state change events.
    pub fn subscribe(&self) -> watch::Receiver<PresenceEvent> {
        self.event_tx.subscribe()
    }

    /// Handle an inbound message — bumps the activity timer and transitions state.
    ///
    /// - If the user was `Gone`, they return to `Active` immediately.
    /// - If the user was `Distracted`, they return to `Active` immediately.
    /// - If already `Active`, just updates the timestamp.
    pub async fn on_message(&self, msg: &InboundMessage) {
        let now = Instant::now();
        let old_state = *self.state.read().await;

        // Update activity timestamp and sender
        *self.last_activity.write().await = now;
        *self.last_sender.write().await = Some(msg.sender_id.clone());

        // Transition back to Active if currently Distracted or Gone
        match old_state {
            PresenceState::Gone | PresenceState::Distracted => {
                let to = PresenceState::Active;
                *self.state.write().await = to;
                self.emit_transition(old_state, to).await;
            }
            PresenceState::Active => {
                // Already active; timer is bumped above
            }
        }

        debug!(
            "Presence: message from {}, state={:?}",
            msg.sender_id,
            self.state.read().await
        );
    }

    /// Periodic idle check — called by the service run loop.
    ///
    /// Compares elapsed time since last activity against thresholds and
    /// transitions state if needed:
    /// - `Active` → `Distracted` when `idle_secs >= active_timeout_s`
    /// - `Distracted` → `Gone` when `idle_secs >= distracted_timeout_s`
    pub async fn check_idle(&self) {
        let now = Instant::now();
        let last = *self.last_activity.read().await;
        let idle_secs = now.duration_since(last).as_secs() as i64;
        let current_state = *self.state.read().await;

        let new_state = match current_state {
            PresenceState::Active if idle_secs >= self.config.active_timeout_s => {
                Some(PresenceState::Distracted)
            }
            PresenceState::Distracted if idle_secs >= self.config.distracted_timeout_s => {
                Some(PresenceState::Gone)
            }
            _ => None,
        };

        if let Some(new_state) = new_state {
            self.emit_transition(current_state, new_state).await;
        }
    }

    /// Emit a state transition: audit event + watch channel notification.
    async fn emit_transition(&self, from: PresenceState, to: PresenceState) {
        // Update internal state (done by caller, but also here for safety)
        *self.state.write().await = to;

        info!("Presence transition: {:?} → {:?}", from, to);

        // Emit audit event
        audit::emit(AuditEvent::PresenceChanged { from, to });

        // Notify subscribers (heartbeat service)
        let _ = self.event_tx.send(PresenceEvent::StateChanged { from, to });
    }

    /// Get current presence status snapshot.
    pub async fn status(&self) -> PresenceStatus {
        let state = *self.state.read().await;
        let last = *self.last_activity.read().await;
        let idle_secs = Instant::now().duration_since(last).as_secs() as i64;
        let last_sender = self.last_sender.read().await.clone();

        PresenceStatus {
            state,
            idle_secs,
            last_activity: None,
            last_sender_id: last_sender,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presence::types::PresenceConfig;

    fn make_msg(sender: &str) -> InboundMessage {
        InboundMessage::new("test", sender, "chat_1", "hello")
    }

    #[tokio::test]
    async fn test_presence_default_state_active() {
        let config = PresenceConfig::default();
        let detector = PresenceDetector::new(config);
        let status = detector.status().await;
        assert_eq!(status.state, PresenceState::Active);
    }

    #[tokio::test]
    async fn test_presence_message_bumps_activity() {
        let config = PresenceConfig::default();
        let detector = PresenceDetector::new(config);
        let msg = make_msg("user_1");
        detector.on_message(&msg).await;
        let status = detector.status().await;
        assert_eq!(status.state, PresenceState::Active);
        assert_eq!(status.last_sender_id.as_deref(), Some("user_1"));
    }

    #[tokio::test]
    async fn test_presence_message_wakes_from_gone() {
        let config = PresenceConfig::default();
        let detector = PresenceDetector::new(config);
        // Simulate Gone state by forcing the state
        *detector.state.write().await = PresenceState::Gone;
        // Send a message — should wake to Active
        let msg = make_msg("user_2");
        detector.on_message(&msg).await;
        let status = detector.status().await;
        assert_eq!(status.state, PresenceState::Active);
    }

    #[tokio::test]
    async fn test_presence_transition_active_to_distracted() {
        let config = PresenceConfig {
            active_timeout_s: 0, // immediate
            distracted_timeout_s: 9999,
            enabled: true,
        };
        let detector = PresenceDetector::new(config);
        // The detector starts with last_activity=Instant::now().
        // With active_timeout_s=0, the first check_idle should transition immediately.
        detector.check_idle().await;
        let status = detector.status().await;
        assert_eq!(status.state, PresenceState::Distracted);
    }

    #[tokio::test]
    async fn test_presence_transition_distracted_to_gone() {
        let config = PresenceConfig {
            active_timeout_s: 9999,
            distracted_timeout_s: 0, // immediate
            enabled: true,
        };
        let detector = PresenceDetector::new(config);
        // Set state to Distracted
        *detector.state.write().await = PresenceState::Distracted;
        // Force last_activity to be sufficiently in the past
        *detector.last_activity.write().await = Instant::now() - std::time::Duration::from_secs(1);
        detector.check_idle().await;
        let status = detector.status().await;
        assert_eq!(status.state, PresenceState::Gone);
    }

    #[tokio::test]
    async fn test_presence_idle_returns_to_active_on_message() {
        let config = PresenceConfig {
            active_timeout_s: 1,
            distracted_timeout_s: 2,
            enabled: true,
        };
        let detector = PresenceDetector::new(config);

        // Trigger Distracted via idle
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
        detector.check_idle().await;
        assert_eq!(detector.status().await.state, PresenceState::Distracted);

        // Message comes in — back to Active
        let msg = make_msg("user_1");
        detector.on_message(&msg).await;
        assert_eq!(detector.status().await.state, PresenceState::Active);
    }

    #[tokio::test]
    async fn test_presence_heartbeat_multiplier() {
        assert_eq!(PresenceState::Active.heartbeat_multiplier(), 1);
        assert_eq!(PresenceState::Distracted.heartbeat_multiplier(), 2);
        assert_eq!(PresenceState::Gone.heartbeat_multiplier(), 0);
    }

    #[test]
    fn test_presence_allows_consolidation() {
        assert!(!PresenceState::Active.allows_consolidation());
        assert!(!PresenceState::Distracted.allows_consolidation());
        assert!(PresenceState::Gone.allows_consolidation());
    }

    #[test]
    fn test_presence_state_default() {
        let state = PresenceState::default();
        assert_eq!(state, PresenceState::Active);
    }
}
