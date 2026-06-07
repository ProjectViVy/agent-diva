pub mod network;

use agent_diva_core::planning::store::PlanningStore;
use std::sync::Arc;

/// Optional planning subsystem configuration.
///
/// When present, planning tools are registered and planning hooks are active.
pub struct PlanningConfig {
    /// The planning store (SQLite-backed).
    pub store: Arc<dyn PlanningStore>,
}
