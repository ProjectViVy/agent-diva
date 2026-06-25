//! Presence state primitives.

mod rhythm;
mod state_machine;
mod types;

pub use rhythm::HeartbeatRhythm;
pub use state_machine::{PresenceManager, PresenceTransition};
pub use types::{PresenceConfig, PresenceState};
