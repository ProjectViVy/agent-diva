use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use agent_diva_core::audit::{emit as audit_emit, AuditEvent};
use agent_diva_core::evolution::{
    EvidenceRef, EvidenceSource, EvolutionProposal, ProposalState, ProposalType, RiskLevel,
};
use agent_diva_core::governance::AuditCorrelation;
use agent_diva_core::memory::{
    memory_content_digest, render_checkpoint_block, render_l1_index_block, CheckpointWriteRequest,
    MemoryAddRequest, MemoryCrudContext, MemoryCrudOutcome, MemoryDistillRequest, MemoryEntry,
    MemoryListRequest, MemoryProvenance, MemoryProvenanceSource, MemoryProvider, MemoryRecord,
    MemoryRecordKind, MemoryRemoveRequest, MemoryScope, MemorySearchRequest, MemorySensitivity,
    MemoryTrust, MemoryUpdateRequest, PrefetchRequest, PrefetchResponse, PrefetchStatus,
    RecallOutcomeRequest, RecallPolicy, RecallRequest, RecallTurnOutcome, SessionEndRequest,
    SessionEndResponse, SessionEndStatus, StartupInjectionShape, SyncTurnRequest, SyncTurnResponse,
    SystemPromptBlock, SystemPromptRequest, SystemPromptResponse, WorkingMemoryRequest,
    WorkingMemoryResponse, DEFAULT_L1_INDEX_LINES, MAX_CONFIDENCE_BPS,
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
    startup_markdown: RwLock<Option<String>>,
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
        Self::open_with_l1_budget(workspace, workspace_id, DEFAULT_L1_INDEX_LINES).await
    }

    pub async fn open_with_l1_budget(
        workspace: impl AsRef<Path>,
        workspace_id: impl Into<String>,
        l1_index_lines: usize,
    ) -> Result<Self, TypedMemoryStoreError> {
        let workspace = workspace.as_ref();
        let workspace_id = workspace_id.into();
        let store = TypedMemoryStore::open_existing(workspace, workspace_id.clone()).await?;
        let integrity = store.integrity().await?;
        if !integrity.corrupt_record_ids.is_empty() || integrity.orphan_fts_rows != 0 {
            return Err(TypedMemoryStoreError::CorruptRecord);
        }
        let startup_markdown =
            RwLock::new(Self::render_startup_index(&store, l1_index_lines).await?);
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

    async fn render_startup_index(
        store: &TypedMemoryStore,
        l1_index_lines: usize,
    ) -> Result<Option<String>, TypedMemoryStoreError> {
        let records = store.list(MAX_MEMORY_RECORDS as u32).await?;
        let superseded = store.superseded_target_ids().await.unwrap_or_default();
        let index_entries = records
            .into_iter()
            .filter(|stored| {
                stored.record.trust == MemoryTrust::AppliedAuthority
                    && stored.record.tombstone.is_none()
                    && stored.record.scope.session_id.is_none()
                    && !superseded.contains(&stored.record.id)
            })
            .map(|stored| (stored.record.id, stored.record.content))
            .collect::<Vec<_>>();
        let rendered = render_l1_index_block(&index_entries, l1_index_lines);
        Ok((!rendered.trim().is_empty())
            .then(|| format!("## Embedded Laputa Typed Memory\n\n{rendered}")))
    }

    async fn refresh_startup_markdown(&self) -> Result<(), TypedMemoryStoreError> {
        let rendered = Self::render_startup_index(&self.crud_store, DEFAULT_L1_INDEX_LINES).await?;
        let mut guard = self.startup_markdown.write().unwrap();
        *guard = rendered;
        Ok(())
    }

    /// Stable record id for a session working checkpoint.
    fn checkpoint_id(&self, session_id: &str) -> String {
        let digest = memory_content_digest(session_id.as_bytes()).value;
        format!("working-checkpoint-{}", &digest[..16])
    }

    /// Startup sweep for abnormal exits (crash, kill -9, power loss):
    /// physically delete every session-scoped working-memory record whose
    /// `session_id` is not in `active_session_ids`. Failures are non-fatal
    /// — the provider keeps serving and the next sweep will retry.
    ///
    /// Safe to call before any session begins; does not touch long-term
    /// authority records (records with NULL `session_id`).
    pub async fn run_startup_gc(&self, active_session_ids: &[&str]) {
        match self
            .crud_store
            .gc_stale_session_scoped(active_session_ids)
            .await
        {
            Ok(0) => {}
            Ok(cleared_rows) => {
                tracing::info!(
                    cleared_rows,
                    active_sessions = active_session_ids.len(),
                    "Startup GC removed stale session-scoped working memory (Wave 5)"
                );
            }
            Err(error) => {
                tracing::warn!(
                    gc_error = %error,
                    "Startup working memory GC failed (non-fatal)"
                );
            }
        }
    }

    async fn checkpoint_record(&self, request: &CheckpointWriteRequest) -> MemoryRecord {
        let now = Utc::now();
        let id = self.checkpoint_id(&request.session_id);
        let content =
            render_checkpoint_block(&request.key_info, &request.related_sops, &request.content);
        let content_digest = memory_content_digest(content.as_bytes());
        MemoryRecord {
            id: id.clone(),
            kind: MemoryRecordKind::WorkingMemory,
            content,
            provenance: MemoryProvenance {
                source: MemoryProvenanceSource::LaputaAppliedSection,
                source_id: "update_working_checkpoint_tool".into(),
                content_digest,
                captured_at: now,
                correlation: AuditCorrelation {
                    request_id: id.clone(),
                    turn_id: "update_working_checkpoint".into(),
                    session_id: request.session_id.clone(),
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
                session_id: Some(request.session_id.clone()),
            },
            created_at: now,
            effective_at: now,
            expires_at: None,
            supersedes: vec![],
            tombstone: None,
        }
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
                excerpt: Some(
                    match request
                        .evidence
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                    {
                        Some(evidence) => format!(
                            "memory_distill evidence: {}",
                            evidence.chars().take(200).collect::<String>()
                        ),
                        None => "bounded memory_distill evidence".into(),
                    },
                ),
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
        let cached = self.startup_markdown.read().unwrap().clone();
        Ok(match cached {
            Some(markdown) => SystemPromptResponse::ready(SystemPromptBlock {
                shape: StartupInjectionShape::CompactRenderedMarkdown,
                markdown,
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
        if let Err(e) = self.refresh_startup_markdown().await {
            tracing::warn!("memory_add: failed to refresh startup cache: {e}");
        }
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
                let superseded = self
                    .crud_store
                    .superseded_target_ids()
                    .await
                    .unwrap_or_default();
                let entries = records
                    .into_iter()
                    .map(|stored| stored.record)
                    .filter(visible_record)
                    .filter(|record| !superseded.contains(&record.id))
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
        let superseded = self
            .crud_store
            .superseded_target_ids()
            .await
            .unwrap_or_default();
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
                    .filter(|record| !superseded.contains(&record.id))
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
        if let Some(evidence) = request
            .evidence
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            let evidence_file = skill_dir.join("EVIDENCE.md");
            let _ = std::fs::write(
                &evidence_file,
                format!("# Evidence\n\n{}\n", evidence.trim()),
            );
        }
        audit_emit(AuditEvent::ToolInvoked {
            tool_name: "memory_distill".into(),
            args: serde_json::json!({
                "skill_name": request.skill_name,
                "created": true,
                "has_evidence": request.evidence.as_deref().is_some_and(|v| !v.trim().is_empty()),
            }),
        });
        Ok(MemoryCrudOutcome::Applied { entry: None })
    }

    async fn checkpoint_write(
        &self,
        request: CheckpointWriteRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        if request.session_id.trim().is_empty()
            || (request.key_info.trim().is_empty() && request.content.trim().is_empty())
        {
            return Ok(MemoryCrudOutcome::Failed {
                reason: "checkpoint_write requires session_id and key_info or content".into(),
            });
        }
        let record = self.checkpoint_record(&request).await;
        let metadata = match self.crud_store.metadata().await {
            Ok(metadata) => metadata,
            Err(error) => {
                return Ok(MemoryCrudOutcome::Failed {
                    reason: format!("checkpoint_write metadata failed:{error}"),
                })
            }
        };
        let existing = match self.crud_store.get(&record.id).await {
            Ok(Some(stored)) => Some(stored.revision),
            Ok(None) => None,
            Err(error) => {
                return Ok(MemoryCrudOutcome::Failed {
                    reason: format!("checkpoint_write lookup failed:{error}"),
                })
            }
        };
        match self
            .crud_store
            .put(record.clone(), metadata.store_revision, existing)
            .await
        {
            Ok(stored) => {
                audit_emit(AuditEvent::ToolInvoked {
                    tool_name: "update_working_checkpoint".into(),
                    args: serde_json::json!({
                        "session_id": request.session_id,
                        "record_id": stored.record.id,
                    }),
                });
                Ok(MemoryCrudOutcome::Applied {
                    entry: Some(entry_from(stored.record)),
                })
            }
            Err(error) => Ok(MemoryCrudOutcome::Failed {
                reason: format!("checkpoint_write failed:{error}"),
            }),
        }
    }

    async fn working_memory_block(
        &self,
        request: WorkingMemoryRequest,
    ) -> agent_diva_core::Result<WorkingMemoryResponse> {
        if request.session_id.trim().is_empty() {
            return Ok(WorkingMemoryResponse::default());
        }
        let id = self.checkpoint_id(&request.session_id);
        let records = match self.crud_store.list(MAX_MEMORY_RECORDS as u32).await {
            Ok(records) => records,
            Err(error) => {
                tracing::warn!(
                    working_memory_error = %error,
                    "Working memory block read failed (non-fatal)"
                );
                return Ok(WorkingMemoryResponse::default());
            }
        };
        let superseded = records
            .iter()
            .filter(|stored| stored.record.tombstone.is_some())
            .flat_map(|stored| stored.record.supersedes.iter())
            .any(|target| target == &id);
        if superseded {
            return Ok(WorkingMemoryResponse::default());
        }
        match records.into_iter().find(|stored| stored.record.id == id) {
            Some(stored)
                if stored.record.tombstone.is_none()
                    && stored.record.scope.session_id.as_deref() == Some(&request.session_id) =>
            {
                Ok(WorkingMemoryResponse {
                    prompt_block: Some(stored.record.content),
                })
            }
            _ => Ok(WorkingMemoryResponse::default()),
        }
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
        let Some(session_id) = request
            .session_id
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
        else {
            return self.proposal_sink.on_session_end(request).await;
        };
        let cleared_rows = match self.crud_store.gc_session_scoped(&session_id).await {
            Ok(n) => n,
            Err(error) => {
                tracing::warn!(
                    session_id = %session_id,
                    gc_error = %error,
                    "Working memory GC on session end failed (non-fatal)"
                );
                0
            }
        };
        let terminal = self.proposal_sink.on_session_end(request).await?;
        if cleared_rows > 0 {
            tracing::info!(
                session_id = %session_id,
                cleared_rows,
                "Working memory physically cleared on session end (Wave 5)"
            );
            Ok(SessionEndResponse {
                status: SessionEndStatus::Triggered,
            })
        } else {
            Ok(terminal)
        }
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
                    evidence: None,
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
                    evidence: None,
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

#[cfg(test)]
mod wave2_tests {
    use super::*;
    use agent_diva_core::memory::{
        CheckpointWriteRequest, SessionEndRequest, WorkingMemoryRequest,
    };
    use agent_diva_core::workspace_identity::canonical_workspace_id;

    async fn open_provider(temp: &tempfile::TempDir) -> TypedLaputaMemoryProvider {
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

    async fn open_provider_with_budget(
        temp: &tempfile::TempDir,
        l1_index_lines: usize,
    ) -> TypedLaputaMemoryProvider {
        TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
        TypedLaputaMemoryProvider::open_with_l1_budget(
            temp.path(),
            canonical_workspace_id(temp.path()),
            l1_index_lines,
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn startup_injects_bounded_l1_index_not_full_content() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider_with_budget(&temp, 2).await;
        for i in 0..5 {
            provider
                .memory_add(
                    &context(&temp),
                    MemoryAddRequest {
                        content: format!(
                            "fact number {i} with a very long tail {}",
                            "y".repeat(120)
                        ),
                    },
                )
                .await
                .unwrap();
        }
        let fresh = open_provider_with_budget(&temp, 2).await;
        let response = fresh
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();
        let block = response.prompt_block.expect("startup block").markdown;
        assert!(block.contains("Long-term Memory Index"));
        assert!(block.contains("use memory_search or memory_list"));
        assert_eq!(block.matches("- [memory-add-").count(), 2);
        assert!(!block.contains(&"y".repeat(120)));
        assert!(!block.contains("<memory-data"));
    }

    #[tokio::test]
    async fn zero_l1_budget_renders_no_index() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider_with_budget(&temp, 0).await;
        provider
            .memory_add(
                &context(&temp),
                MemoryAddRequest {
                    content: "visible fact".into(),
                },
            )
            .await
            .unwrap();
        let fresh = open_provider_with_budget(&temp, 0).await;
        let response = fresh
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();
        assert!(matches!(
            response.status,
            agent_diva_core::memory::StartupStatus::Degraded { .. }
        ));
    }

    #[tokio::test]
    async fn checkpoint_write_read_roundtrip_and_excluded_from_startup() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .checkpoint_write(CheckpointWriteRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: "channel:42".into(),
                key_info: "migrating service B".into(),
                related_sops: vec!["rust-deploy".into()],
                content: "port 8080 confirmed".into(),
            })
            .await
            .unwrap();
        assert!(matches!(outcome, MemoryCrudOutcome::Applied { .. }));

        let block = provider
            .working_memory_block(WorkingMemoryRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: "channel:42".into(),
            })
            .await
            .unwrap();
        let prompt_block = block.prompt_block.expect("checkpoint block expected");
        assert!(prompt_block.starts_with("## Working Memory"));
        assert!(prompt_block.contains("migrating service B"));
        assert!(prompt_block.contains("- rust-deploy"));
        assert!(prompt_block.contains("port 8080 confirmed"));

        let other = provider
            .working_memory_block(WorkingMemoryRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: "channel:other".into(),
            })
            .await
            .unwrap();
        assert_eq!(other.prompt_block, None);

        let startup = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();
        assert!(!startup
            .prompt_block
            .as_ref()
            .map_or(String::new(), |b| b.markdown.clone())
            .contains("migrating service B"));
    }

    #[tokio::test]
    async fn checkpoint_overwrite_replaces_content() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let request = |content: &str| CheckpointWriteRequest {
            workspace_root: temp.path().to_path_buf(),
            session_id: "channel:7".into(),
            key_info: content.into(),
            related_sops: vec![],
            content: String::new(),
        };
        provider
            .checkpoint_write(request("phase one"))
            .await
            .unwrap();
        provider
            .checkpoint_write(request("phase two"))
            .await
            .unwrap();
        let block = provider
            .working_memory_block(WorkingMemoryRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: "channel:7".into(),
            })
            .await
            .unwrap();
        let prompt_block = block.prompt_block.unwrap();
        assert!(prompt_block.contains("phase two"));
        assert!(!prompt_block.contains("phase one"));
    }

    #[tokio::test]
    async fn session_end_clears_checkpoint() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        provider
            .checkpoint_write(CheckpointWriteRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: "channel:9".into(),
                key_info: "in-flight state".into(),
                related_sops: vec![],
                content: String::new(),
            })
            .await
            .unwrap();
        let ended = provider
            .on_session_end(SessionEndRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: Some("channel:9".into()),
            })
            .await
            .unwrap();
        eprintln!("SESSION_END_STATUS={:?}", ended.status);
        let block = provider
            .working_memory_block(WorkingMemoryRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: "channel:9".into(),
            })
            .await
            .unwrap();
        assert_eq!(block.prompt_block, None);
    }

    #[tokio::test]
    async fn checkpoint_write_requires_session_content() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .checkpoint_write(CheckpointWriteRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: "".into(),
                key_info: String::new(),
                related_sops: vec![],
                content: String::new(),
            })
            .await
            .unwrap();
        assert!(matches!(outcome, MemoryCrudOutcome::Failed { .. }));
    }

    #[tokio::test]
    async fn distill_fresh_writes_evidence_file() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .memory_distill(
                &context(&temp),
                MemoryDistillRequest {
                    skill_name: "deploy-runbook".into(),
                    content: "Run the smoke suite before tagging.".into(),
                    evidence: Some("session: rollout of v0.5.0 completed".into()),
                },
            )
            .await
            .unwrap();
        assert!(matches!(outcome, MemoryCrudOutcome::Applied { .. }));
        let evidence_file = temp.path().join("skills/deploy-runbook/EVIDENCE.md");
        assert!(evidence_file.exists());
        let content = std::fs::read_to_string(evidence_file).unwrap();
        assert!(content.contains("rollout of v0.5.0"));
    }
}

