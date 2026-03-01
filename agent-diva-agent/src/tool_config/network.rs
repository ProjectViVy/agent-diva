use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkToolConfig {
    #[serde(default)]
    pub web: WebRuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WebRuntimeConfig {
    #[serde(default)]
    pub search: WebSearchRuntimeConfig,
    #[serde(default)]
    pub fetch: WebFetchRuntimeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSearchRuntimeConfig {
    #[serde(default = "default_search_provider")]
    pub provider: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default = "default_max_results")]
    pub max_results: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebFetchRuntimeConfig {
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

fn default_enabled() -> bool {
    true
}

fn default_search_provider() -> String {
    "brave".to_string()
}

fn default_max_results() -> u32 {
    5
}

impl Default for WebSearchRuntimeConfig {
    fn default() -> Self {
        Self {
            provider: default_search_provider(),
            enabled: default_enabled(),
            api_key: None,
            max_results: default_max_results(),
        }
    }
}

impl Default for WebFetchRuntimeConfig {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
        }
    }
}

impl WebSearchRuntimeConfig {
    pub fn normalized_max_results(&self) -> usize {
        let max = if self.provider.eq_ignore_ascii_case("zhipu") {
            50
        } else {
            10
        };
        self.max_results.clamp(1, max) as usize
    }
}
