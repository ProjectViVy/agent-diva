use std::path::Path;
use std::sync::Arc;

use agent_diva_core::memory::{
    MemoryManager, MemoryProvider, PrefetchRequest, PrefetchResponse, PrefetchStatus,
    SessionEndRequest, SessionEndResponse, SessionEndStatus, SyncTurnRequest, SyncTurnResponse,
    SyncTurnStatus, SystemPromptRequest, SystemPromptResponse,
};
use tracing::warn;

/// Select the default memory provider for the current workspace.
///
/// When `.laputa/` is present and opens successfully, the Laputa adapter owns
/// the authority boundary. If Laputa initialization fails, return a degraded
/// provider instead of silently falling back to the legacy MemoryManager.
pub(crate) fn default_memory_provider(workspace: &Path) -> Arc<dyn MemoryProvider> {
    if workspace.join(".laputa").is_dir() {
        match agent_diva_laputa::LaputaMemoryProvider::open(workspace) {
            Ok(provider) => return Arc::new(provider),
            Err(error) => {
                warn!(
                    "Laputa memory provider unavailable for {}: {}; using degraded authority boundary",
                    workspace.display(),
                    error
                );
                return Arc::new(DegradedMemoryProvider::new(
                    workspace.to_path_buf(),
                    error.to_string(),
                ));
            }
        }
    }

    Arc::new(MemoryManager::new(workspace))
}

#[derive(Debug)]
struct DegradedMemoryProvider {
    workspace: std::path::PathBuf,
    reason: String,
}

impl DegradedMemoryProvider {
    fn new(workspace: std::path::PathBuf, reason: String) -> Self {
        Self { workspace, reason }
    }

    fn degraded_reason(&self) -> String {
        format!(
            "Laputa memory provider unavailable for {}: {}",
            self.workspace.display(),
            self.reason
        )
    }
}

#[async_trait::async_trait]
impl MemoryProvider for DegradedMemoryProvider {
    fn system_prompt_block(
        &self,
        _request: &SystemPromptRequest,
    ) -> agent_diva_core::Result<SystemPromptResponse> {
        Ok(SystemPromptResponse::degraded(self.degraded_reason()))
    }

    async fn prefetch(
        &self,
        _request: PrefetchRequest,
    ) -> agent_diva_core::Result<PrefetchResponse> {
        Ok(PrefetchResponse {
            status: PrefetchStatus::Failed {
                reason: self.degraded_reason(),
            },
            prompt_block: None,
        })
    }

    async fn sync_turn(&self, _request: SyncTurnRequest) -> agent_diva_core::Result<SyncTurnResponse> {
        Ok(SyncTurnResponse {
            status: SyncTurnStatus::Failed {
                reason: self.degraded_reason(),
            },
        })
    }

    async fn on_session_end(
        &self,
        _request: SessionEndRequest,
    ) -> agent_diva_core::Result<SessionEndResponse> {
        Ok(SessionEndResponse {
            status: SessionEndStatus::Failed {
                reason: self.degraded_reason(),
            },
        })
    }
}
