//! DingTalk channel integration using Stream Mode
//!
//! This implementation uses DingTalk Stream WebSocket API for real-time
//! message reception and HTTP API for sending messages.
//!
//! Key features:
//! - Stream Mode WebSocket connection (no public IP required)
//! - Client ID + Client Secret authentication
//! - Message deduplication
//! - Allowlist-based access control
//! - Private chat support (group messages received but replied as private)
//!
//! References:
//! - Python implementation: agent-diva/channels/dingtalk.py
//! - DingTalk Stream Protocol: https://open-dingtalk.github.io/developerpedia/docs/learn/stream/protocol/

use crate::base::{BaseChannel, ChannelError, ChannelHandler, Result};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use agent_diva_core::bus::OutboundMessage;
use agent_diva_core::config::schema::DingTalkConfig;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{debug, error, info, warn};

// DingTalk API endpoints
const DINGTALK_API_BASE: &str = "https://api.dingtalk.com";
const DINGTALK_STREAM_REGISTER_URL: &str = "https://api.dingtalk.com/v1.0/gateway/connections/open";

/// Access token response
#[derive(Debug, Deserialize)]
struct AccessTokenResponse {
    #[serde(rename = "accessToken")]
    access_token: String,
    #[serde(rename = "expireIn")]
    expire_in: i64,
}

/// Stream connection registration request
#[derive(Debug, Serialize)]
struct StreamRegisterRequest {
    #[serde(rename = "clientId")]
    client_id: String,
    #[serde(rename = "clientSecret")]
    client_secret: String,
    #[serde(rename = "localIp")]
    local_ip: String,
    subscriptions: Vec<Subscription>,
    #[serde(rename = "ua")]
    user_agent: String,
}

/// Subscription configuration
#[derive(Debug, Serialize)]
struct Subscription {
    #[serde(rename = "type")]
    sub_type: String,
    topic: String,
}

/// Stream connection registration response
#[derive(Debug, Deserialize)]
struct StreamRegisterResponse {
    endpoint: String,
    ticket: String,
}

/// Stream message envelope
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamMessage {
    #[serde(rename = "specVersion")]
    spec_version: String,
    #[serde(rename = "type")]
    msg_type: String,
    headers: StreamHeaders,
    data: String,
}

/// Stream message headers
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct StreamHeaders {
    #[serde(rename = "messageId")]
    message_id: String,
    topic: String,
    #[serde(rename = "contentType")]
    content_type: String,
    time: String,
    #[serde(rename = "appId", default)]
    app_id: Option<String>,
}

/// Stream response
#[derive(Debug, Serialize)]
struct StreamResponse {
    code: i32,
    message: String,
    headers: ResponseHeaders,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
}

/// Response headers
#[derive(Debug, Serialize)]
struct ResponseHeaders {
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(rename = "contentType")]
    content_type: String,
}

/// Bot message callback data
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct BotMessageData {
    #[serde(rename = "conversationId")]
    conversation_id: String,
    #[serde(rename = "senderStaffId", default)]
    sender_staff_id: Option<String>,
    #[serde(rename = "senderNick", default)]
    sender_nick: Option<String>,
    #[serde(rename = "senderUserId", default)]
    sender_user_id: Option<String>,
    #[serde(rename = "chatbotUserId")]
    chatbot_user_id: String,
    #[serde(rename = "msgId")]
    msg_id: String,
    text: Option<TextContent>,
    #[serde(rename = "content", default)]
    raw_content: Option<String>,
    #[serde(rename = "conversationType", default)]
    conversation_type: Option<String>,
    #[serde(rename = "createAt", default)]
    create_at: Option<i64>,
}

/// Text content in message
#[derive(Debug, Deserialize)]
struct TextContent {
    content: String,
}

