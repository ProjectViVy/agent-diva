//! Built-in tools for agent-diva
//!
//! This crate provides the tool registry and built-in tool implementations.

pub mod base;
pub mod cron;
pub mod filesystem;
pub mod mcp_sdk;
pub mod memory;
pub mod message;
pub mod registry;
pub mod sanitize;
pub mod shell;
pub mod spawn;
pub mod web;
pub mod wtf;

pub use base::{Tool, ToolError};
pub use cron::CronTool;
pub use filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
pub use memory::{DiaryListTool, DiaryReadTool, MemoryGetTool, MemoryRecallTool, MemorySearchTool};
pub use message::MessageTool;
pub use registry::ToolRegistry;
pub use sanitize::sanitize_for_json;
pub use shell::ExecTool;
pub use spawn::SpawnTool;
pub use web::{WebFetchTool, WebSearchTool};
pub use wtf::{print_ascii_agent_diva_logo, ASCII_AGENT_DIVA_LOGO};

// MCP implementation using rust-mcp-sdk from crates.io
pub use mcp_sdk::{
    load_mcp_tools, load_mcp_tools_sync, probe_mcp_server, probe_mcp_server_sync, DiscoveredTool,
    McpClientWrapper, McpError, McpSdkTool,
};
