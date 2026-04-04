mod companion_admin;
mod provider_admin;
mod runtime_control;

use agent_diva_agent::runtime_control::RuntimeControlCommand;
use agent_diva_agent::tool_config::network::{
    NetworkToolConfig, WebFetchRuntimeConfig, WebRuntimeConfig, WebSearchRuntimeConfig,
};
use agent_diva_channels::ChannelManager;
use agent_diva_core::bus::MessageBus;
use agent_diva_core::config::{ConfigLoader, CustomProviderConfig};
use agent_diva_core::cron::CronService;
use agent_diva_providers::{DynamicProvider, ProviderCatalogService, ProviderRegistry};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::state::{ManagerCommand, ProviderCommand};

pub struct Manager {
    api_rx: mpsc::Receiver<ManagerCommand>,
    bus: MessageBus,
    provider: Arc<DynamicProvider>,
    loader: ConfigLoader,
    // Current config state
    current_provider: Option<String>,
    current_model: String,
    current_api_base: Option<String>,
    current_api_key: Option<String>,
    channel_manager: Option<Arc<ChannelManager>>,
    runtime_control_tx: Option<mpsc::UnboundedSender<RuntimeControlCommand>>,
    cron_service: Arc<CronService>,
}

enum ProviderConfigTarget<'a> {
    Builtin(&'a mut agent_diva_core::config::schema::ProviderConfig),
    Shadow(&'a mut CustomProviderConfig),
}

impl ProviderConfigTarget<'_> {
    fn set_api_key(&mut self, api_key: String) {
        match self {
            Self::Builtin(config) => config.api_key = api_key,
            Self::Shadow(config) => config.api_key = api_key,
        }
    }

    fn set_api_base(&mut self, api_base: Option<String>) {
        match self {
            Self::Builtin(config) => config.api_base = api_base,
            Self::Shadow(config) => config.api_base = api_base,
        }
    }
}

