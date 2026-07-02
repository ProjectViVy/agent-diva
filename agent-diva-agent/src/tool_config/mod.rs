pub mod builtin;
pub mod mentle;
pub mod network;

use crate::planning::PlanOrchestrator;
use agent_diva_core::planning::store::PlanningStore;
use agent_diva_core::planning::store::SqlitePlanningStore;
use anyhow::Context;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Optional planning subsystem configuration.
///
/// When present, planning tools are registered and planning hooks are active.
#[derive(Clone)]
pub struct PlanningConfig {
    /// The planning store (SQLite-backed).
    pub store: Arc<dyn PlanningStore>,
    /// Shared plan lifecycle orchestrator.
    pub orchestrator: Arc<Mutex<PlanOrchestrator>>,
}

impl PlanningConfig {
    /// Open the workspace-local planning database.
    pub async fn open_workspace(workspace: &Path) -> anyhow::Result<Self> {
        let db_dir = workspace.join(".agent-diva");
        std::fs::create_dir_all(&db_dir)
            .with_context(|| format!("failed to create {}", db_dir.display()))?;
        let db_url = format!("sqlite:{}?mode=rwc", db_dir.join("planning.db").display());
        let pool = sqlx::SqlitePool::connect(&db_url)
            .await
            .with_context(|| format!("failed to connect planning database at {db_url}"))?;
        let store = SqlitePlanningStore::new(pool)
            .await
            .context("failed to initialize planning store")?;
        Ok(Self {
            store: Arc::new(store),
            orchestrator: Arc::new(Mutex::new(PlanOrchestrator::new())),
        })
    }
}
