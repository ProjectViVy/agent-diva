use super::{build_agent_tools, AgentLoop, SubagentManagerSpawner, ToolConfig};
#[cfg(feature = "mentle")]
use crate::mentle_runtime::MentleRuntime;
use crate::tool_config::mentle::MentleToolRuntimeConfig;
use crate::tool_config::network::NetworkToolConfig;
use agent_diva_core::config::MCPServerConfig;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{info, warn};

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

    pub(super) async fn apply_mentle_config(
        &mut self,
        mentle: MentleToolRuntimeConfig,
        builtin_mentle: bool,
    ) {
        self.tool_config.mentle = mentle.clone();
        self.tool_config.builtin.mentle = builtin_mentle;

        for name in self
            .tools
            .tool_names()
            .into_iter()
            .filter(|name| name.starts_with("memtle_"))
        {
            self.tools.unregister(&name);
        }

        #[cfg(feature = "mentle")]
        {
            let mentle_requested = builtin_mentle && mentle.is_active_request();
            if mentle_requested {
                match MentleRuntime::try_build(&self.workspace, &mentle).await {
                    Some(runtime) => {
                        self.custom_tools = runtime.custom_tools();
                        self.mentle_active = runtime.active();
                        self.memory_provider = runtime.memory_provider();
                        self.mentle_runtime = Some(runtime);
                        if mentle_requested && !self.mentle_active {
                            warn!(
                                "Mentle prompt disabled after update: runtime tools do not contain memtle_status"
                            );
                        }
                    }
                    None => {
                        self.custom_tools.clear();
                        self.mentle_active = false;
                        self.mentle_runtime = None;
                        warn!(
                            "Mentle requested but runtime is unavailable after configuration update"
                        );
                    }
                }
            } else {
                self.custom_tools.clear();
                self.mentle_active = false;
                self.mentle_runtime = None;
            }

            for tool in &self.custom_tools {
                self.tools.register(tool.clone());
            }
        }

        #[cfg(not(feature = "mentle"))]
        {
            if builtin_mentle && mentle.is_active_request() {
                warn!("Mentle configuration updated but the agent-diva-agent `mentle` feature is disabled");
            }
            self.custom_tools.clear();
            self.mentle_active = false;
        }

        let mentle_tool_names = self
            .custom_tools
            .iter()
            .map(|tool| tool.name().to_string())
            .collect::<Vec<_>>();
        self.context
            .set_mentle_prompt_state(self.mentle_active, mentle_tool_names);

        if self.tool_config.cron_service.is_some() {
            self.register_default_tools(self.tool_config.clone());
        }

        info!("Applied runtime Mentle tool configuration update");
    }
}
