//! File-first authority storage boundary for Laputa.
//!
//! This crate owns the `.laputa/` storage layout and low-level persistence
//! primitives used by later governance stories.

pub mod atomic;
pub mod error;
pub mod feedback;
pub mod governed_apply;
pub mod layout;
pub mod lock;
pub mod memory_provider;
pub mod memory_records;
pub mod metrics;
pub mod migration;
pub mod proposals;
pub mod recall;
pub mod service;
pub mod suppression;
pub mod typed_provider;
pub mod typed_store;

pub use atomic::{atomic_write, atomic_write_json};
pub use error::{LaputaError, Result};
pub use feedback::{
    PendingRecallFeedback, RecallFeedbackEvent, RecallFeedbackStore, RecallTaskOutcome,
};
pub use governed_apply::{
    proposal_digest, MemoryGovernanceCoordinator, MemoryGovernanceDecision, MemoryGovernanceError,
    MemoryGovernanceView,
};
pub use layout::{LaputaPaths, LaputaStorage};
pub use lock::{LaputaLock, LockOptions};
pub use memory_provider::LaputaMemoryProvider;
pub use memory_records::{
    adapt_governed_proposal, adapt_laputa_section, adapt_legacy_markdown,
    compare_normalized_records, MemoryAdapterContext, MemoryAdapterOutput, MemoryMigrationManifest,
    MemoryMigrationPlan, MemoryMigrationTestFailure, MemoryRecordMigration, MemoryRollbackManifest,
};
pub use metrics::{LaputaMetrics, LaputaMetricsSnapshot};
pub use migration::{
    LaputaMigration, LaputaMigrationBackup, LaputaMigrationOptions, LaputaMigrationOutcome,
    LaputaMigrationSource, LaputaMigrationSourceKind, LaputaMigrationTestFailure,
};
pub use proposals::{
    ApplyFailurePoint, ApplyOptions, ApplyOutcome, ProposalEdit, ProposalFilter,
    ProposalRepository, ProposalSummary,
};
pub use recall::{
    LaputaRecallCandidateSource, LaputaRecallMetrics, LaputaRecallService, LaputaRecallShadow,
    RecallReasonCount,
};
pub use service::{
    ChangelogFilter, ChangelogPage, LaputaEvent, LaputaEventKind, LaputaSection, LaputaService,
    LaputaSnapshot, RollbackChangelogRequest, RollbackOutcome, SectionStatus,
};
pub use suppression::{CandidateSuppression, CandidateSuppressionStore};
pub use typed_provider::TypedLaputaMemoryProvider;
pub use typed_store::{
    GovernedMemoryApply, MemorySearchHit, MemoryStoreIntegrity, MemoryStoreMetadata,
    StoredMemoryRecord, TypedMemoryStore, TypedMemoryStoreError, MAX_MEMORY_CONTENT_BYTES,
    MAX_MEMORY_RECORDS,
};
