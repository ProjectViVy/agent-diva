use agent_diva_core::config::schema::{
    Config, MentleToolConfig as CoreMentleToolConfig, MentleToolMode as CoreMentleToolMode,
};
use serde::{Deserialize, Serialize};

const READ_ONLY_TOOLS: &[&str] = &["memtle_status", "memtle_search"];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MentleToolRuntimeConfig {
    pub enabled: bool,
    pub mode: MentleToolMode,
    pub allowed_tools: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MentleToolMode {
    Off,
    ReadOnly,
    Full,
    Custom,
}

impl MentleToolRuntimeConfig {
    #[must_use]
    pub fn from_config(config: &Config) -> Self {
        let mut runtime = Self::from_core(&config.mentle);
        if runtime.is_default_off() && config.tools.builtin.mentle {
            runtime.enabled = true;
            runtime.mode = MentleToolMode::Full;
        }
        runtime
    }

    #[must_use]
    pub fn from_core(config: &CoreMentleToolConfig) -> Self {
        Self {
            enabled: config.enabled,
            mode: MentleToolMode::from(config.mode),
            allowed_tools: config.allowed_tools.clone(),
        }
    }

    #[must_use]
    pub fn is_active_request(&self) -> bool {
        self.enabled && self.mode != MentleToolMode::Off
    }

    #[must_use]
    pub fn allows_tool(&self, name: &str) -> bool {
        if !name.starts_with("memtle_") {
            return false;
        }
        match self.mode {
            MentleToolMode::Off => false,
            MentleToolMode::ReadOnly => READ_ONLY_TOOLS.contains(&name),
            MentleToolMode::Full => true,
            MentleToolMode::Custom => self
                .allowed_tools
                .iter()
                .any(|allowed| allowed == name && allowed.starts_with("memtle_")),
        }
    }

    #[must_use]
    pub fn selected_tool_names(
        &self,
        available_names: impl IntoIterator<Item = String>,
    ) -> Vec<String> {
        available_names
            .into_iter()
            .filter(|name| self.allows_tool(name))
            .collect()
    }

    fn is_default_off(&self) -> bool {
        !self.enabled && self.mode == MentleToolMode::Off && self.allowed_tools.is_empty()
    }
}

impl Default for MentleToolRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: MentleToolMode::Off,
            allowed_tools: Vec::new(),
        }
    }
}

impl From<CoreMentleToolMode> for MentleToolMode {
    fn from(value: CoreMentleToolMode) -> Self {
        match value {
            CoreMentleToolMode::Off => Self::Off,
            CoreMentleToolMode::ReadOnly => Self::ReadOnly,
            CoreMentleToolMode::Full => Self::Full,
            CoreMentleToolMode::Custom => Self::Custom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MentleToolMode, MentleToolRuntimeConfig};

    #[test]
    fn default_disables_mentle_tools() {
        let config = MentleToolRuntimeConfig::default();

        assert!(!config.is_active_request());
        assert!(!config.allows_tool("memtle_status"));
    }

    #[test]
    fn read_only_allows_status_and_search_only() {
        let config = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::ReadOnly,
            allowed_tools: Vec::new(),
        };

        assert!(config.allows_tool("memtle_status"));
        assert!(config.allows_tool("memtle_search"));
        assert!(!config.allows_tool("memtle_diary_write"));
    }

    #[test]
    fn custom_ignores_unknown_prefixes() {
        let config = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::Custom,
            allowed_tools: vec!["memtle_status".to_string(), "shell".to_string()],
        };

        assert!(config.allows_tool("memtle_status"));
        assert!(!config.allows_tool("shell"));
        assert!(!config.allows_tool("memtle_search"));
    }
}
