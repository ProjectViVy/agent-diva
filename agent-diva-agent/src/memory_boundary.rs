//! Production Memory boundary for the agent loop.
//!
//! The cognitive clean break (S3/S5) makes the machine-wide BML MemoryHome
//! the only long-term memory authority. The retired Legacy/Shadow/Cutover
//! providers, the workspace-rooted typed store, and the proposal-governed
//! CRUD stack are deleted; Memory CRUD is direct and unapproved.

use std::path::Path;
use std::sync::Arc;

use agent_diva_core::memory::MemoryProvider;

/// Select the default memory provider bound to the machine config root.
pub(crate) fn default_memory_provider(config_dir: &Path) -> Arc<dyn MemoryProvider> {
    Arc::new(agent_diva_laputa::MemoryHome::new(config_dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn default_provider_is_the_machine_memory_home() {
        let temp = tempfile::tempdir().unwrap();
        let provider = default_memory_provider(temp.path());
        let outcome = provider
            .memory_list(
                &agent_diva_core::memory::MemoryCrudContext {
                    workspace_root: temp.path().to_path_buf(),
                },
                agent_diva_core::memory::MemoryListRequest { limit: Some(10) },
            )
            .await
            .unwrap();
        assert!(matches!(
            outcome,
            agent_diva_core::memory::MemoryCrudOutcome::Listed { ref entries }
                if entries.is_empty()
        ));
        assert!(!temp.path().join("MEMORY.md").exists());
        assert!(!temp.path().join(".laputa/memory.sqlite3").exists());
        assert!(!temp.path().join("memory").exists());
    }
}
