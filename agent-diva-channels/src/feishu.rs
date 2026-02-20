//! Feishu/Lark channel integration using WebSocket long connection
//!
//! This implementation uses Feishu Open Platform WebSocket API for real-time
//! message reception and HTTP API for sending messages.
//!
//! Key features:
//! - WebSocket long connection for event receiving (no public IP required)
//! - App ID + App Secret authentication
//! - Message deduplication
//! - Allowlist-based access control
//! - Interactive card message support with markdown and tables
//!
//! References:
//! - Python implementation: agent-diva/channels/feishu.py
//! - Feishu API: https://open.feishu.cn/document/home/index

use crate::base::{BaseChannel, ChannelError, ChannelHandler, Result};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use agent_diva_core::bus::OutboundMessage;
use agent_diva_core::config::schema::FeishuConfig;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{debug, error, info, warn};

// Feishu OpenAPI endpoints
const FEISHU_API_BASE: &str = "https://open.feishu.cn/open-apis";

// Message type display mapping
const MSG_TYPE_MAP: &[(&str, &str)] = &[
    ("image", "[image]"),
    ("audio", "[audio]"),
    ("file", "[file]"),
    ("sticker", "[sticker]"),
];

/// Tenant access token response
#[derive(Debug, Deserialize)]
struct TokenResponse {
    code: i32,
    msg: String,
    #[serde(default)]
    tenant_access_token: Option<String>,
    #[serde(default)]
    expire: Option<i64>,
}

/// WebSocket connection info
#[derive(Debug, Deserialize)]
struct WebSocketInfo {
    url: String,
}

/// WebSocket connection response
#[derive(Debug, Deserialize)]
struct WebSocketResponse {
    code: i32,
    msg: String,
    data: Option<WebSocketInfo>,
}

/// Feishu event payload (im.message.receive_v1)
#[derive(Debug, Deserialize)]
struct FeishuEvent {
    #[serde(rename = "event_type")]
    event_type: String,
    event: Option<MessageEvent>,
}

/// Message event details
#[derive(Debug, Deserialize)]
struct MessageEvent {
    sender: SenderInfo,
    message: MessageInfo,
}

/// Sender information
#[derive(Debug, Deserialize)]
struct SenderInfo {
    #[serde(rename = "sender_id")]
    sender_id: Option<OpenIdInfo>,
    #[serde(rename = "sender_type")]
    sender_type: String,
}

/// Open ID information
#[derive(Debug, Deserialize)]
struct OpenIdInfo {
    #[serde(rename = "open_id")]
    open_id: String,
}

/// Message information
#[derive(Debug, Deserialize)]
struct MessageInfo {
    #[serde(rename = "message_id")]
    message_id: String,
    #[serde(rename = "chat_id")]
    chat_id: String,
    #[serde(rename = "chat_type")]
    chat_type: String,
    #[serde(rename = "message_type")]
    message_type: String,
    content: String,
}

/// Feishu channel handler
pub struct FeishuHandler {
    config: FeishuConfig,
    base: BaseChannel,
    running: Arc<RwLock<bool>>,
    processed_ids: Arc<RwLock<VecDeque<String>>>,
    http_client: reqwest::Client,
    token: Arc<RwLock<Option<String>>>,
    token_expiry: Arc<RwLock<Option<std::time::Instant>>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    inbound_tx: Option<mpsc::Sender<agent_diva_core::bus::InboundMessage>>,
}

impl FeishuHandler {
    /// Create a new Feishu handler
    pub fn new(config: FeishuConfig, base_config: agent_diva_core::config::schema::Config) -> Self {
        let allow_from = config.allow_from.clone();
        let base = BaseChannel::new("feishu", base_config, allow_from);

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
        if self.config.app_id.is_empty() {
            return Err(ChannelError::InvalidConfig(
                "Feishu app_id not configured".to_string(),
            ));
        }
        if self.config.app_secret.is_empty() {
            return Err(ChannelError::InvalidConfig(
                "Feishu app_secret not configured".to_string(),
            ));
        }
        Ok(())
    }

    /// Get tenant access token
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
        let url = format!("{}/auth/v3/tenant_access_token/internal", FEISHU_API_BASE);
        let body = json!({
            "app_id": self.config.app_id,
            "app_secret": self.config.app_secret
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::AuthError(format!("Token request failed: {}", e)))?;

        let token_resp: TokenResponse = response
            .json()
            .await
            .map_err(|e| ChannelError::AuthError(format!("Token parse failed: {}", e)))?;

        if token_resp.code != 0 {
            return Err(ChannelError::AuthError(format!(
                "Token request failed: {} - {}",
                token_resp.code, token_resp.msg
            )));
        }

        let token = token_resp
            .tenant_access_token
            .ok_or_else(|| ChannelError::AuthError("No token in response".to_string()))?;

        // Cache token (expire 5 minutes early for safety)
        let expiry = std::time::Instant::now()
            + Duration::from_secs(token_resp.expire.unwrap_or(7200) as u64 - 300);

        {
            let mut t = self.token.write().await;
            *t = Some(token.clone());
            let mut e = self.token_expiry.write().await;
            *e = Some(expiry);
        }

        info!("Feishu access token obtained successfully");
        Ok(token)
    }

