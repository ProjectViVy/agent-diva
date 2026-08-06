use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_diva_core::audit::{emit as audit_emit, AuditEvent};
use agent_diva_core::evolution::{
    EvidenceRef, EvidenceSource, EvolutionProposal, ProposalState, ProposalType, RiskLevel,
};
use agent_diva_core::governance::AuditCorrelation;
use agent_diva_core::memory::{
    escape_memory_for_prompt, memory_content_digest, MemoryAddRequest, MemoryCrudContext,
    MemoryCrudOutcome, MemoryDistillRequest, MemoryEntry, MemoryListRequest, MemoryProvenance,
    MemoryProvenanceSource, MemoryProvider, MemoryRecord, MemoryRecordKind, MemoryRemoveRequest,
    MemoryScope, MemorySearchRequest, MemorySensitivity, MemoryTrust, MemoryUpdateRequest,
    PrefetchRequest, PrefetchResponse, PrefetchStatus, RecallOutcomeRequest, RecallPolicy,
    RecallRequest, RecallTurnOutcome, SessionEndRequest, SessionEndResponse, StartupInjectionShape,
    SyncTurnRequest, SyncTurnResponse, SystemPromptBlock, SystemPromptRequest,
    SystemPromptResponse, MAX_CONFIDENCE_BPS,
};
use chrono::{Duration, Utc};

use crate::{
    governed_apply::MemoryGovernanceCoordinator, LaputaMemoryProvider, LaputaRecallService,
    LaputaStorage, PendingRecallFeedback, RecallFeedbackStore, RecallTaskOutcome, TypedMemoryStore,
    TypedMemoryStoreError, MAX_MEMORY_RECORDS,
};

/// Production typed-store Memory provider.
pub struct TypedLaputaMemoryProvider {
    workspace: PathBuf,
    workspace_id: String,
    startup_markdown: Option<String>,
    recall: LaputaRecallService,
    crud_store: TypedMemoryStore,
    coordinator: Option<MemoryGovernanceCoordinator>,
    proposal_sink: Arc<LaputaMemoryProvider>,
    feedback: RecallFeedbackStore,
    pending_feedback: tokio::sync::Mutex<Vec<PendingRecallFeedback>>,
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
        let feedback = RecallFeedbackStore::new(
            LaputaStorage::open(workspace).map_err(|_| TypedMemoryStoreError::CorruptRecord)?,
        );
        let crud_store = TypedMemoryStore::open(workspace, workspace_id.clone()).await?;
        let coordinator = MemoryGovernanceCoordinator::open_lazy(workspace, workspace_id.clone())
            .map_err(|_| TypedMemoryStoreError::CorruptRecord)
            .ok();
        Ok(Self {
            workspace: workspace.to_path_buf(),
            workspace_id,
            startup_markdown,
            recall: LaputaRecallService::new(store),
            crud_store,
            coordinator,
            proposal_sink: Arc::new(proposal_sink),
            feedback,
            pending_feedback: tokio::sync::Mutex::new(Vec::new()),
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

    async fn distill_overwrite_proposal(
        &self,
        request: &MemoryDistillRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        let Some(coordinator) = &self.coordinator else {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_distill governance unavailable".into(),
            });
        };
        let now = Utc::now();
        let id = format!("memory-distill-{}", now.timestamp_micros());
        let proposal = EvolutionProposal {
            id: id.clone(),
            created_at: now,
            updated_at: now,
            created_by: "memory_distill_tool".into(),
            proposal_type: ProposalType::SopCreate,
            target_section: ProposalType::SopCreate.target_section(),
            evidence_refs: vec![EvidenceRef {
                id: format!("evidence-{id}"),
                source: EvidenceSource::Session,
                uri: "memory-distill://tool".into(),
                excerpt: Some("bounded memory_distill evidence".into()),
                hash: None,
                created_at: now,
            }],
            proposed_patch: request.content.clone(),
            risk_level: RiskLevel::Medium,
            state: ProposalState::PendingReview,
            source_run_id: None,
        };
        let proposal = match self.proposal_sink.service.create_proposal(proposal) {
            Ok(proposal) => proposal,
            Err(error) => {
                return Ok(MemoryCrudOutcome::Failed {
                    reason: format!("memory_distill proposal failed:{error}"),
                })
            }
        };
        match coordinator.submit(&proposal, None, now).await {
            Ok(_) => Ok(MemoryCrudOutcome::ProposalCreated {
                proposal_id: proposal.id,
            }),
            Err(error) => Ok(MemoryCrudOutcome::Failed {
                reason: format!("memory_distill governance submit failed:{error}"),
            }),
        }
    }
}

