use serde::{Deserialize, Serialize};

use crate::subagent_policy::SubagentPolicy;

/// Built-in tool toggles shared by the main agent and nano runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuiltInToolsConfig {
    #[serde(default = "default_true")]
    pub filesystem: bool,
    #[serde(default = "default_true")]
    pub shell: bool,
    #[serde(default = "default_true")]
    pub web_search: bool,
    #[serde(default = "default_true")]
    pub web_fetch: bool,
    #[serde(default = "default_true")]
    pub spawn: bool,
    #[serde(default)]
    pub cron: bool,
    #[serde(default = "default_true")]
    pub mcp: bool,
    #[serde(default = "default_true")]
    pub attachment: bool,
}

fn default_true() -> bool {
    true
}

impl BuiltInToolsConfig {
    pub fn minimal() -> Self {
        Self {
            filesystem: true,
            shell: false,
            web_search: false,
            web_fetch: false,
            spawn: false,
            cron: false,
            mcp: false,
            attachment: false,
        }
    }

    pub fn none() -> Self {
        Self {
            filesystem: false,
            shell: false,
            web_search: false,
            web_fetch: false,
            spawn: false,
            cron: false,
            mcp: false,
            attachment: false,
        }
    }

    pub fn all() -> Self {
        Self {
            filesystem: true,
            shell: true,
            web_search: true,
            web_fetch: true,
            spawn: true,
            cron: true,
            mcp: true,
            attachment: true,
        }
    }

    pub fn for_subagent(&self, policy: &SubagentPolicy) -> Self {
        Self {
            filesystem: self.filesystem && policy.allow_filesystem,
            shell: self.shell && policy.allow_shell,
            web_search: self.web_search && policy.allow_web_search,
            web_fetch: self.web_fetch && policy.allow_web_fetch,
            spawn: false,
            cron: false,
            mcp: self.mcp && policy.allow_mcp,
            attachment: false,
        }
    }
}

impl Default for BuiltInToolsConfig {
    fn default() -> Self {
        Self {
            filesystem: true,
            shell: true,
            web_search: true,
            web_fetch: true,
            spawn: true,
            cron: false,
            mcp: true,
            attachment: true,
        }
    }
}
