pub mod builtin;
pub mod mentle;
pub mod network;

use agent_diva_core::planning::EphemeralPlanRegistry;
use anyhow::Context;
use std::path::Path;
use std::sync::Arc;

/// Optional planning subsystem configuration.
///
/// When present, planning tools are registered and planning hooks are active.
#[derive(Clone)]
pub struct PlanningConfig {
    /// Process-local state for drafts and approved executions. It is scoped by
    /// session key and intentionally disappears on restart.
    pub registry: Arc<EphemeralPlanRegistry>,
}

impl PlanningConfig {
    /// Start a fresh, in-memory planning runtime and erase obsolete durable
    /// planning data so it cannot be restored accidentally.
    pub async fn open_workspace(workspace: &Path) -> anyhow::Result<Self> {
        let db_dir = workspace.join(".agent-diva");
        std::fs::create_dir_all(&db_dir)
            .with_context(|| format!("failed to create {}", db_dir.display()))?;
        for suffix in ["planning.db", "planning.db-wal", "planning.db-shm"] {
            let path = db_dir.join(suffix);
            if path.exists() {
                std::fs::remove_file(&path)
                    .with_context(|| format!("failed to remove obsolete planning data {}", path.display()))?;
            }
        }
        Ok(Self {
            registry: Arc::new(EphemeralPlanRegistry::new()),
        })
    }
}
