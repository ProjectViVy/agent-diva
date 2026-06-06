//! Agent logic for agent-diva
//!
//! This crate provides the agent loop, context building, and skill loading.

pub mod agent_loop;
pub mod consolidation;
pub mod context;
pub mod context_budget;
pub(crate) mod loop_guard;
pub mod runtime_control;
pub mod skills;
pub mod subagent;
pub mod subagent_policy;
pub mod tool_assembly;
pub mod tool_config;

pub use agent_diva_core::bus::AgentEvent;
pub use agent_loop::{AgentLoop, AgentLoopToolSet, ToolConfig};
pub use context_budget::ContextBudgetPolicy;
pub use runtime_control::RuntimeControlCommand;
pub use subagent_policy::SubagentPolicy;
pub use tool_assembly::{SubagentSpawner, ToolAssembly};
pub use tool_config::builtin::BuiltInToolsConfig;