/// DingTalk channel handler
pub struct DingTalkHandler {
    config: DingTalkConfig,
    base: BaseChannel,
    running: Arc<RwLock<bool>>,
    processed_ids: Arc<RwLock<VecDeque<String>>>,
    http_client: reqwest::Client,
    token: Arc<RwLock<Option<String>>>,
    token_expiry: Arc<RwLock<Option<std::time::Instant>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    inbound_tx: Option<mpsc::Sender<agent_diva_core::bus::InboundMessage>>,
}

impl DingTalkHandler {
    /// Create a new DingTalk handler
    pub fn new(config: DingTalkConfig, base_config: agent_diva_core::config::schema::Config) -> Self {
        let allow_from = config.allow_from.clone();
        let base = BaseChannel::new("dingtalk", base_config, allow_from);

        Self {
            config,
            base,
            running: Arc::new(RwLock::new(false)),
            processed_ids: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            http_client: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            token: Arc::new(RwLock::new(None)),
            token_expiry: Arc::new(RwLock::new(None)),
            shutdown_tx: None,
            inbound_tx: None,
        }
    }

    /// Set the inbound message sender
    pub fn set_inbound_sender(&mut self, tx: mpsc::Sender<agent_diva_core::bus::InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    /// Check if message ID has been processed (deduplication)
    async fn is_processed(&self, message_id: &str) -> bool {
        let ids = self.processed_ids.read().await;
        ids.contains(&message_id.to_string())
    }

    /// Mark message ID as processed
    async fn mark_processed(&self, message_id: String) {
        let mut ids = self.processed_ids.write().await;
        if ids.len() >= 1000 {
            ids.pop_front();
        }
        ids.push_back(message_id);
    }

    /// Validate configuration
    fn validate_config(&self) -> Result<()> {
        if self.config.client_id.is_empty() {
            return Err(ChannelError::InvalidConfig(
                "DingTalk client_id not configured".to_string(),
            ));
        }
        if self.config.client_secret.is_empty() {
            return Err(ChannelError::InvalidConfig(
                "DingTalk client_secret not configured".to_string(),
            ));
        }
        Ok(())
    }

    /// Get access token for sending messages
    async fn get_access_token(&self) -> Result<String> {
        // Check if we have a valid cached token
        {
            let token = self.token.read().await;
            let expiry = self.token_expiry.read().await;
            if let (Some(t), Some(e)) = (token.as_ref(), expiry.as_ref()) {
                if std::time::Instant::now() < *e {
                    return Ok(t.clone());
                }
            }
        }

        // Request new token
        let url = format!("{}/v1.0/oauth2/accessToken", DINGTALK_API_BASE);
        let body = json!({
            "appKey": self.config.client_id,
            "appSecret": self.config.client_secret
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::AuthError(format!("Token request failed: {}", e)))?;

        let token_resp: AccessTokenResponse = response
            .json()
            .await
            .map_err(|e| ChannelError::AuthError(format!("Token parse failed: {}", e)))?;

        let token = token_resp.access_token;

        // Cache token (expire 60 seconds early for safety)
        let expiry =
            std::time::Instant::now() + Duration::from_secs(token_resp.expire_in as u64 - 60);

        {
            let mut t = self.token.write().await;
            *t = Some(token.clone());
            let mut e = self.token_expiry.write().await;
            *e = Some(expiry);
        }

        info!("DingTalk access token obtained successfully");
        Ok(token)
    }

    /// Register Stream connection and get WebSocket endpoint
    async fn register_stream_connection(&self) -> Result<(String, String)> {
        let local_ip = Self::get_local_ip();

        let request = StreamRegisterRequest {
            client_id: self.config.client_id.clone(),
            client_secret: self.config.client_secret.clone(),
            local_ip,
            subscriptions: vec![
                Subscription {
                    sub_type: "EVENT".to_string(),
                    topic: "*".to_string(),
                },
                Subscription {
                    sub_type: "CALLBACK".to_string(),
                    topic: "/v1.0/im/bot/messages/get".to_string(),
                },
            ],
            user_agent: "agent-diva/0.2.0".to_string(),
        };

        let response = self
            .http_client
            .post(DINGTALK_STREAM_REGISTER_URL)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                ChannelError::ConnectionFailed(format!("Stream registration failed: {}", e))
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ChannelError::ConnectionFailed(format!(
                "Stream registration failed with status {}: {}",
                status, error_text
            )));
        }

        let register_resp: StreamRegisterResponse = response.json().await.map_err(|e| {
            ChannelError::ConnectionFailed(format!("Failed to parse registration response: {}", e))
        })?;

        info!("DingTalk Stream connection registered successfully");
        Ok((register_resp.endpoint, register_resp.ticket))
    }

