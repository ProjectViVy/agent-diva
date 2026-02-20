//! Slack channel integration using Socket Mode.
//!
//! P0 scope:
//! - Connect via Socket Mode
//! - Handle `app_mention` events as inbound messages
//! - Send responses via Web API `chat.postMessage` (thread reply by default)

use async_trait::async_trait;
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::SlackConfig;
use serde_json::json;
use slack_morphism::prelude::*;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::base::{ChannelError, ChannelHandler, Result};

#[derive(Debug, Clone)]
struct SlackAgentDivaState {
    inbound_tx: mpsc::Sender<InboundMessage>,
    config: SlackConfig,
    bot_user_id: Option<String>,
}

fn is_sender_allowed_for_group(config: &SlackConfig, sender_id: &str) -> bool {
    let policy = config.group_policy.as_str();
    match policy {
        // Allow all senders.
        "open" => true,
        // Restrict to allowlist.
        "allowlist" => config.group_allow_from.iter().any(|s| s == sender_id),
        // Mention policy still can be combined with allowlist to reduce exposure.
        "mention" => {
            if config.group_allow_from.is_empty() {
                true
            } else {
                config.group_allow_from.iter().any(|s| s == sender_id)
            }
        }
        // Unknown policy: fail open to avoid breaking existing installs.
        _ => true,
    }
}

fn is_sender_allowed_for_dm(config: &SlackConfig, sender_id: &str) -> bool {
    if !config.dm.enabled {
        return false;
    }

    match config.dm.policy.as_str() {
        "open" => true,
        "allowlist" => config.dm.allow_from.iter().any(|s| s == sender_id),
        // Unknown policy: fail open to avoid breaking existing installs.
        _ => true,
    }
}

fn is_direct_message_channel(channel_type: Option<&str>, chat_id: &str) -> bool {
    if let Some(channel_type) = channel_type {
        return channel_type == "im";
    }
    chat_id.starts_with('D')
}

fn strip_bot_mention(text: &str, bot_user_id: Option<&str>) -> String {
    let Some(bot_user_id) = bot_user_id else {
        return text.trim().to_string();
    };
    let mention = format!("<@{}>", bot_user_id);
    text.replace(&mention, "").trim().to_string()
}

fn text_mentions_bot(text: &str, bot_user_id: Option<&str>) -> bool {
    let Some(bot_user_id) = bot_user_id else {
        return false;
    };
    text.contains(&format!("<@{}>", bot_user_id))
}

fn convert_app_mention_to_inbound(event: SlackAppMentionEvent) -> Option<InboundMessage> {
    let sender_id = event.user.to_string();
    let chat_id = event.channel.to_string();

    let text = event.content.text.unwrap_or_default();
    if text.trim().is_empty() {
        return None;
    }

    let thread_ts = event
        .origin
        .thread_ts
        .clone()
        .unwrap_or_else(|| event.origin.ts.clone());

    Some(
        InboundMessage::new("slack", sender_id, chat_id, text)
            .with_metadata("message_ts", json!(event.origin.ts.to_string()))
            .with_metadata("thread_ts", json!(thread_ts.to_string())),
    )
}

fn convert_message_event_to_inbound(
    event: SlackMessageEvent,
    bot_user_id: Option<&str>,
) -> Option<InboundMessage> {
    if event.subtype.is_some() {
        return None;
    }

    let sender_id = event.sender.user?.to_string();
    let chat_id = event.origin.channel?.to_string();
    let text = event
        .content
        .and_then(|content| content.text)
        .unwrap_or_default();
    if text.trim().is_empty() {
        return None;
    }

    let channel_type = event.origin.channel_type.as_ref().map(|t| t.to_string());
    let is_dm = is_direct_message_channel(channel_type.as_deref(), &chat_id);

    // Avoid duplicates: in channels/groups mentions are already delivered as app_mention.
    if !is_dm && text_mentions_bot(&text, bot_user_id) {
        return None;
    }

    let cleaned_text = strip_bot_mention(&text, bot_user_id);
    if cleaned_text.is_empty() {
        return None;
    }

    let thread_ts = event
        .origin
        .thread_ts
        .clone()
        .unwrap_or_else(|| event.origin.ts.clone());

    let mut inbound = InboundMessage::new("slack", sender_id, chat_id, cleaned_text)
        .with_metadata("message_ts", json!(event.origin.ts.to_string()))
        .with_metadata("thread_ts", json!(thread_ts.to_string()));

    if let Some(channel_type) = channel_type {
        inbound = inbound.with_metadata("channel_type", json!(channel_type));
    }

    Some(inbound)
}

