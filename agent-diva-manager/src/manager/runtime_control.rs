use agent_diva_agent::runtime_control::RuntimeControlCommand;
use agent_diva_core::bus::AgentEvent;
use agent_diva_core::channel::DeliveryReceipt;
use agent_diva_core::config::schema::{
    ChannelsConfig, Config, DingTalkConfig, DiscordConfig, EmailConfig, FeishuConfig, QQConfig,
    SelfEvolutionConfig, TelegramConfig, WebToolsConfig,
};
use agent_diva_core::config::ConfigLoader;
use agent_diva_providers::{
    build_llm_provider, LlmProviderBuildOptions, ProviderAccess, ProviderCatalogService,
};
use std::sync::Arc;
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

use super::Manager;
use crate::state::{
    ApiRequest, ChannelUpdate, ConfigResponse, ConfigUpdate, ResetSessionRequest, StopChatRequest,
    ToolsConfigResponse, ToolsConfigUpdate,
};

const CHANNEL_PROBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(35);

/// Run one runtime transition to a definite terminal result.  The transition
/// is intentionally awaited directly: a caller timeout must not drop a future
/// halfway through registry installation or leave a detached late commit.
/// ChannelRuntime bounds its contended lock and each worker shutdown internally
/// before it mutates the active generation.
async fn reconfigure_channel_runtime(
    runtime: &Arc<agent_diva_channels::runtime::ChannelRuntime>,
    config: &Config,
) -> Result<(), String> {
    match runtime.reconfigure(config).await {
        Ok(()) => Ok(()),
        Err(error) => {
            tracing::error!(error = %error, "channel runtime transition failed");
            Err("channel runtime transition failed".to_string())
        }
    }
}

fn persistence_failure_message(
    operation: &str,
    config_rollback_failed: bool,
    runtime_rollback_failed: bool,
) -> String {
    match (config_rollback_failed, runtime_rollback_failed) {
        (false, false) => format!("{operation} persistence failed"),
        (true, false) => format!("{operation} persistence failed; configuration rollback failed"),
        (false, true) => format!("{operation} persistence failed; runtime rollback failed"),
        (true, true) => {
            format!("{operation} persistence failed; configuration and runtime rollback failed")
        }
    }
}

impl Manager {
    pub(super) fn handle_chat(&self, req: ApiRequest) {
        debug!("Processing typed Chat request via Fabric");
        let channel = req.envelope.address.channel.clone();
        let chat_id = req.envelope.address.chat_id.clone();
        let session_key = req.envelope.correlation.session_key.clone();
        let request_id = req.envelope.correlation.request_id.clone();
        let event_tx = req.event_tx.clone();
        let event_rx = self.bus.subscribe_events();
        let Some(fabric) = self.fabric_handle.clone() else {
            let _ = event_tx.send(AgentEvent::Error {
                message: "Fabric ingress is not initialized".to_string(),
            });
            return;
        };
        tokio::spawn(async move {
            let cancel = tokio_util::sync::CancellationToken::new();
            if let Err(error) = fabric
                .admit_ingress(req.envelope, std::time::Duration::from_secs(2), &cancel)
                .await
            {
                let _ = event_tx.send(AgentEvent::Error {
                    message: error.to_string(),
                });
                return;
            }
            forward_chat_events(
                event_rx,
                event_tx,
                channel,
                chat_id,
                session_key,
                request_id,
            )
            .await;
        });
    }

    pub(super) fn handle_stop_chat(
        &self,
        req: StopChatRequest,
        reply: oneshot::Sender<Result<agent_diva_core::bus::SessionControlOutcome, String>>,
    ) {
        let channel = req.channel.unwrap_or_else(|| "api".to_string());
        let chat_id = req.chat_id.unwrap_or_else(|| "default".to_string());
        let session_key = format!("{channel}:{chat_id}");
        let response = self
            .runtime_control_tx
            .as_ref()
            .cloned()
            .ok_or_else(|| "runtime control channel is not initialized".to_string());
        match response {
            Ok(tx) => {
                tokio::spawn(async move {
                    let (reply_tx, reply_rx) = oneshot::channel();
                    let result = tx
                        .send(RuntimeControlCommand::StopSession {
                            session_key,
                            request_id: req.request_id,
                            reply_tx,
                        })
                        .await
                        .map_err(|error| {
                            format!("failed to send runtime control command: {error}")
                        });
                    let result = match result {
                        Ok(()) => reply_rx.await.map_err(|error| {
                            format!("failed to receive runtime control outcome: {error}")
                        }),
                        Err(error) => Err(error),
                    };
                    let _ = reply.send(result);
                });
            }
            Err(error) => {
                let _ = reply.send(Err(error));
            }
        }
    }

