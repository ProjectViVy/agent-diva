use crate::tool_config::{builtin::BuiltInToolsConfig, network::NetworkToolConfig};
use agent_diva_core::config::{MCPServerConfig, SubagentToolsConfig};
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
        Self::from(SubagentToolsConfig::default())
    }
}

impl From<SubagentToolsConfig> for SubagentPolicy {
    fn from(value: SubagentToolsConfig) -> Self {
        Self {
            max_concurrent: value.max_concurrent,
            max_depth: value.max_depth,
            allow_shell: value.allow_shell,
            allow_filesystem: value.allow_filesystem,
            allow_web_fetch: value.allow_web_fetch,
            allow_web_search: value.allow_web_search,
            allow_mcp: value.allow_mcp,
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
