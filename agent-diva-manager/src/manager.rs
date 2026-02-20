use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info, debug, warn};
use agent_diva_core::bus::{MessageBus, AgentEvent};
use agent_diva_core::config::ConfigLoader;
use agent_diva_providers::{LiteLLMClient, DynamicProvider, ProviderRegistry};
use agent_diva_channels::ChannelManager;

use crate::state::{ManagerCommand, ConfigResponse};

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
}

impl Manager {
    pub fn new(
        api_rx: mpsc::Receiver<ManagerCommand>,
        bus: MessageBus,
        provider: Arc<DynamicProvider>,
        loader: ConfigLoader,
        initial_model: String,
        api_key: Option<String>,
        api_base: Option<String>,
        channel_manager: Option<Arc<ChannelManager>>,
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
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        println!("DEBUG: Manager loop started");
        info!("Manager loop started");
        
        loop {
            println!("DEBUG: Waiting for command...");
            tokio::select! {
                msg = self.api_rx.recv() => {
                    let cmd = match msg {
                        Some(cmd) => {
                            println!("DEBUG: Received command");
                            cmd
                        },
                        None => {
                            println!("DEBUG: Manager channel closed, stopping loop");
                            info!("Manager channel closed, stopping loop");
                            break Ok(());
                        }
                    };
                    match cmd {
                        ManagerCommand::Chat(req) => {
                            println!("DEBUG: Processing Chat command");
                            debug!("Processing Chat request via Bus");
                            let channel = req.msg.channel.clone();
                            let chat_id = req.msg.chat_id.clone();
                            let event_tx = req.event_tx.clone();
                            
                            // Subscribe to bus events
                            let mut event_rx = self.bus.subscribe_events();
                            
                            // Publish inbound
                            if let Err(e) = self.bus.publish_inbound(req.msg) {
                                println!("DEBUG: Failed to publish inbound: {}", e);
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
                                                    let _ = event_tx.send(event.clone());
                                                    
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
                        ManagerCommand::UpdateConfig(update) => {
                            println!("DEBUG: Processing UpdateConfig command");
                            println!("DEBUG: Update request: {:?}", update);
                            info!("Processing UpdateConfig request: {:?}", update);
                            
                            // 1. Load current config
                            let mut config = match self.loader.load() {
                                Ok(c) => c,
                                Err(e) => {
                                    println!("DEBUG: Failed to load config: {}", e);
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
                                println!("DEBUG: Failed to save config: {}", e);
                                error!("Failed to save config: {}", e);
                                return Err(e.into());
                            }
                            info!("Configuration saved to disk");
                            
                            // 5. Hot Reload Provider
                            println!("DEBUG: Hot reloading provider for model: {}", model_to_use);
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
                                    );

                                    self.provider.update(Arc::new(new_client));
                                    println!("DEBUG: Provider updated successfully");
                                    info!("Provider updated successfully");
                                } else {
                                    println!("DEBUG: Provider config not found for spec: {}", spec.name);
                                    warn!("Provider config not found for spec: {}", spec.name);
                                }
                            } else {
                                println!("DEBUG: No provider found for model: {}, skipping provider update", model_to_use);
                                warn!("No provider found for model: {}, skipping provider update", model_to_use);
                            }
                        }
                        ManagerCommand::GetConfig(reply) => {
                            println!("DEBUG: Processing GetConfig command");
                            debug!("Processing GetConfig request");
                            let resp = ConfigResponse {
                                api_base: self.current_api_base.clone(),
                                model: self.current_model.clone(),
                                has_api_key: self.current_api_key.is_some(),
                            };
                            let _ = reply.send(resp);
                        }
                        ManagerCommand::GetChannels(reply) => {
                            println!("DEBUG: Processing GetChannels command");
                            if let Ok(config) = self.loader.load() {
                                let _ = reply.send(config.channels);
                            } else {
                                error!("Failed to load config for GetChannels");
                                let _ = reply.send(agent_diva_core::config::schema::ChannelsConfig::default());
                            }
                        }
                        ManagerCommand::UpdateChannel(update) => {
                            println!("DEBUG: Processing UpdateChannel command: {}", update.name);
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
