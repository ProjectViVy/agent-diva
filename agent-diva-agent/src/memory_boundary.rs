use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_diva_core::evolution::{
    EvidenceRef, EvidenceSource, EvolutionProposal, ProposalState, ProposalType, RiskLevel,
};
use agent_diva_core::memory::{
    compare_recall_shadow, memory_content_digest, MemoryAddRequest, MemoryCrudContext,
    MemoryCrudOutcome, MemoryDistillRequest, MemoryEntry, MemoryListRequest, MemoryManager,
    MemoryProvider, MemoryRemoveRequest, MemoryScope, MemorySearchRequest, MemoryUpdateRequest,
    PrefetchRequest, PrefetchResponse, PrefetchStatus, RecallPolicy, RecallRequest,
    SessionEndRequest, SessionEndResponse, SessionEndStatus, SyncTurnRequest, SyncTurnResponse,
    SyncTurnStatus, SystemPromptRequest, SystemPromptResponse,
};
use agent_diva_core::{config::schema::MemoryAuthorityMode, governance::AuditCorrelation};
use chrono::Utc;
use tracing::warn;

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
    l1_index_lines: usize,
) -> Arc<dyn MemoryProvider> {
    if mode == MemoryAuthorityMode::Legacy {
        return Arc::new(LegacyCrudMemoryProvider::new(workspace, l1_index_lines));
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
        return match agent_diva_laputa::TypedLaputaMemoryProvider::open_with_l1_budget(
            workspace,
            agent_diva_core::workspace_identity::canonical_workspace_id(workspace),
            l1_index_lines,
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

    fn system_prompt_revision(&self, request: &SystemPromptRequest) -> u64 {
        self.legacy.system_prompt_revision(request)
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

/// Legacy-mode provider with proposal-first CRUD.
///
/// Reads stay on `MemoryManager` (MEMORY.md compatibility input); writes
/// never mutate MEMORY.md as authority — every write creates a reviewable
/// Laputa proposal (decision 1: proposal-first in Legacy mode).
pub(crate) struct LegacyCrudMemoryProvider {
    workspace: PathBuf,
    legacy: MemoryManager,
    service: Option<agent_diva_laputa::LaputaService>,
    coordinator: Option<agent_diva_laputa::governed_apply::MemoryGovernanceCoordinator>,
}

impl LegacyCrudMemoryProvider {
    pub(crate) fn new(workspace: &Path, l1_index_lines: usize) -> Self {
        let workspace_id = agent_diva_core::workspace_identity::canonical_workspace_id(workspace);
        let coordinator =
            agent_diva_laputa::governed_apply::MemoryGovernanceCoordinator::open_lazy(
                workspace,
                workspace_id,
            )
            .map_err(|error| warn!("legacy CRUD governance unavailable: {error}"))
            .ok();
        Self {
            workspace: workspace.to_path_buf(),
            legacy: MemoryManager::new(workspace).with_l1_index_lines(l1_index_lines),
            service: agent_diva_laputa::LaputaService::open(workspace)
                .map_err(|error| warn!("legacy CRUD laputa unavailable: {error}"))
                .ok(),
            coordinator,
        }
    }

    async fn proposal_outcome(
        &self,
        proposal_type: ProposalType,
        operation: &str,
        record_id: Option<&str>,
        content: String,
        risk_level: RiskLevel,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        let Some(coordinator) = &self.coordinator else {
            return Ok(MemoryCrudOutcome::Failed {
                reason: format!("{operation} governance unavailable"),
            });
        };
        let Some(service) = &self.service else {
            return Ok(MemoryCrudOutcome::Failed {
                reason: format!("{operation} laputa service unavailable"),
            });
        };
        let now = Utc::now();
        let id = format!("{operation}-{}", now.timestamp_micros());
        let proposed_patch = match &proposal_type {
            ProposalType::Deprecation => serde_json::json!({
                "schema_version": 1,
                "target_record_id": record_id.unwrap_or_default(),
                "reason": content,
            })
            .to_string(),
            _ => serde_json::json!({
                "record_id": record_id.unwrap_or_default(),
                "content": content,
            })
            .to_string(),
        };
        let target_section = proposal_type.target_section();
        let proposal = EvolutionProposal {
            id: id.clone(),
            created_at: now,
            updated_at: now,
            created_by: format!("{operation}_tool"),
            proposal_type,
            target_section,
            evidence_refs: vec![EvidenceRef {
                id: format!("evidence-{id}"),
                source: EvidenceSource::Session,
                uri: format!("{operation}://tool"),
                excerpt: Some(format!("bounded {operation} evidence")),
                hash: None,
                created_at: now,
            }],
            proposed_patch,
            risk_level,
            state: ProposalState::PendingReview,
            source_run_id: None,
        };
        let proposal = match service.create_proposal(proposal) {
            Ok(proposal) => proposal,
            Err(error) => {
                return Ok(MemoryCrudOutcome::Failed {
                    reason: format!("{operation} proposal failed:{error}"),
                })
            }
        };
        match coordinator.submit(&proposal, None, now).await {
            Ok(_) => Ok(MemoryCrudOutcome::ProposalCreated {
                proposal_id: proposal.id,
            }),
            Err(error) => Ok(MemoryCrudOutcome::Failed {
                reason: format!("{operation} governance submit failed:{error}"),
            }),
        }
    }

    fn visible_lines(&self) -> Vec<MemoryEntry> {
        self.legacy
            .load_memory()
            .content
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .enumerate()
            .map(|(index, line)| MemoryEntry {
                id: format!("legacy-line-{index}"),
                content: line.to_string(),
                trust: "legacy".to_string(),
                provenance: Some("legacy_markdown".to_string()),
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl MemoryProvider for LegacyCrudMemoryProvider {
    fn system_prompt_block(
        &self,
        request: &SystemPromptRequest,
    ) -> agent_diva_core::Result<SystemPromptResponse> {
        self.legacy.system_prompt_block(request)
    }

    fn system_prompt_revision(&self, request: &SystemPromptRequest) -> u64 {
        self.legacy.system_prompt_revision(request)
    }

    async fn prefetch(
        &self,
        request: PrefetchRequest,
    ) -> agent_diva_core::Result<PrefetchResponse> {
        self.legacy.prefetch(request).await
    }

    async fn sync_turn(
        &self,
        request: SyncTurnRequest,
    ) -> agent_diva_core::Result<SyncTurnResponse> {
        self.legacy.sync_turn(request).await
    }

    async fn on_session_end(
        &self,
        request: SessionEndRequest,
    ) -> agent_diva_core::Result<SessionEndResponse> {
        self.legacy.on_session_end(request).await
    }

    async fn memory_add(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryAddRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        if request.content.trim().is_empty() {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_add content is empty".into(),
            });
        }
        self.proposal_outcome(
            ProposalType::MemoryPatch,
            "memory_add",
            None,
            request.content,
            RiskLevel::Low,
        )
        .await
    }

    async fn memory_list(
        &self,
        _context: &MemoryCrudContext,
        _request: MemoryListRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        Ok(MemoryCrudOutcome::Listed {
            entries: self.visible_lines(),
        })
    }

    async fn memory_search(
        &self,
        _context: &MemoryCrudContext,
        request: MemorySearchRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        if request.query.trim().is_empty() {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_search query is empty".into(),
            });
        }
        let entries = self
            .visible_lines()
            .into_iter()
            .filter(|entry| {
                entry
                    .content
                    .to_lowercase()
                    .contains(&request.query.to_lowercase())
            })
            .collect();
        Ok(MemoryCrudOutcome::Listed { entries })
    }

    async fn memory_update(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryUpdateRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        if request.content.trim().is_empty() {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_update content is empty".into(),
            });
        }
        self.proposal_outcome(
            ProposalType::MemoryPatch,
            "memory_update",
            Some(&request.record_id),
            request.content,
            RiskLevel::High,
        )
        .await
    }

    async fn memory_remove(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryRemoveRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        if request.record_id.trim().is_empty() || request.reason.trim().is_empty() {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_remove requires record_id and reason".into(),
            });
        }
        self.proposal_outcome(
            ProposalType::Deprecation,
            "memory_remove",
            Some(&request.record_id),
            request.reason,
            RiskLevel::High,
        )
        .await
    }

    async fn memory_distill(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryDistillRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        if request.skill_name.trim().is_empty() || request.content.trim().is_empty() {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_distill requires skill_name and content".into(),
            });
        }
        let skill_dir = self.workspace.join("skills").join(&request.skill_name);
        let skill_file = skill_dir.join("SKILL.md");
        if skill_file.exists() {
            return self
                .proposal_outcome(
                    ProposalType::SopCreate,
                    "memory_distill",
                    None,
                    request.content,
                    RiskLevel::Medium,
                )
                .await;
        }
        if let Err(error) = std::fs::create_dir_all(&skill_dir) {
            return Ok(MemoryCrudOutcome::Failed {
                reason: format!("memory_distill mkdir failed:{error}"),
            });
        }
        let body = format!(
            "---\nname: {}\ndescription: Distilled experience from a completed task\n---\n\n{}\n",
            request.skill_name, request.content
        );
        if let Err(error) = std::fs::write(&skill_file, body) {
            return Ok(MemoryCrudOutcome::Failed {
                reason: format!("memory_distill write failed:{error}"),
            });
        }
        Ok(MemoryCrudOutcome::Applied {
            entry: None,
            evidence_advisory: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shadow_requires_a_valid_typed_store() {
        let temp = tempfile::tempdir().unwrap();
        let provider = memory_provider_for_mode(temp.path(), MemoryAuthorityMode::Shadow, 30).await;
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
        let legacy = memory_provider_for_mode(temp.path(), MemoryAuthorityMode::Legacy, 30).await;
        let shadow = memory_provider_for_mode(temp.path(), MemoryAuthorityMode::Shadow, 30).await;
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

    #[tokio::test]
    async fn legacy_add_creates_proposal_not_memory_write() {
        let temp = tempfile::tempdir().unwrap();
        let provider = LegacyCrudMemoryProvider::new(temp.path(), 30);
        let outcome = provider
            .memory_add(
                &MemoryCrudContext {
                    workspace_root: temp.path().to_path_buf(),
                },
                MemoryAddRequest {
                    content: "remember this fact".into(),
                    evidence_refs: vec![],
                },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::ProposalCreated { proposal_id } = outcome else {
            panic!("expected proposal_created, got {outcome:?}");
        };
        let proposals = provider
            .service
            .as_ref()
            .unwrap()
            .list_proposals(agent_diva_laputa::ProposalFilter::default())
            .unwrap();
        assert!(proposals.iter().any(|p| p.id == proposal_id));
        assert_eq!(proposals[0].proposal_type, ProposalType::MemoryPatch);
    }

    #[tokio::test]
    async fn legacy_list_reads_memory_markdown_lines() {
        let temp = tempfile::tempdir().unwrap();
        let memory_dir = temp.path().join("memory");
        std::fs::create_dir_all(&memory_dir).unwrap();
        std::fs::write(
            memory_dir.join("MEMORY.md"),
            "# Memory

- fact one
- fact two
",
        )
        .unwrap();
        let provider = LegacyCrudMemoryProvider::new(temp.path(), 30);
        let outcome = provider
            .memory_list(
                &MemoryCrudContext {
                    workspace_root: temp.path().to_path_buf(),
                },
                MemoryListRequest { limit: None },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::Listed { entries } = outcome else {
            panic!("expected listed, got {outcome:?}");
        };
        assert!(entries.iter().any(|e| e.content == "- fact one"));
        assert!(entries.iter().any(|e| e.content == "- fact two"));
    }
}
