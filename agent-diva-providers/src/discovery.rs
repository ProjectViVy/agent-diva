use crate::registry::ProviderSpec;
use crate::registry::{ApiType, ProviderRegistry, RuntimeBackend};
use agent_diva_core::auth::{ProviderAuthKind, ProviderAuthService};
use agent_diva_core::config::ProviderConfig;
use anyhow::Result;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Path;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelCatalogSource {
    Runtime,
    StaticFallback,
    Unsupported,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderModelCatalog {
    pub provider: String,
    pub source: ModelCatalogSource,
    pub runtime_supported: bool,
    pub api_base: Option<String>,
    pub models: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProviderAccess {
    pub api_key: Option<String>,
    pub api_base: Option<String>,
    pub extra_headers: Vec<(String, String)>,
}

impl ProviderAccess {
    pub fn from_config(config: Option<&ProviderConfig>) -> Self {
        let api_key = config
            .map(|cfg| cfg.api_key.trim().to_string())
            .filter(|value| !value.is_empty());
        let api_base = config
            .and_then(|cfg| cfg.api_base.as_ref())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let extra_headers = config
            .and_then(|cfg| cfg.extra_headers.as_ref())
            .map(|headers| {
                headers
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default();

        Self {
            api_key,
            api_base,
            extra_headers,
        }
    }
}

pub async fn resolve_openai_compatible_oauth_access(
    config_dir: &Path,
    provider: &str,
    fallback: ProviderAccess,
) -> Result<ProviderAccess> {
    let registry = ProviderRegistry::new();
    let Some(spec) = registry.find_by_name(provider) else {
        return Ok(fallback);
    };
    if spec.runtime_backend != RuntimeBackend::OpenaiCompatible || !spec.login_supported {
        return Ok(fallback);
    }

    let auth = ProviderAuthService::new(config_dir);
    let Some(profile) = auth.get_active_profile(provider).await? else {
        return Ok(fallback);
    };

    let bearer = match profile.kind {
        ProviderAuthKind::OAuth => profile
            .token_set
            .as_ref()
            .map(|token_set| token_set.access_token.trim().to_string())
            .filter(|token| !token.is_empty()),
        ProviderAuthKind::Token => profile
            .token
            .as_ref()
            .map(|token| token.trim().to_string())
            .filter(|token| !token.is_empty()),
    };

    let api_base = profile
        .metadata
        .get("api_base")
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .or(fallback.api_base.clone());

    Ok(ProviderAccess {
        api_key: bearer.or(fallback.api_key),
        api_base,
        extra_headers: fallback.extra_headers,
    })
}

pub async fn fetch_provider_model_catalog(
    spec: &ProviderSpec,
    access: &ProviderAccess,
    allow_static_fallback: bool,
) -> ProviderModelCatalog {
    let api_base = effective_api_base(spec, access);

    match runtime_strategy(spec, &api_base) {
        Some(RuntimeDiscoveryStrategy::OpenAiCompatible) => {
            match fetch_openai_compatible_models(spec, access, api_base.as_deref()).await {
                Ok(models) => ProviderModelCatalog {
                    provider: spec.name.clone(),
                    source: ModelCatalogSource::Runtime,
                    runtime_supported: true,
                    api_base,
                    models,
                    warnings: vec![],
                    error: None,
                },
                Err(error) => fallback_or_error(spec, api_base, allow_static_fallback, error, true),
            }
        }
        None => {
            let message = format!(
                "Provider '{}' does not support runtime model discovery in this build",
                provider_identity(spec)
            );
            if allow_static_fallback && !spec.models.is_empty() {
                ProviderModelCatalog {
                    provider: spec.name.clone(),
                    source: ModelCatalogSource::StaticFallback,
                    runtime_supported: false,
                    api_base,
                    models: spec.models.clone(),
                    warnings: vec![],
                    error: None,
                }
            } else {
                ProviderModelCatalog {
                    provider: spec.name.clone(),
                    source: ModelCatalogSource::Unsupported,
                    runtime_supported: false,
                    api_base,
                    models: vec![],
                    warnings: vec![],
                    error: Some(message),
                }
            }
        }
    }
}

fn fallback_or_error(
    spec: &ProviderSpec,
    api_base: Option<String>,
    allow_static_fallback: bool,
    error: String,
    runtime_supported: bool,
) -> ProviderModelCatalog {
    if allow_static_fallback && !spec.models.is_empty() {
        ProviderModelCatalog {
            provider: spec.name.clone(),
            source: ModelCatalogSource::StaticFallback,
            runtime_supported,
            api_base,
            models: spec.models.clone(),
            warnings: vec![error],
            error: None,
        }
    } else {
        ProviderModelCatalog {
            provider: spec.name.clone(),
            source: ModelCatalogSource::Error,
            runtime_supported,
            api_base,
            models: vec![],
            warnings: vec![],
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum RuntimeDiscoveryStrategy {
    OpenAiCompatible,
}

fn runtime_strategy(
    spec: &ProviderSpec,
    api_base: &Option<String>,
) -> Option<RuntimeDiscoveryStrategy> {
    let supports_openai_compatible = matches!(spec.api_type, ApiType::Openai);

    if supports_openai_compatible && api_base.is_some() {
        Some(RuntimeDiscoveryStrategy::OpenAiCompatible)
    } else {
        None
    }
}

fn effective_api_base(spec: &ProviderSpec, access: &ProviderAccess) -> Option<String> {
    access
        .api_base
        .as_ref()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            (!spec.default_api_base.trim().is_empty()).then(|| {
                spec.default_api_base
                    .trim()
                    .trim_end_matches('/')
                    .to_string()
            })
        })
}

async fn fetch_openai_compatible_models(
    spec: &ProviderSpec,
    access: &ProviderAccess,
    api_base: Option<&str>,
) -> Result<Vec<String>, String> {
    let api_base = api_base.ok_or_else(|| {
        format!(
            "Provider '{}' has no configured or default api_base for runtime discovery",
            provider_identity(spec)
        )
    })?;
    let client = Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|error| format!("failed to create HTTP client: {error}"))?;
    let url = format!("{api_base}/models");

    let mut request = client.get(&url);
    if let Some(api_key) = access
        .api_key
        .as_ref()
        .filter(|value| !value.trim().is_empty())
    {
        request = request.bearer_auth(api_key);
    }
    for (key, value) in &access.extra_headers {
        request = request.header(key, value);
    }

    let response = request.send().await.map_err(|error| {
        format!(
            "Runtime model discovery request failed for provider '{}' at '{}': {}",
            provider_identity(spec),
            url,
            error
        )
    })?;

    let status = response.status();
    if !status.is_success() {
        let detail = response.text().await.unwrap_or_default();
        let summary = http_error_summary(status, &detail);
        return Err(format!(
            "Runtime model discovery request returned {} for provider '{}' at '{}': {}",
            status,
            provider_identity(spec),
            url,
            summary
        ));
    }

    let payload: OpenAiModelsResponse = response.json().await.map_err(|error| {
        format!(
            "Runtime model discovery response was invalid JSON for provider '{}' at '{}': {}",
            provider_identity(spec),
            url,
            error
        )
    })?;

    let mut unique_models = BTreeSet::new();
    for model in payload.data {
        let id = model.id.trim();
        if !id.is_empty() {
            unique_models.insert(id.to_string());
        }
    }

    Ok(unique_models.into_iter().collect())
}

fn http_error_summary(status: StatusCode, detail: &str) -> String {
    let trimmed = detail.trim();
    if trimmed.is_empty() {
        return "empty response body".to_string();
    }

    let compact = trimmed.replace('\n', " ");
    let preview: String = compact.chars().take(200).collect();
    if compact.chars().count() > 200 {
        format!("{status}: {preview}...")
    } else {
        format!("{status}: {preview}")
    }
}

fn provider_identity(spec: &ProviderSpec) -> &str {
    if spec.name.trim().is_empty() {
        let display_name = spec.display_name.trim();
        if !display_name.is_empty() {
            display_name
        } else {
            "<unknown>"
        }
    } else {
        spec.name.as_str()
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    #[serde(default)]
    data: Vec<OpenAiModelEntry>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModelEntry {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        registry::{ApiType, AuthMode, CredentialStore, RuntimeBackend},
        ProviderSpec,
    };
    use agent_diva_core::auth::{OAuthProfileState, ProviderAuthService, ProviderTokenSet};
    use mockito::{Matcher, Server};
    use std::collections::HashMap;
    use tempfile::tempdir;

    fn openai_like_spec(name: &str, api_base: &str) -> ProviderSpec {
        ProviderSpec {
            name: name.to_string(),
            api_type: ApiType::Openai,
            keywords: vec![name.to_string()],
            env_key: "TEST_API_KEY".to_string(),
            display_name: name.to_string(),
            default_model: None,
            auth_mode: AuthMode::ApiKey,
            login_supported: false,
            credential_store: CredentialStore::Config,
            runtime_backend: RuntimeBackend::OpenaiCompatible,
            litellm_prefix: String::new(),
            skip_prefixes: vec![],
            env_extras: vec![],
            default_api_base: api_base.to_string(),
            supports_prompt_caching: false,
            models: vec!["fallback-a".to_string(), "fallback-b".to_string()],
            model_overrides: vec![],
        }
    }

    #[tokio::test]
    async fn fetch_provider_model_catalog_returns_runtime_models_for_openai_compatible() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/models")
            .match_header("authorization", "Bearer sk-test")
            .match_header("x-test", "1")
            .with_status(200)
            .with_body(r#"{"data":[{"id":"gpt-4o"},{"id":"gpt-4o-mini"},{"id":"gpt-4o"}]}"#)
            .create_async()
            .await;
        let spec = openai_like_spec("openai", &server.url());
        let access = ProviderAccess {
            api_key: Some("sk-test".to_string()),
            api_base: None,
            extra_headers: vec![("x-test".to_string(), "1".to_string())],
        };

        let catalog = fetch_provider_model_catalog(&spec, &access, false).await;

        mock.assert_async().await;
        assert_eq!(catalog.source, ModelCatalogSource::Runtime);
        assert_eq!(
            catalog.models,
            vec!["gpt-4o".to_string(), "gpt-4o-mini".to_string()]
        );
        assert_eq!(catalog.error, None);
    }

    #[tokio::test]
    async fn fetch_provider_model_catalog_uses_static_fallback_on_http_error() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/models")
            .with_status(401)
            .with_body(r#"{"error":"unauthorized"}"#)
            .create_async()
            .await;
        let spec = openai_like_spec("openai", &server.url());

        let catalog = fetch_provider_model_catalog(
            &spec,
            &ProviderAccess {
                api_key: Some("sk-bad".to_string()),
                api_base: None,
                extra_headers: vec![],
            },
            true,
        )
        .await;

        mock.assert_async().await;
        assert_eq!(catalog.source, ModelCatalogSource::StaticFallback);
        assert_eq!(catalog.models, spec.models);
        assert!(catalog.warnings[0].contains("401 Unauthorized"));
    }

    #[tokio::test]
    async fn fetch_provider_model_catalog_returns_unsupported_without_fallback() {
        let spec = ProviderSpec {
            name: "anthropic".to_string(),
            api_type: ApiType::Anthropic,
            keywords: vec!["claude".to_string()],
            env_key: "ANTHROPIC_API_KEY".to_string(),
            display_name: "Anthropic".to_string(),
            default_model: None,
            auth_mode: AuthMode::ApiKey,
            login_supported: false,
            credential_store: CredentialStore::Config,
            runtime_backend: RuntimeBackend::OpenaiCompatible,
            litellm_prefix: String::new(),
            skip_prefixes: vec![],
            env_extras: vec![],
            default_api_base: String::new(),
            supports_prompt_caching: false,
            models: vec![],
            model_overrides: vec![],
        };

        let catalog =
            fetch_provider_model_catalog(&spec, &ProviderAccess::from_config(None), false).await;

        assert_eq!(catalog.source, ModelCatalogSource::Unsupported);
        assert!(catalog.error.unwrap().contains("does not support"));
    }

    #[tokio::test]
    async fn fetch_provider_model_catalog_supports_generic_openai_type_with_api_base() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/models")
            .with_status(200)
            .with_body(r#"{"data":[{"id":"grok-2-latest"}]}"#)
            .create_async()
            .await;
        let spec = openai_like_spec("xai", &server.url());

        let catalog = fetch_provider_model_catalog(
            &spec,
            &ProviderAccess {
                api_key: None,
                api_base: Some(server.url()),
                extra_headers: vec![],
            },
            false,
        )
        .await;

        mock.assert_async().await;
        assert_eq!(catalog.source, ModelCatalogSource::Runtime);
        assert_eq!(catalog.models, vec!["grok-2-latest".to_string()]);
    }

    #[test]
    fn provider_access_from_config_filters_empty_values() {
        let access = ProviderAccess::from_config(Some(&ProviderConfig {
            api_key: "  ".to_string(),
            api_base: Some("  https://example.com/v1/ ".to_string()),
            extra_headers: Some(HashMap::from([
                ("x-a".to_string(), "1".to_string()),
                ("x-b".to_string(), "2".to_string()),
            ])),
            custom_models: vec![],
        }));

        assert_eq!(access.api_key, None);
        assert_eq!(access.api_base, Some("https://example.com/v1/".to_string()));
        assert_eq!(access.extra_headers.len(), 2);
    }

    #[tokio::test]
    async fn fetch_provider_model_catalog_prefers_configured_api_base() {
        let mut server = Server::new_async().await;
        let mock = server
            .mock("GET", "/models")
            .match_query(Matcher::Missing)
            .with_status(200)
            .with_body(r#"{"data":[{"id":"model-a"}]}"#)
            .create_async()
            .await;
        let spec = openai_like_spec("custom", "http://unused.example/v1");
        let access = ProviderAccess {
            api_key: None,
            api_base: Some(server.url()),
            extra_headers: vec![],
        };

        let catalog = fetch_provider_model_catalog(&spec, &access, false).await;

        mock.assert_async().await;
        assert_eq!(catalog.source, ModelCatalogSource::Runtime);
        assert_eq!(catalog.api_base, Some(server.url()));
    }

    #[tokio::test]
    async fn resolve_openai_compatible_oauth_access_prefers_auth_store_for_qwen_login() {
        let dir = tempdir().unwrap();
        let auth = ProviderAuthService::new(dir.path());
        auth.store_oauth_profile(
            "qwen-login",
            "default",
            OAuthProfileState {
                token_set: ProviderTokenSet {
                    access_token: "oauth-bearer".to_string(),
                    refresh_token: Some("refresh".to_string()),
                    id_token: None,
                    expires_at: None,
                    token_type: Some("Bearer".to_string()),
                    scope: Some("openid".to_string()),
                },
                account_id: None,
                metadata: HashMap::from([(
                    "api_base".to_string(),
                    "https://oauth.example/v1".to_string(),
                )])
                .into_iter()
                .collect(),
            },
            true,
        )
        .await
        .unwrap();

        let access = resolve_openai_compatible_oauth_access(
            dir.path(),
            "qwen-login",
            ProviderAccess {
                api_key: Some("config-api-key".to_string()),
                api_base: Some("https://config.example/v1".to_string()),
                extra_headers: vec![],
            },
        )
        .await
        .unwrap();

        assert_eq!(access.api_key.as_deref(), Some("oauth-bearer"));
        assert_eq!(access.api_base.as_deref(), Some("https://oauth.example/v1"));
    }

    #[tokio::test]
    async fn resolve_openai_compatible_oauth_access_keeps_dashscope_on_config_path() {
        let dir = tempdir().unwrap();
        let access = resolve_openai_compatible_oauth_access(
            dir.path(),
            "dashscope",
            ProviderAccess {
                api_key: Some("dashscope-key".to_string()),
                api_base: Some("https://dashscope.aliyuncs.com/compatible-mode/v1".to_string()),
                extra_headers: vec![],
            },
        )
        .await
        .unwrap();

        assert_eq!(access.api_key.as_deref(), Some("dashscope-key"));
        assert_eq!(
            access.api_base.as_deref(),
            Some("https://dashscope.aliyuncs.com/compatible-mode/v1")
        );
    }
}
