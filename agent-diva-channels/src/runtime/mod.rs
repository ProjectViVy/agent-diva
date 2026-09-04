//! Shared Registry, pacing and listener supervision for native channel adapters.

mod pacing;
mod production;
mod registry;
mod supervisor;

pub use pacing::{AdapterPacingHandle, AdapterPacingLane, PacingError};
pub use production::{ChannelRuntime, ChannelRuntimeError, ChannelRuntimeStatus};
pub use registry::{AdapterRegistry, AdapterRegistryError};
pub use supervisor::{AdapterSupervisor, SupervisorError, SupervisorPolicy};
