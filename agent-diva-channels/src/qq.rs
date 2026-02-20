//! QQ channel implementation using WebSocket
//!
//! This implementation is based on the QQ Bot OpenAPI (botpy SDK).
//! It uses WebSocket for real-time message reception and HTTP API for sending messages.
//!
//! Key features:
//! - WebSocket gateway connection with automatic reconnection
//! - Token-based authentication with automatic refresh
//! - Heartbeat mechanism to keep connection alive
//! - C2C (user-to-bot) private message support
//! - Message deduplication
//! - Allowlist-based access control

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use agent_diva_core::bus::OutboundMessage;
use agent_diva_core::config::schema::QQConfig;
use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, sleep};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{debug, error, info, warn};

use crate::base::{BaseChannel, ChannelError, ChannelHandler, Result};

// WebSocket opcodes
const WS_DISPATCH_EVENT: u8 = 0;
const WS_HEARTBEAT: u8 = 1;
const WS_IDENTITY: u8 = 2;
const WS_RESUME: u8 = 6;
const WS_RECONNECT: u8 = 7;
const WS_INVALID_SESSION: u8 = 9;
const WS_HELLO: u8 = 10;
const WS_HEARTBEAT_ACK: u8 = 11;

// QQ OpenAPI endpoints
const API_BASE: &str = "https://api.sgroup.qq.com";

/// Token for QQ Bot authentication
#[derive(Debug, Clone)]
struct Token {
    app_id: String,
    secret: String,
    access_token: Option<String>,
    token_type: String,
    expires_at: u64,
}

impl Token {
    fn new(app_id: String, secret: String) -> Self {
        Self {
            app_id,
            secret,
            access_token: None,
            token_type: "QQBot".to_string(),
            expires_at: 0,
        }
    }

    fn get_string(&self) -> String {
        if let Some(token) = &self.access_token {
            format!("{} {}", self.token_type, token)
        } else {
            format!("{} {}.{}", self.token_type, self.app_id, self.secret)
        }
    }

    async fn check_token(&mut self, http: &HttpClient) -> Result<()> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if self.access_token.is_some() && now < self.expires_at {
            return Ok(());
        }

        // Get access token from QQ OpenAPI
        let url = "https://bots.qq.com/app/getAppAccessToken";
        let body = json!({
            "appId": self.app_id,
            "clientSecret": self.secret
        });

        let response =
            http.post(url).json(&body).send().await.map_err(|e| {
                ChannelError::ConnectionFailed(format!("Token request failed: {}", e))
            })?;

        let data: Value = response
            .json()
            .await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Token parse failed: {}", e)))?;

        if let Some(token) = data["access_token"].as_str() {
            self.access_token = Some(token.to_string());
            
            let expires_in = data["expires_in"].as_u64().or_else(|| {
                data["expires_in"].as_str().and_then(|s| s.parse().ok())
            }).unwrap_or(7200);

            self.expires_at = now + expires_in - 60; // Refresh 1 minute early

            info!("QQ Bot token obtained successfully, expires in {}s", expires_in);
            Ok(())
        } else {
            Err(ChannelError::AuthError(
                format!("Failed to get access token: {:?}", data)
            ))
        }
    }
}

