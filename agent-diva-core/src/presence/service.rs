//! Presence service — lifecycle management for the presence detector.
//!
//! Owns the `PresenceDetector`, subscribes to the `AgentEventBus`, and manages
//! the background idle-check task. Exposes a watch channel for the
//! `HeartbeatService` to consume presence state changes.

use std::sync::Arc;

use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{debug, info, warn};

use crate::bus::{AgentEventBus, PokeEvent};
use crate::presence::detector::PresenceDetector;
use crate::presence::types::{PresenceConfig, PresenceState, IDLE_CHECK_INTERVAL_MS};

/// Events emitted by the presence service.
#[derive(Debug, Clone)]
pub enum PresenceEvent {
    /// Presence state has changed from `from` to `to`.
    StateChanged {
        from: PresenceState,
        to: PresenceState,
    },
}

/// Manages the presence detection lifecycle.
///
/// Spawns a background tokio task that:
/// 1. Receives identity-only activity events from the bus and calls
///    `detector.on_activity()`.
/// 2. Runs a periodic idle-check tick via `detector.check_idle()`.
pub struct PresenceService {
    detector: Arc<PresenceDetector>,
    config: PresenceConfig,
    bus: AgentEventBus,
    task: tokio::sync::Mutex<Option<JoinHandle<()>>>,
}

impl PresenceService {
    /// Create a new presence service with the given config and event bus.
    pub fn new(config: PresenceConfig, bus: AgentEventBus) -> Self {
        let detector = Arc::new(PresenceDetector::new(config.clone()));
        Self {
            detector,
            config,
            bus,
            task: tokio::sync::Mutex::new(None),
        }
    }

    /// Get a reference to the underlying detector.
    pub fn detector(&self) -> Arc<PresenceDetector> {
        Arc::clone(&self.detector)
    }

    /// Subscribe to presence state change events.
    pub fn subscribe(&self) -> watch::Receiver<PresenceEvent> {
        self.detector.subscribe()
    }

    /// Start the presence service.
    ///
    /// Spawns a background task that processes identity-only activity and runs
    /// periodic idle checks. This is safe to call multiple times — subsequent
    /// calls are no-ops.
    pub async fn start(&self) {
        if !self.config.enabled {
            info!("Presence service disabled");
            return;
        }

        // Prevent double-start
        {
            let guard = self.task.lock().await;
            if guard.is_some() {
                debug!("Presence service already started");
                return;
            }
        }

        let detector = Arc::clone(&self.detector);
        let bus = self.bus.clone();

        let task = tokio::spawn(async move {
            let mut activity_rx = bus.subscribe_poke_events();

            // Periodic idle-check interval
            let mut idle_interval =
                tokio::time::interval(tokio::time::Duration::from_millis(IDLE_CHECK_INTERVAL_MS));

            loop {
                tokio::select! {
                    // Process identity-only activity.
                    activity = activity_rx.recv() => {
                        match activity {
                            Ok(PokeEvent::UserActivity { session_key, sender_id }) => {
                                detector.on_activity(&session_key, sender_id.as_deref()).await;
                            }
                            Ok(_) => {}
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped)) => {
                                warn!(skipped, "Presence: activity notifications lagged");
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                debug!("Presence: activity channel closed");
                                break;
                            }
                        }
                    }
                    // Periodic idle check
                    _ = idle_interval.tick() => {
                        detector.check_idle().await;
                    }
                }
            }
        });

        *self.task.lock().await = Some(task);
        info!("Presence service started");
    }

    /// Stop the presence service.
    ///
    /// Aborts the background task if running.
    pub async fn stop(&self) {
        let mut task_guard = self.task.lock().await;
        if let Some(task) = task_guard.take() {
            task.abort();
            info!("Presence service stopped");
        } else {
            debug!("Presence service not running");
        }
    }
}
