//! Core types and traits for agent-diva
//!
//! This crate provides the foundational types, traits, and utilities
//! used by all other agent-diva components.

pub mod attachment;
pub mod audit;
pub mod audit_parse;
pub mod bus;
pub mod config;
pub mod cron;
pub mod error;
pub mod error_category;
pub mod error_context;
pub mod evolution;
pub mod heartbeat;
pub mod logging;
pub mod memory;
pub mod planning;
pub mod presence;
pub mod reasoning;
pub mod reports;
pub mod scheduler;
pub mod security;
pub mod session;
pub mod soul;
pub mod supervised;
pub mod todo;
pub mod token_ledger;
pub mod utils;

pub use attachment::FileAttachment;
pub use audit::emit;
pub use error::{Error, Result};
