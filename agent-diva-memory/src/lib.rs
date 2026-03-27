//! Enhanced memory subsystem for agent-diva.

mod backend;
pub mod compat;
mod compat_source;
pub mod contracts;
pub mod diary;
pub mod layout;
pub mod recall;
pub mod service;
pub mod sqlite_recall;
pub mod store;
pub mod sync;
pub mod types;

pub use compat::{history_file_path, long_term_memory_file_path, memory_dir_path};
pub use contracts::{
    DiaryReadRequest, DiaryStore, DiaryToolContract, DiaryToolListResult, DiaryToolReadResult,
    MemoryStore, MemoryToolContract, MemoryToolRecallResult, RecallEngine,
};
pub use diary::FileDiaryStore;
pub use layout::{
    brain_db_path, diary_dir_path, emotional_diary_dir_path, index_dir_path,
    rational_diary_dir_path,
};
pub use recall::FileRecallEngine;
pub use service::WorkspaceMemoryService;
pub use sqlite_recall::SqliteRecallEngine;
pub use store::SqliteMemoryStore;
pub use sync::{
    backfill_workspace_sources, stored_compat_record, stored_diary_record,
    sync_diary_entry_to_sqlite,
};
pub use types::{
    DiaryEntry, DiaryFilter, DiaryPartition, MemoryDomain, MemoryQuery, MemoryRecord, MemoryScope,
    MemorySourceRef,
};