fn should_handle_group_message(
    config: &SlackConfig,
    sender_id: &str,
    text: &str,
    bot_user_id: Option<&str>,
) -> bool {
    if !is_sender_allowed_for_group(config, sender_id) {
        return false;
    }

    match config.group_policy.as_str() {
        "open" => true,
        "allowlist" => true,
        "mention" => text_mentions_bot(text, bot_user_id),
        _ => true,
    }
}

async fn slack_push_events_callback(
    event: SlackPushEventCallback,
    _client: Arc<SlackHyperClient>,
    states: SlackClientEventsUserState,
) -> UserCallbackResult<()> {
    let state = {
        let guard = states.read().await;
        match guard.get_user_state::<SlackAgentDivaState>().cloned() {
            Some(state) => state,
            None => {
                warn!("SlackAgentDivaState missing in listener environment");
                return Ok(());
            }
        }
    };

    match event.event {
        SlackEventCallbackBody::AppMention(app_mention) => {
            let sender_id = app_mention.user.to_string();
            if !should_handle_group_message(
                &state.config,
                &sender_id,
                app_mention.content.text.as_deref().unwrap_or_default(),
                state.bot_user_id.as_deref(),
            ) {
                warn!("Slack sender not allowed by group policy: {}", sender_id);
                return Ok(());
            }

            if let Some(inbound) = convert_app_mention_to_inbound(app_mention) {
                if let Err(e) = state.inbound_tx.send(inbound).await {
                    warn!("Failed to forward Slack inbound message: {}", e);
                }
            }
        }
        SlackEventCallbackBody::Message(message_event) => {
            let sender_id = message_event
                .sender
                .user
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            if sender_id.is_empty() {
                return Ok(());
            }

            let chat_id = message_event
                .origin
                .channel
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            if chat_id.is_empty() {
                return Ok(());
            }

            let channel_type = message_event
                .origin
                .channel_type
                .as_ref()
                .map(ToString::to_string);
            let is_dm = is_direct_message_channel(channel_type.as_deref(), &chat_id);

            if is_dm {
                if !is_sender_allowed_for_dm(&state.config, &sender_id) {
                    return Ok(());
                }
            } else {
                let text = message_event
                    .content
                    .as_ref()
                    .and_then(|c| c.text.as_deref())
                    .unwrap_or_default();
                if !should_handle_group_message(
                    &state.config,
                    &sender_id,
                    text,
                    state.bot_user_id.as_deref(),
                ) {
                    return Ok(());
                }
            }

            if let Some(inbound) =
                convert_message_event_to_inbound(message_event, state.bot_user_id.as_deref())
            {
                if let Err(e) = state.inbound_tx.send(inbound).await {
                    warn!("Failed to forward Slack inbound message: {}", e);
                }
            }
        }
        _ => {
            // Ignore other event types.
        }
    }

    Ok(())
}

/// Slack channel handler using Socket Mode.
pub struct SlackHandler {
    config: SlackConfig,
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    running: bool,
    listener_task: Option<JoinHandle<()>>,
    client: Option<Arc<SlackHyperClient>>,
    bot_token: Option<SlackApiToken>,
    bot_user_id: Option<String>,
}

impl SlackHandler {
    /// Create a new Slack handler.
    pub fn new(config: SlackConfig) -> Self {
        Self {
            config,
            inbound_tx: None,
            running: false,
            listener_task: None,
            client: None,
            bot_token: None,
            bot_user_id: None,
        }
    }

    

