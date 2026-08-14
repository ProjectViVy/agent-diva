//! Machine-wide BML authority shared by Manager, Agent, and AutoDream.

use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use agent_diva_core::{
    governance::AuditCorrelation,
    memory::{
        escape_memory_for_prompt, memory_content_digest, render_l1_index_block,
        render_session_checkpoint_block, ActmemEditWorkRequest, ActmemItemRequest,
        ActmemMutationResponse, ActmemReadRequest, ActmemReadResponse, ActmemReadTarget,
        MemoryAddRequest, MemoryCrudContext, MemoryCrudOutcome, MemoryEntry, MemoryGetRequest,
        MemoryListRequest, MemoryProvenance, MemoryProvenanceSource, MemoryProvider, MemoryRecord,
        MemoryRecordKind, MemoryRemoveRequest, MemoryRulesResponse, MemoryScope,
        MemorySearchRequest, MemorySensitivity, MemoryTombstone, MemoryTrust, MemoryUpdateRequest,
        PrefetchRequest, PrefetchResponse, PrefetchStatus, RecallOutcomeRequest,
        SessionCheckpointRequest, SessionCheckpointResponse, SessionCheckpointWriteRequest,
        SessionEndRequest, SessionEndResponse, SessionEndStatus, StartupInjectionShape,
        SyncTurnRequest, SyncTurnResponse, SyncTurnStatus, SystemPromptBlock,
        SystemPromptRefreshRequest, SystemPromptRefreshResponse, SystemPromptRequest,
        SystemPromptResponse, DEFAULT_L1_INDEX_LINES, MAX_CONFIDENCE_BPS,
    },
};
use chrono::{Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    actmem::ActmemStore, atomic_write, cognitive::MemRules, StoredMemoryRecord, TypedMemoryStore,
    TypedMemoryStoreError,
};

const MACHINE_MEMORY_SCOPE: &str = "machine-memory-home";
const MEMORY_DATABASE_FILE: &str = "memory.sqlite3";
const MEMRULES_FILE: &str = "MEMRULES.MD";

