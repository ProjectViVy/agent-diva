//! File-first authority storage boundary for Laputa.
//!
//! This crate owns the `.laputa/` storage layout and low-level persistence
//! primitives used by later governance stories.
//!
//! The BML logical layer (memory storage) lives at [`bml`]; see its module
//! docs for the layer boundary and write-API rules.

pub mod atomic;
pub mod bml;
pub mod cognitive;
pub mod error;
pub mod feedback;
pub mod frozen_core;
pub mod governed_apply;
pub mod layout;
pub mod lock;
pub mod memory_provider;
pub mod memory_records;
pub mod metrics;
pub mod migration;
pub mod persona_retire;
pub mod proposals;
pub mod recall;
pub mod service;
pub mod suppression;
pub mod typed_provider;
pub mod typed_store;

pub use atomic::{atomic_write, atomic_write_json};
pub use bml::{
    adapt_governed_proposal, adapt_laputa_section, adapt_legacy_markdown,
    compare_normalized_records, GovernedMemoryApply, MemoryAdapterContext, MemoryAdapterOutput,
    MemoryMigrationManifest, MemoryMigrationPlan, MemoryMigrationTestFailure,
    MemoryRecordMigration, MemoryRollbackManifest, MemorySearchHit, MemoryStoreIntegrity,
    MemoryStoreMetadata, StoredMemoryRecord, TypedMemoryStore, TypedMemoryStoreError,
    WorkspaceIdentityMigrationManifest, WorkspaceIdentityMigrationState, MAX_MEMORY_CONTENT_BYTES,
    MAX_MEMORY_RECORDS,
};
pub use cognitive::{
    ClaimStatus, MemRule, MemRules, WorldClaim, WorldClaimPayload, WorldError, WorldGovernance,
    WorldGovernanceError, WorldProposalState, WorldStore, WorldUpsertProposal,
};
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
pub use governed_apply::{
    proposal_digest, MemoryGovernanceCoordinator, MemoryGovernanceDecision, MemoryGovernanceError,
    MemoryGovernanceView,
};
pub use layout::{LaputaPaths, LaputaStorage};
pub use lock::{LaputaLock, LockOptions};
pub use memory_provider::LaputaMemoryProvider;
pub use metrics::{LaputaMetrics, LaputaMetricsSnapshot};
pub use migration::{
    LaputaMigration, LaputaMigrationBackup, LaputaMigrationOptions, LaputaMigrationOutcome,
    LaputaMigrationSource, LaputaMigrationSourceKind, LaputaMigrationTestFailure,
};
pub use persona_retire::{
    archive_sources as archive_persona_sources,
    create_proposals as create_persona_retirement_proposals,
    scan_workspace as scan_persona_workspace, PersonaArchiveOutcome, PersonaProposalSpec,
    PersonaRetirementPlan, PersonaSource, PersonaSourceKind,
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
    ChangelogFilter, ChangelogPage, CognitiveFileKind, LaputaEvent, LaputaEventKind, LaputaSection,
    LaputaService, LaputaSnapshot, MemoryListFilter, RollbackChangelogRequest, RollbackOutcome,
    SectionStatus,
};
pub use suppression::{CandidateSuppression, CandidateSuppressionStore};
pub use typed_provider::TypedLaputaMemoryProvider;
