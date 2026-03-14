use crate::discovery::{
    fetch_provider_model_catalog, ModelCatalogSource, ProviderAccess, ProviderModelCatalog,
};
use crate::registry::{ApiType, ProviderRegistry, ProviderSpec};
use agent_diva_core::config::{Config, CustomProviderConfig, ProviderConfig, ProvidersConfig};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSource {
    Builtin,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderModelSource {
    Runtime,
    Static,
    Custom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderView {
    pub id: String,
    pub display_name: String,
    pub source: ProviderSource,
    pub api_type: String,
    pub default_model: Option<String>,
    pub default_api_base: Option<String>,
    pub api_base: Option<String>,
    pub configured: bool,
    pub ready: bool,
    pub runtime_supported: bool,
    pub supports_model_discovery: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelEntry {
    pub id: String,
    pub source: ProviderModelSource,
    pub selectable: bool,
    pub deletable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderModelCatalogView {
    pub provider: String,
    pub catalog_source: String,
    pub runtime_supported: bool,
    pub api_base: Option<String>,
    pub models: Vec<ProviderModelEntry>,
    pub custom_models: Vec<String>,
    pub warnings: Vec<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderUpsert {
    pub id: String,
    pub display_name: String,
    pub api_type: String,
    pub api_key: String,
    pub api_base: Option<String>,
    pub default_model: Option<String>,
    pub models: Vec<String>,
    pub extra_headers: Option<std::collections::HashMap<String, String>>,
}

pub struct ProviderCatalogService {
    registry: ProviderRegistry,
}

impl ProviderCatalogService {
    pub fn new() -> Self {
        Self {
            registry: ProviderRegistry::new(),
        }
    }

    pub fn list_provider_views(&self, config: &Config) -> Vec<ProviderView> {
        let mut views: Vec<ProviderView> = self
            .registry
            .all()
            .iter()
            .map(|spec| self.provider_view_from_builtin(config, spec))
            .collect();

        for (id, provider) in &config.providers.custom_providers {
            if self.registry.find_by_name(id).is_some() {
                continue;
            }
            views.push(self.provider_view_from_custom(id, provider));
        }

        views.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        views
    }

    pub fn get_provider_view(&self, config: &Config, provider_id: &str) -> Option<ProviderView> {
        if let Some(spec) = self.registry.find_by_name(provider_id) {
            return Some(self.provider_view_from_builtin(config, spec));
        }
        config
            .providers
            .get_custom(provider_id)
            .map(|provider| self.provider_view_from_custom(provider_id, provider))
    }

    pub fn resolve_provider_id(
        &self,
        config: &Config,
        model: &str,
        preferred_provider: Option<&str>,
    ) -> Option<String> {
        preferred_provider
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .and_then(|value| {
                self.provider_exists(config, value)
                    .then(|| value.to_string())
            })
            .or_else(|| {
                config
                    .agents
                    .defaults
                    .provider
                    .as_deref()
                    .filter(|value| self.provider_exists(config, value))
                    .map(ToString::to_string)
            })
            .or_else(|| {
                model
                    .split('/')
                    .next()
                    .filter(|prefix| self.provider_exists(config, prefix))
                    .map(ToString::to_string)
            })
            .or_else(|| {
                self.registry
                    .find_by_model(model)
                    .map(|spec| spec.name.clone())
            })
    }

    pub fn get_provider_access(
        &self,
        config: &Config,
        provider_id: &str,
    ) -> Option<ProviderAccess> {
        if let Some(provider) = config.providers.get(provider_id) {
            return Some(ProviderAccess::from_config(Some(provider)));
        }
        config
            .providers
            .get_custom(provider_id)
            .map(provider_access_from_custom)
    }

    pub async fn list_provider_models(
        &self,
        config: &Config,
        provider_id: &str,
        include_runtime: bool,
        access_override: Option<ProviderAccess>,
    ) -> Result<ProviderModelCatalogView, String> {
        let spec = self
            .provider_spec(provider_id, &config.providers)
            .ok_or_else(|| format!("Unknown provider '{provider_id}'"))?;
        let custom_models = self.custom_models_for_provider(&config.providers, provider_id);

        let catalog = if include_runtime {
            let access = access_override
                .or_else(|| self.get_provider_access(config, provider_id))
                .unwrap_or_else(|| ProviderAccess::from_config(None));
            fetch_provider_model_catalog(&spec, &access, true).await
        } else {
            ProviderModelCatalog {
                provider: provider_id.to_string(),
                source: ModelCatalogSource::StaticFallback,
                runtime_supported: supports_runtime_discovery(&spec),
                api_base: self
                    .get_provider_access(config, provider_id)
                    .and_then(|access| access.api_base),
                models: spec.models.clone(),
                warnings: vec![],
                error: None,
            }
        };

        Ok(self.merge_model_catalog(catalog, &custom_models))
    }

    pub fn add_provider_model(
        &self,
        config: &mut Config,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), String> {
        let trimmed = model_id.trim();
        if trimmed.is_empty() {
            return Err("model id must not be empty".to_string());
        }

        if let Some(provider) = config.providers.get_mut(provider_id) {
            push_unique(&mut provider.custom_models, trimmed);
            return Ok(());
        }
        if let Some(provider) = config.providers.get_custom_mut(provider_id) {
            push_unique(&mut provider.models, trimmed);
            return Ok(());
        }

        Err(format!("Unknown provider '{provider_id}'"))
    }

    pub fn remove_provider_model(
        &self,
        config: &mut Config,
        provider_id: &str,
        model_id: &str,
    ) -> Result<(), String> {
        if let Some(provider) = config.providers.get_mut(provider_id) {
            remove_value(&mut provider.custom_models, model_id);
            return Ok(());
        }
        if let Some(provider) = config.providers.get_custom_mut(provider_id) {
            remove_value(&mut provider.models, model_id);
            if provider.default_model.as_deref() == Some(model_id) {
                provider.default_model = provider.models.first().cloned();
            }
            return Ok(());
        }

        Err(format!("Unknown provider '{provider_id}'"))
    }

    pub fn save_custom_provider(
        &self,
        config: &mut Config,
        provider: CustomProviderUpsert,
    ) -> Result<(), String> {
        let id = provider.id.trim();
        if id.is_empty() {
            return Err("provider id must not be empty".to_string());
        }
        if ProvidersConfig::is_builtin_provider(id) {
            return Err(format!(
                "provider id '{id}' is reserved for builtin providers"
            ));
        }
        if provider.api_type.trim().to_lowercase() != "openai" {
            return Err("custom providers currently only support api_type=openai".to_string());
        }

        let cfg = CustomProviderConfig {
            display_name: provider.display_name.trim().to_string(),
            api_type: provider.api_type.trim().to_lowercase(),
            api_key: provider.api_key,
            api_base: provider
                .api_base
                .map(|value| value.trim().trim_end_matches('/').to_string()),
            default_model: provider
                .default_model
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            models: dedupe_models(provider.models),
            extra_headers: provider.extra_headers.filter(|headers| !headers.is_empty()),
        };
        config
            .providers
            .custom_providers
            .insert(id.to_string(), cfg);
        Ok(())
    }

    pub fn delete_custom_provider(
        &self,
        config: &mut Config,
        provider_id: &str,
    ) -> Result<(), String> {
        if config
            .providers
            .custom_providers
            .remove(provider_id)
            .is_some()
        {
            return Ok(());
        }
        Err(format!("Unknown custom provider '{provider_id}'"))
    }

    fn provider_exists(&self, config: &Config, provider_id: &str) -> bool {
        self.registry.find_by_name(provider_id).is_some()
            || config.providers.get_custom(provider_id).is_some()
    }

    fn provider_spec(
        &self,
        provider_id: &str,
        providers: &ProvidersConfig,
    ) -> Option<ProviderSpec> {
        if let Some(spec) = self.registry.find_by_name(provider_id) {
            return Some(spec.clone());
        }
        providers
            .get_custom(provider_id)
            .map(|provider| custom_provider_spec(provider_id, provider))
    }

    fn custom_models_for_provider(
        &self,
        providers: &ProvidersConfig,
        provider_id: &str,
    ) -> Vec<String> {
        if let Some(provider) = providers.get(provider_id) {
            return dedupe_models(provider.custom_models.clone());
        }
        if let Some(provider) = providers.get_custom(provider_id) {
            return dedupe_models(provider.models.clone());
        }
        vec![]
    }

    fn provider_view_from_builtin(&self, config: &Config, spec: &ProviderSpec) -> ProviderView {
        let provider_config = config.providers.get(&spec.name);
        let shadow_config = config.providers.get_custom(&spec.name);
        let configured = provider_config
            .map(provider_configured)
            .or_else(|| shadow_config.map(shadow_provider_configured))
            .unwrap_or(false);
        let api_base = provider_config
            .and_then(|provider| provider.api_base.clone())
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                shadow_config
                    .and_then(|provider| provider.api_base.clone())
                    .filter(|value| !value.trim().is_empty())
            })
            .or_else(|| {
                (!spec.default_api_base.trim().is_empty()).then(|| spec.default_api_base.clone())
            });
        ProviderView {
            id: spec.name.clone(),
            display_name: spec.label(),
            source: ProviderSource::Builtin,
            api_type: api_type_label(&spec.api_type),
            default_model: spec.default_model().map(ToString::to_string),
            default_api_base: (!spec.default_api_base.trim().is_empty())
                .then(|| spec.default_api_base.clone()),
            api_base,
            configured,
            ready: configured,
            runtime_supported: supports_runtime_discovery(spec),
            supports_model_discovery: supports_runtime_discovery(spec),
        }
    }

    fn provider_view_from_custom(
        &self,
        provider_id: &str,
        provider: &CustomProviderConfig,
    ) -> ProviderView {
        let configured = custom_provider_configured(provider);
        ProviderView {
            id: provider_id.to_string(),
            display_name: if provider.display_name.trim().is_empty() {
                provider_id.to_string()
            } else {
                provider.display_name.clone()
            },
            source: ProviderSource::Custom,
            api_type: provider.api_type.trim().to_lowercase(),
            default_model: provider.default_model.clone(),
            default_api_base: provider.api_base.clone(),
            api_base: provider.api_base.clone(),
            configured,
            ready: configured,
            runtime_supported: provider.api_type.trim().eq_ignore_ascii_case("openai")
                && provider
                    .api_base
                    .as_ref()
                    .is_some_and(|value| !value.trim().is_empty()),
            supports_model_discovery: provider.api_type.trim().eq_ignore_ascii_case("openai"),
        }
    }

    fn merge_model_catalog(
        &self,
        catalog: ProviderModelCatalog,
        custom_models: &[String],
    ) -> ProviderModelCatalogView {
        let mut seen = BTreeSet::new();
        let runtime_source = match catalog.source {
            ModelCatalogSource::Runtime => ProviderModelSource::Runtime,
            _ => ProviderModelSource::Static,
        };
        let mut models = Vec::new();
        for model in &catalog.models {
            let trimmed = model.trim();
            if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
                continue;
            }
            models.push(ProviderModelEntry {
                id: trimmed.to_string(),
                source: runtime_source.clone(),
                selectable: true,
                deletable: false,
            });
        }
        for model in custom_models {
            let trimmed = model.trim();
            if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
                continue;
            }
            models.push(ProviderModelEntry {
                id: trimmed.to_string(),
                source: ProviderModelSource::Custom,
                selectable: true,
                deletable: true,
            });
        }

        ProviderModelCatalogView {
            provider: catalog.provider,
            catalog_source: model_catalog_source_label(&catalog.source).to_string(),
            runtime_supported: catalog.runtime_supported,
            api_base: catalog.api_base,
            models,
            custom_models: custom_models.to_vec(),
            warnings: catalog.warnings,
            error: catalog.error,
        }
    }
}

impl Default for ProviderCatalogService {
    fn default() -> Self {
        Self::new()
    }
}

fn provider_configured(provider: &ProviderConfig) -> bool {
    !provider.api_key.trim().is_empty()
        || provider
            .api_base
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn custom_provider_configured(provider: &CustomProviderConfig) -> bool {
    provider
        .api_base
        .as_ref()
        .is_some_and(|value| !value.trim().is_empty())
}

fn shadow_provider_configured(provider: &CustomProviderConfig) -> bool {
    !provider.api_key.trim().is_empty()
        || provider
            .api_base
            .as_ref()
            .is_some_and(|value| !value.trim().is_empty())
}

fn supports_runtime_discovery(spec: &ProviderSpec) -> bool {
    matches!(spec.api_type, ApiType::Openai)
}

fn api_type_label(api_type: &ApiType) -> String {
    match api_type {
        ApiType::Openai => "openai",
        ApiType::Anthropic => "anthropic",
        ApiType::Google => "google",
        ApiType::Other => "other",
    }
    .to_string()
}

fn model_catalog_source_label(source: &ModelCatalogSource) -> &'static str {
    match source {
        ModelCatalogSource::Runtime => "runtime",
        ModelCatalogSource::StaticFallback => "static_fallback",
        ModelCatalogSource::Unsupported => "unsupported",
        ModelCatalogSource::Error => "error",
    }
}

fn provider_access_from_custom(provider: &CustomProviderConfig) -> ProviderAccess {
    ProviderAccess {
        api_key: (!provider.api_key.trim().is_empty()).then(|| provider.api_key.clone()),
        api_base: provider
            .api_base
            .as_ref()
            .map(|value| value.trim().trim_end_matches('/').to_string())
            .filter(|value| !value.is_empty()),
        extra_headers: provider
            .extra_headers
            .as_ref()
            .map(|headers| {
                headers
                    .iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn custom_provider_spec(provider_id: &str, provider: &CustomProviderConfig) -> ProviderSpec {
    ProviderSpec {
        name: provider_id.to_string(),
        api_type: match provider.api_type.trim().to_lowercase().as_str() {
            "openai" => ApiType::Openai,
            "anthropic" => ApiType::Anthropic,
            "google" => ApiType::Google,
            _ => ApiType::Other,
        },
        keywords: vec![provider_id.to_string()],
        env_key: String::new(),
        display_name: if provider.display_name.trim().is_empty() {
            provider_id.to_string()
        } else {
            provider.display_name.clone()
        },
        default_model: provider.default_model.clone(),
        litellm_prefix: String::new(),
        skip_prefixes: vec![],
        env_extras: vec![],
        default_api_base: provider.api_base.clone().unwrap_or_default(),
        supports_prompt_caching: false,
        models: dedupe_models(provider.models.clone()),
        model_overrides: vec![],
    }
}

fn dedupe_models(models: Vec<String>) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut output = Vec::new();
    for model in models {
        let trimmed = model.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_string()) {
            continue;
        }
        output.push(trimmed.to_string());
    }
    output
}

fn push_unique(models: &mut Vec<String>, model: &str) {
    if !models.iter().any(|entry| entry == model) {
        models.push(model.to_string());
    }
}

fn remove_value(models: &mut Vec<String>, value: &str) {
    models.retain(|entry| entry != value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_provider_prefers_explicit_provider() {
        let mut config = Config::default();
        config.agents.defaults.provider = Some("deepseek".to_string());
        config
            .providers
            .custom_providers
            .insert("corp".to_string(), CustomProviderConfig::default());
        let service = ProviderCatalogService::new();

        let resolved = service.resolve_provider_id(&config, "deepseek-chat", Some("corp"));

        assert_eq!(resolved.as_deref(), Some("corp"));
    }

    #[test]
    fn add_remove_provider_model_updates_custom_models() {
        let mut config = Config::default();
        let service = ProviderCatalogService::new();
        service
            .add_provider_model(&mut config, "openai", "gpt-4.1-mini")
            .unwrap();
        assert_eq!(config.providers.openai.custom_models, vec!["gpt-4.1-mini"]);

        service
            .remove_provider_model(&mut config, "openai", "gpt-4.1-mini")
            .unwrap();
        assert!(config.providers.openai.custom_models.is_empty());
    }

    #[test]
    fn save_custom_provider_rejects_non_openai() {
        let service = ProviderCatalogService::new();
        let mut config = Config::default();

        let err = service
            .save_custom_provider(
                &mut config,
                CustomProviderUpsert {
                    id: "corp".to_string(),
                    display_name: "Corp".to_string(),
                    api_type: "anthropic".to_string(),
                    api_key: String::new(),
                    api_base: Some("https://example.com/v1".to_string()),
                    default_model: None,
                    models: vec![],
                    extra_headers: None,
                },
            )
            .unwrap_err();

        assert!(err.contains("only support"));
    }

    #[test]
    fn builtin_provider_view_uses_shadow_config_when_no_fixed_slot_exists() {
        let service = ProviderCatalogService::new();
        let mut config = Config::default();
        config.providers.custom_providers.insert(
            "github".to_string(),
            CustomProviderConfig {
                api_key: "ghp-test".to_string(),
                api_base: Some("https://models.inference.ai.azure.com".to_string()),
                ..Default::default()
            },
        );

        let view = service.get_provider_view(&config, "github").unwrap();

        assert!(view.configured);
        assert!(view.ready);
        assert_eq!(
            view.api_base.as_deref(),
            Some("https://models.inference.ai.azure.com")
        );
    }

    #[test]
    fn list_provider_views_skips_shadow_entries_for_builtin_names() {
        let service = ProviderCatalogService::new();
        let mut config = Config::default();
        config.providers.custom_providers.insert(
            "github".to_string(),
            CustomProviderConfig {
                api_key: "ghp-test".to_string(),
                api_base: Some("https://models.inference.ai.azure.com".to_string()),
                ..Default::default()
            },
        );

        let github_count = service
            .list_provider_views(&config)
            .into_iter()
            .filter(|view| view.id == "github")
            .count();

        assert_eq!(github_count, 1);
    }
}
