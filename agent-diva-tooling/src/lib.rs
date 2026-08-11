//! Shared tool primitives for agent-diva.

mod base;
pub mod module;
mod registry;

pub use base::{Result, Tool, ToolError};
pub use module::{Bootstrap, BootstrapError, Module, ModuleCtx, ModuleRegistry};
pub use registry::{
    ToolDefinitionSet, ToolDiscoveryHandle, ToolDiscoveryState, ToolDiscoveryStateHandle,
    ToolExecutionOutput, ToolRegistry, ToolSchemaPartition, ToolSearchResult,
    TOOL_DISCOVERY_SCHEMA_VERSION,
};