/// Records visible in the applied memory projection.
fn visible_record(record: &MemoryRecord) -> bool {
    record.trust == MemoryTrust::AppliedAuthority
        && record.tombstone.is_none()
        && record.scope.session_id.is_none()
}

fn entry_from(record: MemoryRecord) -> MemoryEntry {
    let trust = serde_json::to_value(&record.trust)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| format!("{:?}", record.trust));
    let provenance = serde_json::to_value(&record.provenance.source)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string));
    MemoryEntry {
        id: record.id,
        content: record.content,
        trust,
        provenance,
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
        let recall_request = self.recall_request(&request);
        let request_id = recall_request.correlation.request_id.clone();
        match self.recall.recall_shadow(&recall_request).await {
            Ok(shadow) => {
                let injected = shadow.outcome.prompt_block.is_some();
                self.pending_feedback
                    .lock()
                    .await
                    .push(PendingRecallFeedback {
                        request_id,
                        selected: shadow
                            .outcome
                            .selected_records
                            .iter()
                            .map(|record| {
                                (record.id.clone(), record.provenance.content_digest.clone())
                            })
                            .collect(),
                        injected,
                        selected_at: Utc::now(),
                    });
                Ok(PrefetchResponse {
                    status: PrefetchStatus::Ready,
                    prompt_block: shadow.outcome.prompt_block,
                })
            }
            Err(error) => Ok(PrefetchResponse {
                status: PrefetchStatus::Failed {
                    reason: format!("typed_recall_degraded:{error}"),
                },
                prompt_block: None,
            }),
        }
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
        let now = Utc::now();
        let digest = memory_content_digest(request.content.as_bytes());
        let id = format!("memory-add-{}", digest.value);
        let record = MemoryRecord {
            id: id.clone(),
            kind: MemoryRecordKind::LongTerm,
            content: request.content.clone(),
            provenance: MemoryProvenance {
                source: MemoryProvenanceSource::LaputaAppliedSection,
                source_id: "memory_add_tool".into(),
                content_digest: digest.clone(),
                captured_at: now,
                correlation: AuditCorrelation {
                    request_id: id.clone(),
                    turn_id: "memory_add".into(),
                    session_id: "global".into(),
                    trace_id: None,
                },
            },
            evidence_refs: vec![],
            confidence_bps: MAX_CONFIDENCE_BPS,
            sensitivity: MemorySensitivity::Internal,
            trust: MemoryTrust::AppliedAuthority,
            scope: MemoryScope {
                tenant_id: "local".into(),
                workspace_id: self.workspace_id.clone(),
                session_id: None,
            },
            created_at: now,
            effective_at: now,
            expires_at: None,
            supersedes: vec![],
            tombstone: None,
        };
        if let Err(error) = record.validate_at(now, Duration::minutes(5)) {
            return Ok(MemoryCrudOutcome::Failed {
                reason: format!("memory_add record invalid:{error}"),
            });
        }
        let metadata = match self.crud_store.metadata().await {
            Ok(metadata) => metadata,
            Err(error) => {
                return Ok(MemoryCrudOutcome::Failed {
                    reason: format!("memory_add metadata failed:{error}"),
                })
            }
        };
        let stored = match self
            .crud_store
            .put(record, metadata.store_revision, None)
            .await
        {
            Ok(stored) => stored,
            Err(error) => {
                return Ok(MemoryCrudOutcome::Failed {
                    reason: format!("memory_add write failed:{error}"),
                })
            }
        };
        audit_emit(AuditEvent::ToolInvoked {
            tool_name: "memory_add".into(),
            args: serde_json::json!({
                "record_id": stored.record.id,
                "content_digest": digest.value,
            }),
        });
        Ok(MemoryCrudOutcome::Applied {
            entry: Some(entry_from(stored.record)),
        })
    }

    async fn memory_list(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryListRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        let limit = request.limit.unwrap_or(MAX_MEMORY_RECORDS as u32);
        match self.crud_store.list(limit).await {
            Ok(records) => {
                let entries = records
                    .into_iter()
                    .map(|stored| stored.record)
                    .filter(visible_record)
                    .map(entry_from)
                    .collect();
                Ok(MemoryCrudOutcome::Listed { entries })
            }
            Err(error) => Ok(MemoryCrudOutcome::Failed {
                reason: format!("memory_list failed:{error}"),
            }),
        }
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
        let limit = request.limit.unwrap_or(16);
        let scope = MemoryScope {
            tenant_id: "local".into(),
            workspace_id: self.workspace_id.clone(),
            session_id: None,
        };
        match self
            .crud_store
            .search_visible(&request.query, &scope, limit)
            .await
        {
            Ok(hits) => {
                let entries = hits
                    .into_iter()
                    .map(|hit| hit.stored.record)
                    .filter(visible_record)
                    .map(entry_from)
                    .collect();
                Ok(MemoryCrudOutcome::Listed { entries })
            }
            Err(error) => Ok(MemoryCrudOutcome::Failed {
                reason: format!("memory_search failed:{error}"),
            }),
        }
    }

    async fn memory_update(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryUpdateRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        let Some(coordinator) = &self.coordinator else {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_update governance unavailable".into(),
            });
        };
        if request.content.trim().is_empty() {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_update content is empty".into(),
            });
        }
        let now = Utc::now();
        let id = format!("memory-update-{}", now.timestamp_micros());
        let proposed_patch = serde_json::json!({
            "record_id": request.record_id,
            "content": request.content,
        })
        .to_string();
        let proposal = EvolutionProposal {
            id: id.clone(),
            created_at: now,
            updated_at: now,
            created_by: "memory_update_tool".into(),
            proposal_type: ProposalType::MemoryPatch,
            target_section: ProposalType::MemoryPatch.target_section(),
            evidence_refs: vec![EvidenceRef {
                id: format!("evidence-{id}"),
                source: EvidenceSource::Session,
                uri: "memory-update://tool".into(),
                excerpt: Some("bounded memory_update evidence".into()),
                hash: None,
                created_at: now,
            }],
            proposed_patch,
            risk_level: RiskLevel::High,
            state: ProposalState::PendingReview,
            source_run_id: None,
        };
        let proposal = match self.proposal_sink.service.create_proposal(proposal) {
            Ok(proposal) => proposal,
            Err(error) => {
                return Ok(MemoryCrudOutcome::Failed {
                    reason: format!("memory_update proposal failed:{error}"),
                })
            }
        };
        match coordinator.submit(&proposal, None, now).await {
            Ok(_) => Ok(MemoryCrudOutcome::ProposalCreated {
                proposal_id: proposal.id,
            }),
            Err(error) => Ok(MemoryCrudOutcome::Failed {
                reason: format!("memory_update governance submit failed:{error}"),
            }),
        }
    }

    async fn memory_remove(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryRemoveRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        let Some(coordinator) = &self.coordinator else {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_remove governance unavailable".into(),
            });
        };
        if request.record_id.trim().is_empty() || request.reason.trim().is_empty() {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "memory_remove requires record_id and reason".into(),
            });
        }
        let now = Utc::now();
        let id = format!("memory-remove-{}", now.timestamp_micros());
        let proposed_patch = serde_json::json!({
            "schema_version": 1,
            "target_record_id": request.record_id,
            "reason": request.reason,
        })
        .to_string();
        let proposal = EvolutionProposal {
            id: id.clone(),
            created_at: now,
            updated_at: now,
            created_by: "memory_remove_tool".into(),
            proposal_type: ProposalType::Deprecation,
            target_section: ProposalType::Deprecation.target_section(),
            evidence_refs: vec![EvidenceRef {
                id: format!("evidence-{id}"),
                source: EvidenceSource::Session,
                uri: "memory-remove://tool".into(),
                excerpt: Some("bounded memory_remove evidence".into()),
                hash: None,
                created_at: now,
            }],
            proposed_patch,
            risk_level: RiskLevel::High,
            state: ProposalState::PendingReview,
            source_run_id: None,
        };
        let proposal = match self.proposal_sink.service.create_proposal(proposal) {
            Ok(proposal) => proposal,
            Err(error) => {
                return Ok(MemoryCrudOutcome::Failed {
                    reason: format!("memory_remove proposal failed:{error}"),
                })
            }
        };
        match coordinator.submit(&proposal, None, now).await {
            Ok(_) => Ok(MemoryCrudOutcome::ProposalCreated {
                proposal_id: proposal.id,
            }),
            Err(error) => Ok(MemoryCrudOutcome::Failed {
                reason: format!("memory_remove governance submit failed:{error}"),
            }),
        }
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
            return self.distill_overwrite_proposal(&request).await;
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
        audit_emit(AuditEvent::ToolInvoked {
            tool_name: "memory_distill".into(),
            args: serde_json::json!({
                "skill_name": request.skill_name,
                "created": true,
            }),
        });
        Ok(MemoryCrudOutcome::Applied { entry: None })
    }

    async fn sync_turn(
        &self,
        request: SyncTurnRequest,
    ) -> agent_diva_core::Result<SyncTurnResponse> {
        self.proposal_sink.sync_turn(request).await
    }

    async fn record_recall_outcome(
        &self,
        request: RecallOutcomeRequest,
    ) -> agent_diva_core::Result<()> {
        let pending = std::mem::take(&mut *self.pending_feedback.lock().await);
        if pending.is_empty() {
            return Ok(());
        }
        let outcome = match request.outcome {
            RecallTurnOutcome::Succeeded => RecallTaskOutcome::Succeeded,
            RecallTurnOutcome::Failed => RecallTaskOutcome::Failed,
        };
        if let Err(error) =
            self.feedback
                .commit_pending(pending.clone(), outcome, request.corrected, Utc::now())
        {
            self.pending_feedback.lock().await.extend(pending);
            return Err(agent_diva_core::Error::Internal(format!(
                "recall_feedback_persistence_failed:{error}"
            )));
        }
        Ok(())
    }

    async fn on_session_end(
        &self,
        request: SessionEndRequest,
    ) -> agent_diva_core::Result<SessionEndResponse> {
        self.proposal_sink.on_session_end(request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ProposalFilter;
    use agent_diva_core::memory::{MemoryRemoveRequest, MemoryUpdateRequest};
    use agent_diva_core::workspace_identity::canonical_workspace_id;

    async fn open_provider(temp: &tempfile::TempDir) -> TypedLaputaMemoryProvider {
        // Mirror production wiring: open_canonical creates the store first.
        TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
        TypedLaputaMemoryProvider::open(temp.path(), canonical_workspace_id(temp.path()))
            .await
            .unwrap()
    }

    fn context(temp: &tempfile::TempDir) -> MemoryCrudContext {
        MemoryCrudContext {
            workspace_root: temp.path().to_path_buf(),
        }
    }

    #[tokio::test]
    async fn add_persists_applied_visible_record() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .memory_add(
                &context(&temp),
                MemoryAddRequest {
                    content: "favorite color is blue".into(),
                },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::Applied { entry } = outcome else {
            panic!("expected applied, got {outcome:?}");
        };
        assert!(entry.is_some());

        let listed = provider
            .memory_list(&context(&temp), MemoryListRequest { limit: None })
            .await
            .unwrap();
        let MemoryCrudOutcome::Listed { entries } = listed else {
            panic!("expected listed, got {listed:?}");
        };
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].content, "favorite color is blue");
        assert_eq!(entries[0].trust, "applied_authority");
    }

    #[tokio::test]
    async fn search_excludes_tombstoned_records() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        provider
            .memory_add(
                &context(&temp),
                MemoryAddRequest {
                    content: "secret project name is Aurora".into(),
                },
            )
            .await
            .unwrap();
        let listed = provider
            .memory_list(&context(&temp), MemoryListRequest { limit: None })
            .await
            .unwrap();
        let MemoryCrudOutcome::Listed { entries } = listed else {
            panic!("expected listed");
        };
        let record_id = entries[0].id.clone();

        let now = Utc::now();
        let tombstone = MemoryRecord {
            id: format!("tombstone-{}", now.timestamp_micros()),
            kind: MemoryRecordKind::LongTerm,
            content: String::new(),
            provenance: MemoryProvenance {
                source: MemoryProvenanceSource::AutoDream,
                source_id: "test".into(),
                content_digest: memory_content_digest(b""),
                captured_at: now,
                correlation: AuditCorrelation {
                    request_id: "t".into(),
                    turn_id: "t".into(),
                    session_id: "g".into(),
                    trace_id: None,
                },
            },
            evidence_refs: vec![],
            confidence_bps: MAX_CONFIDENCE_BPS,
            sensitivity: MemorySensitivity::Internal,
            trust: MemoryTrust::AppliedAuthority,
            scope: MemoryScope {
                tenant_id: "local".into(),
                workspace_id: canonical_workspace_id(temp.path()),
                session_id: None,
            },
            created_at: now,
            effective_at: now,
            expires_at: None,
            supersedes: vec![record_id.clone()],
            tombstone: Some(agent_diva_core::memory::MemoryTombstone {
                target_record_id: record_id.clone(),
                reason_digest: memory_content_digest(b"forgotten"),
                actor_id: "test".into(),
                created_at: now,
            }),
        };
        let metadata = provider.crud_store.metadata().await.unwrap();
        provider
            .crud_store
            .put(tombstone, metadata.store_revision, None)
            .await
            .unwrap();

        let search = provider
            .memory_search(
                &context(&temp),
                MemorySearchRequest {
                    query: "Aurora".into(),
                    limit: Some(10),
                },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::Listed { entries } = search else {
            panic!("expected listed, got {search:?}");
        };
        assert!(
            entries.iter().all(|entry| entry.id != record_id),
            "tombstoned record must not appear in search results"
        );
    }

    #[tokio::test]
    async fn update_creates_reviewable_proposal() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .memory_update(
                &context(&temp),
                MemoryUpdateRequest {
                    record_id: "rec-1".into(),
                    content: "new content".into(),
                },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::ProposalCreated { proposal_id } = outcome else {
            panic!("expected proposal_created, got {outcome:?}");
        };
        let proposals = provider
            .proposal_sink
            .service
            .list_proposals(ProposalFilter::default())
            .unwrap();
        assert!(proposals.iter().any(|p| p.id == proposal_id));
        assert_eq!(proposals[0].state, ProposalState::PendingReview);
    }

    #[tokio::test]
    async fn remove_creates_deprecation_proposal() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .memory_remove(
                &context(&temp),
                MemoryRemoveRequest {
                    record_id: "rec-9".into(),
                    reason: "user asked to forget".into(),
                },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::ProposalCreated { .. } = outcome else {
            panic!("expected proposal_created, got {outcome:?}");
        };
        let proposals = provider
            .proposal_sink
            .service
            .list_proposals(ProposalFilter::default())
            .unwrap();
        assert_eq!(proposals[0].proposal_type, ProposalType::Deprecation);
        assert!(proposals[0].proposed_patch.contains("rec-9"));
    }

    #[tokio::test]
    async fn distill_creates_new_skill_and_proposalizes_overwrite() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let fresh = provider
            .memory_distill(
                &context(&temp),
                MemoryDistillRequest {
                    skill_name: "vue3-gotcha".into(),
                    content: "Always use ref for reactive primitives.".into(),
                },
            )
            .await
            .unwrap();
        assert!(matches!(fresh, MemoryCrudOutcome::Applied { .. }));
        let skill_file = temp.path().join("skills/vue3-gotcha/SKILL.md");
        assert!(skill_file.exists());
        assert!(std::fs::read_to_string(&skill_file)
            .unwrap()
            .contains("ref for reactive primitives"));

        let overwrite = provider
            .memory_distill(
                &context(&temp),
                MemoryDistillRequest {
                    skill_name: "vue3-gotcha".into(),
                    content: "Revised guidance.".into(),
                },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::ProposalCreated { proposal_id } = overwrite else {
            panic!("expected proposal_created, got {overwrite:?}");
        };
        let proposals = provider
            .proposal_sink
            .service
            .list_proposals(ProposalFilter::default())
            .unwrap();
        assert!(proposals.iter().any(|p| p.id == proposal_id));
        assert_eq!(proposals[0].proposal_type, ProposalType::SopCreate);
    }
}