    pub(super) fn handle_reset_session(
        &self,
        req: ResetSessionRequest,
        reply: oneshot::Sender<Result<agent_diva_core::bus::SessionControlOutcome, String>>,
    ) {
        let channel = req.channel.unwrap_or_else(|| "api".to_string());
        let chat_id = req.chat_id.unwrap_or_else(|| "default".to_string());
        let session_key = format!("{channel}:{chat_id}");
        let Some(tx) = self.runtime_control_tx.as_ref().cloned() else {
            let _ = reply.send(Err("runtime control channel is not initialized".to_string()));
            return;
        };
        tokio::spawn(async move {
            let (reply_tx, reply_rx) = oneshot::channel();
            let result = tx
                .send(RuntimeControlCommand::ResetSession {
                    session_key,
                    reply_tx,
                })
                .await
                .map_err(|error| format!("failed to send runtime control command: {error}"));
            let result = match result {
                Ok(()) => reply_rx
                    .await
                    .map_err(|error| format!("failed to receive runtime control outcome: {error}")),
                Err(error) => Err(error),
            };
            let _ = reply.send(result);
        });
    }

    pub(super) async fn handle_get_sessions(
        &self,
        reply: oneshot::Sender<Result<Vec<agent_diva_core::session::SessionInfo>, String>>,
    ) {
        let response = self
            .with_runtime_control(
                |tx| async move {
                    let (reply_tx, reply_rx) = oneshot::channel();
                    tx.send(RuntimeControlCommand::GetSessions { reply_tx })
                        .await
                        .map_err(|e| format!("failed to send GetSessions command: {}", e))?;
                    reply_rx
                        .await
                        .map_err(|e| format!("failed to receive sessions: {}", e))
                },
                "runtime control channel is not initialized",
            )
            .await;
        let _ = reply.send(response);
    }

    pub(super) async fn handle_get_session_history(
        &self,
        session_key: String,
        reply: oneshot::Sender<Result<Option<agent_diva_core::session::store::Session>, String>>,
    ) {
        let response = self
            .with_runtime_control(
                |tx| async move {
                    let (reply_tx, reply_rx) = oneshot::channel();
                    tx.send(RuntimeControlCommand::GetSession {
                        session_key,
                        reply_tx,
                    })
                    .await
                    .map_err(|e| format!("failed to send GetSession command: {}", e))?;
                    reply_rx
                        .await
                        .map_err(|e| format!("failed to receive session: {}", e))
                },
                "runtime control channel is not initialized",
            )
            .await;
        let _ = reply.send(response);
    }

    pub(super) async fn handle_delete_session(
        &self,
        session_key: String,
        reply: oneshot::Sender<Result<bool, String>>,
    ) {
        let response = self
            .with_runtime_control(
                |tx| async move {
                    let (reply_tx, reply_rx) = oneshot::channel();
                    tx.send(RuntimeControlCommand::DeleteSession {
                        session_key,
                        reply_tx,
                    })
                    .await
                    .map_err(|e| format!("failed to send DeleteSession command: {}", e))?;
                    reply_rx
                        .await
                        .map_err(|e| format!("failed to receive delete result: {}", e))?
                },
                "runtime control channel is not initialized",
            )
            .await;
        let _ = reply.send(response);
    }

    pub(super) async fn handle_list_cron_jobs(
        &self,
        reply: oneshot::Sender<Result<Vec<agent_diva_core::cron::CronJobDto>, String>>,
    ) {
        let jobs = self.cron_service.list_job_views(true).await;
        let _ = reply.send(Ok(jobs));
    }

    pub(super) async fn handle_get_cron_job(
        &self,
        job_id: String,
        reply: oneshot::Sender<Result<Option<agent_diva_core::cron::CronJobDto>, String>>,
    ) {
        let _ = reply.send(Ok(self.cron_service.get_job(&job_id).await));
    }

    pub(super) async fn handle_create_cron_job(
        &self,
        request: agent_diva_core::cron::CreateCronJobRequest,
        reply: oneshot::Sender<Result<agent_diva_core::cron::CronJobDto, String>>,
    ) {
        let _ = reply.send(self.cron_service.create_job(request).await);
    }

    pub(super) async fn handle_update_cron_job(
        &self,
        job_id: String,
        request: agent_diva_core::cron::UpdateCronJobRequest,
        reply: oneshot::Sender<Result<agent_diva_core::cron::CronJobDto, String>>,
    ) {
        let _ = reply.send(self.cron_service.update_job(&job_id, request).await);
    }

    pub(super) async fn handle_delete_cron_job(
        &self,
        job_id: String,
        reply: oneshot::Sender<Result<(), String>>,
    ) {
        let _ = reply.send(self.cron_service.delete_job(&job_id).await);
    }

    pub(super) async fn handle_set_cron_job_enabled(
        &self,
        job_id: String,
        enabled: bool,
        reply: oneshot::Sender<Result<agent_diva_core::cron::CronJobDto, String>>,
    ) {
        let _ = reply.send(self.cron_service.set_job_enabled(&job_id, enabled).await);
    }

