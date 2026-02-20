//! Agent logic for agent-diva
//!
//! This crate provides the agent loop, context building, and skill loading.

pub mod agent_loop;
pub mod context;
pub mod skills;
pub mod subagent;

pub use agent_diva_core::bus::AgentEvent;
pub use agent_loop::{AgentLoop, ToolConfig};
