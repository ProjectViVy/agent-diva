pub mod builtin;
pub mod mentle;
pub mod network;

use agent_diva_core::planning::store::SqlitePlanningStore;
use agent_diva_core::planning::EphemeralPlanRegistry;
use anyhow::Context;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
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
    /// Canonical persisted plan and execution-context state.
    pub store: Arc<SqlitePlanningStore>,
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
                std::fs::remove_file(&path).with_context(|| {
                    format!("failed to remove obsolete planning data {}", path.display())
                })?;
            }
        }
        let data_dir = db_dir.join("data");
        std::fs::create_dir_all(&data_dir)
            .with_context(|| format!("failed to create {}", data_dir.display()))?;
        let options = SqliteConnectOptions::new()
            .filename(data_dir.join("planning.sqlite3"))
            .create_if_missing(true);
        let pool = SqlitePoolOptions::new().connect_with(options).await?;
        Ok(Self {
            registry: Arc::new(EphemeralPlanRegistry::new()),
            store: Arc::new(SqlitePlanningStore::new(pool).await?),
        })
    }
}
