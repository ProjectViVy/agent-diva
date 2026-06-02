//! Core types and traits for agent-diva
//!
//! This crate provides the foundational types, traits, and utilities
//! used by all other agent-diva components.

pub mod attachment;
pub mod bus;
pub mod config;
pub mod cron;
pub mod error;
pub mod error_context;
pub mod heartbeat;
pub mod logging;
pub mod memory;
pub mod security;
pub mod session;
pub mod soul;
pub mod utils;

pub use attachment::FileAttachment;
pub use error::{Error, Result};
