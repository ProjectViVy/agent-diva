//! Memory management for long-term storage.
//!
//! Handles loading and updating of `MEMORY.md` and `HISTORY.md`.

#[cfg(feature = "mentle")]
pub mod hybrid;
pub mod manager;
pub mod provider;
pub mod record;
pub mod storage;

#[cfg(feature = "mentle")]
pub use hybrid::HybridMemoryProvider;
pub use manager::MemoryManager;
pub use provider::{
    MemoryProvider, PrefetchRequest, PrefetchResponse, PrefetchStatus, RhythmTrigger,
    SessionEndRequest, SessionEndResponse, SessionEndStatus, StartupContextSnapshot,
    StartupInjectionShape, StartupStatus, SyncTurnRequest, SyncTurnResponse, SyncTurnStatus,
    SystemPromptBlock, SystemPromptRequest, SystemPromptResponse, WakeupPackSummary,
};
pub use record::{
    escape_memory_for_prompt, memory_content_digest, MemoryIntegrityFinding, MemoryIntegrityReport,
    MemoryIntegritySeverity, MemoryProvenance, MemoryProvenanceSource, MemoryRecord,
    MemoryRecordKind, MemoryRecordValidationError, MemoryScope, MemorySensitivity, MemoryTombstone,
    MemoryTrust, MAX_CONFIDENCE_BPS,
};
pub use storage::{DailyNote, Memory};