#[derive(Debug, thiserror::Error)]
pub enum MemoryHomeError {
    #[error("BML is unavailable: {0}")]
    BmlUnavailable(#[from] TypedMemoryStoreError),
    #[error("memory record not found: {0}")]
    NotFound(String),
    #[error("memory revision conflict: expected {expected}, actual {actual:?}")]
    RevisionConflict { expected: i64, actual: Option<i64> },
    #[error("memory kind is forbidden in production BML")]
    KindForbidden,
    #[error("invalid memory request: {0}")]
    Invalid(String),
    #[error("Memory Home I/O failed at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl MemoryHomeError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::BmlUnavailable(_) => "bml_unavailable",
            Self::RevisionConflict { .. } => "memory_revision_conflict",
            Self::KindForbidden => "memory_kind_forbidden",
            Self::NotFound(_) => "memory_not_found",
            Self::Invalid(_) => "memory_invalid_request",
            Self::Io { .. } => "memory_io_error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemRulesDocument {
    pub content: String,
    pub source: MemRulesSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemRulesSource {
    Default,
    File,
}

#[derive(Clone)]
pub struct MemoryHome {
    inner: Arc<MemoryHomeInner>,
}

struct MemoryHomeInner {
    config_dir: PathBuf,
    memory_dir: PathBuf,
    database: PathBuf,
    memrules: PathBuf,
    actmem: ActmemStore,
    store: tokio::sync::Mutex<Option<TypedMemoryStore>>,
    startup_markdown: RwLock<Option<String>>,
    startup_revision: std::sync::atomic::AtomicU64,
    l1_index_lines: usize,
}

impl MemoryHome {
    pub fn new(config_dir: impl Into<PathBuf>) -> Self {
        Self::with_l1_budget(config_dir, DEFAULT_L1_INDEX_LINES)
    }

    pub fn with_l1_budget(config_dir: impl Into<PathBuf>, l1_index_lines: usize) -> Self {
        let config_dir = config_dir.into();
        let memory_dir = config_dir.join("memory");
        let database = memory_dir.join(MEMORY_DATABASE_FILE);
        let memrules = memory_dir.join(MEMRULES_FILE);
        Self {
            inner: Arc::new(MemoryHomeInner {
                actmem: ActmemStore::new(&config_dir),
                config_dir,
                memory_dir,
                database,
                memrules,
                store: tokio::sync::Mutex::new(None),
                startup_markdown: RwLock::new(Some(memrules_pointer())),
                startup_revision: std::sync::atomic::AtomicU64::new(0),
                l1_index_lines,
            }),
        }
    }

    pub fn config_dir(&self) -> &Path {
        &self.inner.config_dir
    }

    pub fn memory_dir(&self) -> &Path {
        &self.inner.memory_dir
    }

    pub fn database_path(&self) -> &Path {
        &self.inner.database
    }

    pub fn memrules_path(&self) -> &Path {
        &self.inner.memrules
    }

    pub fn actmem(&self) -> &ActmemStore {
        &self.inner.actmem
    }

    /// Warm the read-side index only when the machine-home database exists.
    /// A missing database remains absent and projects an empty index.
    pub async fn warmup(&self) -> Result<(), MemoryHomeError> {
        self.refresh_startup_index().await
    }

    async fn existing_store(&self) -> Result<Option<TypedMemoryStore>, MemoryHomeError> {
        let mut guard = self.inner.store.lock().await;
        if let Some(store) = guard.as_ref() {
            return Ok(Some(store.clone()));
        }
        if !self.inner.database.is_file() {
            return Ok(None);
        }
        let store =
            TypedMemoryStore::open_existing_database(&self.inner.database, MACHINE_MEMORY_SCOPE)
                .await?;
        *guard = Some(store.clone());
        Ok(Some(store))
    }

    async fn writable_store(&self) -> Result<TypedMemoryStore, MemoryHomeError> {
        let mut guard = self.inner.store.lock().await;
        if let Some(store) = guard.as_ref() {
            return Ok(store.clone());
        }
        let store =
            TypedMemoryStore::open_database(&self.inner.database, MACHINE_MEMORY_SCOPE).await?;
        *guard = Some(store.clone());
        Ok(store)
    }

    pub async fn list_records(&self, limit: u32) -> Result<Vec<MemoryEntry>, MemoryHomeError> {
        let Some(store) = self.existing_store().await? else {
            return Ok(Vec::new());
        };
        let superseded = store.superseded_target_ids().await?;
        Ok(store
            .list(limit)
            .await?
            .into_iter()
            .filter(|stored| visible_long_term(&stored.record, &superseded))
            .map(entry_from_stored)
            .collect())
    }

    pub async fn get_record(&self, id: &str) -> Result<Option<MemoryEntry>, MemoryHomeError> {
        let Some(store) = self.existing_store().await? else {
            return Ok(None);
        };
        let superseded = store.superseded_target_ids().await?;
        Ok(store
            .get(id)
            .await?
            .filter(|stored| visible_long_term(&stored.record, &superseded))
            .map(entry_from_stored))
    }

    pub async fn add_long_term(
        &self,
        content: String,
        evidence_refs: Vec<agent_diva_core::evolution::EvidenceRef>,
    ) -> Result<MemoryEntry, MemoryHomeError> {
        self.add_record(MemoryRecordKind::LongTerm, content, evidence_refs)
            .await
    }

    pub async fn add_record(
        &self,
        kind: MemoryRecordKind,
        content: String,
        evidence_refs: Vec<agent_diva_core::evolution::EvidenceRef>,
    ) -> Result<MemoryEntry, MemoryHomeError> {
        if kind != MemoryRecordKind::LongTerm {
            return Err(MemoryHomeError::KindForbidden);
        }
        let content = content.trim().to_string();
        if content.is_empty() {
            return Err(MemoryHomeError::Invalid("content is empty".into()));
        }
        let store = self.writable_store().await?;
        let now = Utc::now();
        let digest = memory_content_digest(content.as_bytes());
        let id = format!("memory-{}-{}", now.timestamp_micros(), &digest.value[..12]);
        let record = long_term_record(id, content, evidence_refs, now);
        let metadata = store.metadata().await?;
        let stored = store.put(record, metadata.store_revision, None).await?;
        self.refresh_startup_index().await?;
        Ok(entry_from_stored(stored))
    }

    pub async fn update_record(
        &self,
        id: &str,
        content: String,
        base_revision: i64,
        evidence_refs: Vec<agent_diva_core::evolution::EvidenceRef>,
    ) -> Result<MemoryEntry, MemoryHomeError> {
        let content = content.trim().to_string();
        if content.is_empty() {
            return Err(MemoryHomeError::Invalid("content is empty".into()));
        }
        let Some(store) = self.existing_store().await? else {
            return Err(MemoryHomeError::NotFound(id.to_string()));
        };
        let current = store
            .get(id)
            .await?
            .ok_or_else(|| MemoryHomeError::NotFound(id.to_string()))?;
        if current.record.kind != MemoryRecordKind::LongTerm || current.record.tombstone.is_some() {
            return Err(MemoryHomeError::NotFound(id.to_string()));
        }
        if current.revision != base_revision {
            return Err(MemoryHomeError::RevisionConflict {
                expected: base_revision,
                actual: Some(current.revision),
            });
        }
        let now = Utc::now();
        let mut record = current.record;
        record.content = content;
        record.evidence_refs = evidence_refs;
        record.effective_at = now;
        record.provenance.source = MemoryProvenanceSource::UserInput;
        record.provenance.source_id = "memory_update".into();
        record.provenance.content_digest = memory_content_digest(record.content.as_bytes());
        record.provenance.captured_at = now;
        record.provenance.correlation = correlation("memory_update", id);
        let metadata = store.metadata().await?;
        let stored = store
            .put(record, metadata.store_revision, Some(base_revision))
            .await
            .map_err(map_revision_error)?;
        self.refresh_startup_index().await?;
        Ok(entry_from_stored(stored))
    }

    pub async fn remove_record(
        &self,
        id: &str,
        reason: String,
        base_revision: i64,
    ) -> Result<MemoryEntry, MemoryHomeError> {
        let reason = reason.trim().to_string();
        if reason.is_empty() {
            return Err(MemoryHomeError::Invalid("reason is empty".into()));
        }
        let Some(store) = self.existing_store().await? else {
            return Err(MemoryHomeError::NotFound(id.to_string()));
        };
        let current = store
            .get(id)
            .await?
            .ok_or_else(|| MemoryHomeError::NotFound(id.to_string()))?;
        if current.record.kind != MemoryRecordKind::LongTerm || current.record.tombstone.is_some() {
            return Err(MemoryHomeError::NotFound(id.to_string()));
        }
        if current.revision != base_revision {
            return Err(MemoryHomeError::RevisionConflict {
                expected: base_revision,
                actual: Some(current.revision),
            });
        }
        let now = Utc::now();
        let tombstone_id = format!("memory-tombstone-{}", now.timestamp_micros());
        let tombstone = MemoryRecord {
            id: tombstone_id.clone(),
            kind: MemoryRecordKind::LongTerm,
            content: String::new(),
            provenance: MemoryProvenance {
                source: MemoryProvenanceSource::UserInput,
                source_id: "memory_remove".into(),
                content_digest: memory_content_digest(b""),
                captured_at: now,
                correlation: correlation("memory_remove", id),
            },
            evidence_refs: Vec::new(),
            confidence_bps: MAX_CONFIDENCE_BPS,
            sensitivity: MemorySensitivity::Internal,
            trust: MemoryTrust::UserAsserted,
            scope: machine_scope(None),
            created_at: now,
            effective_at: now,
            expires_at: None,
            supersedes: vec![id.to_string()],
            tombstone: Some(MemoryTombstone {
                target_record_id: id.to_string(),
                reason_digest: memory_content_digest(reason.as_bytes()),
                actor_id: "user".into(),
                created_at: now,
            }),
        };
        let metadata = store.metadata().await?;
        store
            .put_tombstone(tombstone, metadata.store_revision, id, base_revision)
            .await
            .map_err(map_revision_error)?;
        self.refresh_startup_index().await?;
        Ok(entry_from_stored(current))
    }

    pub fn read_memrules(&self) -> Result<MemRulesDocument, MemoryHomeError> {
        let rules = MemRules::load_or_default(&self.inner.memrules)
            .map_err(|error| MemoryHomeError::Invalid(error.to_string()))?;
        Ok(MemRulesDocument {
            content: rules.raw,
            source: if rules.path.is_some() {
                MemRulesSource::File
            } else {
                MemRulesSource::Default
            },
        })
    }

    pub fn write_memrules(&self, content: &str) -> Result<MemRulesDocument, MemoryHomeError> {
        let content = content.replace("\r\n", "\n");
        if content.trim().is_empty() {
            return Err(MemoryHomeError::Invalid("MEMRULES content is empty".into()));
        }
        atomic_write(&self.inner.memrules, content.as_bytes())
            .map_err(|error| MemoryHomeError::Invalid(error.to_string()))?;
        Ok(MemRulesDocument {
            content,
            source: MemRulesSource::File,
        })
    }

    pub async fn clear_session_checkpoint(&self, session_id: &str) -> Result<u64, MemoryHomeError> {
        let Some(store) = self.existing_store().await? else {
            return Ok(0);
        };
        Ok(store.gc_session_scoped(session_id).await?)
    }

    pub async fn run_startup_gc(
        &self,
        active_session_ids: &[&str],
    ) -> Result<u64, MemoryHomeError> {
        if active_session_ids.is_empty() {
            return Ok(0);
        }
        let Some(store) = self.existing_store().await? else {
            return Ok(0);
        };
        Ok(store.gc_stale_session_scoped(active_session_ids).await?)
    }

    async fn refresh_startup_index(&self) -> Result<(), MemoryHomeError> {
        let entries = self.list_records(self.inner.l1_index_lines as u32).await?;
        let pairs = entries
            .into_iter()
            .map(|entry| (entry.id, entry.content))
            .collect::<Vec<_>>();
        let index = render_l1_index_block(&pairs, self.inner.l1_index_lines);
        let rendered = if index.is_empty() {
            memrules_pointer()
        } else {
            format!("{}\n\n{}", memrules_pointer(), index.trim())
        };
        let mut guard = self
            .inner
            .startup_markdown
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if guard.as_deref() != Some(rendered.as_str()) {
            *guard = Some(rendered);
            self.inner
                .startup_revision
                .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        }
        Ok(())
    }

    async fn search_records(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<StoredMemoryRecord>, MemoryHomeError> {
        let Some(store) = self.existing_store().await? else {
            return Ok(Vec::new());
        };
        let superseded = store.superseded_target_ids().await?;
        let hits = store
            .search_visible(query, &machine_scope(None), limit)
            .await?;
        Ok(hits
            .into_iter()
            .map(|hit| hit.stored)
            .filter(|stored| visible_long_term(&stored.record, &superseded))
            .collect())
    }

    async fn write_checkpoint(
        &self,
        request: SessionCheckpointWriteRequest,
    ) -> Result<MemoryEntry, MemoryHomeError> {
        if request.session_id.trim().is_empty() {
            return Err(MemoryHomeError::Invalid("session_id is empty".into()));
        }
        let store = self.writable_store().await?;
        let id = checkpoint_id(&request.session_id);
        let existing = store.get(&id).await?;
        let now = Utc::now();
        let content = render_session_checkpoint_block(
            &request.key_info,
            &request.related_sops,
            &request.content,
        );
        let record = MemoryRecord {
            id: id.clone(),
            kind: MemoryRecordKind::SessionCheckpoint,
            content: content.clone(),
            provenance: MemoryProvenance {
                source: MemoryProvenanceSource::SessionSync,
                source_id: "session_checkpoint".into(),
                content_digest: memory_content_digest(content.as_bytes()),
                captured_at: now,
                correlation: correlation("session_checkpoint", &request.session_id),
            },
            evidence_refs: Vec::new(),
            confidence_bps: MAX_CONFIDENCE_BPS,
            sensitivity: MemorySensitivity::Internal,
            trust: MemoryTrust::Observed,
            scope: machine_scope(Some(request.session_id)),
            created_at: existing
                .as_ref()
                .map_or(now, |stored| stored.record.created_at),
            effective_at: now,
            expires_at: None,
            supersedes: Vec::new(),
            tombstone: None,
        };
        record
            .validate_at(now, Duration::minutes(5))
            .map_err(|error| MemoryHomeError::Invalid(error.to_string()))?;
        let metadata = store.metadata().await?;
        let stored = store
            .put(
                record,
                metadata.store_revision,
                existing.as_ref().map(|stored| stored.revision),
            )
            .await?;
        Ok(entry_from_stored(stored))
    }
}

#[async_trait::async_trait]
impl MemoryProvider for MemoryHome {
    fn system_prompt_block(
        &self,
        _request: &SystemPromptRequest,
    ) -> agent_diva_core::Result<SystemPromptResponse> {
        let markdown = self
            .inner
            .startup_markdown
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
            .unwrap_or_else(memrules_pointer);
        Ok(SystemPromptResponse::ready(SystemPromptBlock {
            shape: StartupInjectionShape::CompactRenderedMarkdown,
            markdown,
        }))
    }

    fn system_prompt_revision(&self, _request: &SystemPromptRequest) -> u64 {
        self.inner
            .startup_revision
            .load(std::sync::atomic::Ordering::Acquire)
    }

    async fn refresh_system_prompt_projection(
        &self,
        _request: SystemPromptRefreshRequest,
    ) -> agent_diva_core::Result<SystemPromptRefreshResponse> {
        self.refresh_startup_index()
            .await
            .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))?;
        Ok(SystemPromptRefreshResponse {
            authority_revision: self.system_prompt_revision(&SystemPromptRequest {
                workspace_root: self.inner.config_dir.clone(),
            }),
            projection_changed: true,
        })
    }

    async fn prefetch(
        &self,
        request: PrefetchRequest,
    ) -> agent_diva_core::Result<PrefetchResponse> {
        let query = request
            .user_message
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&request.intent);
        if query.trim().is_empty() {
            return Ok(PrefetchResponse::default());
        }
        match self.search_records(query, 8).await {
            Ok(records) => {
                let block = records
                    .iter()
                    .map(|stored| escape_memory_for_prompt(&stored.record))
                    .collect::<Vec<_>>()
                    .join("\n\n");
                Ok(PrefetchResponse {
                    status: PrefetchStatus::Ready,
                    prompt_block: (!block.is_empty()).then_some(block),
                })
            }
            Err(error) => Ok(PrefetchResponse {
                status: PrefetchStatus::Failed {
                    reason: format!("bml_unavailable:{error}"),
                },
                prompt_block: None,
            }),
        }
    }

