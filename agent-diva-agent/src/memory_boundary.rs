use std::path::Path;
use std::sync::Arc;

use agent_diva_core::memory::{
    compare_recall_shadow, memory_content_digest, MemoryManager, MemoryProvider, MemoryScope,
    PrefetchRequest, PrefetchResponse, PrefetchStatus, RecallPolicy, RecallRequest,
    SessionEndRequest, SessionEndResponse, SessionEndStatus, SyncTurnRequest, SyncTurnResponse,
    SyncTurnStatus, SystemPromptRequest, SystemPromptResponse,
};
use agent_diva_core::{config::schema::MemoryAuthorityMode, governance::AuditCorrelation};

/// Select the default memory provider for the current workspace.
///
/// When `.laputa/` is present and opens successfully, the Laputa adapter owns
/// the authority boundary. If Laputa initialization fails, return a degraded
/// provider instead of silently falling back to the legacy MemoryManager.
pub(crate) fn default_memory_provider(workspace: &Path) -> Arc<dyn MemoryProvider> {
    Arc::new(MemoryManager::new(workspace))
}

/// Construct the configured production Memory boundary.
pub async fn memory_provider_for_mode(
    workspace: &Path,
    mode: MemoryAuthorityMode,
) -> Arc<dyn MemoryProvider> {
    if mode == MemoryAuthorityMode::Legacy {
        return Arc::new(MemoryManager::new(workspace));
    }
    let store_result = if mode == MemoryAuthorityMode::Shadow {
        agent_diva_laputa::TypedMemoryStore::open_existing_canonical(workspace).await
    } else {
        agent_diva_laputa::TypedMemoryStore::open_canonical(workspace).await
    };
    let existing_store = match store_result {
        Ok(store) => store,
        Err(error) => {
            return Arc::new(DegradedMemoryProvider::new(
                workspace.to_path_buf(),
                format!("typed_store_unavailable:{error}"),
            ));
        }
    };
    let integrity = existing_store.integrity().await;
    match integrity {
        Ok(integrity)
            if integrity.corrupt_record_ids.is_empty() && integrity.orphan_fts_rows == 0 => {}
        Ok(_) => {
            return Arc::new(DegradedMemoryProvider::new(
                workspace.to_path_buf(),
                "typed_store_integrity_failed".into(),
            ));
        }
        Err(error) => {
            return Arc::new(DegradedMemoryProvider::new(
                workspace.to_path_buf(),
                format!("typed_store_integrity_unavailable:{error}"),
            ));
        }
    }
    let typed = Arc::new(agent_diva_laputa::LaputaRecallService::new(existing_store));
    if mode == MemoryAuthorityMode::Typed {
        return match agent_diva_laputa::TypedLaputaMemoryProvider::open(
            workspace,
            agent_diva_core::workspace_identity::canonical_workspace_id(workspace),
        )
        .await
        {
            Ok(provider) => Arc::new(provider),
            Err(error) => Arc::new(DegradedMemoryProvider::new(
                workspace.to_path_buf(),
                format!("typed_provider_unavailable:{error}"),
            )),
        };
    }
    Arc::new(CutoverMemoryProvider {
        mode,
        workspace: workspace.to_path_buf(),
        legacy: Arc::new(MemoryManager::new(workspace)),
        recall: typed,
    })
}

struct CutoverMemoryProvider {
    mode: MemoryAuthorityMode,
    workspace: std::path::PathBuf,
    legacy: Arc<dyn MemoryProvider>,
    recall: Arc<agent_diva_laputa::LaputaRecallService>,
}

impl CutoverMemoryProvider {
    fn recall_request(&self, request: &PrefetchRequest) -> RecallRequest {
        let query = request
            .user_message
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&request.intent)
            .to_string();
        let request_digest = memory_content_digest(query.as_bytes()).value;
        RecallRequest {
            query,
            scope: MemoryScope {
                tenant_id: "local".into(),
                workspace_id: agent_diva_core::workspace_identity::canonical_workspace_id(
                    &self.workspace,
                ),
                session_id: None,
            },
            correlation: AuditCorrelation {
                request_id: format!("recall-{request_digest}"),
                turn_id: "prefetch".into(),
                session_id: "global".into(),
                trace_id: None,
            },
            now: chrono::Utc::now(),
            token_budget: 4_000,
            max_candidates: 32,
            policy: RecallPolicy::default_prompt(),
        }
    }
}

