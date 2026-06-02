//! Shared tool primitives for agent-diva.

mod base;
mod registry;

pub use base::{Result, Tool, ToolError};
pub use registry::ToolRegistry;