    async fn sync_turn(
        &self,
        _request: SyncTurnRequest,
    ) -> agent_diva_core::Result<SyncTurnResponse> {
        Ok(SyncTurnResponse {
            status: SyncTurnStatus::Noop,
        })
    }

    async fn record_recall_outcome(
        &self,
        _request: RecallOutcomeRequest,
    ) -> agent_diva_core::Result<()> {
        Ok(())
    }

    async fn memory_add(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryAddRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        Ok(
            match self
                .add_long_term(request.content, request.evidence_refs.clone())
                .await
            {
                Ok(entry) => MemoryCrudOutcome::Applied {
                    entry: Some(entry),
                    evidence_advisory: request
                        .evidence_refs
                        .is_empty()
                        .then(|| "no evidence_refs: stored without tool verification".into()),
                },
                Err(error) => MemoryCrudOutcome::Failed {
                    reason: format!("{}:{error}", error.code()),
                },
            },
        )
    }

    async fn memory_list(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryListRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        Ok(
            match self.list_records(request.limit.unwrap_or(100)).await {
                Ok(entries) => MemoryCrudOutcome::Listed { entries },
                Err(error) => MemoryCrudOutcome::Failed {
                    reason: format!("{}:{error}", error.code()),
                },
            },
        )
    }

