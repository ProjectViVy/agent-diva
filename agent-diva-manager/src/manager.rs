use agent_diva_agent::runtime_control::RuntimeControlCommand;
use agent_diva_agent::tool_config::network::{
    NetworkToolConfig, WebFetchRuntimeConfig, WebRuntimeConfig, WebSearchRuntimeConfig,
};
use agent_diva_channels::ChannelManager;
use agent_diva_core::bus::{AgentEvent, MessageBus};
use agent_diva_core::config::ConfigLoader;
use agent_diva_core::cron::CronService;
use agent_diva_providers::{DynamicProvider, LiteLLMClient, ProviderRegistry};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::state::{ConfigResponse, ManagerCommand, ToolsConfigResponse};

pub struct Manager {
    api_rx: mpsc::Receiver<ManagerCommand>,
    bus: MessageBus,
    provider: Arc<DynamicProvider>,
    loader: ConfigLoader,
    // Current config state
    current_model: String,
    current_api_base: Option<String>,
    current_api_key: Option<String>,
    channel_manager: Option<Arc<ChannelManager>>,
    runtime_control_tx: Option<mpsc::UnboundedSender<RuntimeControlCommand>>,
    cron_service: Arc<CronService>,
}

impl Manager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        api_rx: mpsc::Receiver<ManagerCommand>,
        bus: MessageBus,
        provider: Arc<DynamicProvider>,
        loader: ConfigLoader,
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
            current_model: initial_model,
            current_api_base: api_base,
            current_api_key: api_key,
            channel_manager,
            runtime_control_tx,
            cron_service,
        }
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

                            // 2. Update model if provided
                            let model_to_use = if let Some(ref m) = update.model {
                                info!("Updating model to: {}", m);
                                config.agents.defaults.model = m.clone();
                                self.current_model = m.clone();
                                m.clone()
                            } else {
                                self.current_model.clone()
                            };

                            // 3. Update API Key/Base if provided
                            // We need to find the provider for the model
                            let registry = ProviderRegistry::new();
                            let name_from_prefix = model_to_use
                                .split('/')
                                .next()
                                .and_then(|prefix| registry.find_by_name(prefix))
                                .map(|spec| spec.name.clone());

                            let spec = name_from_prefix
                                .as_deref()
                                .and_then(|name| registry.find_by_name(name))
                                .or_else(|| registry.find_by_model(&model_to_use));

                            if let Some(spec) = spec {
                                let provider_config = match spec.name.as_str() {
                                    "anthropic" => Some(&mut config.providers.anthropic),
                                    "openai" => Some(&mut config.providers.openai),
                                    "openrouter" => Some(&mut config.providers.openrouter),
                                    "deepseek" => Some(&mut config.providers.deepseek),
                                    "groq" => Some(&mut config.providers.groq),
                                    "zhipu" => Some(&mut config.providers.zhipu),
                                    "dashscope" => Some(&mut config.providers.dashscope),
                                    "vllm" => Some(&mut config.providers.vllm),
                                    "gemini" => Some(&mut config.providers.gemini),
                                    "moonshot" => Some(&mut config.providers.moonshot),
                                    "minimax" => Some(&mut config.providers.minimax),
                                    "aihubmix" => Some(&mut config.providers.aihubmix),
                                    "custom" => Some(&mut config.providers.custom),
                                    _ => None,
                                };

                                if let Some(cfg) = provider_config {
                                    if let Some(ref k) = update.api_key {
                                        info!("Updating API key for provider: {}", spec.name);
                                        cfg.api_key = k.clone();
                                        self.current_api_key = Some(k.clone());
                                    }
                                    if let Some(ref b) = update.api_base {
                                        info!("Updating API base for provider: {}", spec.name);
                                        cfg.api_base = Some(b.clone());
                                        self.current_api_base = Some(b.clone());
                                    }
                                }
                            } else {
                                warn!("No provider found for model: {}", model_to_use);
                            }

                            // 4. Save config
                            if let Err(e) = self.loader.save(&config) {
                                error!("Failed to save config: {}", e);
                                return Err(e.into());
                            }
                            info!("Configuration saved to disk");

                            // 5. Hot Reload Provider
                            info!("Hot reloading provider for model: {}", model_to_use);

                            // Re-resolve spec to get the latest config
                            let registry = ProviderRegistry::new();
                            let name_from_prefix = model_to_use
                                .split('/')
                                .next()
                                .and_then(|prefix| registry.find_by_name(prefix))
                                .map(|spec| spec.name.clone());

                            let spec = name_from_prefix
                                .as_deref()
                                .and_then(|name| registry.find_by_name(name))
                                .or_else(|| registry.find_by_model(&model_to_use));

                            if let Some(spec) = spec {
                                let provider_config = match spec.name.as_str() {
                                    "anthropic" => Some(&config.providers.anthropic),
                                    "openai" => Some(&config.providers.openai),
                                    "openrouter" => Some(&config.providers.openrouter),
                                    "deepseek" => Some(&config.providers.deepseek),
                                    "groq" => Some(&config.providers.groq),
                                    "zhipu" => Some(&config.providers.zhipu),
                                    "dashscope" => Some(&config.providers.dashscope),
                                    "vllm" => Some(&config.providers.vllm),
                                    "gemini" => Some(&config.providers.gemini),
                                    "moonshot" => Some(&config.providers.moonshot),
                                    "minimax" => Some(&config.providers.minimax),
                                    "aihubmix" => Some(&config.providers.aihubmix),
                                    "custom" => Some(&config.providers.custom),
                                    _ => None,
                                };

                                if let Some(cfg) = provider_config {
                                    let api_key = if !cfg.api_key.is_empty() {
                                        Some(cfg.api_key.clone())
                                    } else {
                                        None
                                    };

                                    let api_base = if let Some(base) = &cfg.api_base {
                                        if !base.trim().is_empty() {
                                            Some(base.trim().to_string())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };

                                    let extra_headers = if let Some(headers) = &cfg.extra_headers {
                                        if !headers.is_empty() {
                                            Some(headers.clone())
                                        } else {
                                            None
                                        }
                                    } else {
                                        None
                                    };

                                    let new_client = LiteLLMClient::new(
                                        api_key,
                                        api_base,
                                        model_to_use.clone(),
                                        extra_headers,
                                        Some(spec.name.clone()),
                                        config.agents.defaults.reasoning_effort.clone(),
                                    );

                                    self.provider.update(Arc::new(new_client));
                                    info!("Provider updated successfully");
                                } else {
                                    warn!("Provider config not found for spec: {}", spec.name);
                                }
                            } else {
                                warn!("No provider found for model: {}, skipping provider update", model_to_use);
                            }
                        }
                        ManagerCommand::GetConfig(reply) => {
                            debug!("Processing GetConfig request");
                            let resp = ConfigResponse {
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
