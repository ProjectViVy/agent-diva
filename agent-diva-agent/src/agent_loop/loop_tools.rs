use super::{AgentLoop, ToolConfig};
use crate::tool_assembly::{SubagentSpawner, ToolAssembly};
use crate::tool_config::network::NetworkToolConfig;
use agent_diva_core::config::MCPServerConfig;
use agent_diva_tooling::ToolError;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::info;

struct RuntimeSubagentSpawner {
    manager: Arc<crate::subagent::SubagentManager>,
}

#[async_trait::async_trait]
impl SubagentSpawner for RuntimeSubagentSpawner {
    async fn spawn(
        &self,
        task: String,
        label: Option<String>,
        channel: String,
        chat_id: String,
    ) -> Result<String, ToolError> {
        self.manager
            .spawn(task, label, channel, chat_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
    }
}

impl AgentLoop {
    /// Register default tools (for use after construction)
    pub fn register_default_tools(&mut self, tool_config: ToolConfig) {
        self.tool_config = tool_config.clone();
        let mut assembly = ToolAssembly::new(self.workspace.clone())
            .builtin(tool_config.builtin)
            .with_network_config(tool_config.network)
            .with_exec_timeout(tool_config.exec_timeout)
            .restrict_to_workspace(tool_config.restrict_to_workspace)
            .mcp_servers(tool_config.mcp_servers)
            .with_subagent_spawner(Arc::new(RuntimeSubagentSpawner {
                manager: self.subagent_manager.clone(),
            }))
            .with_file_manager(self.file_manager.clone());
        if let Some(cron_service) = tool_config.cron_service {
            assembly = assembly.with_cron_service(cron_service);
        }
        self.tools = assembly.build();
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