    /// Get WebSocket connection URL
    async fn get_websocket_url(&self) -> Result<String> {
        let token = self.get_access_token().await?;
        let url = format!("{}/bot/v2/websocket", FEISHU_API_BASE);

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| {
                ChannelError::ConnectionFailed(format!("WebSocket URL request failed: {}", e))
            })?;

        let ws_resp: WebSocketResponse = response.json().await.map_err(|e| {
            ChannelError::ConnectionFailed(format!("WebSocket URL parse failed: {}", e))
        })?;

        if ws_resp.code != 0 {
            return Err(ChannelError::ConnectionFailed(format!(
                "WebSocket URL request failed: {} - {}",
                ws_resp.code, ws_resp.msg
            )));
        }

        let ws_info = ws_resp.data.ok_or_else(|| {
            ChannelError::ConnectionFailed("No WebSocket info in response".to_string())
        })?;

        Ok(ws_info.url)
    }

    /// Run WebSocket connection
    async fn run_websocket(
        &self,
        ws_url: String,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        loop {
            // Check if we should shutdown
            if shutdown_rx.try_recv().is_ok() {
                info!("Feishu WebSocket shutting down");
                break;
            }

            // Connect to WebSocket
            let (ws_stream, _) = match connect_async(&ws_url).await {
                Ok(result) => result,
                Err(e) => {
                    error!("Feishu WebSocket connection failed: {}", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("Feishu WebSocket connected");
            let (mut write, mut read) = ws_stream.split();

            // Message receiving loop
            let mut should_reconnect = true;
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        if let Err(e) = self.handle_websocket_message(&text).await {
                            warn!("Error handling WebSocket message: {}", e);
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        info!("Feishu WebSocket closed by server");
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
                        error!("Feishu WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }

                // Check shutdown signal
                if shutdown_rx.try_recv().is_ok() {
                    info!("Feishu WebSocket received shutdown signal");
                    should_reconnect = false;
                    break;
                }
            }

            if !should_reconnect {
                break;
            }

            info!("Feishu WebSocket reconnecting in 5 seconds...");
            sleep(Duration::from_secs(5)).await;
        }

        Ok(())
    }

    /// Handle WebSocket message
    async fn handle_websocket_message(&self, text: &str) -> Result<()> {
        let payload: Value = serde_json::from_str(text).map_err(|e| {
            ChannelError::Error(format!("Failed to parse WebSocket message: {}", e))
        })?;

        // Check if it's a challenge request (for webhook verification)
        if let Some(challenge) = payload.get("challenge").and_then(|v| v.as_str()) {
            debug!("Received challenge request: {}", challenge);
            return Ok(());
        }

        // Parse event
        let event: FeishuEvent = serde_json::from_value(payload)
            .map_err(|e| ChannelError::Error(format!("Failed to parse event: {}", e)))?;

        // Only handle message receive events
        if event.event_type != "im.message.receive_v1" {
            debug!("Ignoring event type: {}", event.event_type);
            return Ok(());
        }

        self.handle_message_event(event).await
    }

    /// Handle message receive event
    async fn handle_message_event(&self, event: FeishuEvent) -> Result<()> {
        let event_data = match event.event {
            Some(e) => e,
            None => {
                debug!("No event data in message");
                return Ok(());
            }
        };

        let message = event_data.message;
        let sender = event_data.sender;

        // Deduplication check
        if self.is_processed(&message.message_id).await {
            return Ok(());
        }
        self.mark_processed(message.message_id.clone()).await;

        // Skip bot messages
        if sender.sender_type == "bot" {
            debug!("Skipping bot message");
            return Ok(());
        }

        let sender_id = sender
            .sender_id
            .as_ref()
            .map(|s| s.open_id.clone())
            .unwrap_or_else(|| "unknown".to_string());
        let chat_id = message.chat_id;
        let chat_type = message.chat_type; // "p2p" or "group"
        let msg_type = message.message_type;

        // Add reaction to indicate "seen"
        let _ = self.add_reaction(&message.message_id, "THUMBSUP").await;

        // Parse message content
        let content = if msg_type == "text" {
            match serde_json::from_str::<Value>(&message.content) {
                Ok(v) => v
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string(),
                Err(_) => message.content,
            }
        } else {
            MSG_TYPE_MAP
                .iter()
                .find(|(k, _)| *k == msg_type)
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| format!("[{}]", msg_type))
        };

        if content.is_empty() {
            return Ok(());
        }

        // Forward to message bus
        let reply_to = if chat_type == "group" {
            chat_id.clone()
        } else {
            sender_id.clone()
        };

        let inbound_msg =
            agent_diva_core::bus::InboundMessage::new("feishu", sender_id.clone(), reply_to, content)
                .with_metadata("message_id", json!(message.message_id))
                .with_metadata("chat_type", json!(chat_type))
                .with_metadata("msg_type", json!(msg_type));

        if let Some(tx) = &self.inbound_tx {
            if let Err(e) = tx.send(inbound_msg).await {
                error!("Failed to send message to bus: {}", e);
            }
        }

        Ok(())
    }

    /// Add reaction to a message
    async fn add_reaction(&self, message_id: &str, emoji_type: &str) -> Result<()> {
        let token = self.get_access_token().await?;
        let url = format!(
            "{}/im/v1/messages/{}/reactions",
            FEISHU_API_BASE, message_id
        );

        let body = json!({
            "reaction_type": {
                "emoji_type": emoji_type
            }
        });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| ChannelError::ApiError(format!("Add reaction failed: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            warn!("Failed to add reaction: {} - {}", status, error_text);
        } else {
            debug!("Added {} reaction to message {}", emoji_type, message_id);
        }

        Ok(())
    }

    /// Regex to match markdown tables
    fn table_regex() -> Regex {
        Regex::new(r"((?:^[ \t]*\|.+\|[ \t]*\n)(?:^[ \t]*\|[-:\s|]+\|[ \t]*\n)(?:^[ \t]*\|.+\|[ \t]*\n?)+)")
            .expect("Invalid table regex")
    }

    /// Parse markdown table into Feishu table element
    fn parse_md_table(table_text: &str) -> Option<Value> {
        let lines: Vec<&str> = table_text
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();

        if lines.len() < 3 {
            return None;
        }

        let split_line = |l: &str| -> Vec<String> {
            l.trim_matches('|')
                .split('|')
                .map(|c| c.trim().to_string())
                .collect()
        };

        let headers = split_line(lines[0]);
        let rows: Vec<Vec<String>> = lines[2..].iter().map(|l| split_line(l)).collect();

        let columns: Vec<Value> = headers
            .iter()
            .enumerate()
            .map(|(i, h)| {
                json!({
                    "tag": "column",
                    "name": format!("c{}", i),
                    "display_name": h,
                    "width": "auto"
                })
            })
            .collect();

        let row_objects: Vec<Value> = rows
            .iter()
            .map(|r| {
                let mut obj = serde_json::Map::new();
                for (i, _h) in headers.iter().enumerate() {
                    obj.insert(
                        format!("c{}", i),
                        json!(r.get(i).map(|s| s.as_str()).unwrap_or("")),
                    );
                }
                Value::Object(obj)
            })
            .collect();

        Some(json!({
            "tag": "table",
            "page_size": rows.len() + 1,
            "columns": columns,
            "rows": row_objects
        }))
    }

    /// Build card elements from content
    fn build_card_elements(&self, content: &str) -> Vec<Value> {
        let table_re = Self::table_regex();
        let mut elements = Vec::new();
        let mut last_end = 0;

        for m in table_re.find_iter(content) {
            let before = content[last_end..m.start()].trim();
            if !before.is_empty() {
                elements.push(json!({
                    "tag": "markdown",
                    "content": before
                }));
            }

            if let Some(table) = Self::parse_md_table(m.as_str()) {
                elements.push(table);
            } else {
                elements.push(json!({
                    "tag": "markdown",
                    "content": m.as_str()
                }));
            }
            last_end = m.end();
        }

        let remaining = content[last_end..].trim();
        if !remaining.is_empty() {
            elements.push(json!({
                "tag": "markdown",
                "content": remaining
            }));
        }

        if elements.is_empty() {
            elements.push(json!({
                "tag": "markdown",
                "content": content
            }));
        }

        elements
    }
}

#[async_trait]
impl ChannelHandler for FeishuHandler {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_running(&self) -> bool {
        *self.running.blocking_read()
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(ChannelError::NotConfigured(
                "Feishu channel not enabled".to_string(),
            ));
        }

        self.validate_config()?;

        // Test authentication
        self.get_access_token().await?;

        // Get WebSocket URL
        let ws_url = self.get_websocket_url().await?;
        info!("Feishu WebSocket URL obtained");

        *self.running.write().await = true;

        // Start WebSocket connection
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let handler = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = handler.run_websocket(ws_url, shutdown_rx).await {
                error!("Feishu WebSocket task failed: {}", e);
            }
        });

        info!("Feishu channel started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        info!("Feishu channel stopped");
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        if !*self.running.read().await {
            return Err(ChannelError::NotRunning(
                "Feishu channel not running".to_string(),
            ));
        }

        let token = self.get_access_token().await?;

        // Determine receive_id_type based on chat_id format
        // open_id starts with "ou_", chat_id starts with "oc_"
        let receive_id_type = if msg.chat_id.starts_with("oc_") {
            "chat_id"
        } else {
            "open_id"
        };

        // Build card with markdown + table support
        let elements = self.build_card_elements(&msg.content);
        let card = json!({
            "config": {
                "wide_screen_mode": true
            },
            "elements": elements
        });

        let url = format!("{}/im/v1/messages", FEISHU_API_BASE);

        let body = json!({
            "receive_id": msg.chat_id,
            "msg_type": "interactive",
            "content": card.to_string()
        });

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .query(&[("receive_id_type", receive_id_type)])
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

        debug!("Feishu message sent to {}", msg.chat_id);
        Ok(())
    }

    fn set_inbound_sender(&mut self, tx: mpsc::Sender<agent_diva_core::bus::InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.base.is_allowed(sender_id)
    }
}

impl FeishuHandler {
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
    fn test_feishu_handler_new() {
        let mut feishu_config = FeishuConfig::default();
        feishu_config.enabled = true;
        feishu_config.app_id = "test_app_id".to_string();
        feishu_config.app_secret = "test_secret".to_string();

        let config = Config::default();
        let handler = FeishuHandler::new(feishu_config, config);

        assert_eq!(handler.name(), "feishu");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_validate_config_missing_app_id() {
        let mut feishu_config = FeishuConfig::default();
        feishu_config.enabled = true;
        feishu_config.app_secret = "test_secret".to_string();

        let config = Config::default();
        let handler = FeishuHandler::new(feishu_config, config);

        let result = handler.validate_config();
        assert!(result.is_err());

        if let Err(ChannelError::InvalidConfig(msg)) = result {
            assert!(msg.contains("app_id"));
        } else {
            panic!("Expected InvalidConfig error");
        }
    }

    #[test]
    fn test_validate_config_missing_app_secret() {
        let mut feishu_config = FeishuConfig::default();
        feishu_config.enabled = true;
        feishu_config.app_id = "test_app_id".to_string();

        let config = Config::default();
        let handler = FeishuHandler::new(feishu_config, config);

        let result = handler.validate_config();
        assert!(result.is_err());

        if let Err(ChannelError::InvalidConfig(msg)) = result {
            assert!(msg.contains("app_secret"));
        } else {
            panic!("Expected InvalidConfig error");
        }
    }

    #[tokio::test]
    async fn test_is_processed() {
        let mut feishu_config = FeishuConfig::default();
        feishu_config.enabled = true;
        feishu_config.app_id = "test_app_id".to_string();
        feishu_config.app_secret = "test_secret".to_string();

        let config = Config::default();
        let handler = FeishuHandler::new(feishu_config, config);

        assert!(!handler.is_processed("msg_123").await);

        handler.mark_processed("msg_123".to_string()).await;

        assert!(handler.is_processed("msg_123").await);
    }

    #[test]
    fn test_is_allowed() {
        let mut feishu_config = FeishuConfig::default();
        feishu_config.allow_from = vec!["user123".to_string()];

        let config = Config::default();
        let handler = FeishuHandler::new(feishu_config, config);

        assert!(handler.is_allowed("user123"));
        assert!(!handler.is_allowed("user456"));
    }

    #[test]
    fn test_build_card_elements_simple() {
        let feishu_config = FeishuConfig::default();
        let config = Config::default();
        let handler = FeishuHandler::new(feishu_config, config);

        let content = "Hello world";
        let elements = handler.build_card_elements(content);

        assert_eq!(elements.len(), 1);
        assert_eq!(elements[0]["tag"], "markdown");
        assert_eq!(elements[0]["content"], "Hello world");
    }

    #[test]
    fn test_parse_md_table() {
        let table_text = "| Name | Age |\n|------|-----|\n| John | 30 |\n| Jane | 25 |";
        let result = FeishuHandler::parse_md_table(table_text);

        assert!(result.is_some());
        let table = result.unwrap();
        assert_eq!(table["tag"], "table");
        assert_eq!(table["page_size"], 3);
    }

    #[test]
    fn test_parse_md_table_invalid() {
        let table_text = "| Name | Age |";
        let result = FeishuHandler::parse_md_table(table_text);

        assert!(result.is_none());
    }
}