impl Manager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_rx: mpsc::Receiver<ManagerCommand>,
        bus: MessageBus,
        provider: Arc<DynamicProvider>,
        loader: ConfigLoader,
        initial_provider: Option<String>,
        initial_model: String,
        api_key: Option<String>,
        api_base: Option<String>,
        channel_manager: Option<Arc<ChannelManager>>,
        runtime_control_tx: Option<mpsc::UnboundedSender<RuntimeControlCommand>>,
        cron_service: Arc<CronService>,
    ) -> Self {
        Self {
            api_rx,
            bus,
            provider,
            loader,
            current_provider: initial_provider
                .or_else(|| Self::provider_name_for_model(None, &initial_model)),
            current_model: initial_model,
            current_api_base: api_base,
            current_api_key: api_key,
            channel_manager,
            runtime_control_tx,
            cron_service,
        }
    }

    fn provider_name_for_model(preferred_provider: Option<&str>, model: &str) -> Option<String> {
        let registry = ProviderRegistry::new();
        preferred_provider
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .and_then(|name| registry.find_by_name(name))
            .map(|spec| spec.name.clone())
            .or_else(|| {
                model
                    .split('/')
                    .next()
                    .and_then(|prefix| registry.find_by_name(prefix))
                    .map(|spec| spec.name.clone())
            })
            .or_else(|| registry.find_by_model(model).map(|spec| spec.name.clone()))
    }

    fn map_network_config(config: &agent_diva_core::config::schema::Config) -> NetworkToolConfig {
        let api_key = config.tools.web.search.api_key.trim().to_string();
        NetworkToolConfig {
            web: WebRuntimeConfig {
                search: WebSearchRuntimeConfig {
                    provider: config.tools.web.search.provider.clone(),
                    enabled: config.tools.web.search.enabled,
                    api_key: if api_key.is_empty() {
                        None
                    } else {
                        Some(api_key)
                    },
                    max_results: config.tools.web.search.max_results,
                },
                fetch: WebFetchRuntimeConfig {
                    enabled: config.tools.web.fetch.enabled,
                },
            },
        }
    }

    fn reload_runtime_mcp(&self) {
        let Some(tx) = &self.runtime_control_tx else {
            return;
        };
        let Ok(config) = self.loader.load() else {
            error!("Failed to load config for MCP runtime update");
            return;
        };
        if let Err(e) = tx.send(RuntimeControlCommand::UpdateMcp {
            servers: config.tools.active_mcp_servers(),
        }) {
            error!("Failed to send runtime MCP update: {}", e);
        }
    }

    fn model_matches_provider(provider_id: &str, model: &str) -> bool {
        let trimmed_provider = provider_id.trim();
        let trimmed_model = model.trim();
        if trimmed_provider.is_empty() || trimmed_model.is_empty() {
            return false;
        }

        if trimmed_model
            .split('/')
            .next()
            .is_some_and(|prefix| prefix == trimmed_provider)
        {
            return true;
        }

        ProviderRegistry::new()
            .find_by_model(trimmed_model)
            .is_some_and(|spec| spec.name == trimmed_provider)
    }

    async fn normalize_model_for_provider(
        config: &agent_diva_core::config::schema::Config,
        catalog: &ProviderCatalogService,
        provider_id: &str,
        requested_model: &str,
        provider_explicit: bool,
        model_explicit: bool,
    ) -> String {
        let requested_model = requested_model.trim();
        let provider_models = catalog
            .list_provider_models(config, provider_id, false, None)
            .await
            .ok();

        if !requested_model.is_empty() {
            if provider_explicit && model_explicit {
                return requested_model.to_string();
            }

            let in_catalog = provider_models.as_ref().is_some_and(|catalog| {
                catalog
                    .models
                    .iter()
                    .any(|entry| entry.id == requested_model)
            });
            if in_catalog || Self::model_matches_provider(provider_id, requested_model) {
                return requested_model.to_string();
            }
        }

        if let Some(default_model) = catalog
            .get_provider_view(config, provider_id)
            .and_then(|view| view.default_model)
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            return default_model;
        }

        if let Some(first_model) = provider_models
            .and_then(|catalog| catalog.models.into_iter().next().map(|entry| entry.id))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            return first_model;
        }

        requested_model.to_string()
    }

    fn ensure_provider_credentials_slot<'a>(
        config: &'a mut agent_diva_core::config::schema::Config,
        provider_id: &str,
    ) -> ProviderConfigTarget<'a> {
        if agent_diva_core::config::schema::ProvidersConfig::is_builtin_provider(provider_id) {
            let provider = config
                .providers
                .get_mut(provider_id)
                .expect("builtin provider slot must exist");
            return ProviderConfigTarget::Builtin(provider);
        }

        let provider = config
            .providers
            .custom_providers
            .entry(provider_id.to_string())
            .or_default();
        ProviderConfigTarget::Shadow(provider)
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        info!("Manager loop started");

        loop {
            debug!("Waiting for command...");
            tokio::select! {
                msg = self.api_rx.recv() => {
                    let cmd = match msg {
                        Some(cmd) => {
                            debug!("Received command");
                            cmd
                        },
                        None => {
                            info!("Manager channel closed, stopping loop");
                            break Ok(());
                        }
                    };
                    match cmd {
                        ManagerCommand::Chat(req) => self.handle_chat(req),
                        ManagerCommand::StopChat(req, reply) => {
                            self.handle_stop_chat(req, reply);
                        }
                        ManagerCommand::ResetSession(req, reply) => {
                            self.handle_reset_session(req, reply);
                        }
                        ManagerCommand::GetSessions(reply) => {
                            self.handle_get_sessions(reply).await;
                        }
                        ManagerCommand::GetSessionHistory(session_key, reply) => {
                            self.handle_get_session_history(session_key, reply).await;
                        }
                        ManagerCommand::DeleteSession(session_key, reply) => {
                            self.handle_delete_session(session_key, reply).await;
                        }
                        ManagerCommand::ListCronJobs(reply) => {
                            self.handle_list_cron_jobs(reply).await;
                        }
                        ManagerCommand::GetCronJob(job_id, reply) => {
                            self.handle_get_cron_job(job_id, reply).await;
                        }
                        ManagerCommand::CreateCronJob(request, reply) => {
                            self.handle_create_cron_job(request, reply).await;
                        }
                        ManagerCommand::UpdateCronJob(job_id, request, reply) => {
                            self.handle_update_cron_job(job_id, request, reply).await;
                        }
                        ManagerCommand::DeleteCronJob(job_id, reply) => {
                            self.handle_delete_cron_job(job_id, reply).await;
                        }
                        ManagerCommand::SetCronJobEnabled(job_id, enabled, reply) => {
                            self.handle_set_cron_job_enabled(job_id, enabled, reply)
                                .await;
                        }
                        ManagerCommand::RunCronJobNow(job_id, force, reply) => {
                            self.handle_run_cron_job_now(job_id, force, reply).await;
                        }
                        ManagerCommand::StopCronJobRun(job_id, reply) => {
                            self.handle_stop_cron_job_run(job_id, reply).await;
                        }
                        ManagerCommand::UpdateConfig(update) => {
                            self.handle_update_config(update).await?;
                        }
                        ManagerCommand::GetConfig(reply) => {
                            self.handle_get_config(reply);
                        }
                        ManagerCommand::GetChannels(reply) => {
                            self.handle_get_channels(reply);
                        }
                        ManagerCommand::GetTools(reply) => {
                            self.handle_get_tools(reply);
                        }
                        ManagerCommand::GetSkills(reply) => self.handle_get_skills(reply),
                        ManagerCommand::GetMcps(reply) => self.handle_get_mcps(reply),
                        ManagerCommand::CreateMcp(payload, reply) => {
                            self.handle_create_mcp(payload, reply);
                        }
                        ManagerCommand::UpdateMcp(name, payload, reply) => {
                            self.handle_update_mcp(name, payload, reply);
                        }
                        ManagerCommand::DeleteMcp(name, reply) => {
                            self.handle_delete_mcp(name, reply);
                        }
                        ManagerCommand::SetMcpEnabled(name, enabled, reply) => {
                            self.handle_set_mcp_enabled(name, enabled, reply);
                        }
                        ManagerCommand::RefreshMcpStatus(name, reply) => {
                            self.handle_refresh_mcp_status(name, reply);
                        }
                        ManagerCommand::UploadSkill(request, reply) => {
                            self.handle_upload_skill(request, reply);
                        }
                        ManagerCommand::DeleteSkill(name, reply) => {
                            self.handle_delete_skill(name, reply);
                        }
                        ManagerCommand::UploadFile(request, reply) => {
                            self.handle_upload_file(request, reply).await;
                        }
                        ManagerCommand::Provider(command) => {
                            self.handle_provider_command(command).await;
                        }
                        ManagerCommand::UpdateTools(update) => {
                            self.handle_update_tools(update);
                        }
                        ManagerCommand::UpdateChannel(update) => {
                            self.handle_update_channel(update).await;
                        }
                    }
                }
            }
        }
    }
}

