//! Shared tool primitives for agent-diva.

mod base;
pub mod module;
mod registry;

pub use base::{Result, Tool, ToolError};
pub use module::{Bootstrap, BootstrapError, Module, ModuleCtx, ModuleRegistry};
pub use registry::{ToolDefinitionSet, ToolRegistry, ToolSchemaPartition};