#[async_trait::async_trait]
impl MemoryProvider for CutoverMemoryProvider {
    fn system_prompt_block(
        &self,
        request: &SystemPromptRequest,
    ) -> agent_diva_core::Result<SystemPromptResponse> {
        match self.mode {
            MemoryAuthorityMode::Shadow => self.legacy.system_prompt_block(request),
            MemoryAuthorityMode::Typed => unreachable!("typed mode uses TypedLaputaMemoryProvider"),
            MemoryAuthorityMode::Legacy => self.legacy.system_prompt_block(request),
        }
    }

    async fn prefetch(
        &self,
        request: PrefetchRequest,
    ) -> agent_diva_core::Result<PrefetchResponse> {
        if request.intent.trim().is_empty() {
            return Ok(PrefetchResponse::default());
        }
        let legacy = self.legacy.prefetch(request.clone()).await?;
        let shadow = match self
            .recall
            .recall_shadow(&self.recall_request(&request))
            .await
        {
            Ok(shadow) => shadow,
            Err(error) => {
                if self.mode == MemoryAuthorityMode::Shadow {
                    tracing::warn!(
                        degraded_reason = "typed_recall_failed",
                        "Embedded Laputa shadow recall degraded"
                    );
                    return Ok(legacy);
                }
                return Ok(PrefetchResponse {
                    status: PrefetchStatus::Failed {
                        reason: format!("typed_recall_degraded:{error}"),
                    },
                    prompt_block: None,
                });
            }
        };
        let comparison = compare_recall_shadow(legacy.prompt_block.as_deref(), &shadow.outcome);
        tracing::info!(
            authority_mode = ?self.mode,
            selected_count = shadow.metrics.selected_count,
            rejected_count = shadow.metrics.rejected_count,
            duplicate_rate_bps = shadow.metrics.duplicate_rate_bps,
            retrieval_micros = shadow.metrics.retrieval_micros,
            total_micros = shadow.metrics.total_micros,
            used_tokens = shadow.metrics.used_tokens,
            content_changed = comparison.content_changed,
            token_delta = comparison.token_delta,
            "Embedded Laputa payload-free recall comparison"
        );
        if self.mode == MemoryAuthorityMode::Shadow {
            Ok(legacy)
        } else {
            Ok(PrefetchResponse {
                status: PrefetchStatus::Ready,
                prompt_block: shadow.outcome.prompt_block,
            })
        }
    }

    async fn sync_turn(
        &self,
        request: SyncTurnRequest,
    ) -> agent_diva_core::Result<SyncTurnResponse> {
        match self.mode {
            MemoryAuthorityMode::Shadow => self.legacy.sync_turn(request).await,
            MemoryAuthorityMode::Typed => unreachable!("typed mode uses TypedLaputaMemoryProvider"),
            MemoryAuthorityMode::Legacy => self.legacy.sync_turn(request).await,
        }
    }

    async fn on_session_end(
        &self,
        request: SessionEndRequest,
    ) -> agent_diva_core::Result<SessionEndResponse> {
        match self.mode {
            MemoryAuthorityMode::Shadow => self.legacy.on_session_end(request).await,
            MemoryAuthorityMode::Typed => unreachable!("typed mode uses TypedLaputaMemoryProvider"),
            MemoryAuthorityMode::Legacy => self.legacy.on_session_end(request).await,
        }
    }
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

    async fn sync_turn(
        &self,
        _request: SyncTurnRequest,
    ) -> agent_diva_core::Result<SyncTurnResponse> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shadow_requires_a_valid_typed_store() {
        let temp = tempfile::tempdir().unwrap();
        let provider = memory_provider_for_mode(temp.path(), MemoryAuthorityMode::Shadow).await;
        let response = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();
        assert!(matches!(
            response.status,
            agent_diva_core::memory::StartupStatus::Degraded { .. }
        ));
        assert!(!temp.path().join(".laputa").exists());
    }

    #[tokio::test]
    async fn shadow_prefetch_preserves_legacy_result() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(temp.path().join("memory")).unwrap();
        std::fs::write(
            temp.path().join("memory/MEMORY.md"),
            "# Memory\n\nlegacy recall",
        )
        .unwrap();
        agent_diva_laputa::TypedMemoryStore::open(
            temp.path(),
            temp.path().to_string_lossy().to_string(),
        )
        .await
        .unwrap();
        let legacy = memory_provider_for_mode(temp.path(), MemoryAuthorityMode::Legacy).await;
        let shadow = memory_provider_for_mode(temp.path(), MemoryAuthorityMode::Shadow).await;
        let request = PrefetchRequest {
            workspace_root: temp.path().to_path_buf(),
            intent: "legacy recall".into(),
            current_room: None,
            user_message: Some("legacy recall".into()),
        };
        assert_eq!(
            legacy.prefetch(request.clone()).await.unwrap(),
            shadow.prefetch(request).await.unwrap()
        );
    }
}
