use super::{AgentLoop, ToolConfig};
use crate::tool_config::network::NetworkToolConfig;
use agent_diva_tools::{
    load_mcp_tools, CronTool, EditFileTool, ExecTool, ListDirTool, ReadFileTool, SpawnTool,
    ToolError, ToolRegistry, WebFetchTool, WebSearchTool, WriteFileTool,
};
use std::sync::Arc;
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

        // Register file system tools
        let allowed_dir = if tool_config.restrict_to_workspace {
            Some(self.workspace.clone())
        } else {
            None
        };
        self.tools
            .register(Arc::new(ReadFileTool::new(allowed_dir.clone())));
        self.tools
            .register(Arc::new(WriteFileTool::new(allowed_dir.clone())));
        self.tools
            .register(Arc::new(EditFileTool::new(allowed_dir.clone())));
        self.tools.register(Arc::new(ListDirTool::new(allowed_dir)));

        // Register shell tool
        self.tools.register(Arc::new(ExecTool::with_config(
            tool_config.exec_timeout,
            Some(self.workspace.clone()),
            tool_config.restrict_to_workspace,
        )));

        // Register web tools
        Self::register_web_tools(&mut self.tools, &tool_config.network);

        // Register MCP tools discovered from configured servers
        for mcp_tool in load_mcp_tools(&tool_config.mcp_servers) {
            self.tools.register(mcp_tool);
        }

        // Register cron tool when scheduling is configured
        if let Some(cron_service) = tool_config.cron_service {
            self.tools.register(Arc::new(CronTool::new(cron_service)));
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
}
