//! Enhanced memory subsystem for agent-diva.

mod backend;
pub mod compat;
mod compat_source;
pub mod contracts;
pub mod derived;
pub mod diary;
pub mod embeddings;
pub mod layout;
pub mod recall;
pub mod retrieval;
pub mod service;
pub mod snapshot;
pub mod sqlite_recall;
pub mod store;
pub mod sync;
pub mod types;

pub use compat::{history_file_path, long_term_memory_file_path, memory_dir_path};
pub use contracts::{
    DiaryReadRequest, DiaryStore, DiaryToolContract, DiaryToolListResult, DiaryToolReadResult,
    MemoryStore, MemoryToolContract, MemoryToolRecallResult, RecallEngine,
};
pub use derived::derive_structured_memory_records;
pub use diary::FileDiaryStore;
pub use embeddings::{
    cosine_similarity, provider_from_config, EmbeddingProvider, EmbeddingProviderConfig,
    NoopEmbeddingProvider, OpenAiCompatibleEmbeddingProvider,
};
pub use layout::{
    brain_db_path, diary_dir_path, emotional_diary_dir_path, index_dir_path,
    rational_diary_dir_path,
};
pub use recall::FileRecallEngine;
pub use retrieval::{
    CachedSemanticRetriever, DefaultHybridReranker, HybridReranker, KeywordRetriever,
    MergedKeywordRetriever, RetrievalEngine, SemanticRetriever,
};
pub use service::WorkspaceMemoryService;
pub use snapshot::{export_snapshot, hydrate_snapshot, snapshot_exists};
pub use sqlite_recall::SqliteRecallEngine;
pub use store::SqliteMemoryStore;
pub use sync::{
    backfill_workspace_sources, stored_compat_record, stored_diary_record,
    sync_diary_entry_to_sqlite,
};
pub use types::{
    DiaryEntry, DiaryFilter, DiaryPartition, MemoryDomain, MemoryGetRequest, MemoryGetResult,
    MemoryQuery, MemoryRecord, MemoryScope, MemorySearchResult, MemorySearchResultItem,
    MemorySourceRef, RecallMode,
};
