//! Built-in tools for agent-diva
//!
//! This crate provides the tool registry and built-in tool implementations.

pub mod ask_user;
pub mod attachment;
pub mod base;
pub mod cron;
pub mod enqueue_background_task;
pub mod execution_todo;
pub mod filesystem;
pub mod mcp_sdk;
pub mod message;
pub mod planning;
pub mod registry;
pub mod sanitize;
pub mod shell;
pub mod spawn;
pub mod update_plan;
pub mod web;
pub mod wtf;

pub use agent_diva_tooling::{Result, Tool, ToolError, ToolRegistry};
pub use ask_user::AskUserTool;
pub use attachment::ReadAttachmentTool;
pub use cron::CronTool;
pub use enqueue_background_task::{BackgroundTaskContext, EnqueueBackgroundTaskTool};
pub use execution_todo::{ExecutionTodoShowTool, ExecutionTodoWriteTool};
pub use filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
pub use message::MessageTool;
pub use sanitize::sanitize_for_json;
pub use shell::ExecTool;
pub use spawn::SpawnTool;
pub use update_plan::UpdatePlanTool;
pub use web::{WebFetchTool, WebSearchTool};
pub use wtf::{print_ascii_agent_diva_logo, ASCII_AGENT_DIVA_LOGO};

// MCP implementation using rust-mcp-sdk from crates.io
pub use mcp_sdk::{
    load_mcp_tools, load_mcp_tools_sync, probe_mcp_server, probe_mcp_server_sync, DiscoveredTool,
    McpClientWrapper, McpError, McpSdkTool,
};
