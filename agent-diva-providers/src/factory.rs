//! Provider construction helpers.

use std::collections::HashMap;
use std::sync::Arc;

use crate::anthropic::AnthropicClient;
use crate::base::{LLMProvider, ProviderError, ProviderResult};
use crate::discovery::ProviderAccess;
use crate::openai_compatible::OpenAiCompatibleClient;
use crate::registry::{ApiType, ProviderSpec};
use crate::tap::ProviderTap;

#[derive(Debug, Clone)]
pub struct LlmProviderBuildOptions {
    pub spec: ProviderSpec,
    pub access: ProviderAccess,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub reasoning_config: Option<agent_diva_core::reasoning::ReasoningConfig>,
    pub response_protocol: agent_diva_core::config::ProviderResponseProtocol,
}

pub fn build_llm_provider(
    options: LlmProviderBuildOptions,
) -> ProviderResult<Arc<dyn LLMProvider>> {
    let extra_headers = extra_headers(options.access.extra_headers);
    let provider_name = Some(options.spec.name.clone());

    if options.response_protocol
        == agent_diva_core::config::ProviderResponseProtocol::DeepseekV4Dsml
        && !options
            .model
            .trim()
            .to_ascii_lowercase()
            .contains("deepseek-v4")
    {
        return Err(ProviderError::ConfigError(
            "response_protocol=deepseek_v4_dsml requires a DeepSeek V4 model".to_string(),
        ));
    }

    match options.spec.api_type {
        ApiType::Openai => Ok(Arc::new(ProviderTap::new(
            OpenAiCompatibleClient::new_with_config(
                options.access.api_key,
                options.access.api_base,
                options.model,
                extra_headers,
                provider_name,
                options.reasoning_effort,
                options.reasoning_config,
                options.response_protocol,
            ),
        ))),
        ApiType::Anthropic => Ok(Arc::new(ProviderTap::new(AnthropicClient::new(
            options.access.api_key,
            options
                .access
                .api_base
                .or_else(|| non_empty(options.spec.default_api_base.clone())),
            options.model,
            extra_headers,
            provider_name,
        )))),
        ApiType::Google | ApiType::Other => Err(ProviderError::ConfigError(format!(
            "Provider '{}' api_type '{:?}' is not implemented",
            options.spec.name, options.spec.api_type
        ))),
    }
}

fn extra_headers(headers: Vec<(String, String)>) -> Option<HashMap<String, String>> {
    (!headers.is_empty()).then(|| headers.into_iter().collect())
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ApiType;

    fn spec(api_type: ApiType) -> ProviderSpec {
        ProviderSpec {
            name: "test".to_string(),
            api_type,
            keywords: vec!["test".to_string()],
            env_key: "TEST_API_KEY".to_string(),
            display_name: "Test".to_string(),
            default_model: Some("test-model".to_string()),
            gateway_prefix: String::new(),
            skip_prefixes: vec![],
            env_extras: vec![],
            default_api_base: "https://example.test".to_string(),
            supports_prompt_caching: false,
            models: vec![],
            model_overrides: vec![],
        }
    }

    fn access() -> ProviderAccess {
        ProviderAccess {
            api_key: Some("sk-test".to_string()),
            api_base: None,
            extra_headers: vec![],
        }
    }

    #[test]
    fn builds_openai_compatible_provider() {
        let provider = build_llm_provider(LlmProviderBuildOptions {
            spec: spec(ApiType::Openai),
            access: access(),
            model: "raw-model".to_string(),
            reasoning_effort: None,
            reasoning_config: None,
            response_protocol: agent_diva_core::config::ProviderResponseProtocol::OpenaiJson,
        })
        .unwrap();

        assert_eq!(provider.get_default_model(), "raw-model");
    }

    #[test]
    fn builds_anthropic_provider() {
        let provider = build_llm_provider(LlmProviderBuildOptions {
            spec: spec(ApiType::Anthropic),
            access: access(),
            model: "claude-sonnet-4-5".to_string(),
            reasoning_effort: None,
            reasoning_config: None,
            response_protocol: agent_diva_core::config::ProviderResponseProtocol::OpenaiJson,
        })
        .unwrap();

        assert_eq!(provider.get_default_model(), "claude-sonnet-4-5");
    }

    #[test]
    fn unsupported_api_type_returns_config_error() {
        let result = build_llm_provider(LlmProviderBuildOptions {
            spec: spec(ApiType::Google),
            access: access(),
            model: "gemini-native".to_string(),
            reasoning_effort: None,
            reasoning_config: None,
            response_protocol: agent_diva_core::config::ProviderResponseProtocol::OpenaiJson,
        });

        assert!(matches!(result, Err(ProviderError::ConfigError(_))));
    }

    #[test]
    fn dsml_protocol_requires_deepseek_v4_model() {
        let result = build_llm_provider(LlmProviderBuildOptions {
            spec: spec(ApiType::Openai),
            access: access(),
            model: "deepseek-chat".to_string(),
            reasoning_effort: None,
            reasoning_config: None,
            response_protocol: agent_diva_core::config::ProviderResponseProtocol::DeepseekV4Dsml,
        });

        assert!(matches!(result, Err(ProviderError::ConfigError(_))));
    }
}
