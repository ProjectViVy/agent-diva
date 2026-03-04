//! Session management for conversation history
//!
//! Sessions store conversation history in JSONL format for easy
//! reading and persistence.

pub mod manager;
pub mod store;

pub use manager::{SessionManager, SessionInfo};
pub use store::{ChatMessage, Session};