    pub(super) async fn handle_run_cron_job_now(
        &self,
        job_id: String,
        force: bool,
        reply: oneshot::Sender<Result<agent_diva_core::cron::CronJobDto, String>>,
    ) {
        let _ = reply.send(self.cron_service.run_job_now(&job_id, force).await);
    }

    pub(super) async fn handle_stop_cron_job_run(
        &self,
        job_id: String,
        reply: oneshot::Sender<Result<agent_diva_core::cron::CronRunSnapshot, String>>,
    ) {
        let _ = reply.send(self.cron_service.stop_run(&job_id).await);
    }

    pub(super) async fn handle_update_config(
        &mut self,
        update: ConfigUpdate,
    ) -> anyhow::Result<()> {
        debug!("Processing UpdateConfig command");
        debug!("Update request: {:?}", update);
        info!("Processing UpdateConfig request: {:?}", update);

        let mut config = self
            .loader
            .load()
            .map_err(|e| anyhow::anyhow!("Failed to load config: {}", e))?;

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
        let clear_selection = requested_provider.is_none() && requested_model.is_none();

        if clear_selection {
            info!("Clearing active provider/model selection");
            config.agents.defaults.provider = None;
            config.agents.defaults.model.clear();
            self.current_provider = None;
            self.current_model.clear();
            self.current_api_base = None;
            self.current_api_key = None;
        } else {
            self.apply_provider_selection_update(
                &mut config,
                update,
                requested_provider,
                requested_model,
            )
            .await;
        }

        self.loader
            .save(&config)
            .map_err(|e| anyhow::anyhow!("Failed to save config: {}", e))?;
        info!("Configuration saved to disk");
        self.hot_reload_provider(&config);

        Ok(())
    }

    pub(super) fn handle_get_config(&self, reply: oneshot::Sender<ConfigResponse>) {
        debug!("Processing GetConfig request");
        let _ = reply.send(ConfigResponse {
            provider: self.current_provider.clone(),
            api_base: self.current_api_base.clone(),
            model: self.current_model.clone(),
            has_api_key: self.current_api_key.is_some(),
        });
    }

    pub(super) fn handle_get_self_evolution_config(
        &self,
        reply: oneshot::Sender<Result<SelfEvolutionConfig, String>>,
    ) {
        debug!("Processing GetSelfEvolutionConfig command");
        let response = self
            .loader
            .load()
            .map(|config| config.self_evolution)
            .map_err(|error| {
                error!("Failed to load self-evolution config: {}", error);
                error.to_string()
            });
        let _ = reply.send(response);
    }

    pub(super) fn handle_update_self_evolution_config(
        &self,
        self_evolution: SelfEvolutionConfig,
        reply: oneshot::Sender<Result<SelfEvolutionConfig, String>>,
    ) {
        info!("Processing UpdateSelfEvolutionConfig request");
        let response = (|| {
            let mut config = self.loader.load().map_err(|error| error.to_string())?;
            config.self_evolution = self_evolution;
            self.loader
                .save(&config)
                .map_err(|error| error.to_string())?;
            Ok(config.self_evolution)
        })();
        if let Err(error) = &response {
            error!("Failed to save self-evolution config: {}", error);
        }
        let _ = reply.send(response);
    }

    pub(super) fn handle_get_channels(&self, reply: oneshot::Sender<ChannelsConfig>) {
        debug!("Processing GetChannels command");
        let response = self
            .loader
            .load()
            .map(|config| config.channels)
            .unwrap_or_else(|error| {
                error!("Failed to load config for GetChannels: {}", error);
                ChannelsConfig::default()
            });
        let _ = reply.send(response);
    }

    pub(super) fn handle_get_tools(&self, reply: oneshot::Sender<ToolsConfigResponse>) {
        debug!("Processing GetTools command");
        let response = self
            .loader
            .load()
            .map(|config| ToolsConfigResponse {
                web: config.tools.web.into(),
                budget: config.tools.budget,
            })
            .unwrap_or_else(|error| {
                error!("Failed to load config for GetTools: {}", error);
                ToolsConfigResponse {
                    web: WebToolsConfig::default().into(),
                    budget: agent_diva_core::config::CompactionBudgetConfig::default(),
                }
            });
        let _ = reply.send(response);
    }

    pub(super) fn handle_update_tools(&self, update: ToolsConfigUpdate) {
        info!("Processing UpdateTools request");
        let mut config = match self.loader.load() {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load config: {}", e);
                return;
            }
        };

        config.tools.web.search = update.web.search;
        config.tools.web.fetch = update.web.fetch;
        config.tools.budget = update.budget;

        if let Err(e) = self.loader.save(&config) {
            error!("Failed to save tools config: {}", e);
            return;
        }

