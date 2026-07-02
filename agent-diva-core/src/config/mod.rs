//! Configuration management
//!
//! Handles loading and validation of agent-diva configuration from files
//! and environment variables.

pub mod loader;
pub mod schema;
pub mod validate;
pub mod versioning;

pub use loader::{compute_diff, ChangedField, ConfigDiff, ConfigLoader};
pub use schema::*;
pub use versioning::{current_version, migrate, ConfigVersion};
