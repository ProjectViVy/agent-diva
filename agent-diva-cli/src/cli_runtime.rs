use agent_diva_agent::swarm_doctor::swarm_cortex_doctor_section_for_diagnostics;
pub use agent_diva_agent::swarm_doctor::SwarmCortexDoctorV1;
use agent_diva_core::config::validate::validate_config;
use agent_diva_core::config::{Config, ConfigLoader, ProviderConfig, ProvidersConfig};
use agent_diva_core::cron::CronService;
use agent_diva_core::utils::sync_workspace_templates;
use agent_diva_providers::{
    fetch_provider_model_catalog, LiteLLMClient, ProviderAccess, ProviderCatalogService,
    ProviderModelCatalog, ProviderRegistry, ProviderSpec,
};
use anyhow::Result;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct CliRuntime {
    loader: ConfigLoader,
    workspace_override: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PathReport {
    pub config_path: String,
    pub config_dir: String,
    pub runtime_dir: String,
    pub workspace: String,
    pub cron_store: String,
    pub bridge_dir: String,
    pub whatsapp_auth_dir: String,
    pub whatsapp_media_dir: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProviderStatus {
    pub name: String,
    pub display_name: String,
    pub default_model: Option<String>,
    pub configurable: bool,
    pub configured: bool,
    pub ready: bool,
    pub uses_api_base: bool,
    pub provider_for_default_model: bool,
    pub current: bool,
    pub model: Option<String>,
    pub api_base: Option<String>,
    pub missing_fields: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ChannelStatus {
    pub name: String,
    pub enabled: bool,
    pub ready: bool,
    pub missing_fields: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorReport {
    pub valid: bool,
    pub ready: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub provider: Option<String>,
    pub channels: Vec<ChannelStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swarm_cortex: Option<SwarmCortexDoctorV1>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProviderStatusReport {
    pub current_model: String,
    pub current_provider: Option<String>,
    pub providers: Vec<ProviderStatus>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusReport {
    pub config: PathReport,
    pub default_model: String,
    pub default_provider: Option<String>,
    pub logging: StatusLoggingReport,
    pub providers: Vec<ProviderStatus>,
    pub channels: Vec<ChannelStatus>,
    pub cron_jobs: usize,
    pub mcp_servers: StatusMcpReport,
    pub doctor: StatusDoctorSummary,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusLoggingReport {
    pub level: String,
    pub format: String,
    pub dir: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusMcpReport {
    pub configured: usize,
    pub disabled: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct StatusDoctorSummary {
    pub valid: bool,
    pub ready: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub swarm_cortex: Option<SwarmCortexDoctorV1>,
}

pub fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

impl CliRuntime {
    pub fn from_paths(
        config: Option<PathBuf>,
        config_dir: Option<PathBuf>,
        workspace_override: Option<PathBuf>,
    ) -> Self {
        let loader = if let Some(path) = config {
            ConfigLoader::with_file(path)
        } else if let Some(dir) = config_dir {
            ConfigLoader::with_dir(dir)
        } else {
            ConfigLoader::new()
        };

        Self {
            loader,
            workspace_override,
        }
    }

    pub fn loader(&self) -> &ConfigLoader {
        &self.loader
    }

    pub fn config_path(&self) -> &Path {
        self.loader.config_path()
    }

    pub fn config_dir(&self) -> &Path {
        self.loader.config_dir()
    }

    pub fn runtime_dir(&self) -> &Path {
        self.loader.config_dir()
    }

    pub fn load_config(&self) -> Result<Config> {
        Ok(self.loader.load()?)
    }

    pub fn effective_workspace(&self, config: &Config) -> PathBuf {
        if let Some(workspace) = &self.workspace_override {
            workspace.clone()
        } else {
            expand_tilde(&config.agents.defaults.workspace)
        }
    }

    pub fn cron_store_path(&self) -> PathBuf {
        self.config_dir()
            .join("data")
            .join("cron")
            .join("jobs.json")
    }

    pub fn bridge_dir(&self) -> PathBuf {
        self.config_dir().join("bridge")
    }

    pub fn whatsapp_auth_dir(&self) -> PathBuf {
        self.config_dir().join("whatsapp-auth")
    }

    pub fn whatsapp_media_dir(&self) -> PathBuf {
        self.config_dir().join("whatsapp-media")
    }

    pub fn path_report(&self, config: &Config) -> PathReport {
        PathReport {
            config_path: self.config_path().display().to_string(),
            config_dir: self.config_dir().display().to_string(),
            runtime_dir: self.runtime_dir().display().to_string(),
            workspace: self.effective_workspace(config).display().to_string(),
            cron_store: self.cron_store_path().display().to_string(),
            bridge_dir: self.bridge_dir().display().to_string(),
            whatsapp_auth_dir: self.whatsapp_auth_dir().display().to_string(),
            whatsapp_media_dir: self.whatsapp_media_dir().display().to_string(),
        }
    }
}

pub fn provider_config_by_name<'a>(
    providers: &'a ProvidersConfig,
    name: &str,
) -> Option<&'a ProviderConfig> {
    providers.get(name)
}

pub fn provider_config_by_name_mut<'a>(
    providers: &'a mut ProvidersConfig,
    name: &str,
) -> Option<&'a mut ProviderConfig> {
    providers.get_mut(name)
}

pub fn provider_has_config_slot(name: &str) -> bool {
    ProvidersConfig::is_builtin_provider(name)
}

pub fn provider_registry() -> ProviderRegistry {
    ProviderRegistry::new()
}

pub fn manageable_provider_specs() -> Vec<ProviderSpec> {
    provider_registry()
        .all()
        .iter()
        .filter(|spec| provider_has_config_slot(&spec.name))
        .cloned()
        .collect()
}

pub fn provider_spec_by_name(name: &str) -> Option<ProviderSpec> {
    manageable_provider_specs()
        .into_iter()
        .find(|spec| spec.name == name)
}

pub fn default_model_from_registry(provider_name: &str) -> Option<String> {
    provider_registry()
        .find_by_name(provider_name)
        .and_then(|spec| spec.default_model().map(ToString::to_string))
}

pub fn infer_provider_name_from_model(model: &str) -> Option<String> {
    let registry = provider_registry();
    model
        .split('/')
        .next()
        .and_then(|prefix| registry.find_by_name(prefix))
        .or_else(|| registry.find_by_model(model))
        .map(|spec| spec.name.clone())
}

pub fn current_provider_name(config: &Config) -> Option<String> {
    let preferred_provider = config
        .agents
        .defaults
        .provider
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let inferred_provider = infer_provider_name_from_model(&config.agents.defaults.model);

    if let Some(provider_name) = preferred_provider {
        if config.providers.get_custom(provider_name).is_some() {
            return Some(provider_name.to_string());
        }
        if inferred_provider
            .as_deref()
            .is_some_and(|inferred| inferred != provider_name)
        {
            return inferred_provider;
        }
        if ProviderCatalogService::new()
            .get_provider_view(config, provider_name)
            .is_some()
        {
            return Some(provider_name.to_string());
        }
    }

    inferred_provider
}

pub fn resolve_provider_name_for_model(
    config: &Config,
    model: &str,
    preferred_provider: Option<&str>,
) -> Option<String> {
    let preferred_provider = preferred_provider
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let inferred_provider = infer_provider_name_from_model(model);

    if let Some(provider_name) = preferred_provider {
        if config.providers.get_custom(provider_name).is_some() {
            return Some(provider_name.to_string());
        }
        if inferred_provider
            .as_deref()
            .is_some_and(|inferred| inferred != provider_name)
        {
            return inferred_provider;
        }
        if let Some(spec) = provider_spec_by_name(provider_name) {
            return Some(spec.name);
        }
    }

    inferred_provider.or_else(|| {
        (model == config.agents.defaults.model)
            .then(|| current_provider_name(config))
            .flatten()
    })
}

pub fn session_channel_and_chat_id(session_key: &str) -> (&str, &str) {
    session_key.split_once(':').unwrap_or(("cli", session_key))
}

pub fn build_provider(config: &Config, model: &str) -> Result<LiteLLMClient> {
    let catalog = ProviderCatalogService::new();
    let provider_name = resolve_provider_name_for_model(
        config,
        model,
        (model == config.agents.defaults.model)
            .then_some(config.agents.defaults.provider.as_deref())
            .flatten(),
    )
    .ok_or_else(|| anyhow::anyhow!("No provider found for model: {}", model))?;
    let access = catalog
        .get_provider_access(config, &provider_name)
        .unwrap_or_else(|| ProviderAccess::from_config(None));
    let api_key = access.api_key;
    let api_base = access.api_base;
    let extra_headers = (!access.extra_headers.is_empty()).then(|| {
        access
            .extra_headers
            .into_iter()
            .collect::<std::collections::HashMap<String, String>>()
    });

    Ok(LiteLLMClient::new(
        api_key,
        api_base,
        model.to_string(),
        extra_headers,
        Some(provider_name),
        config.agents.defaults.reasoning_effort.clone(),
    ))
}

pub fn set_provider_credentials(
    config: &mut Config,
    provider_name: &str,
    api_key: Option<String>,
    api_base: Option<String>,
) {
    if let Some(provider) = provider_config_by_name_mut(&mut config.providers, provider_name) {
        if let Some(api_key) = api_key {
            provider.api_key = api_key;
        }
        if api_base.is_some() {
            provider.api_base = api_base;
        }
    }
}

pub fn available_provider_names() -> Vec<String> {
    ProvidersConfig::builtin_provider_names()
        .iter()
        .map(|name| (*name).to_string())
        .collect()
}

pub fn provider_access_by_name(config: &Config, provider_name: &str) -> ProviderAccess {
    ProviderCatalogService::new()
        .get_provider_access(config, provider_name)
        .unwrap_or_else(|| ProviderAccess::from_config(None))
}

pub async fn fetch_provider_models(
    config: &Config,
    provider_name: &str,
    allow_static_fallback: bool,
) -> Result<ProviderModelCatalog> {
    let spec = provider_registry()
        .find_by_name(provider_name)
        .cloned()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "Unknown or unmanaged provider '{}'. Supported: {}",
                provider_name,
                available_provider_names().join(", ")
            )
        })?;
    let access = provider_access_by_name(config, provider_name);

    Ok(fetch_provider_model_catalog(&spec, &access, allow_static_fallback).await)
}

pub fn ensure_workspace_templates(workspace: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(workspace)?;
    std::fs::create_dir_all(workspace.join("skills"))?;
    Ok(sync_workspace_templates(workspace)?)
}

pub fn redact_sensitive_value(key: &str, value: &mut serde_json::Value) {
    let lowered = key.to_ascii_lowercase();
    let looks_sensitive = ["api_key", "token", "secret", "password"]
        .iter()
        .any(|segment| lowered.contains(segment));

    match value {
        serde_json::Value::Object(map) => {
            for (nested_key, nested_value) in map.iter_mut() {
                redact_sensitive_value(nested_key, nested_value);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                redact_sensitive_value(key, item);
            }
        }
        serde_json::Value::String(text) if looks_sensitive && !text.is_empty() => {
            *text = "***REDACTED***".to_string();
        }
        _ => {}
    }
}

pub fn redacted_config_value(config: &Config) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(config)?;
    redact_sensitive_value("root", &mut value);
    Ok(value)
}

pub fn print_json<T: serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn provider_status_report(config: &Config) -> ProviderStatusReport {
    ProviderStatusReport {
        current_model: config.agents.defaults.model.clone(),
        current_provider: current_provider_name(config),
        providers: provider_statuses(config),
    }
}

pub fn resolve_provider_model_with_default(
    config: &Config,
    provider_name: &str,
    provider_default_model: Option<&str>,
    requested_model: Option<String>,
) -> Result<String> {
    if let Some(model) = requested_model {
        return Ok(model);
    }

    if let Some(default_model) = provider_default_model.filter(|value| !value.trim().is_empty()) {
        return Ok(default_model.to_string());
    }

    let current_provider = infer_provider_name_from_model(&config.agents.defaults.model)
        .or_else(|| current_provider_name(config));
    if current_provider.as_deref() == Some(provider_name) {
        return Ok(config.agents.defaults.model.clone());
    }

    anyhow::bail!(
        "Provider '{}' does not expose a default model in registry; pass --model explicitly",
        provider_name
    );
}

pub fn resolve_provider_model(
    config: &Config,
    provider_name: &str,
    requested_model: Option<String>,
) -> Result<String> {
    resolve_provider_model_with_default(
        config,
        provider_name,
        default_model_from_registry(provider_name).as_deref(),
        requested_model,
    )
}

pub fn provider_statuses(config: &Config) -> Vec<ProviderStatus> {
    let catalog = ProviderCatalogService::new();
    let active_provider = current_provider_name(config);

    catalog
        .list_provider_views(config)
        .into_iter()
        .map(|view| {
            let current = active_provider.as_deref() == Some(view.id.as_str());
            let missing_fields = if view.ready {
                vec![]
            } else if view
                .api_base
                .as_ref()
                .map(|value| value.trim().is_empty())
                .unwrap_or(true)
            {
                vec!["api_base".to_string()]
            } else {
                vec!["api_key".to_string()]
            };
            ProviderStatus {
                name: view.id,
                display_name: view.display_name,
                default_model: view.default_model,
                configurable: true,
                configured: view.configured,
                ready: view.ready,
                uses_api_base: !missing_fields.iter().any(|field| field == "api_key"),
                provider_for_default_model: current,
                current,
                model: current.then(|| config.agents.defaults.model.clone()),
                api_base: view.api_base,
                missing_fields,
            }
        })
        .collect()
}

pub fn channel_statuses(config: &Config) -> Vec<ChannelStatus> {
    vec![
        ChannelStatus {
            name: "telegram".to_string(),
            enabled: config.channels.telegram.enabled,
            ready: config.channels.telegram.enabled && !config.channels.telegram.token.is_empty(),
            missing_fields: if config.channels.telegram.enabled
                && config.channels.telegram.token.is_empty()
            {
                vec!["token".to_string()]
            } else {
                vec![]
            },
            notes: vec![],
        },
        ChannelStatus {
            name: "discord".to_string(),
            enabled: config.channels.discord.enabled,
            ready: config.channels.discord.enabled && !config.channels.discord.token.is_empty(),
            missing_fields: if config.channels.discord.enabled
                && config.channels.discord.token.is_empty()
            {
                vec!["token".to_string()]
            } else {
                vec![]
            },
            notes: vec![],
        },
        ChannelStatus {
            name: "whatsapp".to_string(),
            enabled: config.channels.whatsapp.enabled,
            ready: config.channels.whatsapp.enabled,
            missing_fields: vec![],
            notes: vec!["requires bridge login".to_string()],
        },
        ChannelStatus {
            name: "feishu".to_string(),
            enabled: config.channels.feishu.enabled,
            ready: config.channels.feishu.enabled
                && !config.channels.feishu.app_id.is_empty()
                && !config.channels.feishu.app_secret.is_empty(),
            missing_fields: [
                ("app_id", config.channels.feishu.app_id.is_empty()),
                ("app_secret", config.channels.feishu.app_secret.is_empty()),
            ]
            .into_iter()
            .filter(|(_, missing)| config.channels.feishu.enabled && *missing)
            .map(|(name, _)| name.to_string())
            .collect(),
            notes: vec![],
        },
        ChannelStatus {
            name: "dingtalk".to_string(),
            enabled: config.channels.dingtalk.enabled,
            ready: config.channels.dingtalk.enabled
                && !config.channels.dingtalk.client_id.is_empty()
                && !config.channels.dingtalk.client_secret.is_empty(),
            missing_fields: [
                ("client_id", config.channels.dingtalk.client_id.is_empty()),
                (
                    "client_secret",
                    config.channels.dingtalk.client_secret.is_empty(),
                ),
            ]
            .into_iter()
            .filter(|(_, missing)| config.channels.dingtalk.enabled && *missing)
            .map(|(name, _)| name.to_string())
            .collect(),
            notes: vec![],
        },
        ChannelStatus {
            name: "email".to_string(),
            enabled: config.channels.email.enabled,
            ready: config.channels.email.enabled
                && !config.channels.email.imap_host.is_empty()
                && !config.channels.email.imap_username.is_empty()
                && !config.channels.email.imap_password.is_empty()
                && !config.channels.email.smtp_host.is_empty()
                && !config.channels.email.smtp_username.is_empty()
                && !config.channels.email.smtp_password.is_empty()
                && !config.channels.email.from_address.is_empty(),
            missing_fields: [
                ("imap_host", config.channels.email.imap_host.is_empty()),
                (
                    "imap_username",
                    config.channels.email.imap_username.is_empty(),
                ),
                (
                    "imap_password",
                    config.channels.email.imap_password.is_empty(),
                ),
                ("smtp_host", config.channels.email.smtp_host.is_empty()),
                (
                    "smtp_username",
                    config.channels.email.smtp_username.is_empty(),
                ),
                (
                    "smtp_password",
                    config.channels.email.smtp_password.is_empty(),
                ),
                (
                    "from_address",
                    config.channels.email.from_address.is_empty(),
                ),
            ]
            .into_iter()
            .filter(|(_, missing)| config.channels.email.enabled && *missing)
            .map(|(name, _)| name.to_string())
            .collect(),
            notes: vec![],
        },
        ChannelStatus {
            name: "slack".to_string(),
            enabled: config.channels.slack.enabled,
            ready: config.channels.slack.enabled
                && !config.channels.slack.bot_token.is_empty()
                && !config.channels.slack.app_token.is_empty(),
            missing_fields: [
                ("bot_token", config.channels.slack.bot_token.is_empty()),
                ("app_token", config.channels.slack.app_token.is_empty()),
            ]
            .into_iter()
            .filter(|(_, missing)| config.channels.slack.enabled && *missing)
            .map(|(name, _)| name.to_string())
            .collect(),
            notes: vec![],
        },
        ChannelStatus {
            name: "qq".to_string(),
            enabled: config.channels.qq.enabled,
            ready: config.channels.qq.enabled
                && !config.channels.qq.app_id.is_empty()
                && !config.channels.qq.secret.is_empty(),
            missing_fields: [
                ("app_id", config.channels.qq.app_id.is_empty()),
                ("secret", config.channels.qq.secret.is_empty()),
            ]
            .into_iter()
            .filter(|(_, missing)| config.channels.qq.enabled && *missing)
            .map(|(name, _)| name.to_string())
            .collect(),
            notes: vec![],
        },
        ChannelStatus {
            name: "matrix".to_string(),
            enabled: config.channels.matrix.enabled,
            ready: config.channels.matrix.enabled
                && !config.channels.matrix.user_id.is_empty()
                && !config.channels.matrix.access_token.is_empty(),
            missing_fields: [
                ("user_id", config.channels.matrix.user_id.is_empty()),
                (
                    "access_token",
                    config.channels.matrix.access_token.is_empty(),
                ),
            ]
            .into_iter()
            .filter(|(_, missing)| config.channels.matrix.enabled && *missing)
            .map(|(name, _)| name.to_string())
            .collect(),
            notes: vec![],
        },
    ]
}

pub fn doctor_report(
    runtime: &CliRuntime,
    config: &Config,
    include_swarm_cortex: bool,
) -> DoctorReport {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    if let Err(err) = validate_config(config) {
        errors.push(err.to_string());
    }

    let active_provider = current_provider_name(config);

    if active_provider.is_none() {
        errors.push(format!(
            "No provider found for model '{}'",
            config.agents.defaults.model
        ));
    } else if let Some(provider_name) = active_provider.as_deref() {
        if let Some(provider_config) = provider_config_by_name(&config.providers, provider_name) {
            let missing_key = provider_config.api_key.trim().is_empty();
            let missing_api_base = provider_name == "custom"
                && provider_config
                    .api_base
                    .as_ref()
                    .map(|base| base.trim().is_empty())
                    .unwrap_or(true);

            if missing_key && provider_name != "vllm" {
                warnings.push(format!(
                    "Provider '{}' is selected by the default model but api_key is empty",
                    provider_name
                ));
            }
            if missing_api_base {
                warnings.push("Provider 'custom' requires api_base".to_string());
            }
        }
    }

    let workspace = runtime.effective_workspace(config);
    if !workspace.exists() {
        warnings.push(format!(
            "Workspace does not exist yet: {}",
            workspace.display()
        ));
    }

    let channels = channel_statuses(config);
    for channel in &channels {
        if channel.enabled && !channel.ready {
            warnings.push(format!(
                "Channel '{}' is enabled but missing fields: {}",
                channel.name,
                channel.missing_fields.join(", ")
            ));
        }
    }

    let workspace = runtime.effective_workspace(config);
    let swarm_cortex = include_swarm_cortex.then(|| {
        swarm_cortex_doctor_section_for_diagnostics(config, None, Some(&workspace))
    });

    DoctorReport {
        valid: errors.is_empty(),
        ready: errors.is_empty() && warnings.is_empty(),
        errors,
        warnings,
        provider: active_provider,
        channels,
        swarm_cortex,
    }
}

pub async fn collect_status_report(
    runtime: &CliRuntime,
    include_swarm_cortex: bool,
) -> Result<StatusReport> {
    let config = runtime.load_config()?;
    let doctor = doctor_report(runtime, &config, include_swarm_cortex);
    let cron_store = runtime.cron_store_path();
    let cron_jobs = if cron_store.exists() {
        let service = Arc::new(CronService::new(cron_store.clone(), None));
        service.start().await;
        let jobs = service.list_jobs(true).await.len();
        service.stop().await;
        jobs
    } else {
        0
    };

    Ok(StatusReport {
        config: runtime.path_report(&config),
        default_model: config.agents.defaults.model.clone(),
        default_provider: doctor.provider.clone(),
        logging: StatusLoggingReport {
            level: config.logging.level.clone(),
            format: config.logging.format.clone(),
            dir: config.logging.dir.clone(),
        },
        providers: provider_statuses(&config),
        channels: channel_statuses(&config),
        cron_jobs,
        mcp_servers: StatusMcpReport {
            configured: config.tools.mcp_servers.len(),
            disabled: config.tools.mcp_manager.disabled_servers.len(),
        },
        doctor: StatusDoctorSummary {
            valid: doctor.valid,
            ready: doctor.ready,
            errors: doctor.errors,
            warnings: doctor.warnings,
            swarm_cortex: doctor.swarm_cortex.clone(),
        },
    })
}

