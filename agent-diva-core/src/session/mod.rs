//! Session management for conversation history
//!
//! Sessions store conversation history in JSONL format for easy
//! reading and persistence.

pub mod manager;
pub mod search;
pub mod store;

pub use manager::{SessionInfo, SessionManager};
pub use search::{
    SessionSearchDiagnostic, SessionSearchHit, SessionSearchQuery, SessionSearchResponse,
};
pub use store::{
    align_chat_history, bound_checkpoint_body, session_channel_from_key, CanonicalCheckpoint,
    ChatMessage, CheckpointTrigger, Session, SessionKind, CANONICAL_CHECKPOINT_MAX_CHARS,
    CANONICAL_CHECKPOINT_SCHEMA_VERSION,
};

// Re-export TokenUsage for convenience — it's used alongside ChatMessage
// to record per-turn LLM token consumption.
pub use crate::config::schema::TokenUsage;
