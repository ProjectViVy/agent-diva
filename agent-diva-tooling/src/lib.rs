//! Shared tool primitives for agent-diva.

mod base;
mod module;
mod registry;

pub use base::{Result, Tool, ToolError};
pub use module::{
    Module, ModuleBuildContext, ModuleCtx, ModuleRegistration, ModuleStartup, PresenceService,
    SafetyService, SandboxService,
};
pub use registry::ToolRegistry;
