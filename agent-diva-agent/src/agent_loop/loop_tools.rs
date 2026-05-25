use super::{build_agent_tools, AgentLoop, SubagentManagerSpawner, ToolConfig};
use crate::tool_config::network::NetworkToolConfig;
use agent_diva_core::config::MCPServerConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

impl AgentLoop {
    /// Register default tools (for use after construction)
    pub fn register_default_tools(&mut self, tool_config: ToolConfig) {
        self.tool_config = tool_config.clone();
        self.tools = build_agent_tools(
            self.workspace.clone(),
            &self.tool_config,
            Arc::new(SubagentManagerSpawner {
                manager: self.subagent_manager.clone(),
            }),
            self.file_manager.clone(),
            self.custom_tools.clone(),
            self.tool_config.cron_service.clone(),
        );
    }

    pub(super) async fn apply_network_config(&mut self, network: NetworkToolConfig) {
        self.tool_config.network = network.clone();
        self.tools.unregister("web_search");
        self.tools.unregister("web_fetch");
        if self.tool_config.builtin.web_search && network.web.search.enabled {
            self.tools.register(Arc::new(
                agent_diva_tools::WebSearchTool::with_provider_and_max_results(
                    network.web.search.provider.clone(),
                    network.web.search.api_key.clone(),
                    network.web.search.normalized_max_results(),
                ),
            ));
        }
        if self.tool_config.builtin.web_fetch && network.web.fetch.enabled {
            self.tools
                .register(Arc::new(agent_diva_tools::WebFetchTool::new()));
        }
        self.subagent_manager.update_network_config(network).await;
        info!("Applied runtime network tool configuration update");
    }

    pub(super) async fn apply_mcp_config(&mut self, servers: HashMap<String, MCPServerConfig>) {
        self.tool_config.mcp_servers = servers.clone();
        for name in self
            .tools
            .tool_names()
            .into_iter()
            .filter(|name| name.starts_with("mcp_"))
        {
            self.tools.unregister(&name);
        }

        if self.tool_config.builtin.mcp {
            for mcp_tool in agent_diva_tools::load_mcp_tools_sync(&servers) {
                self.tools.register(mcp_tool);
            }
        }

        self.subagent_manager.update_mcp_servers(servers).await;

        info!("Applied runtime MCP tool configuration update");
    }
}
