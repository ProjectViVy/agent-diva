//! BML (Basic Memory Layer) logical layer boundary.
//!
//! BML is the memory storage layer; its production entity is the
//! profile-local typed SQLite + FTS5
//! authority (`.laputa/memory.sqlite3`), the sole production Memory authority.
//! See AGENTS.md "Memory is layered into three concepts" for the frozen
//! BML / Laputa / Garden model (decision D3, 2026-08-08).
//!
//! This module is the single public face of the BML storage core inside
//! `agent-diva-laputa`. Governance modules (service / proposals /
//! governed_apply / cognitive / ...) must NOT call BML write APIs directly
//! (`put` / `put_governed` / `import_records` / `rollback_governed` / gc /
//! backup / restore). Writes are only legitimate through the governed apply
//! seam, migration tooling, and management endpoints. `tests/bml_boundary_guard`
//! enforces this boundary (run via `just bml-boundary-check`).
//!
//! Notes:
//! - `TypedMemoryStore::put_governed` remains an unregistered seam until the
//!   GMH-24 write cutover; production governed apply goes through the file
//!   `ProposalRepository` + `governance.sqlite3`.
//! - If BML is later extracted into its own crate (`agent-diva-bml`), this
//!   re-export surface is the candidate public API; the `memory_records`
//!   adapters that depend on governance types (EvolutionProposal /
//!   LaputaSection) would then be split between the two crates.

pub mod memory_home;

pub use memory_home::{MemRulesDocument, MemRulesSource, MemoryHome, MemoryHomeError};

pub use crate::memory_records::{
    adapt_laputa_section, adapt_legacy_markdown, compare_normalized_records, MemoryAdapterContext,
    MemoryAdapterOutput, MemoryMigrationManifest, MemoryMigrationPlan, MemoryMigrationTestFailure,
    MemoryRecordMigration, MemoryRollbackManifest,
};
pub use crate::typed_store::{
    MemorySearchHit, MemoryStoreIntegrity, MemoryStoreMetadata, StoredMemoryRecord,
    TypedMemoryStore, TypedMemoryStoreError, WorkspaceIdentityMigrationManifest,
    WorkspaceIdentityMigrationState, MAX_MEMORY_CONTENT_BYTES, MAX_MEMORY_RECORDS,
};