/// WebSocket gateway information
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct GatewayInfo {
    url: String,
    shards: u32,
    session_start_limit: SessionStartLimit,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct SessionStartLimit {
    total: u32,
    remaining: u32,
    reset_after: u32,
    max_concurrency: u32,
}

/// WebSocket message payload
#[derive(Debug, Serialize, Deserialize)]
struct WsPayload {
    op: u8,
    d: Option<Value>,
    s: Option<u64>,
    t: Option<String>,
}

/// C2C Message structure
#[derive(Debug, Deserialize)]
struct C2CMessage {
    id: String,
    content: Option<String>,
    timestamp: String,
    author: C2CAuthor,
}

#[derive(Debug, Deserialize)]
struct C2CAuthor {
    user_openid: Option<String>,
    id: Option<String>,
}

/// QQ channel handler
pub struct QQHandler {
    config: QQConfig,
    base: BaseChannel,
    running: Arc<RwLock<bool>>,
    processed_ids: Arc<RwLock<VecDeque<String>>>,
    http_client: HttpClient,
    token: Arc<RwLock<Token>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    inbound_tx: Option<mpsc::Sender<agent_diva_core::bus::InboundMessage>>,
}

impl QQHandler {
    /// Create a new QQ handler
    pub fn new(config: QQConfig, base_config: agent_diva_core::config::schema::Config) -> Self {
        let allow_from = config.allow_from.clone();
        let base = BaseChannel::new("qq", base_config, allow_from);
        let token = Token::new(config.app_id.clone(), config.secret.clone());

        Self {
            config,
            base,
            running: Arc::new(RwLock::new(false)),
            processed_ids: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            http_client: HttpClient::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            token: Arc::new(RwLock::new(token)),
            shutdown_tx: None,
            inbound_tx: None,
        }
    }

    /// Check if message ID has been processed (deduplication)
    async fn is_processed_qq(&self, message_id: &str) -> bool {
        let ids = self.processed_ids.read().await;
        ids.contains(&message_id.to_string())
    }

    /// Mark message ID as processed
    async fn mark_processed_qq(&self, message_id: String) {
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
                "QQ app_id not configured".to_string(),
            ));
        }
        if self.config.secret.is_empty() {
            return Err(ChannelError::InvalidConfig(
                "QQ secret not configured".to_string(),
            ));
        }
        Ok(())
    }

    /// Get WebSocket gateway URL
    async fn get_gateway(&self) -> Result<GatewayInfo> {
        let url = format!("{}/gateway/bot", API_BASE);
        let token = self.token.read().await;

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", token.get_string())
            .header("X-Union-Appid", &token.app_id)
            .send()
            .await
            .map_err(|e| {
                ChannelError::ConnectionFailed(format!("Gateway request failed: {}", e))
            })?;

        let gateway: GatewayInfo = response
            .json()
            .await
            .map_err(|e| ChannelError::ConnectionFailed(format!("Gateway parse failed: {}", e)))?;

        Ok(gateway)
    }

    /// Run WebSocket connection
    async fn run_websocket(
        &self,
        gateway_url: String,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        let mut session_id: Option<String> = None;
        let mut last_seq: u64 = 0;

        loop {
            // Check if we should shutdown
            if shutdown_rx.try_recv().is_ok() {
                info!("QQ WebSocket shutting down");
                break;
            }

            // Connect to WebSocket
            let (ws_stream, _) = match connect_async(&gateway_url).await {
                Ok(result) => result,
                Err(e) => {
                    error!("QQ WebSocket connection failed: {}", e);
                    sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };

            info!("QQ WebSocket connected");
            let (mut write, mut read) = ws_stream.split();

            // Send identify or resume
            let token = self.token.read().await;
            let identify_payload = if let Some(ref sid) = session_id {
                // Resume
                WsPayload {
                    op: WS_RESUME,
                    d: Some(json!({
                        "token": token.get_string(),
                        "session_id": sid,
                        "seq": last_seq,
                    })),
                    s: None,
                    t: None,
                }
            } else {
                // Identify
                WsPayload {
                    op: WS_IDENTITY,
                    d: Some(json!({
                        "token": token.get_string(),
                        "intents": 1 << 25 | 1 << 12, // public_messages + direct_message
                        "shard": [0, 1],
                    })),
                    s: None,
                    t: None,
                }
            };
            drop(token);

            if let Err(e) = write
                .send(WsMessage::Text(
                    serde_json::to_string(&identify_payload).unwrap(),
                ))
                .await
            {
                error!("Failed to send identify: {}", e);
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Heartbeat task
            let (heartbeat_tx, mut heartbeat_rx) = mpsc::channel::<()>(1);
            let heartbeat_task = {
                let mut write = write;
                tokio::spawn(async move {
                    let mut ticker = interval(Duration::from_secs(30));
                    loop {
                        tokio::select! {
                            _ = ticker.tick() => {
                                let heartbeat = WsPayload {
                                    op: WS_HEARTBEAT,
                                    d: Some(json!(last_seq)),
                                    s: None,
                                    t: None,
                                };
                                if let Err(e) = write
                                    .send(WsMessage::Text(serde_json::to_string(&heartbeat).unwrap()))
                                    .await
                                {
                                    error!("Heartbeat failed: {}", e);
                                    break;
                                }
                                debug!("QQ heartbeat sent");
                            }
                            _ = heartbeat_rx.recv() => {
                                break;
                            }
                        }
                    }
                    write
                })
            };

            // Message receiving loop
            let mut should_reconnect = true;
            while let Some(msg) = read.next().await {
                match msg {
                    Ok(WsMessage::Text(text)) => {
                        if let Ok(payload) = serde_json::from_str::<WsPayload>(&text) {
                            if let Some(seq) = payload.s {
                                if seq > 0 {
                                    last_seq = seq;
                                }
                            }

                            match payload.op {
                                WS_HELLO => {
                                    info!("QQ Bot WebSocket HELLO received");
                                }
                                WS_DISPATCH_EVENT => {
                                    if let Some(ref event_type) = payload.t {
                                        self.handle_event(event_type, payload.d.clone()).await;
                                    }

                                    // Special handling for READY event
                                    if payload.t.as_deref() == Some("READY") {
                                        if let Some(d) = &payload.d {
                                            if let Some(sid) = d["session_id"].as_str() {
                                                session_id = Some(sid.to_string());
                                                info!("QQ Bot ready with session: {}", sid);
                                            }
                                        }
                                    }
                                }
                                WS_HEARTBEAT_ACK => {
                                    debug!("QQ heartbeat ACK received");
                                }
                                WS_RECONNECT => {
                                    info!("QQ server requested reconnect");
                                    break;
                                }
                                WS_INVALID_SESSION => {
                                    warn!("QQ invalid session, resetting");
                                    session_id = None;
                                    last_seq = 0;
                                    should_reconnect = false;
                                    break;
                                }
                                _ => {
                                    debug!("Unknown opcode: {}", payload.op);
                                }
                            }
                        }
                    }
                    Ok(WsMessage::Close(_)) => {
                        info!("QQ WebSocket closed by server");
                        break;
                    }
                    Err(e) => {
                        error!("QQ WebSocket error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }

            // Stop heartbeat
            let _ = heartbeat_tx.send(()).await;
            let _ = heartbeat_task.await;

            if !should_reconnect {
                break;
            }

            info!("QQ WebSocket reconnecting in 5 seconds...");
            sleep(Duration::from_secs(5)).await;
        }

        Ok(())
    }

    /// Handle WebSocket events
    async fn handle_event(&self, event_type: &str, data: Option<Value>) {
        match event_type.to_lowercase().as_str() {
            "c2c_message_create" => {
                if let Some(d) = data {
                    self.handle_c2c_message(d).await;
                }
            }
            "ready" => {
                if let Some(d) = data {
                    if let Some(user) = d["user"]["username"].as_str() {
                        info!("QQ Bot ready: {}", user);
                    }
                }
            }
            _ => {
                debug!("Unhandled event type: {}", event_type);
            }
        }
    }

    /// Handle C2C (user-to-bot) message
    async fn handle_c2c_message(&self, data: Value) {
        // Parse message
        let message: C2CMessage = match serde_json::from_value(data) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to parse C2C message: {}", e);
                return;
            }
        };

        // Check if already processed (deduplication)
        if self.is_processed_qq(&message.id).await {
            return;
        }
        self.mark_processed_qq(message.id.clone()).await;

        // Extract user ID
        let user_id = message
            .author
            .user_openid
            .or(message.author.id)
            .unwrap_or_else(|| "unknown".to_string());

        // Check allowlist
        if !self.is_allowed(&user_id) {
            debug!("User {} not in allowlist", user_id);
            return;
        }

        // Get message content
        let content = match message.content {
            Some(c) if !c.trim().is_empty() => c.trim().to_string(),
            _ => {
                debug!("Empty message content");
                return;
            }
        };

        // Send to message bus
        let inbound_msg =
            agent_diva_core::bus::InboundMessage::new("qq", user_id.clone(), user_id.clone(), content)
                .with_metadata("message_id", json!(message.id))
                .with_metadata("timestamp", json!(message.timestamp));

        if let Some(tx) = &self.inbound_tx {
            if let Err(e) = tx.send(inbound_msg).await {
                error!("Failed to send message to bus: {}", e);
            }
        }
    }
}

#[async_trait]
impl ChannelHandler for QQHandler {
    fn set_inbound_sender(&mut self, tx: mpsc::Sender<agent_diva_core::bus::InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_running(&self) -> bool {
        *self.running.blocking_read()
    }

    async fn start(&mut self) -> Result<()> {
        if !self.config.enabled {
            return Err(ChannelError::NotConfigured(
                "QQ channel not enabled".to_string(),
            ));
        }

        self.validate_config()?;

        // Authenticate and get token
        {
            let mut token = self.token.write().await;
            token.check_token(&self.http_client).await?;
        }

        // Get gateway URL
        let gateway = self.get_gateway().await?;
        info!("QQ Gateway URL: {}", gateway.url);

        *self.running.write().await = true;

        // Start WebSocket connection
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let handler = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = handler.run_websocket(gateway.url, shutdown_rx).await {
                error!("QQ WebSocket task failed: {}", e);
            }
        });

        info!("QQ channel started (C2C private message)");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        info!("QQ channel stopped");
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        if !*self.running.read().await {
            return Err(ChannelError::NotRunning(
                "QQ channel not running".to_string(),
            ));
        }

        // Ensure token is valid
        {
            let mut token = self.token.write().await;
            token.check_token(&self.http_client).await?;
        }

        // Send C2C message via HTTP API
        let url = format!("{}/v2/users/{}/messages", API_BASE, msg.chat_id);
        let token = self.token.read().await;

        let mut body = json!({
            "msg_type": 0,  // Text message
            "content": msg.content,
        });

        if let Some(msg_id) = msg.reply_to {
            if let Some(obj) = body.as_object_mut() {
                obj.insert("msg_id".to_string(), json!(msg_id));
            }
        }

        let response = self
            .http_client
            .post(&url)
            .header("Authorization", token.get_string())
            .header("X-Union-Appid", &token.app_id)
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

        debug!("QQ message sent to {}", msg.chat_id);
        Ok(())
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.base.is_allowed(sender_id)
    }
}

impl QQHandler {
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
    fn test_qq_handler_new() {
        let mut qq_config = QQConfig::default();
        qq_config.enabled = true;
        qq_config.app_id = "test_app_id".to_string();
        qq_config.secret = "test_secret".to_string();

        let config = Config::default();
        let handler = QQHandler::new(qq_config, config);

        assert_eq!(handler.name(), "qq");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_validate_config_missing_app_id() {
        let mut qq_config = QQConfig::default();
        qq_config.enabled = true;
        qq_config.secret = "test_secret".to_string();

        let config = Config::default();
        let handler = QQHandler::new(qq_config, config);

        let result = handler.validate_config();
        assert!(result.is_err());

        if let Err(ChannelError::InvalidConfig(msg)) = result {
            assert!(msg.contains("app_id"));
        } else {
            panic!("Expected InvalidConfig error");
        }
    }

    #[test]
    fn test_validate_config_missing_secret() {
        let mut qq_config = QQConfig::default();
        qq_config.enabled = true;
        qq_config.app_id = "test_app_id".to_string();

        let config = Config::default();
        let handler = QQHandler::new(qq_config, config);

        let result = handler.validate_config();
        assert!(result.is_err());

        if let Err(ChannelError::InvalidConfig(msg)) = result {
            assert!(msg.contains("secret"));
        } else {
            panic!("Expected InvalidConfig error");
        }
    }

    #[tokio::test]
    async fn test_is_processed_qq() {
        let mut qq_config = QQConfig::default();
        qq_config.enabled = true;
        qq_config.app_id = "test_app_id".to_string();
        qq_config.secret = "test_secret".to_string();

        let config = Config::default();
        let handler = QQHandler::new(qq_config, config);

        assert!(!handler.is_processed_qq("msg_123").await);

        handler.mark_processed_qq("msg_123".to_string()).await;

        assert!(handler.is_processed_qq("msg_123").await);
    }

    #[test]
    fn test_is_allowed() {
        let mut qq_config = QQConfig::default();
        qq_config.allow_from = vec!["user123".to_string()];

        let config = Config::default();
        let handler = QQHandler::new(qq_config, config);

        assert!(handler.is_allowed("user123"));
        assert!(!handler.is_allowed("user456"));
    }

    #[test]
    fn test_token_new() {
        let token = Token::new("app123".to_string(), "secret456".to_string());
        assert_eq!(token.app_id, "app123");
        assert_eq!(token.secret, "secret456");
        assert!(token.access_token.is_none());
    }

    #[test]
    fn test_token_get_string_without_access_token() {
        let token = Token::new("app123".to_string(), "secret456".to_string());
        let token_string = token.get_string();
        assert_eq!(token_string, "QQBot app123.secret456");
    }

    #[test]
    fn test_token_get_string_with_access_token() {
        let mut token = Token::new("app123".to_string(), "secret456".to_string());
        token.access_token = Some("access_token_xyz".to_string());
        let token_string = token.get_string();
        assert_eq!(token_string, "QQBot access_token_xyz");
    }
}
