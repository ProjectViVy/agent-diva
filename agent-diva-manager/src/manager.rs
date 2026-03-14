use agent_diva_agent::runtime_control::RuntimeControlCommand;
use agent_diva_agent::tool_config::network::{
    NetworkToolConfig, WebFetchRuntimeConfig, WebRuntimeConfig, WebSearchRuntimeConfig,
};
use agent_diva_channels::ChannelManager;
use agent_diva_core::bus::{AgentEvent, MessageBus};
use agent_diva_core::config::{ConfigLoader, CustomProviderConfig};
use agent_diva_core::cron::CronService;
use agent_diva_providers::{
    DynamicProvider, LiteLLMClient, ProviderCatalogService, ProviderRegistry,
};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::mcp_service::McpService;
use crate::skill_service::SkillService;
use crate::state::{ConfigResponse, ManagerCommand, ToolsConfigResponse};

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
                        ManagerCommand::Chat(req) => {
                            debug!("Processing Chat request via Bus");
                            let channel = req.msg.channel.clone();
                            let chat_id = req.msg.chat_id.clone();
                            let event_tx = req.event_tx.clone();

                            // Subscribe to bus events
                            let mut event_rx = self.bus.subscribe_events();

                            // Publish inbound
                            if let Err(e) = self.bus.publish_inbound(req.msg) {
                                error!("Failed to publish inbound: {}", e);
                                let _ = event_tx.send(AgentEvent::Error { message: e.to_string() });
                            } else {
                                // Spawn task to forward events
                                tokio::spawn(async move {
                                    loop {
                                        // Wait for event (with timeout)
                                        match tokio::time::timeout(std::time::Duration::from_secs(60), event_rx.recv()).await {
                                            Ok(Ok(bus_event)) => {
                                                if bus_event.channel == channel && bus_event.chat_id == chat_id {
                                                    // Forward event
                                                    let event = bus_event.event;
                                                    if event_tx.send(event.clone()).is_err() {
                                                        break;
                                                    }

                                                    // Check if final
                                                    match event {
                                                        AgentEvent::FinalResponse { .. } | AgentEvent::Error { .. } => break,
                                                        _ => {}
                                                    }
                                                }
                                            }
                                            Ok(Err(_)) => break, // Lagged or closed
                                            Err(_) => break, // Timeout
                                        }
                                    }
                                });
                            }
                        }
                        ManagerCommand::StopChat(req, reply) => {
                            let channel = req.channel.unwrap_or_else(|| "api".to_string());
                            let chat_id = req.chat_id.unwrap_or_else(|| "default".to_string());
                            let session_key = format!("{}:{}", channel, chat_id);
                            if let Some(tx) = &self.runtime_control_tx {
                                match tx.send(RuntimeControlCommand::StopSession { session_key }) {
                                    Ok(_) => {
                                        let _ = reply.send(Ok(true));
                                    }
                                    Err(e) => {
                                        let _ = reply.send(Err(format!(
                                            "failed to send stop command: {}",
                                            e
                                        )));
                                    }
                                }
                            } else {
                                let _ = reply.send(Err(
                                    "runtime control channel is not initialized".to_string()
                                ));
                            }
                        }
                        ManagerCommand::ResetSession(req, reply) => {
                            let channel = req.channel.unwrap_or_else(|| "api".to_string());
                            let chat_id = req.chat_id.unwrap_or_else(|| "default".to_string());
                            let session_key = format!("{}:{}", channel, chat_id);
                            if let Some(tx) = &self.runtime_control_tx {
                                match tx.send(RuntimeControlCommand::ResetSession { session_key }) {
                                    Ok(_) => {
                                        let _ = reply.send(Ok(true));
                                    }
                                    Err(e) => {
                                        let _ = reply.send(Err(format!(
                                            "failed to send reset command: {}",
                                            e
                                        )));
                                    }
                                }
                            } else {
                                let _ = reply.send(Err(
                                    "runtime control channel is not initialized".to_string()
                                ));
                            }
                        }
                        ManagerCommand::GetSessions(reply) => {
                            if let Some(tx) = &self.runtime_control_tx {
                                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                                if let Err(e) = tx.send(RuntimeControlCommand::GetSessions { reply_tx }) {
                                    let _ = reply.send(Err(format!("failed to send GetSessions command: {}", e)));
                                } else {
                                    match reply_rx.await {
                                        Ok(sessions) => {
                                            let _ = reply.send(Ok(sessions));
                                        }
                                        Err(e) => {
                                            let _ = reply.send(Err(format!(
                                                "failed to receive sessions: {}",
                                                e
                                            )));
                                        }
                                    }
                                }
                            } else {
                                let _ = reply.send(Err("runtime control channel is not initialized".to_string()));
                            }
                        }
                        ManagerCommand::GetSessionHistory(session_key, reply) => {
                            if let Some(tx) = &self.runtime_control_tx {
                                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                                if let Err(e) = tx.send(RuntimeControlCommand::GetSession { session_key, reply_tx }) {
                                    let _ = reply.send(Err(format!("failed to send GetSession command: {}", e)));
                                } else {
                                    match reply_rx.await {
                                        Ok(session) => {
                                            let _ = reply.send(Ok(session));
                                        }
                                        Err(e) => {
                                            let _ = reply.send(Err(format!(
                                                "failed to receive session: {}",
                                                e
                                            )));
                                        }
                                    }
                                }
                            } else {
                                let _ = reply.send(Err("runtime control channel is not initialized".to_string()));
                            }
                        }
                        ManagerCommand::DeleteSession(session_key, reply) => {
                            if let Some(tx) = &self.runtime_control_tx {
                                let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();
                                if let Err(e) = tx.send(RuntimeControlCommand::DeleteSession {
                                    session_key: session_key.clone(),
                                    reply_tx,
                                }) {
                                    let _ = reply.send(Err(format!(
                                        "failed to send DeleteSession command: {}",
                                        e
                                    )));
                                } else {
                                    match reply_rx.await {
                                        Ok(result) => {
                                            let _ = reply.send(result);
                                        }
                                        Err(e) => {
                                            let _ = reply.send(Err(format!(
                                                "failed to receive delete result: {}",
                                                e
                                            )));
                                        }
                                    }
                                }
                            } else {
                                let _ = reply.send(Err(
                                    "runtime control channel is not initialized".to_string(),
                                ));
                            }
                        }
                        ManagerCommand::ListCronJobs(reply) => {
                            let jobs = self.cron_service.list_job_views(true).await;
                            let _ = reply.send(Ok(jobs));
                        }
                        ManagerCommand::GetCronJob(job_id, reply) => {
                            let job = self.cron_service.get_job(&job_id).await;
                            let _ = reply.send(Ok(job));
                        }
                        ManagerCommand::CreateCronJob(request, reply) => {
                            let result = self.cron_service.create_job(request).await;
                            let _ = reply.send(result);
                        }
                        ManagerCommand::UpdateCronJob(job_id, request, reply) => {
                            let result = self.cron_service.update_job(&job_id, request).await;
                            let _ = reply.send(result);
                        }
                        ManagerCommand::DeleteCronJob(job_id, reply) => {
                            let result = self.cron_service.delete_job(&job_id).await;
                            let _ = reply.send(result);
                        }
                        ManagerCommand::SetCronJobEnabled(job_id, enabled, reply) => {
                            let result = self.cron_service.set_job_enabled(&job_id, enabled).await;
                            let _ = reply.send(result);
                        }
                        ManagerCommand::RunCronJobNow(job_id, force, reply) => {
                            let result = self.cron_service.run_job_now(&job_id, force).await;
                            let _ = reply.send(result);
                        }
                        ManagerCommand::StopCronJobRun(job_id, reply) => {
                            let result = self.cron_service.stop_run(&job_id).await;
                            let _ = reply.send(result);
                        }
                        ManagerCommand::UpdateConfig(update) => {
                            debug!("Processing UpdateConfig command");
                            debug!("Update request: {:?}", update);
                            info!("Processing UpdateConfig request: {:?}", update);

                            // 1. Load current config
                            let mut config = match self.loader.load() {
                                Ok(c) => c,
                                Err(e) => {
                                    error!("Failed to load config: {}", e);
                                    return Err(e.into());
                                }
                            };

                            let requested_provider = update
                                .provider
                                .clone()
                                .map(|value| value.trim().to_string())
                                .filter(|value| !value.is_empty());
                            let requested_model = update
                                .model
                                .clone()
                                .map(|value| value.trim().to_string())
                                .filter(|value| !value.is_empty());
                            let clear_selection =
                                requested_provider.is_none() && requested_model.is_none();

                            if clear_selection {
                                info!("Clearing active provider/model selection");
                                config.agents.defaults.provider = None;
                                config.agents.defaults.model.clear();
                                self.current_provider = None;
                                self.current_model.clear();
                                self.current_api_base = None;
                                self.current_api_key = None;
                            } else {
                                let provider_to_use = requested_provider
                                    .clone()
                                .or_else(|| config.agents.defaults.provider.clone())
                                .or_else(|| self.current_provider.clone());
                                let catalog = ProviderCatalogService::new();
                                let requested_model = requested_model
                                    .clone()
                                    .unwrap_or_else(|| config.agents.defaults.model.clone());
                                let provider_explicit = requested_provider.is_some();
                                let model_explicit = update
                                    .model
                                    .as_deref()
                                    .map(str::trim)
                                    .is_some_and(|value| !value.is_empty());
                                let provider_id = provider_to_use
                                    .as_deref()
                                    .filter(|value| catalog.get_provider_view(&config, value).is_some())
                                    .map(ToString::to_string)
                                    .or_else(|| {
                                        catalog.resolve_provider_id(
                                            &config,
                                            &requested_model,
                                            provider_to_use.as_deref(),
                                        )
                                    });

                                if let Some(provider_id) = provider_id {
                                    let model_to_use = Self::normalize_model_for_provider(
                                        &config,
                                        &catalog,
                                        &provider_id,
                                        &requested_model,
                                        provider_explicit,
                                        model_explicit,
                                    )
                                    .await;
                                    info!(
                                        "Resolved config update to provider={}, model={}",
                                        provider_id, model_to_use
                                    );
                                    config.agents.defaults.provider = Some(provider_id.clone());
                                    config.agents.defaults.model = model_to_use.clone();
                                    self.current_provider = Some(provider_id.clone());
                                    self.current_model = model_to_use.clone();

                                    let mut credentials = Self::ensure_provider_credentials_slot(
                                        &mut config,
                                        &provider_id,
                                    );
                                    if let Some(ref k) = update.api_key {
                                        info!("Updating API key for provider: {}", provider_id);
                                        credentials.set_api_key(k.clone());
                                        self.current_api_key = Some(k.clone());
                                    }
                                    if let Some(ref b) = update.api_base {
                                        info!("Updating API base for provider: {}", provider_id);
                                        credentials.set_api_base(Some(b.clone()));
                                        self.current_api_base = Some(b.clone());
                                    }
                                } else {
                                    warn!("No provider found for model: {}", requested_model);
                                }
                            }

                            // 4. Save config
                            if let Err(e) = self.loader.save(&config) {
                                error!("Failed to save config: {}", e);
                                return Err(e.into());
                            }
                            info!("Configuration saved to disk");

                            // 5. Hot Reload Provider
                            let model_to_use = config.agents.defaults.model.trim().to_string();
                            if model_to_use.is_empty() {
                                info!("Active model cleared; skipping provider hot reload");
                            } else {
                                let catalog = ProviderCatalogService::new();
                                info!("Hot reloading provider for model: {}", model_to_use);

                                let provider_id = catalog.resolve_provider_id(
                                    &config,
                                    &model_to_use,
                                    config.agents.defaults.provider.as_deref(),
                                );

                                if let Some(provider_id) = provider_id {
                                    self.current_provider = Some(provider_id.clone());
                                    let access = catalog
                                        .get_provider_access(&config, &provider_id)
                                        .unwrap_or_else(|| agent_diva_providers::ProviderAccess::from_config(None));
                                    let extra_headers = (!access.extra_headers.is_empty()).then(|| {
                                        access
                                            .extra_headers
                                            .into_iter()
                                            .collect::<std::collections::HashMap<String, String>>()
                                    });
                                    let resolved_api_base = access.api_base.clone().or_else(|| {
                                        catalog
                                            .get_provider_view(&config, &provider_id)
                                            .and_then(|view| view.api_base)
                                    });
                                    self.current_api_key = access.api_key.clone();
                                    self.current_api_base = resolved_api_base.clone();

                                    let new_client = LiteLLMClient::new(
                                        access.api_key,
                                        resolved_api_base,
                                        model_to_use.clone(),
                                        extra_headers,
                                        Some(provider_id),
                                        config.agents.defaults.reasoning_effort.clone(),
                                    );

                                    self.provider.update(Arc::new(new_client));
                                    info!("Provider updated successfully");
                                } else {
                                    warn!(
                                        "No provider found for model: {}, skipping provider update",
                                        model_to_use
                                    );
                                }
                            }
                        }
                        ManagerCommand::GetConfig(reply) => {
                            debug!("Processing GetConfig request");
                            let resp = ConfigResponse {
                                provider: self.current_provider.clone(),
                                api_base: self.current_api_base.clone(),
                                model: self.current_model.clone(),
                                has_api_key: self.current_api_key.is_some(),
                            };
                            let _ = reply.send(resp);
                        }
                        ManagerCommand::GetChannels(reply) => {
                            debug!("Processing GetChannels command");
                            if let Ok(config) = self.loader.load() {
                                let _ = reply.send(config.channels);
                            } else {
                                error!("Failed to load config for GetChannels");
                                let _ = reply.send(agent_diva_core::config::schema::ChannelsConfig::default());
                            }
                        }
                        ManagerCommand::GetTools(reply) => {
                            debug!("Processing GetTools command");
                            if let Ok(config) = self.loader.load() {
                                let _ = reply.send(ToolsConfigResponse {
                                    web: config.tools.web.into(),
                                });
                            } else {
                                error!("Failed to load config for GetTools");
                                let _ = reply.send(ToolsConfigResponse {
                                    web: agent_diva_core::config::schema::WebToolsConfig::default()
                                        .into(),
                                });
                            }
                        }
                        ManagerCommand::GetSkills(reply) => {
                            let service = SkillService::new(self.loader.clone());
                            let result = service.list_skills().map_err(|e| e.to_string());
                            let _ = reply.send(result);
                        }
                        ManagerCommand::GetMcps(reply) => {
                            let service = McpService::new(self.loader.clone());
                            let result = service.list_mcps().map_err(|e| e.to_string());
                            let _ = reply.send(result);
                        }
                        ManagerCommand::CreateMcp(payload, reply) => {
                            let service = McpService::new(self.loader.clone());
                            let result = service.create_mcp(payload).map_err(|e| e.to_string());
                            if result.is_ok() {
                                self.reload_runtime_mcp();
                            }
                            let _ = reply.send(result);
                        }
                        ManagerCommand::UpdateMcp(name, payload, reply) => {
                            let service = McpService::new(self.loader.clone());
                            let result = service
                                .update_mcp(&name, payload)
                                .map_err(|e| e.to_string());
                            if result.is_ok() {
                                self.reload_runtime_mcp();
                            }
                            let _ = reply.send(result);
                        }
                        ManagerCommand::DeleteMcp(name, reply) => {
                            let service = McpService::new(self.loader.clone());
                            let result = service.delete_mcp(&name).map_err(|e| e.to_string());
                            if result.is_ok() {
                                self.reload_runtime_mcp();
                            }
                            let _ = reply.send(result);
                        }
                        ManagerCommand::SetMcpEnabled(name, enabled, reply) => {
                            let service = McpService::new(self.loader.clone());
                            let result = service.set_enabled(&name, enabled).map_err(|e| e.to_string());
                            if result.is_ok() {
                                self.reload_runtime_mcp();
                            }
                            let _ = reply.send(result);
                        }
                        ManagerCommand::RefreshMcpStatus(name, reply) => {
                            let service = McpService::new(self.loader.clone());
                            let result = service.get_mcp(&name).map_err(|e| e.to_string());
                            if result.is_ok() {
                                self.reload_runtime_mcp();
                            }
                            let _ = reply.send(result);
                        }
                        ManagerCommand::UploadSkill(request, reply) => {
                            let service = SkillService::new(self.loader.clone());
                            let result = service
                                .upload_skill_zip(&request.file_name, request.bytes)
                                .map_err(|e| e.to_string());
                            let _ = reply.send(result);
                        }
                        ManagerCommand::DeleteSkill(name, reply) => {
                            let service = SkillService::new(self.loader.clone());
                            let result = service.delete_skill(&name).map_err(|e| e.to_string());
                            let _ = reply.send(result);
                        }
                        ManagerCommand::UpdateTools(update) => {
                            info!("Processing UpdateTools request");

                            let mut config = match self.loader.load() {
                                Ok(c) => c,
                                Err(e) => {
                                    error!("Failed to load config: {}", e);
                                    continue;
                                }
                            };

                            config.tools.web.search = update.web.search;
                            config.tools.web.fetch = update.web.fetch;

                            if let Err(e) = self.loader.save(&config) {
                                error!("Failed to save tools config: {}", e);
                                continue;
                            }

                            if let Some(tx) = &self.runtime_control_tx {
                                let network = Self::map_network_config(&config);
                                if let Err(e) = tx.send(RuntimeControlCommand::UpdateNetwork(network)) {
                                    error!("Failed to send runtime tools update: {}", e);
                                }
                            }
                        }
                        ManagerCommand::TestChannel(update, reply) => {
                            info!("Processing TestChannel request: {}", update.name);

                            // 1. Load config
                            let mut config = match self.loader.load() {
                                Ok(c) => c,
                                Err(e) => {
                                    error!("Failed to load config: {}", e);
                                    let _ = reply.send(Err(e.to_string()));
                                    continue;
                                }
                            };

                            // 2. Update specific channel in config COPY
                            let name = update.name.as_str();
                            let result: anyhow::Result<()> = (|| {
                                match name {
                                    "telegram" => {
                                        let mut cfg: agent_diva_core::config::schema::TelegramConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.telegram = cfg;
                                    },
                                    "discord" => {
                                        let mut cfg: agent_diva_core::config::schema::DiscordConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.discord = cfg;
                                    },
                                    "feishu" => {
                                        let mut cfg: agent_diva_core::config::schema::FeishuConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.feishu = cfg;
                                    },
                                    "whatsapp" => {
                                        let mut cfg: agent_diva_core::config::schema::WhatsAppConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.whatsapp = cfg;
                                    },
                                    "dingtalk" => {
                                        let mut cfg: agent_diva_core::config::schema::DingTalkConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.dingtalk = cfg;
                                    },
                                    "email" => {
                                        let mut cfg: agent_diva_core::config::schema::EmailConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.email = cfg;
                                    },
                                    "slack" => {
                                        let mut cfg: agent_diva_core::config::schema::SlackConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.slack = cfg;
                                    },
                                    "qq" => {
                                        let mut cfg: agent_diva_core::config::schema::QQConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.qq = cfg;
                                    },
                                    "matrix" => {
                                        let mut cfg: agent_diva_core::config::schema::MatrixConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.matrix = cfg;
                                    },
                                    _ => {
                                        warn!("Unknown channel: {}", name);
                                    }
                                }
                                Ok(())
                            })();

                            if let Err(e) = result {
                                error!("Failed to update channel config for test: {}", e);
                                let _ = reply.send(Err(e.to_string()));
                                continue;
                            }

                            // 3. Test connection (Do NOT save config)
                            if let Some(cm) = &self.channel_manager {
                                match cm.test_channel(name, config).await {
                                    Ok(_) => {
                                        info!("Channel {} test connection successful", name);
                                        let _ = reply.send(Ok(()));
                                    }
                                    Err(e) => {
                                        error!("Channel {} test connection failed: {}", name, e);
                                        let _ = reply.send(Err(e.to_string()));
                                    }
                                }
                            } else {
                                let _ = reply.send(Err("Channel manager not initialized".to_string()));
                            }
                        }
                        ManagerCommand::UpdateChannel(update) => {
                            info!("Processing UpdateChannel request: {}", update.name);

                            // 1. Load config
                            let mut config = match self.loader.load() {
                                Ok(c) => c,
                                Err(e) => {
                                    error!("Failed to load config: {}", e);
                                    continue;
                                }
                            };

                            // 2. Update specific channel
                            let name = update.name.as_str();
                            let result: anyhow::Result<()> = (|| {
                                match name {
                                    "telegram" => {
                                        let mut cfg: agent_diva_core::config::schema::TelegramConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.telegram = cfg;
                                    },
                                    "discord" => {
                                        let mut cfg: agent_diva_core::config::schema::DiscordConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.discord = cfg;
                                    },
                                    "feishu" => {
                                        let mut cfg: agent_diva_core::config::schema::FeishuConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.feishu = cfg;
                                    },
                                    "whatsapp" => {
                                        let mut cfg: agent_diva_core::config::schema::WhatsAppConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.whatsapp = cfg;
                                    },
                                    "dingtalk" => {
                                        let mut cfg: agent_diva_core::config::schema::DingTalkConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.dingtalk = cfg;
                                    },
                                    "email" => {
                                        let mut cfg: agent_diva_core::config::schema::EmailConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.email = cfg;
                                    },
                                    "slack" => {
                                        let mut cfg: agent_diva_core::config::schema::SlackConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.slack = cfg;
                                    },
                                    "qq" => {
                                        let mut cfg: agent_diva_core::config::schema::QQConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.qq = cfg;
                                    },
                                    "matrix" => {
                                        let mut cfg: agent_diva_core::config::schema::MatrixConfig = serde_json::from_value(update.config)?;
                                        if let Some(enabled) = update.enabled { cfg.enabled = enabled; }
                                        config.channels.matrix = cfg;
                                    },
                                    _ => {
                                        warn!("Unknown channel: {}", name);
                                    }
                                }
                                Ok(())
                            })();

                            if let Err(e) = result {
                                error!("Failed to update channel config: {}", e);
                                continue;
                            }

                            // 3. Save config
                            if let Err(e) = self.loader.save(&config) {
                                error!("Failed to save config: {}", e);
                                continue;
                            }

                            // 4. Hot reload
                            if let Some(cm) = &self.channel_manager {
                                if let Err(e) = cm.update_channel(name, config).await {
                                    error!("Failed to reload channel {}: {}", name, e);
                                } else {
                                    info!("Channel {} reloaded successfully", name);
                                }
                            }
                        }
                    }
                }
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