        if let Some(tx) = &self.runtime_control_tx {
            let network = Self::map_network_config(&config);
            if let Err(e) = tx.try_send(RuntimeControlCommand::UpdateNetwork(network)) {
                error!("Failed to send runtime tools update: {}", e);
            }
        }
    }

    pub(super) async fn handle_update_channel(
        &self,
        update: ChannelUpdate,
        reply: oneshot::Sender<Result<(), String>>,
    ) {
        info!("Processing UpdateChannel request: {}", update.name);
        let channel_name = update.name.clone();

        let mut config = match self.loader.load() {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load config: {}", e);
                let _ = reply.send(Err("channel configuration is unavailable".to_string()));
                return;
            }
        };
        let previous = config.clone();

        if let Err(e) = Self::apply_channel_update(&mut config, &update) {
            error!("Failed to update channel config: {}", e);
            let message = if e.to_string().starts_with("Unknown channel:") {
                e.to_string()
            } else {
                "invalid channel configuration".to_string()
            };
            let _ = reply.send(Err(message));
            return;
        }

        if let Some(runtime) = &self.channel_runtime {
            if let Err(e) = reconfigure_channel_runtime(runtime, &config).await {
                error!(channel = %channel_name, error = %e, "failed to reload channel");
                // Persistence has not started yet, so a failed runtime
                // transition cannot leave a late config rollback behind.
                let _ = reply.send(Err(e));
                return;
            }
        }

        if let Err(e) = self.loader.save(&config) {
            error!(channel = %channel_name, error = %e, "failed to persist channel config");
            let config_rollback_failed = match self.loader.save(&previous) {
                Ok(()) => false,
                Err(rollback_error) => {
                    error!(
                        channel = %channel_name,
                        error = %rollback_error,
                        "failed to restore channel config after persistence failure"
                    );
                    true
                }
            };
            let runtime_rollback_failed = if let Some(runtime) = &self.channel_runtime {
                match reconfigure_channel_runtime(runtime, &previous).await {
                    Ok(()) => false,
                    Err(restore_error) => {
                        error!(
                            channel = %channel_name,
                            error = %restore_error,
                            "failed to restore channel runtime after persistence failure"
                        );
                        true
                    }
                }
            } else {
                false
            };
            let _ = reply.send(Err(persistence_failure_message(
                "channel configuration",
                config_rollback_failed,
                runtime_rollback_failed,
            )));
            return;
        }
        info!("Channel {} reloaded successfully", channel_name);
        let _ = reply.send(Ok(()));
    }

    pub(super) fn handle_probe_channel(
        &self,
        name: String,
        candidate: serde_json::Value,
        reply: oneshot::Sender<Result<DeliveryReceipt, agent_diva_channels::ChannelProbeError>>,
    ) {
        let loader = self.loader.clone();
        let runtime = self.channel_runtime.clone();
        let file_manager = self.file_manager.clone();
        tokio::spawn(async move {
            let result = Self::probe_channel_candidate(
                &loader,
                runtime.as_ref(),
                file_manager,
                &name,
                candidate,
            )
            .await;
            let _ = reply.send(result);
        });
    }

    async fn probe_channel_candidate(
        loader: &ConfigLoader,
        runtime: Option<&Arc<agent_diva_channels::runtime::ChannelRuntime>>,
        file_manager: Arc<agent_diva_files::FileManager>,
        name: &str,
        candidate: serde_json::Value,
    ) -> Result<DeliveryReceipt, agent_diva_channels::ChannelProbeError> {
        let name = name.trim();
        if !is_fixed_channel(name) {
            return Err(agent_diva_channels::ChannelProbeError::UnknownChannel {
                channel: name.to_string(),
            });
        }

        let mut config = loader
            .load()
            .map_err(|_| agent_diva_channels::ChannelProbeError::Build)?;
        Self::apply_channel_update(
            &mut config,
            &ChannelUpdate {
                name: name.to_string(),
                enabled: None,
                config: candidate,
            },
        )
        .map_err(|_| agent_diva_channels::ChannelProbeError::InvalidConfig)?;

        if let Some(runtime) = runtime {
            runtime
                .probe_candidate(&config, name, CHANNEL_PROBE_TIMEOUT)
                .await
        } else {
            let attachments = Arc::new(
                crate::channel_attachment_store::FileManagerAttachmentStore::new(file_manager),
            );
            let services = agent_diva_channels::AdapterServices::new(attachments);
            agent_diva_channels::probe_candidate(&config, name, services, CHANNEL_PROBE_TIMEOUT)
                .await
        }
    }

    pub(super) async fn handle_delete_channel(
        &self,
        name: String,
        reply: oneshot::Sender<Result<(), String>>,
    ) {
        let channel_name = name.trim().to_string();
        let response = self.delete_channel_transaction(&channel_name).await;
        let _ = reply.send(response);
    }

    async fn delete_channel_transaction(&self, name: &str) -> Result<(), String> {
        if !is_fixed_channel(name) {
            return Err(format!("Unknown channel: {name}"));
        }

        let mut config = self.loader.load().map_err(|error| error.to_string())?;
        let previous = config.clone();
        reset_channel(&mut config, name).map_err(|error| error.to_string())?;

        if let Some(runtime) = &self.channel_runtime {
            if let Err(error) = reconfigure_channel_runtime(runtime, &config).await {
                error!(channel = %name, error = %error, "failed to reload channel for deletion");
                // The tombstone is not persisted until the final runtime
                // generation is known to be installed.
                return Err("channel deletion could not be applied".to_string());
            }
        }

        if let Err(error) = self.loader.save(&config) {
            error!(channel = %name, error = %error, "failed to persist channel deletion");
            let config_rollback_failed = match self.loader.save(&previous) {
                Ok(()) => false,
                Err(rollback_error) => {
                    error!(
                        channel = %name,
                        error = %rollback_error,
                        "failed to restore channel config after deletion persistence failure"
                    );
                    true
                }
            };
            let runtime_rollback_failed = if let Some(runtime) = &self.channel_runtime {
                match reconfigure_channel_runtime(runtime, &previous).await {
                    Ok(()) => false,
                    Err(restore_error) => {
                        error!(
                            channel = %name,
                            error = %restore_error,
                            "failed to restore channel runtime after deletion persistence failure"
                        );
                        true
                    }
                }
            } else {
                false
            };
            return Err(persistence_failure_message(
                "channel deletion",
                config_rollback_failed,
                runtime_rollback_failed,
            ));
        }
        Ok(())
    }

    async fn apply_provider_selection_update(
        &mut self,
        config: &mut Config,
        update: ConfigUpdate,
        requested_provider: Option<String>,
        requested_model: Option<String>,
    ) {
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
            .filter(|value| catalog.get_provider_view(config, value).is_some())
            .map(ToString::to_string)
            .or_else(|| {
                catalog.resolve_provider_id(config, &requested_model, provider_to_use.as_deref())
            });

        if let Some(provider_id) = provider_id {
            let model_to_use = Self::normalize_model_for_provider(
                config,
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
            self.current_model = model_to_use;

            let mut credentials = Self::ensure_provider_credentials_slot(config, &provider_id);
            if let Some(ref api_key) = update.api_key {
                info!("Updating API key for provider: {}", provider_id);
                credentials.set_api_key(api_key.clone());
                self.current_api_key = Some(api_key.clone());
            }
            if let Some(ref api_base) = update.api_base {
                info!("Updating API base for provider: {}", provider_id);
                credentials.set_api_base(Some(api_base.clone()));
                self.current_api_base = Some(api_base.clone());
            }
        } else {
            warn!("No provider found for model: {}", requested_model);
        }
    }

    fn hot_reload_provider(&mut self, config: &Config) {
        let model_to_use = config.agents.defaults.model.trim().to_string();
        if model_to_use.is_empty() {
            info!("Active model cleared; skipping provider hot reload");
            return;
        }

        let catalog = ProviderCatalogService::new();
        info!("Hot reloading provider for model: {}", model_to_use);

        let provider_id = catalog.resolve_provider_id(
            config,
            &model_to_use,
            config.agents.defaults.provider.as_deref(),
        );

        let Some(provider_id) = provider_id else {
            warn!(
                "No provider found for model: {}, skipping provider update",
                model_to_use
            );
            return;
        };

        self.current_provider = Some(provider_id.clone());
        let access = catalog
            .get_provider_access(config, &provider_id)
            .unwrap_or_else(|| ProviderAccess::from_config(None));
        let resolved_api_base = access.api_base.clone().or_else(|| {
            catalog
                .get_provider_view(config, &provider_id)
                .and_then(|view| view.api_base)
        });
        self.current_api_key = access.api_key.clone();
        self.current_api_base = resolved_api_base.clone();
        let spec = match catalog.provider_spec(&provider_id, &config.providers) {
            Some(spec) => spec,
            None => {
                warn!(
                    "Unknown provider: {}, skipping provider update",
                    provider_id
                );
                return;
            }
        };
        let mut access = access;
        access.api_base = resolved_api_base;
        let new_client = match build_llm_provider(LlmProviderBuildOptions {
            spec,
            access,
            model: model_to_use,
            reasoning_effort: config.agents.defaults.reasoning_effort.clone(),
            reasoning_config: None,
            response_protocol: config
                .providers
                .get(&provider_id)
                .map(|provider| provider.response_protocol)
                .unwrap_or_default(),
        }) {
            Ok(provider) => provider,
            Err(error) => {
                warn!("Failed to build provider '{}': {}", provider_id, error);
                return;
            }
        };

        self.provider.update(new_client);
        info!("Provider updated successfully");
    }

    fn apply_channel_update(config: &mut Config, update: &ChannelUpdate) -> anyhow::Result<()> {
        let name = update.name.trim();
        match name {
            "telegram" => set_channel(&mut config.channels.telegram, update)?,
            "discord" => set_channel(&mut config.channels.discord, update)?,
            "feishu" => set_channel(&mut config.channels.feishu, update)?,
            "dingtalk" => set_channel(&mut config.channels.dingtalk, update)?,
            "email" => set_channel(&mut config.channels.email, update)?,
            "qq" => set_channel(&mut config.channels.qq, update)?,
            _ => anyhow::bail!("Unknown channel: {}", name),
        }
        // A valid save is an explicit re-add/update, so it clears a prior
        // delete tombstone from the additive channel projection.
        config.channels.removed.remove(name);
        Ok(())
    }

    pub(super) async fn with_runtime_control<T, F, Fut>(
        &self,
        f: F,
        missing_message: &str,
    ) -> Result<T, String>
    where
        F: FnOnce(tokio::sync::mpsc::Sender<RuntimeControlCommand>) -> Fut,
        Fut: std::future::Future<Output = Result<T, String>>,
    {
        let tx = self
            .runtime_control_tx
            .as_ref()
            .cloned()
            .ok_or_else(|| missing_message.to_string())?;
        f(tx).await
    }
}

