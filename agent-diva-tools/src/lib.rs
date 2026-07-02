//! Built-in tools for agent-diva
//!
//! This crate provides the tool registry and built-in tool implementations.

pub mod attachment;
pub mod base;
pub mod cron;
pub mod delegate;
pub mod execute_code_tool;
pub mod filesystem;
pub mod mcp_sdk;
pub mod message;
pub mod patch_tool;
pub mod planning;
pub mod process_tool;
pub mod registry;
pub mod sanitize;
pub mod search_files_tool;
pub mod shell;
pub mod spawn;
pub mod toolsets;
pub mod web;
pub mod wtf;

pub use agent_diva_tooling::{Result, Tool, ToolError, ToolRegistry};
pub use attachment::ReadAttachmentTool;
pub use cron::CronTool;
pub use delegate::DelegateTool;
pub use execute_code_tool::ExecuteCodeTool;
pub use filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
pub use message::MessageTool;
pub use patch_tool::PatchTool;
pub use process_tool::ProcessTool;
pub use sanitize::sanitize_for_json;
pub use search_files_tool::SearchFilesTool;
pub use shell::ExecTool;
pub use spawn::SpawnTool;
pub use toolsets::{default_toolsets, Toolset};
pub use web::{WebFetchTool, WebSearchTool};
pub use wtf::{print_ascii_agent_diva_logo, ASCII_AGENT_DIVA_LOGO};

// MCP implementation using rust-mcp-sdk from crates.io
pub use mcp_sdk::{
    load_mcp_tools, load_mcp_tools_sync, probe_mcp_server, probe_mcp_server_sync, DiscoveredTool,
    McpClientWrapper, McpError, McpSdkTool,
};
