//! Channel manager

use crate::base::{ChannelError, ChannelHandler, ChannelHandlerPtr, Result};
use crate::dingtalk::DingTalkHandler;
use crate::discord::DiscordHandler;
use crate::email::EmailHandler;
use crate::feishu::FeishuHandler;
use crate::qq::QQHandler;
use crate::slack::SlackHandler;
use crate::telegram::TelegramHandler;
use crate::whatsapp::WhatsAppHandler;
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::Config;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

/// Channel manager that coordinates all channel handlers
pub struct ChannelManager {
    /// Configuration
    config: Config,
    /// Channel handlers
    handlers: RwLock<HashMap<String, ChannelHandlerPtr>>,
    /// Inbound message sender
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    /// Outbound message receiver
    #[allow(dead_code)]
    outbound_rx: Option<mpsc::Receiver<OutboundMessage>>,
    /// Running state
    #[allow(dead_code)]
    running: bool,
}

impl ChannelManager {
    /// Create a new channel manager
    pub fn new(config: Config) -> Self {
        Self {
            config,
            handlers: RwLock::new(HashMap::new()),
            inbound_tx: None,
            outbound_rx: None,
            running: false,
        }
    }

    /// Set the inbound message sender
    pub fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    /// Initialize channels based on configuration
    pub async fn initialize(&self) -> Result<()> {
        let mut handlers = self.handlers.write().await;

        // Initialize Telegram channel
        if self.config.channels.telegram.enabled {
            if !self.config.channels.telegram.token.is_empty() {
                let mut handler = TelegramHandler::new(&self.config.channels.telegram);
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "telegram".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("Telegram channel initialized");
            } else {
                tracing::warn!("Telegram channel enabled but token not configured");
            }
        }

        // Initialize Discord channel
        if self.config.channels.discord.enabled {
            if !self.config.channels.discord.token.is_empty() {
                let mut handler = DiscordHandler::new(&self.config.channels.discord);
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "discord".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("Discord channel initialized");
            } else {
                tracing::warn!("Discord channel enabled but token not configured");
            }
        }

        // Initialize Feishu channel
        if self.config.channels.feishu.enabled {
            if !self.config.channels.feishu.app_id.is_empty() {
                let mut handler =
                    FeishuHandler::new(self.config.channels.feishu.clone(), self.config.clone());
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "feishu".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("Feishu channel initialized");
            } else {
                tracing::warn!("Feishu channel enabled but app_id not configured");
            }
        }

        // Initialize WhatsApp channel
        if self.config.channels.whatsapp.enabled {
            let mut handler = WhatsAppHandler::new(self.config.channels.whatsapp.clone());
            if let Some(ref tx) = self.inbound_tx {
                handler.set_inbound_sender(tx.clone());
            }
            handlers.insert(
                "whatsapp".to_string(),
                Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
            );
            tracing::info!("WhatsApp channel initialized");
        }

        // Initialize DingTalk channel
        if self.config.channels.dingtalk.enabled {
            if !self.config.channels.dingtalk.client_id.is_empty() {
                let mut handler = DingTalkHandler::new(
                    self.config.channels.dingtalk.clone(),
                    self.config.clone(),
                );
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "dingtalk".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("DingTalk channel initialized");
            } else {
                tracing::warn!("DingTalk channel enabled but client_id not configured");
            }
        }

        // Initialize Email channel
        if self.config.channels.email.enabled {
            if !self.config.channels.email.imap_username.is_empty() {
                let mut handler =
                    EmailHandler::new(self.config.channels.email.clone(), self.config.clone());
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "email".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("Email channel initialized");
            } else {
                tracing::warn!("Email channel enabled but imap_username not configured");
            }
        }

        // Initialize Slack channel
        if self.config.channels.slack.enabled {
            if !self.config.channels.slack.bot_token.is_empty()
                && !self.config.channels.slack.app_token.is_empty()
            {
                let mut handler = SlackHandler::new(self.config.channels.slack.clone());
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "slack".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("Slack channel initialized");
            } else {
                tracing::warn!("Slack channel enabled but bot_token/app_token not configured");
            }
        }

        // Initialize QQ channel
        if self.config.channels.qq.enabled {
            if !self.config.channels.qq.app_id.is_empty() {
                let mut handler =
                    QQHandler::new(self.config.channels.qq.clone(), self.config.clone());
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "qq".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("QQ channel initialized");
            } else {
                tracing::warn!("QQ channel enabled but app_id not configured");
            }
        }

        Ok(())
    }

    /// Start all channel handlers
    pub async fn start_all(&self) -> Result<()> {
        let handlers = self.handlers.read().await;

        for (name, handler) in handlers.iter() {
            tracing::info!("Starting {} channel...", name);
            let mut handler = handler.write().await;
            if let Err(e) = handler.start().await {
                tracing::error!("Failed to start {} channel: {}", name, e);
            }
        }

        Ok(())
    }

    /// Stop all channel handlers
    pub async fn stop_all(&self) -> Result<()> {
        let mut handlers = self.handlers.write().await;

        for (name, handler) in handlers.iter_mut() {
            tracing::info!("Stopping {} channel...", name);
            let mut handler = handler.write().await;
            if let Err(e) = handler.stop().await {
                tracing::error!("Failed to stop {} channel: {}", name, e);
            }
        }

        handlers.clear();
        Ok(())
    }