    fn validate_config(&self) -> Result<()> {
        if !self.config.enabled {
            return Err(ChannelError::NotConfigured(
                "Slack channel is not enabled".to_string(),
            ));
        }

        if self.config.mode.as_str() != "socket" {
            return Err(ChannelError::NotConfigured(
                "Only 'socket' mode is supported for Slack".to_string(),
            ));
        }

        if self.config.bot_token.trim().is_empty() {
            return Err(ChannelError::InvalidConfig(
                "Slack bot_token not configured".to_string(),
            ));
        }

        if self.config.app_token.trim().is_empty() {
            return Err(ChannelError::InvalidConfig(
                "Slack app_token not configured".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl ChannelHandler for SlackHandler {
    fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn name(&self) -> &str {
        "slack"
    }

    fn is_running(&self) -> bool {
        self.running
    }

    async fn start(&mut self) -> Result<()> {
        self.validate_config()?;

        if self.running {
            return Ok(());
        }

        let inbound_tx = self.inbound_tx.clone().ok_or_else(|| {
            ChannelError::NotConfigured("Inbound message sender not set".to_string())
        })?;

        let connector = SlackClientHyperConnector::new().map_err(|e| {
            ChannelError::ConnectionFailed(format!("Slack hyper connector init failed: {}", e))
        })?;
        let client = Arc::new(SlackClient::new(connector));

        let bot_token_value: SlackApiTokenValue = self.config.bot_token.clone().into();
        let bot_token = SlackApiToken::new(bot_token_value);

        // Best-effort auth check early to surface bad tokens quickly.
        let session = client.open_session(&bot_token);
        let auth = session
            .auth_test()
            .await
            .map_err(|e| ChannelError::AuthError(format!("Slack auth.test failed: {}", e)))?;
        let bot_user_id = auth.user_id.to_string();

        if bot_user_id.trim().is_empty() {
            return Err(ChannelError::AuthError(
                "Slack auth.test returned an empty user_id".to_string(),
            ));
        }

        let state = SlackAgentDivaState {
            inbound_tx,
            config: self.config.clone(),
            bot_user_id: Some(bot_user_id.clone()),
        };

        let callbacks =
            SlackSocketModeListenerCallbacks::new().with_push_events(slack_push_events_callback);

        let listener_environment = Arc::new(
            SlackClientEventsListenerEnvironment::new(client.clone()).with_user_state(state),
        );

        let socket_mode_listener = SlackClientSocketModeListener::new(
            &SlackClientSocketModeConfig::new(),
            listener_environment,
            callbacks,
        );

        let app_token_value: SlackApiTokenValue = self.config.app_token.clone().into();
        let app_token = SlackApiToken::new(app_token_value);

        self.running = true;
        self.client = Some(client);
        self.bot_token = Some(bot_token);
        self.bot_user_id = Some(bot_user_id);

        self.listener_task = Some(tokio::spawn(async move {
            if let Err(e) = socket_mode_listener.listen_for(&app_token).await {
                error!("Slack Socket Mode listen_for failed: {}", e);
                return;
            }
            socket_mode_listener.serve().await;
        }));

        info!("Slack channel started (Socket Mode, app_mention + dm message)");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.running = false;

        if let Some(handle) = self.listener_task.take() {
            handle.abort();
            let _ = handle.await;
        }
        self.bot_user_id = None;

        info!("Slack channel stopped");
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        if !self.running {
            return Err(ChannelError::NotRunning(
                "Slack handler not running".to_string(),
            ));
        }

        let client = self.client.clone().ok_or_else(|| {
            ChannelError::NotConfigured("Slack client not initialized".to_string())
        })?;
        let bot_token = self.bot_token.clone().ok_or_else(|| {
            ChannelError::NotConfigured("Slack bot token not initialized".to_string())
        })?;

        let session = client.open_session(&bot_token);

        let mut req = SlackApiChatPostMessageRequest::new(
            msg.chat_id.clone().into(),
            SlackMessageContent::new().with_text(msg.content.clone()),
        );

        let thread_ts_from_metadata = msg
            .metadata
            .get("thread_ts")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(ToString::to_string);
        let thread_ts_from_nested_slack = msg
            .metadata
            .get("slack")
            .and_then(|v| v.get("thread_ts"))
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .map(ToString::to_string);
        let thread_ts_from_reply_to = msg.reply_to.as_ref().and_then(|s| {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        // Thread replies by default. If explicit thread_ts is missing, fall back to reply_to.
        if let Some(thread_ts) = thread_ts_from_metadata
            .or(thread_ts_from_nested_slack)
            .or(thread_ts_from_reply_to)
        {
            req.thread_ts = Some(thread_ts.to_string().into());
        }

        session.chat_post_message(&req).await.map_err(|e| {
            ChannelError::SendFailed(format!("Slack chat.postMessage failed: {}", e))
        })?;

        Ok(())
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        is_sender_allowed_for_group(&self.config, sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sender_allowed_for_group_open() {
        let mut cfg = SlackConfig::default();
        cfg.group_policy = "open".to_string();
        cfg.group_allow_from = vec!["U123".to_string()];

        assert!(is_sender_allowed_for_group(&cfg, "U999"));
        assert!(is_sender_allowed_for_group(&cfg, "U123"));
    }

    #[test]
    fn test_is_sender_allowed_for_group_allowlist() {
        let mut cfg = SlackConfig::default();
        cfg.group_policy = "allowlist".to_string();
        cfg.group_allow_from = vec!["U123".to_string()];

        assert!(is_sender_allowed_for_group(&cfg, "U123"));
        assert!(!is_sender_allowed_for_group(&cfg, "U999"));
    }

    #[test]
    fn test_is_sender_allowed_for_dm() {
        let mut cfg = SlackConfig::default();
        cfg.dm.enabled = true;
        cfg.dm.policy = "allowlist".to_string();
        cfg.dm.allow_from = vec!["U123".to_string()];

        assert!(is_sender_allowed_for_dm(&cfg, "U123"));
        assert!(!is_sender_allowed_for_dm(&cfg, "U999"));
    }

    #[test]
    fn test_strip_bot_mention() {
        let text = "<@U123> hello there";
        assert_eq!(strip_bot_mention(text, Some("U123")), "hello there");
        assert_eq!(strip_bot_mention(text, Some("U999")), "<@U123> hello there");
    }

    #[test]
    fn test_convert_app_mention_to_inbound_sets_thread_ts() {
        let ev = SlackAppMentionEvent {
            user: SlackUserId("U123".to_string()),
            channel: SlackChannelId("C123".to_string()),
            content: SlackMessageContent {
                text: Some("hi".to_string()),
                blocks: None,
                attachments: None,
                upload: None,
                files: None,
                reactions: None,
                metadata: None,
            },
            origin: SlackMessageOrigin {
                ts: SlackTs("1700000000.000100".to_string()),
                channel: None,
                channel_type: None,
                thread_ts: Some(SlackTs("1700000000.000000".to_string())),
                client_msg_id: None,
            },
            edited: None,
        };

        let inbound = convert_app_mention_to_inbound(ev).expect("inbound");
        assert_eq!(inbound.channel, "slack");
        assert_eq!(inbound.sender_id, "U123");
        assert_eq!(inbound.chat_id, "C123");
        assert_eq!(inbound.content, "hi");
        assert_eq!(
            inbound.metadata.get("thread_ts").and_then(|v| v.as_str()),
            Some("1700000000.000000")
        );
    }

    #[test]
    fn test_convert_message_event_to_inbound_for_dm() {
        let ev = SlackMessageEvent {
            origin: SlackMessageOrigin {
                ts: SlackTs("1700000000.000200".to_string()),
                channel: Some(SlackChannelId("D123".to_string())),
                channel_type: Some(SlackChannelType("im".to_string())),
                thread_ts: None,
                client_msg_id: None,
            },
            content: Some(SlackMessageContent {
                text: Some("hello".to_string()),
                blocks: None,
                attachments: None,
                upload: None,
                files: None,
                reactions: None,
                metadata: None,
            }),
            sender: SlackMessageSender {
                user: Some(SlackUserId("U123".to_string())),
                bot_id: None,
                username: None,
                display_as_bot: None,
                user_profile: None,
                bot_profile: None,
            },
            subtype: None,
            hidden: None,
            message: None,
            previous_message: None,
            deleted_ts: None,
        };

        let inbound = convert_message_event_to_inbound(ev, Some("UBOT")).expect("inbound");
        assert_eq!(inbound.channel, "slack");
        assert_eq!(inbound.sender_id, "U123");
        assert_eq!(inbound.chat_id, "D123");
        assert_eq!(inbound.content, "hello");
        assert_eq!(
            inbound
                .metadata
                .get("channel_type")
                .and_then(|v| v.as_str()),
            Some("im")
        );
        assert_eq!(
            inbound.metadata.get("thread_ts").and_then(|v| v.as_str()),
            Some("1700000000.000200")
        );
    }

    #[test]
    fn test_convert_message_event_to_inbound_skips_non_dm_mention_duplicates() {
        let ev = SlackMessageEvent {
            origin: SlackMessageOrigin {
                ts: SlackTs("1700000000.000200".to_string()),
                channel: Some(SlackChannelId("C123".to_string())),
                channel_type: Some(SlackChannelType("channel".to_string())),
                thread_ts: None,
                client_msg_id: None,
            },
            content: Some(SlackMessageContent {
                text: Some("<@UBOT> hello".to_string()),
                blocks: None,
                attachments: None,
                upload: None,
                files: None,
                reactions: None,
                metadata: None,
            }),
            sender: SlackMessageSender {
                user: Some(SlackUserId("U123".to_string())),
                bot_id: None,
                username: None,
                display_as_bot: None,
                user_profile: None,
                bot_profile: None,
            },
            subtype: None,
            hidden: None,
            message: None,
            previous_message: None,
            deleted_ts: None,
        };

        let inbound = convert_message_event_to_inbound(ev, Some("UBOT"));
        assert!(inbound.is_none());
    }
}