/// Idle timeout for chat SSE event forwarding. After one timeout window with no
/// bus events a non-terminal stall hint is sent; after a second window the stream
/// is closed with an explicit error so the client never hangs silently.
const STREAM_IDLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

async fn forward_chat_events(
    mut event_rx: tokio::sync::broadcast::Receiver<agent_diva_core::bus::AgentBusEvent>,
    event_tx: tokio::sync::mpsc::UnboundedSender<AgentEvent>,
    channel: String,
    chat_id: String,
    session_key: String,
    request_id: Option<String>,
) {
    let mut stalled_notified = false;
    loop {
        match tokio::time::timeout(STREAM_IDLE_TIMEOUT, event_rx.recv()).await {
            Ok(Ok(bus_event)) => {
                let request_matches = match request_id.as_deref() {
                    Some(expected) => bus_event.request_id.as_deref() == Some(expected),
                    None => true,
                };
                if bus_event.channel == channel
                    && bus_event.chat_id == chat_id
                    && bus_event.session_key.as_deref() == Some(session_key.as_str())
                    && request_matches
                {
                    let event = bus_event.event;
                    if event_tx.send(event.clone()).is_err() {
                        break;
                    }
                    if matches!(
                        event,
                        AgentEvent::FinalResponse { .. } | AgentEvent::Error { .. }
                    ) {
                        break;
                    }
                }
            }
            // Bus dropped (manager shutting down).
            Ok(Err(_)) => break,
            Err(_elapsed) => {
                if !stalled_notified {
                    // Non-terminal hint: keep waiting so the real terminal event
                    // (e.g. a provider failure after a long retry window) still arrives.
                    stalled_notified = true;
                    let _ = event_tx.send(AgentEvent::ProviderStalled { model: None });
                } else {
                    let _ = event_tx.send(AgentEvent::Error {
                        message: "长时间未收到响应，连接已断开。请检查 Provider 状态或网络后重试。"
                            .to_string(),
                    });
                    break;
                }
            }
        }
    }
}

