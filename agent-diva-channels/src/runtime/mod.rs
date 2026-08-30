//! Shared Registry, pacing and listener supervision for native channel adapters.

mod pacing;
mod registry;
mod supervisor;

pub use pacing::{AdapterPacingHandle, AdapterPacingLane, PacingError};
pub use registry::{AdapterRegistry, AdapterRegistryError};
pub use supervisor::{AdapterSupervisor, SupervisorPolicy};