impl Manager {
    async fn handle_provider_command(&self, command: ProviderCommand) {
        match command {
            ProviderCommand::GetProviders(reply) => self.handle_get_providers(reply),
            ProviderCommand::GetProvider(name, reply) => self.handle_get_provider(name, reply),
            ProviderCommand::GetProviderModels(name, runtime, reply) => {
                self.handle_get_provider_models(name, runtime, reply).await;
            }
            ProviderCommand::ResolveProvider(model, preferred_provider, reply) => {
                self.handle_resolve_provider(model, preferred_provider, reply);
            }
            ProviderCommand::AddProviderModel(name, model, reply) => {
                self.handle_add_provider_model(name, model, reply).await;
            }
            ProviderCommand::DeleteProviderModel(name, model_id, reply) => {
                self.handle_delete_provider_model(name, model_id, reply)
                    .await;
            }
            ProviderCommand::CreateProvider(payload, reply) => {
                self.handle_create_provider(payload, reply).await;
            }
            ProviderCommand::UpdateProvider(name, payload, reply) => {
                self.handle_update_provider(name, payload, reply).await;
            }
            ProviderCommand::DeleteProvider(name, reply) => {
                self.handle_delete_provider(name, reply).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_matches_provider_accepts_registry_resolved_models() {
        assert!(Manager::model_matches_provider("openai", "gpt-4o"));
        assert!(Manager::model_matches_provider("deepseek", "deepseek-chat"));
        assert!(Manager::model_matches_provider(
            "openrouter",
            "openrouter/anthropic/claude-sonnet-4"
        ));
        assert!(!Manager::model_matches_provider("openai", "deepseek-chat"));
    }

    #[tokio::test]
    async fn normalize_model_for_provider_replaces_cross_provider_model() {
        let catalog = ProviderCatalogService::new();
        let config = agent_diva_core::config::schema::Config::default();

        let model = Manager::normalize_model_for_provider(
            &config,
            &catalog,
            "openai",
            "deepseek-chat",
            true,
            false,
        )
        .await;

        assert_eq!(model, "openai/gpt-4o");
    }

    #[tokio::test]
    async fn normalize_model_for_provider_keeps_explicit_model_for_explicit_provider() {
        let catalog = ProviderCatalogService::new();
        let config = agent_diva_core::config::schema::Config::default();

        let model = Manager::normalize_model_for_provider(
            &config,
            &catalog,
            "silicon",
            "ByteDance-Seed/Seed-OSS-36B-Instruct",
            true,
            true,
        )
        .await;

        assert_eq!(model, "ByteDance-Seed/Seed-OSS-36B-Instruct");
    }
}