fn set_channel<T>(slot: &mut T, update: &ChannelUpdate) -> anyhow::Result<()>
where
    T: serde::de::DeserializeOwned + serde::Serialize + ChannelToggle,
{
    let mut cfg: T = serde_json::from_value(update.config.clone())?;
    if let Some(enabled) = update.enabled {
        cfg.set_enabled(enabled);
    }
    *slot = cfg;
    Ok(())
}

trait ChannelToggle {
    fn set_enabled(&mut self, enabled: bool);
}

macro_rules! impl_channel_toggle {
    ($($ty:ty),* $(,)?) => {
        $(
            impl ChannelToggle for $ty {
                fn set_enabled(&mut self, enabled: bool) {
                    self.enabled = enabled;
                }
            }
        )*
    };
}

impl_channel_toggle!(
    TelegramConfig,
    DiscordConfig,
    FeishuConfig,
    DingTalkConfig,
    EmailConfig,
    QQConfig,
);

fn is_fixed_channel(name: &str) -> bool {
    matches!(
        name,
        "telegram" | "discord" | "feishu" | "dingtalk" | "email" | "qq"
    )
}

fn reset_channel(config: &mut Config, name: &str) -> anyhow::Result<()> {
    match name {
        "telegram" => config.channels.telegram = TelegramConfig::default(),
        "discord" => config.channels.discord = DiscordConfig::default(),
        "feishu" => config.channels.feishu = FeishuConfig::default(),
        "dingtalk" => config.channels.dingtalk = DingTalkConfig::default(),
        "email" => config.channels.email = EmailConfig::default(),
        "qq" => config.channels.qq = QQConfig::default(),
        _ => anyhow::bail!("Unknown channel: {name}"),
    }
    config.channels.removed.insert(name.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::bus::AgentEventBus;
    use tokio::sync::mpsc;
    use tokio::time::{advance, Duration};

    #[tokio::test(start_paused = true)]
    async fn forward_chat_events_emits_stall_then_disconnect_error_when_idle() {
        let bus = AgentEventBus::new();
        let event_rx = bus.subscribe_events();
        let (event_tx, mut event_out) = mpsc::unbounded_channel();

        let handle = tokio::spawn(forward_chat_events(
            event_rx,
            event_tx,
            "gui".to_string(),
            "chat-1".to_string(),
            "gui:chat-1".to_string(),
            None,
        ));

        // Let the forwarder start polling before advancing the clock.
        tokio::task::yield_now().await;
        // First idle window: non-terminal stall hint, stream stays open.
        advance(Duration::from_secs(61)).await;
        tokio::task::yield_now().await;
        let first = event_out.try_recv().expect("expected stall hint");
        assert!(matches!(first, AgentEvent::ProviderStalled { .. }));

        // Second idle window: explicit error and the forwarder terminates.
        advance(Duration::from_secs(61)).await;
        tokio::task::yield_now().await;
        let second = event_out.try_recv().expect("expected disconnect error");
        assert!(matches!(second, AgentEvent::Error { .. }));

        handle.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn forward_chat_events_forwards_events_and_terminates_on_final() {
        let bus = AgentEventBus::new();
        let event_rx = bus.subscribe_events();
        let (event_tx, mut event_out) = mpsc::unbounded_channel();

        let handle = tokio::spawn(forward_chat_events(
            event_rx,
            event_tx,
            "gui".to_string(),
            "chat-1".to_string(),
            "gui:chat-1".to_string(),
            None,
        ));

        bus.publish_correlated_event(
            "gui",
            "chat-1",
            "gui:chat-1",
            "request-1",
            "trace-1",
            AgentEvent::AssistantDelta {
                text: "hi".to_string(),
            },
        )
        .unwrap();
        bus.publish_correlated_event(
            "gui",
            "chat-1",
            "gui:chat-1",
            "request-1",
            "trace-1",
            AgentEvent::FinalResponse {
                content: "done".to_string(),
            },
        )
        .unwrap();
        advance(Duration::from_millis(100)).await;
        tokio::task::yield_now().await;

        let mut seen = Vec::new();
        while let Ok(ev) = event_out.try_recv() {
            seen.push(ev);
        }
        assert!(matches!(seen[0], AgentEvent::AssistantDelta { .. }));
        assert!(matches!(seen[1], AgentEvent::FinalResponse { .. }));

        handle.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn forward_chat_events_ignores_other_chat_ids() {
        let bus = AgentEventBus::new();
        let event_rx = bus.subscribe_events();
        let (event_tx, mut event_out) = mpsc::unbounded_channel();

        let handle = tokio::spawn(forward_chat_events(
            event_rx,
            event_tx,
            "gui".to_string(),
            "chat-1".to_string(),
            "gui:chat-1".to_string(),
            None,
        ));

        bus.publish_correlated_event(
            "gui",
            "chat-other",
            "gui:chat-other",
            "request-other",
            "trace-other",
            AgentEvent::FinalResponse {
                content: "noise".to_string(),
            },
        )
        .unwrap();
        advance(Duration::from_millis(100)).await;
        tokio::task::yield_now().await;
        assert!(
            event_out.try_recv().is_err(),
            "foreign chat event forwarded"
        );

        bus.publish_correlated_event(
            "gui",
            "chat-1",
            "gui:chat-1",
            "request-1",
            "trace-1",
            AgentEvent::Error {
                message: "boom".to_string(),
            },
        )
        .unwrap();
        advance(Duration::from_millis(100)).await;
        tokio::task::yield_now().await;
        assert!(matches!(
            event_out.try_recv().unwrap(),
            AgentEvent::Error { .. }
        ));

        handle.await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn forward_chat_events_isolates_concurrent_requests_in_same_chat() {
        let bus = AgentEventBus::new();
        let event_rx = bus.subscribe_events();
        let (event_tx, mut event_out) = mpsc::unbounded_channel();

        let handle = tokio::spawn(forward_chat_events(
            event_rx,
            event_tx,
            "gui".to_string(),
            "chat-1".to_string(),
            "gui:chat-1".to_string(),
            Some("request-a".to_string()),
        ));

        bus.publish_correlated_event(
            "gui",
            "chat-1",
            "gui:chat-1",
            "request-b",
            "trace-b",
            AgentEvent::SessionAdmission {
                observation: agent_diva_core::bus::SessionAdmissionObservation {
                    code: None,
                    phase: agent_diva_core::bus::SessionAdmissionPhase::Queued,
                    session_key: "gui:chat-1".to_string(),
                    request_id: "request-b".to_string(),
                    trace_id: "trace-b".to_string(),
                    queue_depth: 1,
                    wait_latency_ms: 0,
                },
            },
        )
        .unwrap();
        bus.publish_correlated_event(
            "gui",
            "chat-1",
            "gui:chat-1",
            "request-a",
            "trace-a",
            AgentEvent::SessionAdmission {
                observation: agent_diva_core::bus::SessionAdmissionObservation {
                    code: None,
                    phase: agent_diva_core::bus::SessionAdmissionPhase::Queued,
                    session_key: "gui:chat-1".to_string(),
                    request_id: "request-a".to_string(),
                    trace_id: "trace-a".to_string(),
                    queue_depth: 1,
                    wait_latency_ms: 0,
                },
            },
        )
        .unwrap();
        bus.publish_correlated_event(
            "gui",
            "chat-1",
            "gui:chat-1",
            "request-b",
            "trace-b",
            AgentEvent::FinalResponse {
                content: "foreign".to_string(),
            },
        )
        .unwrap();
        bus.publish_correlated_event(
            "gui",
            "chat-1",
            "gui:chat-1",
            "request-a",
            "trace-a",
            AgentEvent::AssistantDelta {
                text: "owned".to_string(),
            },
        )
        .unwrap();
        bus.publish_correlated_event(
            "gui",
            "chat-1",
            "gui:chat-1",
            "request-a",
            "trace-a",
            AgentEvent::FinalResponse {
                content: "done".to_string(),
            },
        )
        .unwrap();
        advance(Duration::from_millis(100)).await;
        tokio::task::yield_now().await;

        let mut seen = Vec::new();
        while let Ok(event) = event_out.try_recv() {
            seen.push(event);
        }
        assert_eq!(seen.len(), 3);
        assert!(matches!(
            &seen[0],
            AgentEvent::SessionAdmission { observation }
                if observation.request_id == "request-a" && observation.trace_id == "trace-a"
        ));
        assert!(matches!(
            &seen[1],
            AgentEvent::AssistantDelta { text } if text == "owned"
        ));
        assert!(matches!(
            &seen[2],
            AgentEvent::FinalResponse { content } if content == "done"
        ));
        handle.await.unwrap();
    }

    fn channel_update(
        name: &str,
        enabled: Option<bool>,
        config: serde_json::Value,
    ) -> ChannelUpdate {
        ChannelUpdate {
            name: name.to_string(),
            enabled,
            config,
        }
    }

    #[test]
    fn apply_channel_update_rejects_retired_channels() {
        let mut config = Config::default();
        for name in [
            "neuro-link",
            "generic_pipe",
            "whatsapp",
            "slack",
            "matrix",
            "irc",
            "mattermost",
            "nextcloud_talk",
        ] {
            assert!(Manager::apply_channel_update(
                &mut config,
                &channel_update(name, Some(true), serde_json::json!({})),
            )
            .is_err());
        }
    }

    #[test]
    fn apply_channel_update_rejects_unknown_channel() {
        let mut config = Config::default();
        let result = Manager::apply_channel_update(
            &mut config,
            &channel_update("carrier-pigeon", Some(true), serde_json::json!({})),
        );
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("carrier-pigeon"));
    }

    #[test]
    fn apply_channel_update_clears_a_removed_tombstone() {
        let mut config = Config::default();
        config.channels.removed.insert("telegram".to_string());

        Manager::apply_channel_update(
            &mut config,
            &channel_update(
                "telegram",
                Some(true),
                serde_json::json!({"enabled": false, "token": "candidate"}),
            ),
        )
        .unwrap();

        assert!(!config.channels.removed.contains("telegram"));
        assert!(config.channels.telegram.enabled);
        assert_eq!(config.channels.telegram.token, "candidate");
    }

    #[test]
    fn reset_channel_restores_defaults_and_records_a_tombstone() {
        let mut config = Config::default();
        config.channels.telegram.enabled = true;
        config.channels.telegram.token = "secret-that-must-be-removed".to_string();

        reset_channel(&mut config, "telegram").unwrap();

        assert!(!config.channels.telegram.enabled);
        assert!(config.channels.telegram.token.is_empty());
        assert!(config.channels.removed.contains("telegram"));
    }

    #[test]
    fn reset_channel_rejects_retired_and_unknown_names() {
        let mut config = Config::default();
        for name in ["neuro-link", "slack", "whatsapp", "carrier-pigeon"] {
            assert!(reset_channel(&mut config, name).is_err(), "{name}");
        }
    }

    #[test]
    fn persistence_failure_reports_each_rollback_failure() {
        assert_eq!(
            persistence_failure_message("channel configuration", false, false),
            "channel configuration persistence failed"
        );
        assert_eq!(
            persistence_failure_message("channel deletion", true, false),
            "channel deletion persistence failed; configuration rollback failed"
        );
        assert_eq!(
            persistence_failure_message("channel deletion", false, true),
            "channel deletion persistence failed; runtime rollback failed"
        );
        assert_eq!(
            persistence_failure_message("channel deletion", true, true),
            "channel deletion persistence failed; configuration and runtime rollback failed"
        );
    }
}