    /// Get local IP address
    fn get_local_ip() -> String {
        // Try to get a non-loopback IP address
        if let Ok(addrs) = std::net::TcpStream::connect("223.5.5.5:80") {
            if let Ok(local_addr) = addrs.local_addr() {
                return local_addr.ip().to_string();
            }
        }
        "127.0.0.1".to_string()
    }

    /// Run WebSocket connection
    async fn run_websocket(
        &self,
        endpoint: String,
        ticket: String,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        // Build WebSocket URL with ticket
        let ws_url = format!("{}?ticket={}", endpoint, ticket);

        loop {
            // Check if we should shutdown
            if shutdown_rx.try_recv().is_ok() {
                info!("DingTalk WebSocket shutting down");
                break;
            }

            // Connect to WebSocket
            let (ws_stream, _) = match connect_async(&ws_url).await {
                Ok(result) => result,
                Err(e) => {
                    error!("DingTalk WebSocket connection failed: {}", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("DingTalk WebSocket connected");
            let (mut write, mut read) = ws_stream.split();

            // Message receiving loop
            let mut should_reconnect = true;
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        if let Err(e) = self.handle_websocket_message(&text, &mut write).await {
                            warn!("Error handling WebSocket message: {}", e);
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        info!("DingTalk WebSocket closed by server");
                        break;
                    }
                    Ok(WsMessage::Ping(data)) => {
                        // Respond with pong
                        if let Err(e) = write.send(WsMessage::Pong(data)).await {
                            warn!("Failed to send pong: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("DingTalk WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }

                // Check shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    info!("DingTalk WebSocket received shutdown signal");
                    should_reconnect = false;
                    break;
                }
            }

            if !should_reconnect {
                break;
            }

            info!("DingTalk WebSocket reconnecting in 5 seconds...");
            sleep(Duration::from_secs(5)).await;
        }

        Ok(())
    }

    /// Handle WebSocket message
    async fn handle_websocket_message(
        &self,
        text: &str,
        write: &mut futures::stream::SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            WsMessage,
        >,
    ) -> Result<()> {
        let msg: StreamMessage = serde_json::from_str(text)
            .map_err(|e| ChannelError::Error(format!("Failed to parse Stream message: {}", e)))?;

        debug!(
            "Received DingTalk Stream message: type={}, topic={}",
            msg.msg_type, msg.headers.topic
        );

        // Handle based on message type
        match msg.msg_type.as_str() {
            "CALLBACK" => {
                if msg.headers.topic == "/v1.0/im/bot/messages/get" {
                    self.handle_bot_message(&msg).await?;
                }
            }
            "SYSTEM" => {
                // Handle system messages (ping, etc.)
                debug!("Received system message");
            }
            "EVENT" => {
                // Handle events
                debug!("Received event message");
            }
            _ => {
                debug!("Unknown message type: {}", msg.msg_type);
            }
        }

        // Send acknowledgment response
        let response = StreamResponse {
            code: 200,
            message: "OK".to_string(),
            headers: ResponseHeaders {
                message_id: msg.headers.message_id.clone(),
                content_type: "application/json".to_string(),
            },
            data: Some("{}".to_string()),
        };

        let response_text = serde_json::to_string(&response)
            .map_err(|e| ChannelError::Error(format!("Failed to serialize response: {}", e)))?;

        if let Err(e) = write.send(WsMessage::Text(response_text)).await {
            warn!("Failed to send acknowledgment: {}", e);
        }

        Ok(())
    }

    /// Handle bot message callback
    async fn handle_bot_message(&self, msg: &StreamMessage) -> Result<()> {
        // Parse message data
        let bot_msg: BotMessageData = serde_json::from_str(&msg.data)
            .map_err(|e| ChannelError::Error(format!("Failed to parse bot message: {}", e)))?;

        // Deduplication check
        if self.is_processed(&bot_msg.msg_id).await {
            return Ok(());
        }
        self.mark_processed(bot_msg.msg_id.clone()).await;

        // Extract content
        let content = if let Some(text) = &bot_msg.text {
            text.content.trim().to_string()
        } else if let Some(raw) = &bot_msg.raw_content {
            // Try to parse as JSON for rich content
            if let Ok(json_content) = serde_json::from_str::<Value>(raw) {
                json_content
                    .get("text")
                    .and_then(|t| t.get("content"))
                    .and_then(|c| c.as_str())
                    .unwrap_or(raw)
                    .trim()
                    .to_string()
            } else {
                raw.trim().to_string()
            }
        } else {
            String::new()
        };

        if content.is_empty() {
            debug!("Empty message content, skipping");
            return Ok(());
        }

        // Get sender info
        let sender_id = bot_msg
            .sender_staff_id
            .clone()
            .or_else(|| bot_msg.sender_user_id.clone())
            .unwrap_or_else(|| "unknown".to_string());

        let sender_name = bot_msg
            .sender_nick
            .clone()
            .unwrap_or_else(|| "Unknown".to_string());

        info!(
            "Received DingTalk message from {} ({}): {}",
            sender_name, sender_id, content
        );

        // Forward to message bus
        let inbound_msg = agent_diva_core::bus::InboundMessage::new(
            "dingtalk",
            sender_id.clone(),
            sender_id.clone(), // For private chat, chat_id == sender_id
            content,
        )
        .with_metadata("sender_name", json!(sender_name))
        .with_metadata("conversation_id", json!(bot_msg.conversation_id))
        .with_metadata(
            "conversation_type",
            json!(bot_msg.conversation_type.unwrap_or_default()),
        )
        .with_metadata("message_id", json!(bot_msg.msg_id));

        if let Some(tx) = &self.inbound_tx {
            if let Err(e) = tx.send(inbound_msg).await {
                error!("Failed to send message to bus: {}", e);
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ChannelHandler for DingTalkHandler {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_running(&self) -> bool {
        *self.running.blocking_read()
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(ChannelError::NotConfigured(
                "DingTalk channel not enabled".to_string(),
            ));
        }

        self.validate_config()?;

        // Register Stream connection
        let (endpoint, ticket) = self.register_stream_connection().await?;
        info!("DingTalk Stream endpoint obtained: {}", endpoint);

        *self.running.write().await = true;

        // Start WebSocket connection
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let handler = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = handler.run_websocket(endpoint, ticket, shutdown_rx).await {
                error!("DingTalk WebSocket task failed: {}", e);
            }
        });

        info!("DingTalk channel started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        info!("DingTalk channel stopped");
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        if !*self.running.read().await {
            return Err(ChannelError::NotRunning(
                "DingTalk channel not running".to_string(),
            ));
        }

        let token = self.get_access_token().await?;

        // Use robot oToMessages/batchSend API for private chat
        // https://open.dingtalk.com/document/orgapp/robot-batch-send-messages
        let url = format!("{}/v1.0/robot/oToMessages/batchSend", DINGTALK_API_BASE);

        let body = json!({
            "robotCode": self.config.client_id,
            "userIds": [msg.chat_id], // chat_id is the user's staffId
            "msgKey": "sampleMarkdown",
            "msgParam": json!({
                "text": msg.content,
                "title": "agent-diva reply",
            }).to_string(),
        });

        let response = self
            .http_client
            .post(&url)
            .header("x-acs-dingtalk-access-token", token)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::SendFailed(format!("Send request failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ChannelError::SendFailed(format!(
                "Send failed with status {}: {}",
                status, error_text
            )));
        }

        debug!("DingTalk message sent to {}", msg.chat_id);
        Ok(())
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.base.is_allowed(sender_id)
    }
}

impl DingTalkHandler {
    /// Clone necessary fields for async task
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            base: BaseChannel::new(
                self.base.name.clone(),
                self.base.config.clone(),
                self.base.allow_from.clone(),
            ),
            running: Arc::clone(&self.running),
            processed_ids: Arc::clone(&self.processed_ids),
            http_client: self.http_client.clone(),
            token: Arc::clone(&self.token),
            token_expiry: Arc::clone(&self.token_expiry),
            shutdown_tx: None,
            inbound_tx: self.inbound_tx.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::config::schema::Config;

    #[test]
    fn test_dingtalk_handler_new() {
        let mut dingtalk_config = DingTalkConfig::default();
        dingtalk_config.enabled = true;
        dingtalk_config.client_id = "test_client_id".to_string();
        dingtalk_config.client_secret = "test_secret".to_string();

        let config = Config::default();
        let handler = DingTalkHandler::new(dingtalk_config, config);

        assert_eq!(handler.name(), "dingtalk");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_validate_config_missing_client_id() {
        let mut dingtalk_config = DingTalkConfig::default();
        dingtalk_config.enabled = true;
        dingtalk_config.client_secret = "test_secret".to_string();

        let config = Config::default();
        let handler = DingTalkHandler::new(dingtalk_config, config);

        let result = handler.validate_config();
        assert!(result.is_err());

        if let Err(ChannelError::InvalidConfig(msg)) = result {
            assert!(msg.contains("client_id"));
        } else {
            panic!("Expected InvalidConfig error");
        }
    }

    #[test]
    fn test_validate_config_missing_client_secret() {
        let mut dingtalk_config = DingTalkConfig::default();
        dingtalk_config.enabled = true;
        dingtalk_config.client_id = "test_client_id".to_string();

        let config = Config::default();
        let handler = DingTalkHandler::new(dingtalk_config, config);

        let result = handler.validate_config();
        assert!(result.is_err());

        if let Err(ChannelError::InvalidConfig(msg)) = result {
            assert!(msg.contains("client_secret"));
        } else {
            panic!("Expected InvalidConfig error");
        }
    }

    #[tokio::test]
    async fn test_is_processed() {
        let mut dingtalk_config = DingTalkConfig::default();
        dingtalk_config.enabled = true;
        dingtalk_config.client_id = "test_client_id".to_string();
        dingtalk_config.client_secret = "test_secret".to_string();

        let config = Config::default();
        let handler = DingTalkHandler::new(dingtalk_config, config);

        assert!(!handler.is_processed("msg_123").await);

        handler.mark_processed("msg_123".to_string()).await;

        assert!(handler.is_processed("msg_123").await);
    }

    #[test]
    fn test_is_allowed() {
        let mut dingtalk_config = DingTalkConfig::default();
        dingtalk_config.allow_from = vec!["user123".to_string()];

        let config = Config::default();
        let handler = DingTalkHandler::new(dingtalk_config, config);

        assert!(handler.is_allowed("user123"));
        assert!(!handler.is_allowed("user456"));
    }

    #[test]
    fn test_get_local_ip() {
        let ip = DingTalkHandler::get_local_ip();
        // Should return a valid IP address
        assert!(!ip.is_empty());
        assert!(ip.contains('.') || ip.contains(':'));
    }
}