    /// Get a channel handler by name
    pub async fn get_handler(&self, name: &str) -> Option<ChannelHandlerPtr> {
        let handlers = self.handlers.read().await;
        handlers.get(name).cloned()
    }

    /// Send a message through a specific channel
    pub async fn send(&self, channel: &str, message: OutboundMessage) -> Result<()> {
        let handlers = self.handlers.read().await;
        let handler = handlers
            .get(channel)
            .ok_or_else(|| ChannelError::NotConfigured(format!("Channel {} not found", channel)))?;

        let handler = handler.read().await;
        handler.send(message).await
    }

    /// Update a specific channel configuration
    pub async fn update_channel(&self, name: &str, new_config: Config) -> Result<()> {
        let mut handlers = self.handlers.write().await;

        // 1. Stop and remove existing handler
        if let Some(handler) = handlers.get(name) {
            tracing::info!("Stopping {} channel for update...", name);
            let mut handler = handler.write().await;
            if let Err(e) = handler.stop().await {
                tracing::error!("Failed to stop {} channel: {}", name, e);
            }
        }
        handlers.remove(name);

        // 2. Initialize new handler if enabled
        let handler: Option<Arc<RwLock<dyn ChannelHandler>>> = match name {
            "telegram" => {
                if new_config.channels.telegram.enabled && !new_config.channels.telegram.token.is_empty() {
                    Some(Arc::new(RwLock::new(TelegramHandler::new(&new_config.channels.telegram))))
                } else {
                    None
                }
            }
            "discord" => {
                if new_config.channels.discord.enabled && !new_config.channels.discord.token.is_empty() {
                    Some(Arc::new(RwLock::new(DiscordHandler::new(&new_config.channels.discord))))
                } else {
                    None
                }
            }
            "feishu" => {
                if new_config.channels.feishu.enabled && !new_config.channels.feishu.app_id.is_empty() {
                    Some(Arc::new(RwLock::new(FeishuHandler::new(new_config.channels.feishu.clone(), new_config.clone()))))
                } else {
                    None
                }
            }
            "whatsapp" => {
                if new_config.channels.whatsapp.enabled {
                    Some(Arc::new(RwLock::new(WhatsAppHandler::new(new_config.channels.whatsapp.clone()))))
                } else {
                    None
                }
            }
            "dingtalk" => {
                if new_config.channels.dingtalk.enabled && !new_config.channels.dingtalk.client_id.is_empty() {
                    Some(Arc::new(RwLock::new(DingTalkHandler::new(new_config.channels.dingtalk.clone(), new_config.clone()))))
                } else {
                    None
                }
            }
            "email" => {
                if new_config.channels.email.enabled && !new_config.channels.email.imap_username.is_empty() {
                    Some(Arc::new(RwLock::new(EmailHandler::new(new_config.channels.email.clone(), new_config.clone()))))
                } else {
                    None
                }
            }
            "slack" => {
                if new_config.channels.slack.enabled && !new_config.channels.slack.bot_token.is_empty() {
                    Some(Arc::new(RwLock::new(SlackHandler::new(new_config.channels.slack.clone()))))
                } else {
                    None
                }
            }
            "qq" => {
                if new_config.channels.qq.enabled && !new_config.channels.qq.app_id.is_empty() {
                    Some(Arc::new(RwLock::new(QQHandler::new(new_config.channels.qq.clone(), new_config.clone()))))
                } else {
                    None
                }
            }
            _ => None,
        };

        // 3. Register and start new handler
        if let Some(handler) = handler {
            // Set inbound sender
            if let Some(ref tx) = self.inbound_tx {
                let mut h = handler.write().await;
                h.set_inbound_sender(tx.clone());
            }

            // Start handler
            tracing::info!("Starting {} channel after update...", name);
            {
                let mut h = handler.write().await;
                if let Err(e) = h.start().await {
                    tracing::error!("Failed to start {} channel: {}", name, e);
                    // Don't insert if failed to start? Or insert anyway?
                    // If we don't insert, it's effectively disabled.
                    return Err(e);
                }
            }

            handlers.insert(name.to_string(), handler);
            tracing::info!("{} channel updated and started", name);
        } else {
            tracing::info!("{} channel disabled or invalid config", name);
        }

        Ok(())
    }

    /// Check if a channel is running
    pub async fn is_channel_running(&self, name: &str) -> bool {
        use crate::base::ChannelHandler;
        let handlers = self.handlers.read().await;
        if let Some(handler) = handlers.get(name) {
            let handler: tokio::sync::RwLockReadGuard<'_, dyn ChannelHandler> =
                handler.read().await;
            handler.is_running()
        } else {
            false
        }
    }

    /// Get list of active channels
    pub async fn list_channels(&self) -> Vec<String> {
        let handlers = self.handlers.read().await;
        handlers.keys().cloned().collect()
    }
}

impl Default for ChannelManager {
    fn default() -> Self {
        Self::new(Config::default())
    }
}