    async fn memory_get(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryGetRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        Ok(match self.get_record(&request.record_id).await {
            Ok(Some(entry)) => MemoryCrudOutcome::Listed {
                entries: vec![entry],
            },
            Ok(None) => MemoryCrudOutcome::Failed {
                reason: "memory_not_found".into(),
            },
            Err(error) => MemoryCrudOutcome::Failed {
                reason: format!("{}:{error}", error.code()),
            },
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
        Ok(
            match self
                .search_records(&request.query, request.limit.unwrap_or(20))
                .await
            {
                Ok(entries) => MemoryCrudOutcome::Listed {
                    entries: entries.into_iter().map(entry_from_stored).collect(),
                },
                Err(error) => MemoryCrudOutcome::Failed {
                    reason: format!("{}:{error}", error.code()),
                },
            },
        )
    }

    async fn memory_update(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryUpdateRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        Ok(
            match self
                .update_record(
                    &request.record_id,
                    request.content,
                    request.base_revision,
                    request.evidence_refs,
                )
                .await
            {
                Ok(entry) => MemoryCrudOutcome::Applied {
                    entry: Some(entry),
                    evidence_advisory: None,
                },
                Err(error) => MemoryCrudOutcome::Failed {
                    reason: format!("{}:{error}", error.code()),
                },
            },
        )
    }

    async fn memory_remove(
        &self,
        _context: &MemoryCrudContext,
        request: MemoryRemoveRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        Ok(
            match self
                .remove_record(&request.record_id, request.reason, request.base_revision)
                .await
            {
                Ok(entry) => MemoryCrudOutcome::Applied {
                    entry: Some(entry),
                    evidence_advisory: None,
                },
                Err(error) => MemoryCrudOutcome::Failed {
                    reason: format!("{}:{error}", error.code()),
                },
            },
        )
    }

    async fn actmem_read(
        &self,
        request: ActmemReadRequest,
    ) -> agent_diva_core::Result<ActmemReadResponse> {
        let head = self
            .actmem()
            .read()
            .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))?;
        let content = match request.target {
            ActmemReadTarget::Pulse => head.pulse,
            ActmemReadTarget::Recap => head.recap,
            ActmemReadTarget::Work => head.work,
            ActmemReadTarget::Head => head.markdown,
            ActmemReadTarget::Capsules => serde_json::to_string(
                &self
                    .actmem()
                    .list_capsules()
                    .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))?,
            )?,
            ActmemReadTarget::Capsule => {
                let name = request.capsule_name.ok_or_else(|| {
                    agent_diva_core::Error::Validation("capsule_name is required".into())
                })?;
                self.actmem()
                    .read_capsule(&name)
                    .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))?
                    .markdown
            }
        };
        Ok(ActmemReadResponse {
            revision: head.revision,
            content: truncate_text(&content, crate::actmem::ACTMEM_READ_CAP_CHARS),
        })
    }

    async fn actmem_edit_work(
        &self,
        request: ActmemEditWorkRequest,
    ) -> agent_diva_core::Result<ActmemMutationResponse> {
        let document = self
            .actmem()
            .edit_work(
                &request.section,
                &request.replacement,
                request.base_revision,
            )
            .await
            .map_err(|error| {
                agent_diva_core::Error::Validation(format!("{}:{error}", error.code()))
            })?;
        Ok(actmem_mutation(document))
    }

    async fn actmem_complete(
        &self,
        request: ActmemItemRequest,
    ) -> agent_diva_core::Result<ActmemMutationResponse> {
        if !request.section.eq_ignore_ascii_case("open") {
            return Err(agent_diva_core::Error::Validation(
                "actmem_complete only accepts section Open".into(),
            ));
        }
        let document = self
            .actmem()
            .complete_open_item(request.item_index, request.base_revision)
            .await
            .map_err(|error| {
                agent_diva_core::Error::Validation(format!("{}:{error}", error.code()))
            })?;
        Ok(actmem_mutation(document))
    }

    async fn actmem_drop(
        &self,
        request: ActmemItemRequest,
    ) -> agent_diva_core::Result<ActmemMutationResponse> {
        let document = self
            .actmem()
            .drop_item(&request.section, request.item_index, request.base_revision)
            .await
            .map_err(|error| {
                agent_diva_core::Error::Validation(format!("{}:{error}", error.code()))
            })?;
        Ok(actmem_mutation(document))
    }

    async fn memory_rules(&self) -> agent_diva_core::Result<MemoryRulesResponse> {
        let rules = self
            .read_memrules()
            .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))?;
        Ok(MemoryRulesResponse {
            content: rules.content,
            source: match rules.source {
                MemRulesSource::Default => "default",
                MemRulesSource::File => "file",
            }
            .into(),
        })
    }

    async fn record_user_pulse(
        &self,
        session_id: &str,
        content: &str,
    ) -> agent_diva_core::Result<()> {
        self.actmem()
            .append_pulse(session_id, content)
            .await
            .map(|_| ())
            .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))
    }

    async fn record_assistant_recap(
        &self,
        session_id: &str,
        content: &str,
    ) -> agent_diva_core::Result<()> {
        let recap = crate::actmem::recap_from_final_response(content);
        self.actmem()
            .append_recap(session_id, &recap)
            .await
            .map(|_| ())
            .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))
    }

    async fn fold_actmem_session(&self, session_id: &str) -> agent_diva_core::Result<()> {
        self.actmem()
            .fold_session(session_id)
            .await
            .map(|_| ())
            .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))
    }

    async fn session_checkpoint_block(
        &self,
        request: SessionCheckpointRequest,
    ) -> agent_diva_core::Result<SessionCheckpointResponse> {
        let Some(store) = self
            .existing_store()
            .await
            .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))?
        else {
            return Ok(SessionCheckpointResponse::default());
        };
        let id = checkpoint_id(&request.session_id);
        let prompt_block = store
            .get(&id)
            .await
            .map_err(|error| agent_diva_core::Error::Internal(error.to_string()))?
            .filter(|stored| {
                stored.record.kind == MemoryRecordKind::SessionCheckpoint
                    && stored.record.scope.session_id.as_deref() == Some(&request.session_id)
                    && stored.record.tombstone.is_none()
            })
            .map(|stored| stored.record.content);
        Ok(SessionCheckpointResponse { prompt_block })
    }

    async fn session_checkpoint_write(
        &self,
        request: SessionCheckpointWriteRequest,
    ) -> agent_diva_core::Result<MemoryCrudOutcome> {
        Ok(match self.write_checkpoint(request).await {
            Ok(entry) => MemoryCrudOutcome::Applied {
                entry: Some(entry),
                evidence_advisory: None,
            },
            Err(error) => MemoryCrudOutcome::Failed {
                reason: format!("{}:{error}", error.code()),
            },
        })
    }

    async fn on_session_end(
        &self,
        request: SessionEndRequest,
    ) -> agent_diva_core::Result<SessionEndResponse> {
        let Some(session_id) = request.session_id else {
            return Ok(SessionEndResponse::default());
        };
        match self.clear_session_checkpoint(&session_id).await {
            Ok(0) => Ok(SessionEndResponse {
                status: SessionEndStatus::Noop,
            }),
            Ok(_) => Ok(SessionEndResponse {
                status: SessionEndStatus::Triggered,
            }),
            Err(error) => Ok(SessionEndResponse {
                status: SessionEndStatus::Failed {
                    reason: error.to_string(),
                },
            }),
        }
    }
}

