//! Planning integration for the agent loop.
//!
//! Provides context injection, hooks, system-message assembly,
//! lifecycle orchestration, and todo generation for the planning subsystem.

pub mod context;
pub mod hooks;
pub mod nag;
pub mod orchestrator;
pub mod todo_planner;
pub mod tools;
pub mod verifier;

pub use context::*;
pub use hooks::*;
pub use nag::*;
pub use orchestrator::*;
pub use todo_planner::*;
pub use tools::*;
pub use verifier::*;

use agent_diva_core::planning::policy::ToolCapability;

/// Classify every built-in runtime tool for planning policy enforcement.
///
/// Names outside this closed list include MCP and custom tools. They are
/// deliberately classified as `Unknown` so an active plan never grants them
/// authority solely because a registry happened to contain them.
pub fn builtin_tool_capability(tool_name: &str) -> ToolCapability {
    match tool_name {
        "read_file" | "list_dir" | "read_attachment" | "plan_show" | "todo_show" => {
            ToolCapability::Inspect
        }
        "plan_create" | "plan_transition" | "plan_submit" => ToolCapability::PlanningRecord,
        "todo_write" => ToolCapability::WorkItem,
        "write_file" | "edit_file" => ToolCapability::WorkspaceWrite,
        "exec" => ToolCapability::Execute,
        "cron" | "spawn" | "enqueue_background_task" | "web_search" | "web_fetch" => {
            ToolCapability::External
        }
        _ => ToolCapability::Unknown,
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn builtin_tool_capabilities_are_closed_and_complete() {
        for (name, expected) in [
            ("read_file", ToolCapability::Inspect),
            ("list_dir", ToolCapability::Inspect),
            ("read_attachment", ToolCapability::Inspect),
            ("plan_show", ToolCapability::Inspect),
            ("todo_show", ToolCapability::Inspect),
            ("plan_create", ToolCapability::PlanningRecord),
            ("plan_transition", ToolCapability::PlanningRecord),
            ("plan_submit", ToolCapability::PlanningRecord),
            ("todo_write", ToolCapability::WorkItem),
            ("write_file", ToolCapability::WorkspaceWrite),
            ("edit_file", ToolCapability::WorkspaceWrite),
            ("exec", ToolCapability::Execute),
            ("cron", ToolCapability::External),
            ("spawn", ToolCapability::External),
            ("enqueue_background_task", ToolCapability::External),
            ("web_search", ToolCapability::External),
            ("web_fetch", ToolCapability::External),
        ] {
            assert_eq!(builtin_tool_capability(name), expected, "{name}");
        }

        for name in ["mcp_server_tool", "custom_tool", "plan_approve", "unknown"] {
            assert_eq!(
                builtin_tool_capability(name),
                ToolCapability::Unknown,
                "{name}"
            );
        }
    }
}
