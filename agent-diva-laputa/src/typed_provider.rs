use std::{path::Path, sync::Arc};

use agent_diva_core::governance::AuditCorrelation;
use agent_diva_core::memory::{
    escape_memory_for_prompt, memory_content_digest, MemoryProvider, MemoryScope, MemoryTrust,
    PrefetchRequest, PrefetchResponse, PrefetchStatus, RecallPolicy, RecallRequest,
    SessionEndRequest, SessionEndResponse, StartupInjectionShape, SyncTurnRequest,
    SyncTurnResponse, SystemPromptBlock, SystemPromptRequest, SystemPromptResponse,
};
use chrono::Utc;

use crate::{
    LaputaMemoryProvider, LaputaRecallService, TypedMemoryStore, TypedMemoryStoreError,
    MAX_MEMORY_RECORDS,
};

/// Production typed-store Memory provider.
pub struct TypedLaputaMemoryProvider {
    workspace_id: String,
    startup_markdown: Option<String>,
    recall: LaputaRecallService,
    proposal_sink: Arc<LaputaMemoryProvider>,
}

impl TypedLaputaMemoryProvider {
    pub async fn open(
        workspace: impl AsRef<Path>,
        workspace_id: impl Into<String>,
    ) -> Result<Self, TypedMemoryStoreError> {
        let workspace = workspace.as_ref();
        let workspace_id = workspace_id.into();
        let store = TypedMemoryStore::open_existing(workspace, workspace_id.clone()).await?;
        let integrity = store.integrity().await?;
        if !integrity.corrupt_record_ids.is_empty() || integrity.orphan_fts_rows != 0 {
            return Err(TypedMemoryStoreError::CorruptRecord);
        }
        let records = store.list(MAX_MEMORY_RECORDS as u32).await?;
        let rendered = records
            .into_iter()
            .filter(|stored| {
                stored.record.trust == MemoryTrust::AppliedAuthority
                    && stored.record.tombstone.is_none()
                    && stored.record.scope.session_id.is_none()
            })
            .map(|stored| escape_memory_for_prompt(&stored.record))
            .collect::<Vec<_>>()
            .join("\n\n");
        let startup_markdown = (!rendered.is_empty())
            .then(|| format!("## Embedded Laputa Typed Memory\n\n{rendered}"));
        let proposal_sink = LaputaMemoryProvider::open(workspace)
            .map_err(|_| TypedMemoryStoreError::CorruptRecord)?;
        Ok(Self {
            workspace_id,
            startup_markdown,
            recall: LaputaRecallService::new(store),
            proposal_sink: Arc::new(proposal_sink),
        })
    }

    fn recall_request(&self, request: &PrefetchRequest) -> RecallRequest {
        let query = request
            .user_message
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&request.intent)
            .to_string();
        let digest = memory_content_digest(query.as_bytes()).value;
        RecallRequest {
            query,
            scope: MemoryScope {
                tenant_id: "local".into(),
                workspace_id: self.workspace_id.clone(),
                session_id: None,
            },
            correlation: AuditCorrelation {
                request_id: format!("typed-recall-{digest}"),
                turn_id: "prefetch".into(),
                session_id: "global".into(),
                trace_id: None,
            },
            now: Utc::now(),
            token_budget: 4_000,
            max_candidates: 32,
            policy: RecallPolicy::default_prompt(),
        }
    }
}

#[async_trait::async_trait]
impl MemoryProvider for TypedLaputaMemoryProvider {
    fn system_prompt_block(
        &self,
        _request: &SystemPromptRequest,
    ) -> agent_diva_core::Result<SystemPromptResponse> {
        Ok(match &self.startup_markdown {
            Some(markdown) => SystemPromptResponse::ready(SystemPromptBlock {
                shape: StartupInjectionShape::CompactRenderedMarkdown,
                markdown: markdown.clone(),
            }),
            None => SystemPromptResponse::degraded(
                "Embedded Laputa typed authority is empty; no startup Memory rendered",
            ),
        })
    }

    async fn prefetch(
        &self,
        request: PrefetchRequest,
    ) -> agent_diva_core::Result<PrefetchResponse> {
        if request.intent.trim().is_empty() {
            return Ok(PrefetchResponse::default());
        }
        match self
            .recall
            .recall_shadow(&self.recall_request(&request))
            .await
        {
            Ok(shadow) => Ok(PrefetchResponse {
                status: PrefetchStatus::Ready,
                prompt_block: shadow.outcome.prompt_block,
            }),
            Err(error) => Ok(PrefetchResponse {
                status: PrefetchStatus::Failed {
                    reason: format!("typed_recall_degraded:{error}"),
                },
                prompt_block: None,
            }),
        }
    }

    async fn sync_turn(
        &self,
        request: SyncTurnRequest,
    ) -> agent_diva_core::Result<SyncTurnResponse> {
        self.proposal_sink.sync_turn(request).await
    }

    async fn on_session_end(
        &self,
        request: SessionEndRequest,
    ) -> agent_diva_core::Result<SessionEndResponse> {
        self.proposal_sink.on_session_end(request).await
    }
}