fn long_term_record(
    id: String,
    content: String,
    evidence_refs: Vec<agent_diva_core::evolution::EvidenceRef>,
    now: chrono::DateTime<Utc>,
) -> MemoryRecord {
    MemoryRecord {
        id: id.clone(),
        kind: MemoryRecordKind::LongTerm,
        provenance: MemoryProvenance {
            source: MemoryProvenanceSource::UserInput,
            source_id: "memory_add".into(),
            content_digest: memory_content_digest(content.as_bytes()),
            captured_at: now,
            correlation: correlation("memory_add", &id),
        },
        content,
        evidence_refs,
        confidence_bps: MAX_CONFIDENCE_BPS,
        sensitivity: MemorySensitivity::Internal,
        trust: MemoryTrust::UserAsserted,
        scope: machine_scope(None),
        created_at: now,
        effective_at: now,
        expires_at: None,
        supersedes: Vec::new(),
        tombstone: None,
    }
}

fn machine_scope(session_id: Option<String>) -> MemoryScope {
    MemoryScope {
        tenant_id: "local".into(),
        workspace_id: MACHINE_MEMORY_SCOPE.into(),
        session_id,
    }
}

fn correlation(operation: &str, key: &str) -> AuditCorrelation {
    AuditCorrelation {
        request_id: format!("{operation}-{key}"),
        turn_id: operation.into(),
        session_id: key.into(),
        trace_id: None,
    }
}

