//! BML (Basic Memory Layer) logical layer boundary.
//!
//! BML is the memory storage layer; its production entity is the
//! machine-wide typed SQLite + FTS5 authority (`{config_dir}/memory/
//! memory.sqlite3`) exposed through [`MemoryHome`], the sole production
//! Memory authority. See AGENTS.md "Memory is layered into three concepts"
//! for the frozen BML / Laputa / Garden model.
//!
//! The cognitive clean break removed the governed-apply pipeline: memory
//! CRUD is direct and unapproved. Non-BML modules must NOT call the raw
//! write APIs (`put` / `import_records` / gc / backup / restore);
//! `tests/bml_boundary_guard` enforces this boundary (run via
//! `just bml-boundary-check`).
//!
//! Notes:
//! - `put_governed` / `rollback_governed` are deleted store APIs. The
//!   `memory_apply_journal` table remains (D4 §3.2: keep schema, drop the
//!   code path). `tests/bml_boundary_guard` still forbids those names so
//!   the seam cannot be reintroduced.
//! - The `memory_records` adapters remain the offline-migration import
//!   vocabulary for legacy section/Markdown files.

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
