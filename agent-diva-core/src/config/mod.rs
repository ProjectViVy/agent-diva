//! Configuration management
//!
//! Handles loading and validation of agent-diva configuration from files
//! and environment variables.

pub mod hot_reload;
pub mod loader;
pub mod schema;
pub mod validate;

pub use hot_reload::{
    compute_changed_fields, ConfigChangeEvent, ConfigWatcher, HotReloadable, HotReloadableField,
};
pub use loader::ConfigLoader;
pub use schema::*;
