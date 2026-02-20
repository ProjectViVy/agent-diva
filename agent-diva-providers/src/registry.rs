//! Provider registry - single source of truth for LLM provider metadata

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// API specification type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiType {
    Openai,
    Anthropic,
    Google,
    Other,
}

impl Default for ApiType {
    fn default() -> Self {
        Self::Openai
    }
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

    // Model prefixing
    pub litellm_prefix: String,
    pub skip_prefixes: Vec<String>,

    // Extra env vars
    pub env_extras: Vec<(String, String)>,

    // Gateway / local detection
    pub is_gateway: bool,
    pub is_local: bool,
    pub detect_by_key_prefix: String,
    pub detect_by_base_keyword: String,
    pub default_api_base: String,

    // Gateway behavior
    pub strip_model_prefix: bool,

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
}

/// Registry of available LLM providers
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
            .filter(|spec| !spec.is_gateway && !spec.is_local)
            .find(|spec| spec.keywords.iter().any(|kw| model_lower.contains(kw)))
    }

    /// Find a gateway/local provider
    pub fn find_gateway(
        &self,
        provider_name: Option<&str>,
        api_key: Option<&str>,
        api_base: Option<&str>,
    ) -> Option<&ProviderSpec> {
        // 1. Direct match by config key
        if let Some(name) = provider_name {
            if let Some(spec) = self.find_by_name(name) {
                if spec.is_gateway || spec.is_local {
                    return Some(spec);
                }
            }
        }

        // 2. Auto-detect by api_key prefix / api_base keyword
        for spec in &self.providers {
            if !spec.detect_by_key_prefix.is_empty() {
                if let Some(key) = api_key {
                    if key.starts_with(&spec.detect_by_key_prefix) {
                        return Some(spec);
                    }
                }
            }
            if !spec.detect_by_base_keyword.is_empty() {
                if let Some(base) = api_base {
                    if base.contains(&spec.detect_by_base_keyword) {
                        return Some(spec);
                    }
                }
            }
        }

        None
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
    fn test_find_gateway() {
        let registry = ProviderRegistry::new();

        // Test OpenRouter by api_key prefix
        let spec = registry.find_gateway(None, Some("sk-or-v1-abc123"), None);
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().name, "openrouter");

        // Test AiHubMix by api_base keyword
        let spec = registry.find_gateway(None, None, Some("https://aihubmix.com/v1"));
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().name, "aihubmix");

        // Test vLLM by provider_name
        let spec = registry.find_gateway(Some("vllm"), None, None);
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().name, "vllm");
    }

    #[test]
    fn test_find_by_name() {
        let registry = ProviderRegistry::new();
        let spec = registry.find_by_name("anthropic");
        assert!(spec.is_some());
        assert_eq!(spec.unwrap().display_name, "Anthropic");
    }
}