#[cfg(test)]
mod wave3_tests {
    use super::*;
    use agent_diva_core::memory::{
        MemorySearchRequest, MemoryTombstone, PrefetchRequest, SessionEndRequest,
        SystemPromptRequest,
    };
    use agent_diva_core::workspace_identity::canonical_workspace_id;

    async fn open_provider(temp: &tempfile::TempDir) -> TypedLaputaMemoryProvider {
        TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
        TypedLaputaMemoryProvider::open(temp.path(), canonical_workspace_id(temp.path()))
            .await
            .unwrap()
    }

    async fn open_provider_with_budget(
        temp: &tempfile::TempDir,
        l1_index_lines: usize,
    ) -> TypedLaputaMemoryProvider {
        TypedMemoryStore::open_canonical(temp.path()).await.unwrap();
        TypedLaputaMemoryProvider::open_with_l1_budget(
            temp.path(),
            canonical_workspace_id(temp.path()),
            l1_index_lines,
        )
        .await
        .unwrap()
    }

    fn context(temp: &tempfile::TempDir) -> MemoryCrudContext {
        MemoryCrudContext {
            workspace_root: temp.path().to_path_buf(),
        }
    }

    /// D4 — `prefetch` must return a populated prompt block that actually
    /// contains the recalled content (not just a status flag).
    #[tokio::test]
    async fn recall_returns_prompt_block_with_recalled_content() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        provider
            .memory_add(
                &context(&temp),
                MemoryAddRequest {
                    content: "the release summary must mention kestrel-7".into(),
                },
            )
            .await
            .unwrap();

