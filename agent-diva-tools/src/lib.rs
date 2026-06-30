//! Built-in tools for agent-diva
//!
//! This crate provides the tool registry and built-in tool implementations.

pub mod attachment;
pub mod cron;
pub mod filesystem;
pub mod mcp_sdk;
pub mod message;
pub mod patch;
pub mod process;
pub mod sanitize;
pub mod search_files;
pub mod shell;
pub mod spawn;
pub mod toolsets;
pub mod web;
pub mod execute_code;
pub mod mcp_reconnect;
pub mod wtf;

pub use agent_diva_tooling::{Result, Tool, ToolError, ToolRegistry};
pub use attachment::ReadAttachmentTool;
pub use cron::CronTool;
pub use filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
pub use message::MessageTool;
pub use patch::PatchTool;
pub use sanitize::sanitize_for_json;
pub use search_files::SearchFilesTool;
pub use process::ProcessTool;
pub use shell::ExecTool;
pub use spawn::SpawnTool;
pub use execute_code::ExecuteCodeTool;
pub use mcp_reconnect::McpReconnectManager;
pub use web::{WebFetchTool, WebSearchTool};
pub use wtf::{print_ascii_agent_diva_logo, ASCII_AGENT_DIVA_LOGO};

// MCP implementation using rust-mcp-sdk from crates.io
pub use mcp_sdk::{
    load_mcp_tools, load_mcp_tools_sync, probe_mcp_server, probe_mcp_server_sync, DiscoveredTool,
    McpClientWrapper, McpError, McpSdkTool,
};
