use std::collections::HashMap;
use std::sync::Arc;

use agent_diva_agent::runtime_control::RuntimeControlCommand;
use agent_diva_core::bus::AgentEvent;
use agent_diva_core::config::schema::{
    ChannelsConfig, Config, DingTalkConfig, DiscordConfig, EmailConfig, FeishuConfig, MatrixConfig,
    QQConfig, SlackConfig, TelegramConfig, WebToolsConfig, WhatsAppConfig,
};
use agent_diva_providers::{LiteLLMClient, ProviderAccess, ProviderCatalogService};
use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

use super::Manager;
use crate::state::{
    ApiRequest, ChannelUpdate, ConfigResponse, ConfigUpdate, ResetSessionRequest, StopChatRequest,
    ToolsConfigResponse, ToolsConfigUpdate,
};

impl Manager {
    pub(super) fn handle_chat(&self, req: ApiRequest) {
        debug!("Processing Chat request via Bus");
        let channel = req.msg.channel.clone();
        let chat_id = req.msg.chat_id.clone();
        let event_tx = req.event_tx.clone();
        let mut event_rx = self.bus.subscribe_events();

        if let Err(e) = self.bus.publish_inbound(req.msg) {
            error!("Failed to publish inbound: {}", e);
            let _ = event_tx.send(AgentEvent::Error {
                message: e.to_string(),
            });
            return;
        }

        tokio::spawn(async move {
            while let Ok(Ok(bus_event)) =
                tokio::time::timeout(std::time::Duration::from_secs(60), event_rx.recv()).await
            {
                if bus_event.channel == channel && bus_event.chat_id == chat_id {
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
        });
    }

    pub(super) fn handle_stop_chat(
        &self,
        req: StopChatRequest,
        reply: oneshot::Sender<Result<bool, String>>,
    ) {
        self.send_runtime_session_command(
            req.channel,
            req.chat_id,
            |session_key| RuntimeControlCommand::StopSession { session_key },
            reply,
        );
    }

    pub(super) fn handle_reset_session(
        &self,
        req: ResetSessionRequest,
        reply: oneshot::Sender<Result<bool, String>>,
    ) {
        self.send_runtime_session_command(
            req.channel,
            req.chat_id,
            |session_key| RuntimeControlCommand::ResetSession { session_key },
            reply,
        );
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
            })
            .unwrap_or_else(|error| {
                error!("Failed to load config for GetTools: {}", error);
                ToolsConfigResponse {
                    web: WebToolsConfig::default().into(),
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

        if let Err(e) = self.loader.save(&config) {
            error!("Failed to save tools config: {}", e);
            return;
        }

        if let Some(tx) = &self.runtime_control_tx {
            let network = Self::map_network_config(&config);
            if let Err(e) = tx.send(RuntimeControlCommand::UpdateNetwork(network)) {
                error!("Failed to send runtime tools update: {}", e);
            }
        }
    }

    pub(super) async fn handle_update_channel(&self, update: ChannelUpdate) {
        info!("Processing UpdateChannel request: {}", update.name);
        let channel_name = update.name.clone();

        let mut config = match self.loader.load() {
            Ok(config) => config,
            Err(e) => {
                error!("Failed to load config: {}", e);
                return;
            }
        };

        if let Err(e) = Self::apply_channel_update(&mut config, &update) {
            error!("Failed to update channel config: {}", e);
            return;
        }
        if let Err(e) = self.loader.save(&config) {
            error!("Failed to save config: {}", e);
            return;
        }

        if let Some(cm) = &self.channel_manager {
            if let Err(e) = cm.update_channel(&channel_name, config).await {
                error!("Failed to reload channel {}: {}", channel_name, e);
            } else {
                info!("Channel {} reloaded successfully", channel_name);
            }
        }
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
        let extra_headers = (!access.extra_headers.is_empty()).then(|| {
            access
                .extra_headers
                .into_iter()
                .collect::<HashMap<String, String>>()
        });
        let resolved_api_base = access.api_base.clone().or_else(|| {
            catalog
                .get_provider_view(config, &provider_id)
                .and_then(|view| view.api_base)
        });
        self.current_api_key = access.api_key.clone();
        self.current_api_base = resolved_api_base.clone();

        let new_client = LiteLLMClient::new(
            access.api_key,
            resolved_api_base,
            model_to_use,
            extra_headers,
            Some(provider_id),
            config.agents.defaults.reasoning_effort.clone(),
        );

        self.provider.update(Arc::new(new_client));
        info!("Provider updated successfully");
    }

    fn apply_channel_update(config: &mut Config, update: &ChannelUpdate) -> anyhow::Result<()> {
        let name = update.name.as_str();
        match name {
            "telegram" => set_channel(&mut config.channels.telegram, update)?,
            "discord" => set_channel(&mut config.channels.discord, update)?,
            "feishu" => set_channel(&mut config.channels.feishu, update)?,
            "whatsapp" => set_channel(&mut config.channels.whatsapp, update)?,
            "dingtalk" => set_channel(&mut config.channels.dingtalk, update)?,
            "email" => set_channel(&mut config.channels.email, update)?,
            "slack" => set_channel(&mut config.channels.slack, update)?,
            "qq" => set_channel(&mut config.channels.qq, update)?,
            "matrix" => set_channel(&mut config.channels.matrix, update)?,
            _ => warn!("Unknown channel: {}", name),
        }
        Ok(())
    }

    fn send_runtime_session_command(
        &self,
        channel: Option<String>,
        chat_id: Option<String>,
        build: impl FnOnce(String) -> RuntimeControlCommand,
        reply: oneshot::Sender<Result<bool, String>>,
    ) {
        let channel = channel.unwrap_or_else(|| "api".to_string());
        let chat_id = chat_id.unwrap_or_else(|| "default".to_string());
        let session_key = format!("{}:{}", channel, chat_id);

        let response = self
            .runtime_control_tx
            .as_ref()
            .ok_or_else(|| "runtime control channel is not initialized".to_string())
            .and_then(|tx| {
                tx.send(build(session_key))
                    .map(|_| true)
                    .map_err(|e| format!("failed to send runtime control command: {}", e))
            });
        let _ = reply.send(response);
    }

    async fn with_runtime_control<T, F, Fut>(
        &self,
        f: F,
        missing_message: &str,
    ) -> Result<T, String>
    where
        F: FnOnce(tokio::sync::mpsc::UnboundedSender<RuntimeControlCommand>) -> Fut,
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
    WhatsAppConfig,
    DingTalkConfig,
    EmailConfig,
    SlackConfig,
    QQConfig,
    MatrixConfig,
);
