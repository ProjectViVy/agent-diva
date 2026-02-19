//! Built-in tools for agent-diva
//!
//! This crate provides the tool registry and built-in tool implementations.

pub mod base;
pub mod cron;
pub mod filesystem;
pub mod mcp;
pub mod message;
pub mod registry;
pub mod shell;
pub mod spawn;
pub mod web;

pub use base::{Tool, ToolError};
pub use cron::CronTool;
pub use filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
pub use mcp::load_mcp_tools;
pub use message::MessageTool;
pub use registry::ToolRegistry;
pub use shell::ExecTool;
pub use spawn::SpawnTool;
pub use web::{WebFetchTool, WebSearchTool};
