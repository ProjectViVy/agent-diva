//! File-first authority storage boundary for Laputa.
//!
//! This crate owns the `.laputa/` storage layout and low-level persistence
//! primitives used by later governance stories.

pub mod atomic;
pub mod error;
pub mod layout;
pub mod lock;

pub use atomic::{atomic_write, atomic_write_json};
pub use error::{LaputaError, Result};
pub use layout::{LaputaPaths, LaputaStorage};
pub use lock::{LaputaLock, LockOptions};
