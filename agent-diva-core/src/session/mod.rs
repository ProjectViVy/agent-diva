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
pub use store::{ChatMessage, CompactSummary, CompactTrigger, CompactionRange, Session};
