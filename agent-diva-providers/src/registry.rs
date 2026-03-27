//! Provider registry - single source of truth for LLM provider metadata

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API specification type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ApiType {
    #[default]
    Openai,
    Anthropic,
    Google,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    #[default]
    ApiKey,
    #[serde(rename = "oauth")]
    OAuth,
    Token,
    DeviceFlow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CredentialStore {
    #[default]
    Config,
    ExternalSecureStore,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeBackend {
    #[default]
    OpenaiCompatible,
    OpenaiCodex,
}

/// One LLM provider's metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSpec {
    // Identity
    pub name: String,
    #[serde(default)]
    pub api_type: ApiType,
    pub keywords: Vec<String>,
    pub env_key: String,
    pub display_name: String,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub auth_mode: AuthMode,
    #[serde(default)]
    pub login_supported: bool,
    #[serde(default)]
    pub credential_store: CredentialStore,
    #[serde(default)]
    pub runtime_backend: RuntimeBackend,

    // Model prefixing
    pub litellm_prefix: String,
    pub skip_prefixes: Vec<String>,

    // Extra env vars
    pub env_extras: Vec<(String, String)>,

    pub default_api_base: String,

    // Prompt caching support
    #[serde(default)]
    pub supports_prompt_caching: bool,

    // Models list
    #[serde(default)]
    pub models: Vec<String>,

    // Per-model param overrides
    pub model_overrides: Vec<(String, HashMap<String, serde_json::Value>)>,
}

impl ProviderSpec {
    pub fn label(&self) -> String {
        if !self.display_name.is_empty() {
            self.display_name.clone()
        } else {
            let mut name = self.name.clone();
            if let Some(first_char) = name.chars().next() {
                name = first_char.to_uppercase().to_string() + &name[first_char.len_utf8()..];
            }
            name
        }
    }

    pub fn default_model(&self) -> Option<&str> {
        self.default_model
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
}

/// Registry of available LLM providers
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: Vec<ProviderSpec>,
}

impl ProviderRegistry {
    /// Create a new provider registry with default providers
    pub fn new() -> Self {
        Self {
            providers: Self::default_providers(),
        }
    }

    /// Get all provider specs
    pub fn all(&self) -> &[ProviderSpec] {
        &self.providers
    }

    /// Find a provider by model name (case-insensitive keyword matching)
    pub fn find_by_model(&self, model: &str) -> Option<&ProviderSpec> {
        let model_lower = model.to_lowercase();
        self.providers
            .iter()
            .find(|spec| spec.keywords.iter().any(|kw| model_lower.contains(kw)))
    }

    /// Find a provider by config field name
    pub fn find_by_name(&self, name: &str) -> Option<&ProviderSpec> {
        self.providers.iter().find(|spec| spec.name == name)
    }

    fn default_providers() -> Vec<ProviderSpec> {
        let yaml = include_str!("providers.yaml");
        serde_yaml::from_str(yaml).expect("Failed to parse default providers configuration")
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_by_model() {
        let registry = ProviderRegistry::new();

        // Test Claude
        let spec = registry.find_by_model("claude-3-opus");
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().name, "anthropic");

        // Test DeepSeek
        let spec = registry.find_by_model("deepseek-chat");
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().name, "deepseek");

        // Test Qwen
        let spec = registry.find_by_model("qwen-max");
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().name, "dashscope");

        // Test MiniMax
        let spec = registry.find_by_model("MiniMax-M2.1");
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().name, "minimax");
    }

    #[test]
    fn test_find_by_name() {
        let registry = ProviderRegistry::new();
        let spec = registry.find_by_name("anthropic");
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().display_name, "Anthropic");
    }

    #[test]
    fn openai_codex_metadata_is_oauth_enabled() {
        let registry = ProviderRegistry::new();
        let spec = registry.find_by_name("openai-codex").unwrap();
        assert_eq!(spec.auth_mode, AuthMode::OAuth);
        assert!(spec.login_supported);
        assert_eq!(spec.credential_store, CredentialStore::ExternalSecureStore);
        assert_eq!(spec.runtime_backend, RuntimeBackend::OpenaiCodex);
    }
}