fn checkpoint_id(session_id: &str) -> String {
    let digest = memory_content_digest(session_id.as_bytes()).value;
    format!("session-checkpoint-{}", &digest[..16])
}

fn visible_long_term(record: &MemoryRecord, superseded: &HashSet<String>) -> bool {
    record.kind == MemoryRecordKind::LongTerm
        && record.tombstone.is_none()
        && record.scope.session_id.is_none()
        && !superseded.contains(&record.id)
}

fn entry_from_stored(stored: StoredMemoryRecord) -> MemoryEntry {
    MemoryEntry {
        id: stored.record.id,
        content: stored.record.content,
        trust: serde_json::to_value(&stored.record.trust)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_else(|| "unknown".into()),
        provenance: serde_json::to_value(&stored.record.provenance.source)
            .ok()
            .and_then(|value| value.as_str().map(str::to_string)),
        evidence_refs: stored.record.evidence_refs,
        revision: stored.revision,
        created_at: stored.record.created_at.to_rfc3339(),
        updated_at: stored.record.effective_at.to_rfc3339(),
    }
}

fn map_revision_error(error: TypedMemoryStoreError) -> MemoryHomeError {
    match error {
        TypedMemoryStoreError::RecordRevisionConflict {
            expected, actual, ..
        } => MemoryHomeError::RevisionConflict {
            expected: expected.unwrap_or_default(),
            actual,
        },
        other => MemoryHomeError::BmlUnavailable(other),
    }
}

