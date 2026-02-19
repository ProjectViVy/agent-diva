//! Memory management for long-term storage.
//!
//! Handles loading and updating of `MEMORY.md` and `HISTORY.md`.

pub mod manager;
pub mod storage;

pub use manager::MemoryManager;
pub use storage::{DailyNote, Memory};
