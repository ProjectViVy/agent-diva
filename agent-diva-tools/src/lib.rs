//! Built-in tools for agent-diva
//!
//! This crate provides the tool registry and built-in tool implementations.

pub mod ask_user;
pub mod attachment;
pub mod base;
pub mod cron;
pub mod distill_guard;
pub mod enqueue_background_task;
pub mod execution_todo;
pub mod filesystem;
pub mod laputa_propose_section_write;
pub mod mcp_sdk;
pub mod memory_add;
pub mod memory_distill;
pub mod memory_list;
pub mod memory_remove;
pub mod memory_search;
pub mod memory_update;
pub mod message;
pub mod planning;
pub mod read_tool_result;
pub mod registry;
pub mod sanitize;
pub mod shell;
pub mod spawn;
pub mod tool_discovery;
pub mod update_plan;
pub mod update_working_checkpoint;
pub mod web;
pub mod wtf;

pub use agent_diva_tooling::{Result, Tool, ToolError, ToolRegistry};
pub use ask_user::AskUserTool;
pub use attachment::ReadAttachmentTool;
pub use cron::CronTool;
pub use enqueue_background_task::{BackgroundTaskContext, EnqueueBackgroundTaskTool};
pub use execution_todo::{ExecutionTodoShowTool, ExecutionTodoWriteTool};
pub use filesystem::{EditFileTool, ListDirTool, ReadFileTool, WriteFileTool};
pub use laputa_propose_section_write::LaputaProposeSectionWriteTool;
pub use memory_add::MemoryAddTool;
pub use memory_distill::MemoryDistillTool;
pub use memory_list::MemoryListTool;
pub use memory_remove::MemoryRemoveTool;
pub use memory_search::MemorySearchTool;
pub use memory_update::MemoryUpdateTool;
pub use message::MessageTool;
pub use read_tool_result::ReadToolResultTool;
pub use sanitize::sanitize_for_json;
pub use shell::ExecTool;
pub use spawn::SpawnTool;
pub use tool_discovery::ToolSearchTool;
pub use update_plan::UpdatePlanTool;
pub use update_working_checkpoint::UpdateWorkingCheckpointTool;
pub use web::{WebFetchTool, WebSearchTool};
pub use wtf::{print_ascii_agent_diva_logo, ASCII_AGENT_DIVA_LOGO};

// MCP implementation using rust-mcp-sdk from crates.io
pub use mcp_sdk::{
    load_mcp_tools, load_mcp_tools_sync, probe_mcp_server, probe_mcp_server_sync, DiscoveredTool,
    McpClientWrapper, McpError, McpSdkTool,
};
