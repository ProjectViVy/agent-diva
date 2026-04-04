//! Core types and traits for agent-diva
//!
//! This crate provides the foundational types, traits, and utilities
//! used by all other agent-diva components.

pub mod bus;
pub mod config;
pub mod cron;
pub mod error;
pub mod error_context;
pub mod heartbeat;
pub mod logging;
pub mod memory;
pub mod session;
pub mod soul;
pub mod usage;
pub mod utils;

pub use error::{Error, Result};
