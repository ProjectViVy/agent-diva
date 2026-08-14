//! Memory management for long-term storage.
//!
//! Handles loading and updating of `MEMORY.md` and `HISTORY.md`.

pub mod actmem;
pub mod crud;
pub mod manager;
pub mod provider;
pub mod recall;
pub mod record;
pub mod storage;
pub mod working;

pub use actmem::{
    ActmemEditWorkRequest, ActmemItemRequest, ActmemMutationResponse, ActmemReadRequest,
    ActmemReadResponse, ActmemReadTarget, MemoryRulesResponse,
};
pub use crud::{
    MemoryAddRequest, MemoryCrudContext, MemoryCrudOutcome, MemoryDistillRequest, MemoryEntry,
    MemoryGetRequest, MemoryListRequest, MemoryRemoveRequest, MemorySearchRequest,
    MemoryUpdateRequest, SectionWriteProposalRequest,
};
pub use manager::MemoryManager;
pub use provider::{
    MemoryProvider, PrefetchRequest, PrefetchResponse, PrefetchStatus, RecallOutcomeRequest,
    RecallTurnOutcome, SessionEndRequest, SessionEndResponse, SessionEndStatus,
    StartupContextSnapshot, StartupInjectionShape, StartupStatus, SyncTurnRequest,
    SyncTurnResponse, SyncTurnStatus, SystemPromptBlock, SystemPromptRefreshRequest,
    SystemPromptRefreshResponse, SystemPromptRequest, SystemPromptResponse, WakeupPackSummary,
};
pub use recall::{
    compare_recall_shadow, estimate_recall_tokens, ConservativeRecallTokenEstimator,
    RecallBudgetUsage, RecallCandidate, RecallCandidateSource, RecallDecision, RecallFailure,
    RecallOutcome, RecallPipeline, RecallPolicy, RecallRequest, RecallRetrievalSource,
    RecallSelectionReason, RecallShadowReport, RecallSourceError, RecallStatus,
    RecallTokenEstimator, RecallTrace, RecallValidationError,
};
pub use record::{
    escape_memory_for_prompt, memory_content_digest, render_l1_index_block, render_l1_index_line,
    MemoryIntegrityFinding, MemoryIntegrityReport, MemoryIntegritySeverity, MemoryProvenance,
    MemoryProvenanceSource, MemoryRecord, MemoryRecordKind, MemoryRecordValidationError,
    MemoryScope, MemorySensitivity, MemoryTombstone, MemoryTrust, DEFAULT_L1_INDEX_LINES,
    MAX_CONFIDENCE_BPS,
};
pub use storage::{DailyNote, Memory};
pub use working::{
    render_session_checkpoint_block, SessionCheckpointRequest, SessionCheckpointResponse,
    SessionCheckpointWriteRequest, L0_MEMORY_POLICY,
};
