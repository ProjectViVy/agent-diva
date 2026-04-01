//! Catalog of tools available inside [`crate::subagent::SubagentManager`] tasks (SWARM-MIG-01 / Story 6.5).
//!
//! The LLM-facing [`agent_diva_tools::registry::ToolRegistry`] is built from this table so capability
//! metadata (stable ids, risk tier placeholders) stays aligned with registration order and doctor output.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_diva_tools::registry::ToolRegistry;
use agent_diva_tools::{
    filesystem::{ListDirTool, ReadFileTool, WriteFileTool},
    shell::ExecTool,
    web::{WebFetchTool, WebSearchTool},
};

use crate::tool_config::network::NetworkToolConfig;

/// Placeholder for future Shannon-style enforcement (Story 6.5 AC).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SubagentToolRiskTier {
    Low,
    Medium,
    High,
}

/// One row in the subagent tool capability catalog (registry-aligned metadata + wiring key).
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct SubagentToolCapabilitySpec {
    /// Stable capability id (distinct from LLM `tool_name`).
    pub capability_id: &'static str,
    /// OpenAI-style tool name passed to the model.
    pub tool_name: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
    pub risk_tier: SubagentToolRiskTier,
    /// Empty = always registered when subagent runs; otherwise explains gating (doctor / operators).
    pub availability_note: &'static str,
}

/// Canonical list: keep in sync with [`build_subagent_tool_registry`].
pub static SUBAGENT_TOOL_CAPABILITY_SPECS: &[SubagentToolCapabilitySpec] = &[
    SubagentToolCapabilitySpec {
        capability_id: "tool.subagent.read_file",
        tool_name: "read_file",
        display_name: "Read file",
        description: "Read file contents within workspace constraints.",
        risk_tier: SubagentToolRiskTier::Low,
        availability_note: "",
    },
    SubagentToolCapabilitySpec {
        capability_id: "tool.subagent.write_file",
        tool_name: "write_file",
        display_name: "Write file",
        description: "Create or overwrite files within workspace constraints.",
        risk_tier: SubagentToolRiskTier::Medium,
        availability_note: "",
    },
    SubagentToolCapabilitySpec {
        capability_id: "tool.subagent.list_dir",
        tool_name: "list_dir",
        display_name: "List directory",
        description: "List directory entries within workspace constraints.",
        risk_tier: SubagentToolRiskTier::Low,
        availability_note: "",
    },
    SubagentToolCapabilitySpec {
        capability_id: "tool.subagent.exec",
        tool_name: "exec",
        display_name: "Shell exec",
        description: "Execute shell commands (high blast radius).",
        risk_tier: SubagentToolRiskTier::High,
        availability_note: "",
    },
    SubagentToolCapabilitySpec {
        capability_id: "tool.subagent.web_search",
        tool_name: "web_search",
        display_name: "Web search",
        description: "Search the web when network tools are enabled in config.",
        risk_tier: SubagentToolRiskTier::Medium,
        availability_note: "Registered only when `network.web.search.enabled` is true.",
    },
    SubagentToolCapabilitySpec {
        capability_id: "tool.subagent.web_fetch",
        tool_name: "web_fetch",
        display_name: "Web fetch",
        description: "Fetch URLs when network tools are enabled in config.",
        risk_tier: SubagentToolRiskTier::Medium,
        availability_note: "Registered only when `network.web.fetch.enabled` is true.",
    },
];

/// Build the tool registry used by background subagents (no `spawn`, no `message` tool).
#[must_use]
pub fn build_subagent_tool_registry(
    workspace: &Path,
    network_config: &NetworkToolConfig,
    restrict_to_workspace: bool,
) -> ToolRegistry {
    let mut tools = ToolRegistry::new();
    let allowed_dir: Option<PathBuf> = if restrict_to_workspace {
        Some(workspace.to_path_buf())
    } else {
        None
    };

    tools.register(Arc::new(ReadFileTool::new(allowed_dir.clone())));
    tools.register(Arc::new(WriteFileTool::new(allowed_dir.clone())));
    tools.register(Arc::new(ListDirTool::new(allowed_dir)));
    tools.register(Arc::new(ExecTool::new()));

    if network_config.web.search.enabled {
        tools.register(Arc::new(WebSearchTool::with_provider_and_max_results(
            network_config.web.search.provider.clone(),
            network_config.web.search.api_key.clone(),
            network_config.web.search.normalized_max_results(),
        )));
    }
    if network_config.web.fetch.enabled {
        tools.register(Arc::new(WebFetchTool::new()));
    }

    tools
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_ids_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for s in SUBAGENT_TOOL_CAPABILITY_SPECS {
            assert!(
                seen.insert(s.capability_id),
                "duplicate capability_id {}",
                s.capability_id
            );
        }
    }

    #[test]
    fn tool_names_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for s in SUBAGENT_TOOL_CAPABILITY_SPECS {
            assert!(
                seen.insert(s.tool_name),
                "duplicate tool_name {}",
                s.tool_name
            );
        }
    }

    #[test]
    fn default_network_registers_all_six_tools() {
        let tmp = tempfile::tempdir().unwrap();
        let net = NetworkToolConfig::default();
        let reg = build_subagent_tool_registry(tmp.path(), &net, true);
        for spec in SUBAGENT_TOOL_CAPABILITY_SPECS {
            assert!(
                reg.has(spec.tool_name),
                "expected {} ({})",
                spec.tool_name,
                spec.capability_id
            );
        }
    }

    #[test]
    fn web_tools_omit_when_disabled() {
        let tmp = tempfile::tempdir().unwrap();
        let mut net = NetworkToolConfig::default();
        net.web.search.enabled = false;
        net.web.fetch.enabled = false;
        let reg = build_subagent_tool_registry(tmp.path(), &net, true);
        assert!(!reg.has("web_search"));
        assert!(!reg.has("web_fetch"));
        assert!(reg.has("read_file"));
        assert!(reg.has("exec"));
    }
}
