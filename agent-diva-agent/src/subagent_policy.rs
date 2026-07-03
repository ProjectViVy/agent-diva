use crate::tool_config::{builtin::BuiltInToolsConfig, network::NetworkToolConfig};
use agent_diva_core::config::MCPServerConfig;
use std::collections::HashMap;

/// Runtime policy applied to every spawned subagent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentPolicy {
    pub max_concurrent: usize,
    pub max_depth: usize,
    pub allow_shell: bool,
    pub allow_filesystem: bool,
    pub allow_web_fetch: bool,
    pub allow_web_search: bool,
    pub allow_mcp: bool,
}

impl Default for SubagentPolicy {
    fn default() -> Self {
        Self {
            max_concurrent: 5,
            max_depth: 3,
            allow_shell: false,
            allow_filesystem: true,
            allow_web_fetch: false,
            allow_web_search: false,
            allow_mcp: false,
        }
    }
}

impl SubagentPolicy {
    pub fn builtin_tools(&self, parent: &BuiltInToolsConfig) -> BuiltInToolsConfig {
        parent.for_subagent(self)
    }

    pub fn network_config(&self, parent: &NetworkToolConfig) -> NetworkToolConfig {
        let mut config = parent.clone();
        if !self.allow_web_search {
            config.web.search.enabled = false;
            config.web.search.api_key = None;
        }
        if !self.allow_web_fetch {
            config.web.fetch.enabled = false;
        }
        config
    }

    pub fn mcp_servers(
        &self,
        parent: &HashMap<String, MCPServerConfig>,
    ) -> HashMap<String, MCPServerConfig> {
        if self.allow_mcp {
            parent.clone()
        } else {
            HashMap::new()
        }
    }
}
