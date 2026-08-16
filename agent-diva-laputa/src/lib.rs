//! File-first authority storage boundary for Laputa.
//!
//! The cognitive clean break (S5) removed the legacy section/proposal
//! governance kernel. This crate now owns the surviving authorities:
//! the BML logical layer (machine-wide memory home, ACTMEM, MEMRULES),
//! the persona Markdown workspace, the persona Frozen Core snapshot, and
//! the payload-free recall-feedback reader. See the `bml` module docs for
//! the memory write-API boundary rules.

pub mod actmem;
pub mod atomic;
pub mod bml;
pub mod cognitive;
pub mod error;
pub mod feedback;
pub mod frozen_core;
pub mod layout;
pub mod lock;
pub mod memory_records;
pub mod persona;
pub mod typed_store;

pub use actmem::{
    recap_from_final_response, ActmemDocument, ActmemError, ActmemPatch, ActmemStore,
    CapsuleDocument, CapsuleSummary, ACTMEM_CAPSULE_CAP_CHARS, ACTMEM_READ_CAP_CHARS,
    ACTMEM_RING_CAP_CHARS, ACTMEM_WORK_CAP_CHARS,
};
pub use atomic::{atomic_write, atomic_write_json};
pub use bml::{
    adapt_laputa_section, adapt_legacy_markdown, compare_normalized_records, MemRulesDocument,
    MemRulesSource, MemoryAdapterContext, MemoryHome, MemoryHomeError, MemoryRecordMigration,
    MemorySearchHit, MemoryStoreIntegrity, MemoryStoreMetadata, StoredMemoryRecord,
    TypedMemoryStore, TypedMemoryStoreError, WorkspaceIdentityMigrationState,
    MAX_MEMORY_CONTENT_BYTES, MAX_MEMORY_RECORDS,
};
pub use cognitive::MemRules;
pub use error::{LaputaError, Result};
pub use feedback::{
    PendingRecallFeedback, RecallFeedbackEvent, RecallFeedbackStore, RecallTaskOutcome,
};
pub use frozen_core::{
    capture_for_session as capture_frozen_core_for_session, content_version,
    release_session_projection as release_frozen_core_session,
    session_projection as frozen_core_session_projection, FrozenCoreSessionProjection,
    FrozenCoreSnapshot, DEFAULT_FROZEN_CORE_BUDGET, FROZEN_CORE_SECTIONS,
};
pub use layout::{LaputaPaths, LaputaStorage};
pub use lock::{LaputaLock, LockOptions};
pub use memory_records::{LaputaSection, SectionStatus};
pub use persona::{
    PersonaChangeRequest, PersonaDocument, PersonaError, PersonaFileState, PersonaHistoryEntry,
    PersonaHistoryRevision, PersonaInitialization, PersonaKind, PersonaRepair, PersonaRequestActor,
    PersonaRequestState, PersonaService, PersonaStatus, PersonaStatusView, PersonaWriteOutcome,
    PersonaWriteSource,
};
