//! Agent logic for agent-diva
//!
//! This crate provides the agent loop, context building, and skill loading.

pub mod agent_loop;
pub mod consolidation;
pub mod context;
pub mod mentle_discovery;
#[cfg(feature = "mentle")]
mod mentle_runtime;
pub mod runtime_control;
pub mod skills;
pub mod subagent;
pub mod tool_assembly;
pub mod tool_config;
pub mod planning;

pub use agent_diva_core::bus::AgentEvent;
pub use agent_loop::{AgentLoop, AgentLoopToolSet, ToolConfig};
pub use mentle_discovery::{discover_mentle_tool_names, mentle_discovery_available};
pub use runtime_control::RuntimeControlCommand;
pub use tool_assembly::{SubagentSpawner, ToolAssembly};
pub use tool_config::builtin::BuiltInToolsConfig;
