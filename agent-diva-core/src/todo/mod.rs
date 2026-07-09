//! Runtime todo module — JSONL append-only store

pub mod store;
pub mod types;

pub use store::JsonlTodoStore;
pub use types::{TodoItem, TodoStatus, TodoStatusFilter};
