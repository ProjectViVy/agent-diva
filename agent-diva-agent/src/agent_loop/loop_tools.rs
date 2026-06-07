use super::{AgentLoop, ToolConfig};
use crate::planning::orchestrator::PlanOrchestrator;
use crate::planning::nag::NagTracker;
use crate::tool_config::network::NetworkToolConfig;
use agent_diva_core::config::MCPServerConfig;
use agent_diva_core::security::{SecurityConfig, SecurityLevel, SecurityPolicy};
use agent_diva_tools::{
    load_mcp_tools_sync, CronTool, EditFileTool, ExecTool, ListDirTool, ReadAttachmentTool,
    ReadFileTool, SpawnTool, ToolError, ToolRegistry, WebFetchTool, WebSearchTool, WriteFileTool,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

impl AgentLoop {
    /// Register default tools (for use after construction)
    pub fn register_default_tools(&mut self, tool_config: ToolConfig) {
        // Register spawn tool
        let sm = self.subagent_manager.clone();
        self.tools.register(Arc::new(SpawnTool::new(
            move |task, label, channel, chat_id| {
                let sm = sm.clone();
                async move {
                    sm.spawn(task, label, channel, chat_id)
                        .await
                        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
                }
            },
        )));

        // Register file system tools with SecurityPolicy
        let security_config = if tool_config.restrict_to_workspace {
            SecurityConfig {
                level: SecurityLevel::Standard,
                workspace_only: true,
                ..SecurityConfig::default()
            }
        } else {
            SecurityConfig::default()
        };
        let security = Arc::new(SecurityPolicy::with_config(
            self.workspace.clone(),
            security_config,
        ));
        self.tools
            .register(Arc::new(ReadFileTool::new(security.clone())));
        self.tools
            .register(Arc::new(WriteFileTool::new(security.clone())));
        self.tools
            .register(Arc::new(EditFileTool::new(security.clone())));
        self.tools.register(Arc::new(ListDirTool::new(security)));

        // Register attachment read tool with shared FileManager
        self.tools
            .register(Arc::new(ReadAttachmentTool::new(self.file_manager.clone())));

        // Register shell tool
        self.tools.register(Arc::new(ExecTool::with_config(
            tool_config.exec_timeout,
            Some(self.workspace.clone()),
            tool_config.restrict_to_workspace,
        )));

        // Register web tools
        Self::register_web_tools(&mut self.tools, &tool_config.network);

        // Register MCP tools discovered from configured servers
        for mcp_tool in load_mcp_tools_sync(&tool_config.mcp_servers) {
            self.tools.register(mcp_tool);
        }

        // Register cron tool when scheduling is configured
        if let Some(cron_service) = tool_config.cron_service {
            self.tools.register(Arc::new(CronTool::new(cron_service)));
        }

        // Register planning tools when planning store is configured
        if let Some(store) = tool_config.planning_store {
            let orch = Arc::new(Mutex::new(PlanOrchestrator::new()));
            // Store-only tools from agent-diva-tools
            use agent_diva_tools::planning::{PlanCreateTool, TodoShowTool, TodoWriteTool};
            self.tools.register(Arc::new(TodoShowTool::new(store.clone())));
            self.tools.register(Arc::new(TodoWriteTool::new(store.clone())));
            self.tools.register(Arc::new(PlanCreateTool::new(store.clone())));
            // Orchestrator-dependent tools from this crate
            use crate::planning::tools::{PlanApproveTool, PlanShowTool, PlanTransitionTool};
            self.tools.register(Arc::new(PlanApproveTool::new(orch.clone(), store.clone())));
            self.tools.register(Arc::new(PlanTransitionTool::new(orch.clone(), store.clone())));
            self.tools.register(Arc::new(PlanShowTool::new(store.clone())));
            // Set planning state
            self.planning_store = Some(store);
            self.orchestrator = Some(orch);
            self.nag_tracker = NagTracker::new();
        }
    }

    pub(super) fn register_web_tools(tools: &mut ToolRegistry, network: &NetworkToolConfig) {
        if network.web.search.enabled {
            tools.register(Arc::new(WebSearchTool::with_provider_and_max_results(
                network.web.search.provider.clone(),
                network.web.search.api_key.clone(),
                network.web.search.normalized_max_results(),
            )));
        }
        if network.web.fetch.enabled {
            tools.register(Arc::new(WebFetchTool::new()));
        }
    }

    pub(super) async fn apply_network_config(&mut self, network: NetworkToolConfig) {
        self.tools.unregister("web_search");
        self.tools.unregister("web_fetch");
        Self::register_web_tools(&mut self.tools, &network);
        self.subagent_manager.update_network_config(network).await;
        info!("Applied runtime network tool configuration update");
    }

    pub(super) async fn apply_mcp_config(&mut self, servers: HashMap<String, MCPServerConfig>) {
        for name in self
            .tools
            .tool_names()
            .into_iter()
            .filter(|name| name.starts_with("mcp_"))
        {
            self.tools.unregister(&name);
        }

        for mcp_tool in load_mcp_tools_sync(&servers) {
            self.tools.register(mcp_tool);
        }

        info!("Applied runtime MCP tool configuration update");
    }
}