        let response = provider
            .prefetch(PrefetchRequest {
                workspace_root: temp.path().to_path_buf(),
                intent: "release summary".into(),
                current_room: None,
                user_message: Some("what did the release summary say?".into()),
            })
            .await
            .unwrap();

        assert!(
            matches!(response.status, PrefetchStatus::Ready),
            "expected Ready, got {:?}",
            response.status
        );
        let block = response
            .prompt_block
            .expect("prefetch prompt_block expected");
        assert!(
            block.contains("kestrel-7"),
            "recalled block must mention the indexed content, got: {block}"
        );
    }

    /// D4 + H3 — records written via `memory_add` must be visible in the next
    /// startup rendering (the "next session sees the write" contract).
    #[tokio::test]
    async fn memory_add_visible_in_next_startup_rendering() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        provider
            .memory_add(
                &context(&temp),
                MemoryAddRequest {
                    content: "kestrel release plan for Q3".into(),
                },
            )
            .await
            .unwrap();
        drop(provider);

        let fresh = open_provider(&temp).await;
        let response = fresh
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();
        let markdown = response.prompt_block.expect("startup block").markdown;
        assert!(
            markdown.contains("memory-add-"),
            "startup markdown must list the memory-add record id, got: {markdown}"
        );
        assert!(
            markdown.contains("kestrel release plan"),
            "startup L1 index must include the content preview, got: {markdown}"
        );
    }

    /// G3 / U3 — a supersedes tombstone record must be excluded from the
    /// next startup rendering (read-side forget contract). Typed mode
    /// creates tombstones via governed proposal, so we construct the
    /// canonical tombstone shape directly against the store — matching the
    /// Wave 2 session-end tombstone and the `search_excludes_tombstoned_
    /// records` pattern — to assert the read-side projection contract.
    #[tokio::test]
    async fn supersedes_tombstone_filtered_from_next_startup_rendering() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .memory_add(
                &context(&temp),
                MemoryAddRequest {
                    content: "obsolete staging path /tmp/staging-xyz".into(),
                },
            )
            .await
            .unwrap();
        let record_id = match outcome {
            MemoryCrudOutcome::Applied { entry } => entry.expect("entry").id,
            other => panic!("expected Applied, got {other:?}"),
        };

        let now = Utc::now();
        let tombstone = MemoryRecord {
            id: format!("tombstone-{}", now.timestamp_micros()),
            kind: MemoryRecordKind::LongTerm,
            content: String::new(),
            provenance: MemoryProvenance {
                source: MemoryProvenanceSource::AutoDream,
                source_id: "wave3-test".into(),
                content_digest: memory_content_digest(b""),
                captured_at: now,
                correlation: AuditCorrelation {
                    request_id: format!("tombstone-{record_id}"),
                    turn_id: "wave3".into(),
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
                workspace_id: canonical_workspace_id(temp.path()),
                session_id: None,
            },
            created_at: now,
            effective_at: now,
            expires_at: None,
            supersedes: vec![record_id.clone()],
            tombstone: Some(MemoryTombstone {
                target_record_id: record_id.clone(),
                reason_digest: memory_content_digest(b"staging path retired"),
                actor_id: "wave3-test".into(),
                created_at: now,
            }),
        };
        let metadata = provider.crud_store.metadata().await.unwrap();
        provider
            .crud_store
            .put(tombstone, metadata.store_revision, None)
            .await
            .unwrap();
        drop(provider);

        let fresh = open_provider(&temp).await;
        let response = fresh
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();
        let markdown = response.prompt_block.expect("startup block").markdown;
        assert!(
            !markdown.contains(&record_id),
            "startup markdown must not mention the tombstoned record id '{record_id}', got: {markdown}"
        );
        assert!(
            !markdown.contains("staging-xyz"),
            "startup markdown must not leak tombstoned content preview"
        );
    }

    /// G3 / U3 — a supersedes tombstone must also hide its target from
    /// `memory_search`, even when the query text matches the original
    /// record (read-side forget contract extends to recall).
    #[tokio::test]
    async fn supersedes_tombstone_filtered_from_search() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .memory_add(
                &context(&temp),
                MemoryAddRequest {
                    content: "secret project name is kestrel_nine".into(),
                },
            )
            .await
            .unwrap();
        let record_id = match outcome {
            MemoryCrudOutcome::Applied { entry } => entry.expect("entry").id,
            other => panic!("expected Applied, got {other:?}"),
        };

        let now = Utc::now();
        let tombstone = MemoryRecord {
            id: format!("tombstone-{}", now.timestamp_micros()),
            kind: MemoryRecordKind::LongTerm,
            content: String::new(),
            provenance: MemoryProvenance {
                source: MemoryProvenanceSource::AutoDream,
                source_id: "wave3-search-test".into(),
                content_digest: memory_content_digest(b""),
                captured_at: now,
                correlation: AuditCorrelation {
                    request_id: format!("tombstone-{record_id}-search"),
                    turn_id: "wave3".into(),
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
                workspace_id: canonical_workspace_id(temp.path()),
                session_id: None,
            },
            created_at: now,
            effective_at: now,
            expires_at: None,
            supersedes: vec![record_id.clone()],
            tombstone: Some(MemoryTombstone {
                target_record_id: record_id.clone(),
                reason_digest: memory_content_digest(b"retracted"),
                actor_id: "wave3-search-test".into(),
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
                    query: "kestrel_nine".into(),
                    limit: Some(10),
                },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::Listed { entries } = search else {
            panic!("expected Listed, got {search:?}");
        };
        assert!(
            entries.iter().all(|hit| hit.id != record_id),
            "search must not return supersedes-targeted record {record_id}, got {entries:?}"
        );
    }

    /// G3 — `memory_remove` against the typed authority must create a
    /// durable governed proposal (proposal-first contract, not direct
    /// mutation). The subsequent apply step (F3 / GMH-52 desktop
    /// acceptance) finalizes the tombstone; that handoff is out of scope
    /// for this read-side assertion.
    #[tokio::test]
    async fn memory_remove_creates_governed_proposal() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .memory_add(
                &context(&temp),
                MemoryAddRequest {
                    content: "ephemeral note about merlin-cache".into(),
                },
            )
            .await
            .unwrap();
        let record_id = match outcome {
            MemoryCrudOutcome::Applied { entry } => entry.expect("entry").id,
            other => panic!("expected Applied, got {other:?}"),
        };

        let remove_outcome = provider
            .memory_remove(
                &context(&temp),
                MemoryRemoveRequest {
                    record_id: record_id.clone(),
                    reason: "retracted".into(),
                },
            )
            .await
            .unwrap();
        assert!(
            matches!(remove_outcome, MemoryCrudOutcome::ProposalCreated { .. }),
            "typed memory_remove must produce a governed proposal, got {remove_outcome:?}"
        );
    }

    /// B2 — the L1 index must respect the configured line budget; surplus
    /// records fall through to the recall path instead of bloating startup.
    #[tokio::test]
    async fn startup_respects_l1_budget_ceiling() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider_with_budget(&temp, 2).await;
        for i in 0..5 {
            provider
                .memory_add(
                    &context(&temp),
                    MemoryAddRequest {
                        content: format!("budgeted fact number {i} about kestrel"),
                    },
                )
                .await
                .unwrap();
        }
        drop(provider);

        let fresh = open_provider_with_budget(&temp, 2).await;
        let response = fresh
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();
        let markdown = response.prompt_block.expect("startup block").markdown;
        assert_eq!(
            markdown.matches("- [memory-add-").count(),
            2,
            "L1 index must render exactly 2 lines under a budget of 2"
        );
    }

    /// F5 / H3 — `memory_search` after a fresh provider open must hit the
    /// record that was added in a prior provider lifetime (typed FTS
    /// durability contract).
    #[tokio::test]
    async fn apply_then_search_remains_consistent_across_reopens() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        let outcome = provider
            .memory_add(
                &context(&temp),
                MemoryAddRequest {
                    content: "kestrel rollback procedure requires two reviewers".into(),
                },
            )
            .await
            .unwrap();
        let record_id = match outcome {
            MemoryCrudOutcome::Applied { entry } => entry.expect("entry").id,
            other => panic!("expected Applied, got {other:?}"),
        };
        drop(provider);

        let fresh = open_provider(&temp).await;
        let search = fresh
            .memory_search(
                &context(&temp),
                MemorySearchRequest {
                    query: "rollback reviewers".into(),
                    limit: Some(10),
                },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::Listed { entries } = search else {
            panic!("expected Listed, got {search:?}");
        };
        assert!(
            entries.iter().any(|hit| hit.id == record_id),
            "search across a fresh provider must hit record {record_id}, got {entries:?}"
        );
    }

    /// F5 / H3 — working checkpoint (session-scoped) must not leak into the
    /// next session's search results or startup rendering after session end.
    #[tokio::test]
    async fn checkpoint_invisible_to_search_and_subsequent_startup() {
        let temp = tempfile::tempdir().unwrap();
        let provider = open_provider(&temp).await;
        provider
            .checkpoint_write(agent_diva_core::memory::CheckpointWriteRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: "channel:wave3-session".into(),
                key_info: "migrating kestrel cache".into(),
                related_sops: vec![],
                content: "port 8443 confirmed for kestrel".into(),
            })
            .await
            .unwrap();
        provider
            .on_session_end(SessionEndRequest {
                workspace_root: temp.path().to_path_buf(),
                session_id: Some("channel:wave3-session".into()),
            })
            .await
            .unwrap();
        drop(provider);

        let fresh = open_provider(&temp).await;
        let search = fresh
            .memory_search(
                &context(&temp),
                MemorySearchRequest {
                    query: "kestrel port 8443".into(),
                    limit: Some(10),
                },
            )
            .await
            .unwrap();
        let MemoryCrudOutcome::Listed { entries } = search else {
            panic!("expected Listed, got {search:?}");
        };
        assert!(
            entries.is_empty(),
            "post-session-end search must not return checkpoint content, got {entries:?}"
        );
        let startup = fresh
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp.path().to_path_buf(),
            })
            .unwrap();
        let markdown = startup.prompt_block.expect("startup block").markdown;
        assert!(
            !markdown.contains("port 8443"),
            "startup must not leak cleared checkpoint content"
        );
    }
}
