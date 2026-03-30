//! Channel manager

use crate::base::{ChannelError, ChannelHandler, ChannelHandlerPtr, Result};
use crate::dingtalk::DingTalkHandler;
use crate::discord::DiscordHandler;
use crate::email::EmailHandler;
use crate::feishu::FeishuHandler;
use crate::irc::IrcHandler;
use crate::matrix::MatrixHandler;
use crate::mattermost::MattermostHandler;
use crate::neuro_link::NeuroLinkHandler;
use crate::nextcloud_talk::NextcloudTalkHandler;
use crate::qq::QQHandler;
use crate::slack::SlackHandler;
use crate::telegram::TelegramHandler;
use crate::whatsapp::WhatsAppHandler;
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::Config;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelValidation {
    pub enabled: bool,
    pub missing_fields: Vec<&'static str>,
}

impl ChannelValidation {
    pub fn ready(&self) -> bool {
        self.enabled && self.missing_fields.is_empty()
    }
}

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
    fn channel_validation(config: &Config, name: &str) -> Option<ChannelValidation> {
        let validation = match name {
            "telegram" => ChannelValidation {
                enabled: config.channels.telegram.enabled,
                missing_fields: required_fields([("token", &config.channels.telegram.token)]),
            },
            "discord" => ChannelValidation {
                enabled: config.channels.discord.enabled,
                missing_fields: required_fields([("token", &config.channels.discord.token)]),
            },
            "whatsapp" => ChannelValidation {
                enabled: config.channels.whatsapp.enabled,
                missing_fields: required_fields([(
                    "bridge_url",
                    &config.channels.whatsapp.bridge_url,
                )]),
            },
            "feishu" => ChannelValidation {
                enabled: config.channels.feishu.enabled,
                missing_fields: required_fields([
                    ("app_id", &config.channels.feishu.app_id),
                    ("app_secret", &config.channels.feishu.app_secret),
                ]),
            },
            "dingtalk" => ChannelValidation {
                enabled: config.channels.dingtalk.enabled,
                missing_fields: required_fields([
                    ("client_id", &config.channels.dingtalk.client_id),
                    ("client_secret", &config.channels.dingtalk.client_secret),
                ]),
            },
            "email" => ChannelValidation {
                enabled: config.channels.email.enabled,
                missing_fields: required_fields([
                    ("imap_host", &config.channels.email.imap_host),
                    ("imap_username", &config.channels.email.imap_username),
                    ("imap_password", &config.channels.email.imap_password),
                    ("smtp_host", &config.channels.email.smtp_host),
                    ("smtp_username", &config.channels.email.smtp_username),
                    ("smtp_password", &config.channels.email.smtp_password),
                    ("from_address", &config.channels.email.from_address),
                ]),
            },
            "slack" => ChannelValidation {
                enabled: config.channels.slack.enabled,
                missing_fields: required_fields([
                    ("bot_token", &config.channels.slack.bot_token),
                    ("app_token", &config.channels.slack.app_token),
                ]),
            },
            "qq" => ChannelValidation {
                enabled: config.channels.qq.enabled,
                missing_fields: required_fields([
                    ("app_id", &config.channels.qq.app_id),
                    ("secret", &config.channels.qq.secret),
                ]),
            },
            "matrix" => ChannelValidation {
                enabled: config.channels.matrix.enabled,
                missing_fields: required_fields([
                    ("homeserver", &config.channels.matrix.homeserver),
                    ("user_id", &config.channels.matrix.user_id),
                    ("access_token", &config.channels.matrix.access_token),
                ]),
            },
            "neuro-link" | "generic_pipe" => ChannelValidation {
                enabled: config.channels.neuro_link.enabled,
                missing_fields: Vec::new(),
            },
            "irc" => ChannelValidation {
                enabled: config.channels.irc.enabled,
                missing_fields: required_fields([("server", &config.channels.irc.server)]),
            },
            "mattermost" => ChannelValidation {
                enabled: config.channels.mattermost.enabled,
                missing_fields: required_fields([
                    ("base_url", &config.channels.mattermost.base_url),
                    ("bot_token", &config.channels.mattermost.bot_token),
                    ("channel_id", &config.channels.mattermost.channel_id),
                ]),
            },
            "nextcloud_talk" => ChannelValidation {
                enabled: config.channels.nextcloud_talk.enabled,
                missing_fields: required_fields([
                    ("base_url", &config.channels.nextcloud_talk.base_url),
                    ("app_token", &config.channels.nextcloud_talk.app_token),
                    ("room_token", &config.channels.nextcloud_talk.room_token),
                ]),
            },
            _ => return None,
        };
        Some(validation)
    }

    pub fn configured_channel_names(config: &Config) -> Vec<String> {
        [
            "telegram",
            "discord",
            "whatsapp",
            "feishu",
            "dingtalk",
            "email",
            "slack",
            "qq",
            "matrix",
            "neuro-link",
            "irc",
            "mattermost",
            "nextcloud_talk",
        ]
        .into_iter()
        .filter(|name| {
            Self::channel_validation(config, name)
                .map(|validation| validation.ready())
                .unwrap_or(false)
        })
        .map(str::to_string)
        .collect()
    }

    fn log_skipped_channel(config: &Config, name: &str) {
        let Some(validation) = Self::channel_validation(config, name) else {
            return;
        };
        if !validation.enabled || validation.missing_fields.is_empty() {
            return;
        }

        tracing::warn!(
            "{} channel enabled but skipped: missing {}",
            name,
            validation.missing_fields.join(", ")
        );
    }

    async fn start_handler(name: &str, handler: &ChannelHandlerPtr) -> Result<()> {
        tracing::info!("Starting {} channel...", name);
        let mut handler = handler.write().await;
        handler.start().await.map_err(|e| {
            tracing::error!("Failed to start {} channel: {}", name, e);
            e
        })
    }

    fn build_updated_handler(name: &str, new_config: &Config) -> Option<ChannelHandlerPtr> {
        match name {
            "telegram" => {
                if Self::channel_validation(new_config, "telegram")?.ready() {
                    Some(Arc::new(RwLock::new(TelegramHandler::new(
                        &new_config.channels.telegram,
                    ))))
                } else {
                    None
                }
            }
            "discord" => {
                if Self::channel_validation(new_config, "discord")?.ready() {
                    Some(Arc::new(RwLock::new(DiscordHandler::new(
                        &new_config.channels.discord,
                        new_config.clone(),
                    ))))
                } else {
                    None
                }
            }
            "feishu" => {
                if Self::channel_validation(new_config, "feishu")?.ready() {
                    Some(Arc::new(RwLock::new(FeishuHandler::new(
                        new_config.channels.feishu.clone(),
                        new_config.clone(),
                    ))))
                } else {
                    None
                }
            }
            "whatsapp" => {
                if Self::channel_validation(new_config, "whatsapp")?.ready() {
                    Some(Arc::new(RwLock::new(WhatsAppHandler::new(
                        new_config.channels.whatsapp.clone(),
                    ))))
                } else {
                    None
                }
            }
            "dingtalk" => {
                if Self::channel_validation(new_config, "dingtalk")?.ready() {
                    Some(Arc::new(RwLock::new(DingTalkHandler::new(
                        new_config.channels.dingtalk.clone(),
                        new_config.clone(),
                    ))))
                } else {
                    None
                }
            }
            "email" => {
                if Self::channel_validation(new_config, "email")?.ready() {
                    Some(Arc::new(RwLock::new(EmailHandler::new(
                        new_config.channels.email.clone(),
                        new_config.clone(),
                    ))))
                } else {
                    None
                }
            }
            "slack" => Self::channel_validation(new_config, "slack")?
                .ready()
                .then(|| {
                    Arc::new(RwLock::new(SlackHandler::new(
                        new_config.channels.slack.clone(),
                    ))) as ChannelHandlerPtr
                }),
            "qq" => {
                if Self::channel_validation(new_config, "qq")?.ready() {
                    Some(Arc::new(RwLock::new(QQHandler::new(
                        new_config.channels.qq.clone(),
                        new_config.clone(),
                    ))))
                } else {
                    None
                }
            }
            "matrix" => {
                if Self::channel_validation(new_config, "matrix")?.ready() {
                    Some(Arc::new(RwLock::new(MatrixHandler::new(
                        new_config.channels.matrix.clone(),
                        new_config.clone(),
                    ))))
                } else {
                    None
                }
            }
            "neuro-link" | "generic_pipe" => {
                if Self::channel_validation(new_config, "neuro-link")?.ready() {
                    Some(Arc::new(RwLock::new(NeuroLinkHandler::new(
                        new_config.channels.neuro_link.clone(),
                    ))))
                } else {
                    None
                }
            }
            "irc" => {
                if Self::channel_validation(new_config, "irc")?.ready() {
                    Some(Arc::new(RwLock::new(IrcHandler::new(
                        new_config.channels.irc.clone(),
                    ))))
                } else {
                    None
                }
            }
            "mattermost" => {
                if Self::channel_validation(new_config, "mattermost")?.ready() {
                    Some(Arc::new(RwLock::new(MattermostHandler::new(
                        new_config.channels.mattermost.clone(),
                    ))))
                } else {
                    None
                }
            }
            "nextcloud_talk" => {
                if Self::channel_validation(new_config, "nextcloud_talk")?.ready() {
                    Some(Arc::new(RwLock::new(NextcloudTalkHandler::new(
                        new_config.channels.nextcloud_talk.clone(),
                    ))))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

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
            if Self::channel_validation(&self.config, "telegram")
                .is_some_and(|validation| validation.ready())
            {
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
                Self::log_skipped_channel(&self.config, "telegram");
            }
        }

        // Initialize Discord channel
        if self.config.channels.discord.enabled {
            if Self::channel_validation(&self.config, "discord")
                .is_some_and(|validation| validation.ready())
            {
                let mut handler =
                    DiscordHandler::new(&self.config.channels.discord, self.config.clone());
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "discord".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("Discord channel initialized");
            } else {
                Self::log_skipped_channel(&self.config, "discord");
            }
        }

        // Initialize Feishu channel
        if self.config.channels.feishu.enabled {
            if Self::channel_validation(&self.config, "feishu")
                .is_some_and(|validation| validation.ready())
            {
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
                Self::log_skipped_channel(&self.config, "feishu");
            }
        }

        // Initialize WhatsApp channel
        if Self::channel_validation(&self.config, "whatsapp")
            .is_some_and(|validation| validation.ready())
        {
            let mut handler = WhatsAppHandler::new(self.config.channels.whatsapp.clone());
            if let Some(ref tx) = self.inbound_tx {
                handler.set_inbound_sender(tx.clone());
            }
            handlers.insert(
                "whatsapp".to_string(),
                Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
            );
            tracing::info!("WhatsApp channel initialized");
        } else if self.config.channels.whatsapp.enabled {
            Self::log_skipped_channel(&self.config, "whatsapp");
        }

        // Initialize DingTalk channel
        if self.config.channels.dingtalk.enabled {
            if Self::channel_validation(&self.config, "dingtalk")
                .is_some_and(|validation| validation.ready())
            {
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
                Self::log_skipped_channel(&self.config, "dingtalk");
            }
        }

        // Initialize Email channel
        if self.config.channels.email.enabled {
            if Self::channel_validation(&self.config, "email")
                .is_some_and(|validation| validation.ready())
            {
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
                Self::log_skipped_channel(&self.config, "email");
            }
        }

        // Initialize Slack channel
        if Self::channel_validation(&self.config, "slack")
            .is_some_and(|validation| validation.ready())
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
        } else if self.config.channels.slack.enabled {
            Self::log_skipped_channel(&self.config, "slack");
        }

        // Initialize QQ channel
        if self.config.channels.qq.enabled {
            if Self::channel_validation(&self.config, "qq")
                .is_some_and(|validation| validation.ready())
            {
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
                Self::log_skipped_channel(&self.config, "qq");
            }
        }

        // Initialize Matrix channel
        if self.config.channels.matrix.enabled {
            if Self::channel_validation(&self.config, "matrix")
                .is_some_and(|validation| validation.ready())
            {
                let mut handler =
                    MatrixHandler::new(self.config.channels.matrix.clone(), self.config.clone());
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "matrix".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("Matrix channel initialized");
            } else {
                Self::log_skipped_channel(&self.config, "matrix");
            }
        }

        // Initialize Neuro-link channel
        if Self::channel_validation(&self.config, "neuro-link")
            .is_some_and(|validation| validation.ready())
        {
            let mut handler = NeuroLinkHandler::new(self.config.channels.neuro_link.clone());
            if let Some(ref tx) = self.inbound_tx {
                handler.set_inbound_sender(tx.clone());
            }
            handlers.insert(
                "neuro-link".to_string(),
                Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
            );
            tracing::info!("Neuro-link channel initialized");
        }

        // Initialize IRC channel
        if self.config.channels.irc.enabled {
            if Self::channel_validation(&self.config, "irc")
                .is_some_and(|validation| validation.ready())
            {
                let mut handler = IrcHandler::new(self.config.channels.irc.clone());
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "irc".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("IRC channel initialized");
            } else {
                Self::log_skipped_channel(&self.config, "irc");
            }
        }

        // Initialize Mattermost channel
        if self.config.channels.mattermost.enabled {
            if Self::channel_validation(&self.config, "mattermost")
                .is_some_and(|validation| validation.ready())
            {
                let mut handler = MattermostHandler::new(self.config.channels.mattermost.clone());
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "mattermost".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("Mattermost channel initialized");
            } else {
                Self::log_skipped_channel(&self.config, "mattermost");
            }
        }

        // Initialize Nextcloud Talk channel
        if self.config.channels.nextcloud_talk.enabled {
            if Self::channel_validation(&self.config, "nextcloud_talk")
                .is_some_and(|validation| validation.ready())
            {
                let mut handler =
                    NextcloudTalkHandler::new(self.config.channels.nextcloud_talk.clone());
                if let Some(ref tx) = self.inbound_tx {
                    handler.set_inbound_sender(tx.clone());
                }
                handlers.insert(
                    "nextcloud_talk".to_string(),
                    Arc::new(RwLock::new(handler)) as Arc<RwLock<dyn ChannelHandler>>,
                );
                tracing::info!("Nextcloud Talk channel initialized");
            } else {
                Self::log_skipped_channel(&self.config, "nextcloud_talk");
            }
        }

        Ok(())
    }

    /// Start all channel handlers
    pub async fn start_all(&self) -> Result<()> {
        let handlers = self.handlers.read().await;
        let mut failed_channels = Vec::new();

        for (name, handler) in handlers.iter() {
            if let Err(e) = Self::start_handler(name, handler).await {
                failed_channels.push(format!("{}: {}", name, e));
            }
        }

        if failed_channels.is_empty() {
            Ok(())
        } else {
            tracing::warn!(
                "One or more channels failed to start (others keep running): {}",
                failed_channels.join("; ")
            );
            Ok(())
        }
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
        let handler = Self::build_updated_handler(name, &new_config);

        // 3. Register and start new handler
        if let Some(handler) = handler {
            // Set inbound sender
            if let Some(ref tx) = self.inbound_tx {
                let mut h = handler.write().await;
                h.set_inbound_sender(tx.clone());
            }

            // Start handler
            Self::start_handler(name, &handler).await?;

            handlers.insert(name.to_string(), handler);
            tracing::info!("{} channel updated and started", name);
        } else {
            Self::log_skipped_channel(&new_config, name);
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

fn required_fields<const N: usize>(entries: [(&'static str, &str); N]) -> Vec<&'static str> {
    entries
        .into_iter()
        .filter_map(|(name, value)| value.trim().is_empty().then_some(name))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::config::schema::Config;

    #[test]
    fn invalid_enabled_discord_channel_is_not_ready() {
        let mut config = Config::default();
        config.channels.discord.enabled = true;

        let validation = ChannelManager::channel_validation(&config, "discord").unwrap();
        assert!(validation.enabled);
        assert_eq!(validation.missing_fields, vec!["token"]);
        assert!(!validation.ready());
    }

    #[test]
    fn configured_channel_names_skip_invalid_channels() {
        let mut config = Config::default();
        config.channels.discord.enabled = true;
        config.channels.neuro_link.enabled = true;

        let configured = ChannelManager::configured_channel_names(&config);
        assert!(configured.iter().any(|name| name == "neuro-link"));
        assert!(!configured.iter().any(|name| name == "discord"));
    }
}
