//! Core types and traits for agent-diva
//!
//! This crate provides the foundational types, traits, and utilities
//! used by all other agent-diva components.

pub mod attachment;
pub mod audit;
pub mod bus;
pub mod config;
pub mod cron;
pub mod debug;
pub mod error;
pub mod error_context;
pub mod error_kind;
pub mod heartbeat;
pub mod logging;
pub mod memory;
pub mod presence;
pub mod redaction;
pub mod security;
pub mod session;
pub use session::Usage;
pub mod soul;
pub mod trace;
pub mod utils;

pub use attachment::{FileAttachment, FileAttachmentRef};
pub use error::{Error, Result};
pub use error_kind::ErrorKind;
