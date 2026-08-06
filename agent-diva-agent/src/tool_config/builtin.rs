use serde::{Deserialize, Serialize};

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
    #[serde(default = "default_true")]
    pub enqueue_background_task: bool,
    #[serde(default = "default_true")]
    pub update_plan: bool,
    #[serde(default = "default_true")]
    pub ask_user: bool,
    #[serde(default = "default_true")]
    pub memory: bool,
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
            enqueue_background_task: false,
            update_plan: false,
            ask_user: false,
            memory: false,
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
            enqueue_background_task: false,
            update_plan: false,
            ask_user: false,
            memory: false,
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
            enqueue_background_task: true,
            update_plan: true,
            ask_user: true,
            memory: true,
        }
    }

    pub fn for_subagent(&self) -> Self {
        Self {
            filesystem: self.filesystem,
            shell: self.shell,
            web_search: self.web_search,
            web_fetch: self.web_fetch,
            spawn: false,
            cron: false,
            mcp: self.mcp,
            attachment: false,
            enqueue_background_task: false,
            update_plan: false,
            ask_user: false,
            memory: false,
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
            enqueue_background_task: false,
            update_plan: true,
            ask_user: true,
            memory: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::BuiltInToolsConfig;

    #[test]
    fn subagent_configuration_removes_recursive_and_control_plane_tools() {
        let config = BuiltInToolsConfig::all().for_subagent();
        assert!(!config.spawn);
        assert!(!config.cron);
        assert!(!config.enqueue_background_task);
        assert!(!config.update_plan);
        assert!(!config.attachment);
        assert!(!config.ask_user);
        assert!(config.filesystem);
        assert!(config.shell);
        assert!(config.web_search);
        assert!(config.web_fetch);
    }
}