fn memrules_pointer() -> String {
    "## Memory Policy Pointer\n\nMemory writes must consult the machine-wide MEMRULES handbook. Full rules are injected only before a Memory or ACTMEM write tool executes.".into()
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    let mut text = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars && max_chars > 0 {
        text.pop();
        text.push('…');
    }
    text
}

fn actmem_mutation(document: crate::actmem::ActmemDocument) -> ActmemMutationResponse {
    ActmemMutationResponse {
        revision: document.revision,
        updated_at: document.updated_at.to_rfc3339(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn paths_are_machine_home_and_database_is_lazy() {
        let temp = tempfile::tempdir().unwrap();
        let legacy = temp.path().join("workspace/.laputa/memory.sqlite3");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, b"legacy must be ignored").unwrap();
        let home = MemoryHome::new(temp.path().join("config"));
        assert_eq!(
            home.database_path(),
            temp.path().join("config/memory/memory.sqlite3")
        );
        assert!(home.list_records(20).await.unwrap().is_empty());
        assert!(!home.database_path().exists());
        home.add_long_term("remember this".into(), Vec::new())
            .await
            .unwrap();
        assert!(home.database_path().is_file());
    }

    #[tokio::test]
    async fn long_term_crud_is_direct_revisioned_and_tombstoned() {
        let temp = tempfile::tempdir().unwrap();
        let home = MemoryHome::new(temp.path());
        let added = home
            .add_long_term("first".into(), Vec::new())
            .await
            .unwrap();
        assert_eq!(added.revision, 1);
        let updated = home
            .update_record(&added.id, "second".into(), added.revision, Vec::new())
            .await
            .unwrap();
        assert_eq!(updated.revision, 2);
        let conflict = home
            .update_record(&added.id, "stale".into(), 1, Vec::new())
            .await
            .unwrap_err();
        assert_eq!(conflict.code(), "memory_revision_conflict");
        home.remove_record(&added.id, "done".into(), updated.revision)
            .await
            .unwrap();
        assert!(home.get_record(&added.id).await.unwrap().is_none());
        assert!(home.list_records(20).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn persona_kinds_are_forbidden() {
        let temp = tempfile::tempdir().unwrap();
        let home = MemoryHome::new(temp.path());
        let error = home
            .add_record(MemoryRecordKind::Identity, "persona".into(), Vec::new())
            .await
            .unwrap_err();
        assert_eq!(error.code(), "memory_kind_forbidden");
        assert!(!home.database_path().exists());
    }

    #[test]
    fn memrules_default_is_read_only_until_user_saves() {
        let temp = tempfile::tempdir().unwrap();
        let home = MemoryHome::new(temp.path());
        let rules = home.read_memrules().unwrap();
        assert_eq!(rules.source, MemRulesSource::Default);
        assert!(!home.memrules_path().exists());
        home.write_memrules("# User rules\n").unwrap();
        assert_eq!(home.read_memrules().unwrap().source, MemRulesSource::File);
    }

    #[tokio::test]
    async fn startup_gc_keeps_active_checkpoints_and_removes_stale_ones() {
        let temp = tempfile::tempdir().unwrap();
        let home = MemoryHome::new(temp.path());
        for session_id in ["gui:active", "gui:stale"] {
            home.write_checkpoint(SessionCheckpointWriteRequest {
                workspace_root: temp.path().into(),
                session_id: session_id.into(),
                key_info: "state".into(),
                related_sops: Vec::new(),
                content: String::new(),
            })
            .await
            .unwrap();
        }

        assert_eq!(home.run_startup_gc(&[]).await.unwrap(), 0);
        assert_eq!(home.run_startup_gc(&["gui:active"]).await.unwrap(), 1);
        let store = home.existing_store().await.unwrap().unwrap();
        assert!(store
            .get(&checkpoint_id("gui:active"))
            .await
            .unwrap()
            .is_some());
        assert!(store
            .get(&checkpoint_id("gui:stale"))
            .await
            .unwrap()
            .is_none());
    }
}
