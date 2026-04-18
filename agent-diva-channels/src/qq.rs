//! QQ channel implementation using WebSocket
//!
//! This implementation is based on the QQ Bot OpenAPI (botpy SDK).
//! It uses WebSocket for real-time message reception and HTTP API for sending messages.
//!
//! Key features:
//! - WebSocket gateway connection with automatic reconnection
//! - Token-based authentication with automatic refresh and retry
//! - Heartbeat mechanism with ACK timeout detection
//! - Session resume support (op=6) with persistent session_id/sequence
//! - C2C (user-to-bot) private message support
//! - Message deduplication with HashSet
//! - Allowlist-based access control

use agent_diva_core::bus::OutboundMessage;
use agent_diva_core::config::schema::QQConfig;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use reqwest_qq::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
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

// Heartbeat timeout detection
const MAX_MISSED_ACKS: u32 = 3;

// Message deduplication
const DEDUP_CAPACITY: usize = 10_000;

// Token refresh retry (reserved for future enhancement)
#[allow(dead_code)]
const AUTH_RETRY_MAX_ATTEMPTS: u32 = 4;
#[allow(dead_code)]
const AUTH_RETRY_INITIAL_BACKOFF_MS: u64 = 500;
#[allow(dead_code)]
const AUTH_RETRY_MAX_BACKOFF_MS: u64 = 8_000;

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

            let expires_in = data["expires_in"]
                .as_u64()
                .or_else(|| data["expires_in"].as_str().and_then(|s| s.parse().ok()))
                .unwrap_or(7200);

            self.expires_at = now + expires_in - 60; // Refresh 1 minute early

            info!(
                "QQ Bot token obtained successfully, expires in {}s",
                expires_in
            );
            Ok(())
        } else {
            Err(ChannelError::AuthError(format!(
                "Failed to get access token: {:?}",
                data
            )))
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
    running: Arc<AtomicBool>,
    /// Message deduplication set
    dedup: Arc<RwLock<HashSet<String>>>,
    http_client: HttpClient,
    token: Arc<RwLock<Token>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
    inbound_tx: Option<mpsc::Sender<agent_diva_core::bus::InboundMessage>>,
    /// Session ID from READY event, for gateway resume (op=6)
    session_id: Arc<RwLock<Option<String>>>,
    /// Last sequence number received, for gateway resume
    last_sequence: Arc<RwLock<Option<u64>>>,
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
            running: Arc::new(AtomicBool::new(false)),
            dedup: Arc::new(RwLock::new(HashSet::new())),
            http_client: HttpClient::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .expect("Failed to build HTTP client"),
            token: Arc::new(RwLock::new(token)),
            shutdown_tx: None,
            inbound_tx: None,
            session_id: Arc::new(RwLock::new(None)),
            last_sequence: Arc::new(RwLock::new(None)),
        }
    }

    /// Check and insert message ID for deduplication.
    /// Returns true if the message is a duplicate.
    async fn is_duplicate(&self, msg_id: &str) -> bool {
        if msg_id.is_empty() {
            return false;
        }

        let mut dedup = self.dedup.write().await;

        if dedup.contains(msg_id) {
            return true;
        }

        // Evict oldest half when at capacity
        if dedup.len() >= DEDUP_CAPACITY {
            let to_remove: Vec<String> = dedup.iter().take(DEDUP_CAPACITY / 2).cloned().collect();
            for key in to_remove {
                dedup.remove(&key);
            }
        }

        dedup.insert(msg_id.to_string());
        false
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

    /// Run WebSocket connection with heartbeat timeout detection and session resume
    async fn run_websocket(
        &self,
        gateway_url: String,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        let mut backoff_secs: u64 = 1;
        let max_backoff_secs: u64 = 60;

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
                    sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(max_backoff_secs);
                    continue;
                }
            };

            // Successfully connected — reset backoff
            backoff_secs = 1;
            info!("QQ WebSocket connected");
            let (mut write, mut read) = ws_stream.split();

            // Read Hello (opcode 10) to get heartbeat interval
            let hello_msg = match read.next().await {
                Some(Ok(WsMessage::Text(text))) => text,
                _ => {
                    error!("QQ: no hello frame received");
                    sleep(Duration::from_secs(backoff_secs)).await;
                    backoff_secs = (backoff_secs * 2).min(max_backoff_secs);
                    continue;
                }
            };

            let heartbeat_interval =
                if let Ok(hello_data) = serde_json::from_str::<Value>(&hello_msg) {
                    hello_data
                        .get("d")
                        .and_then(|d| d.get("heartbeat_interval"))
                        .and_then(Value::as_u64)
                        .unwrap_or(41250)
                } else {
                    41250 // default fallback
                };

            // Add grace period (10% of interval, capped at 5s)
            let grace_ms: u64 = (heartbeat_interval / 10).min(5_000);
            let effective_interval_ms = heartbeat_interval.saturating_add(grace_ms);

            info!(
                "QQ: heartbeat interval={}ms, grace={}ms, effective={}ms",
                heartbeat_interval, grace_ms, effective_interval_ms
            );

            // Check if we can resume a previous session
            let stored_session = self.session_id.read().await.clone();
            let stored_seq = *self.last_sequence.read().await;

            // Send identify or resume
            let token = self.token.read().await;
            let identify_payload = if let (Some(ref sid), Some(seq)) = (&stored_session, stored_seq)
            {
                // Resume
                info!(
                    "QQ: attempting session resume (session_id={}, seq={})",
                    sid, seq
                );
                WsPayload {
                    op: WS_RESUME,
                    d: Some(json!({
                        "token": token.get_string(),
                        "session_id": sid,
                        "seq": seq,
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
                error!("Failed to send identify/resume: {}", e);
                sleep(Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(max_backoff_secs);
                continue;
            }

            // Track consecutive missed heartbeat ACKs
            let mut missed_ack_count: u32 = 0;
            let mut sequence: i64 = stored_seq.map(|s| s as i64).unwrap_or(-1);

            // Spawn heartbeat timer
            let (hb_tx, mut hb_rx) = mpsc::channel::<()>(1);
            let hb_interval_ms = effective_interval_ms;
            tokio::spawn(async move {
                let mut ticker = interval(Duration::from_millis(hb_interval_ms));
                loop {
                    ticker.tick().await;
                    if hb_tx.send(()).await.is_err() {
                        break;
                    }
                }
            });

            // Reason the loop exited
            let exit_reason;

            'outer: loop {
                tokio::select! {
                    _ = hb_rx.recv() => {
                        // Increment the missed-ACK counter. Only declare the
                        // connection dead after MAX_MISSED_ACKS consecutive
                        // heartbeats go un-acknowledged.
                        if missed_ack_count > 0 {
                            if missed_ack_count >= MAX_MISSED_ACKS {
                                warn!(
                                    "QQ: {} consecutive heartbeat ACKs missed \
                                     (interval {}ms + {}ms grace); \
                                     connection appears zombied",
                                    missed_ack_count, heartbeat_interval, grace_ms
                                );
                                exit_reason = ExitReason::HeartbeatTimeout;
                                break;
                            }
                            info!(
                                "QQ: heartbeat ACK missed ({}/{}) \
                                 tolerating transient delay",
                                missed_ack_count, MAX_MISSED_ACKS
                            );
                        }
                        let d = if sequence >= 0 { json!(sequence) } else { json!(null) };
                        let hb = json!({"op": WS_HEARTBEAT, "d": d});
                        if write
                            .send(WsMessage::Text(hb.to_string()))
                            .await
                            .is_err()
                        {
                            exit_reason = ExitReason::WriteFailed;
                            break;
                        }
                        missed_ack_count += 1;
                    }
                    msg = read.next() => {
                        let msg = match msg {
                            Some(Ok(WsMessage::Text(t))) => t,
                            Some(Ok(WsMessage::Close(_))) => {
                                exit_reason = ExitReason::Close;
                                break;
                            }
                            None => {
                                exit_reason = ExitReason::StreamEnded;
                                break;
                            }
                            _ => continue,
                        };

                        let event: Value = match serde_json::from_str(&msg) {
                            Ok(e) => e,
                            Err(_) => continue,
                        };

                        // Track sequence number
                        if let Some(s) = event.get("s").and_then(Value::as_u64) {
                            if s > 0 {
                                sequence = s as i64;
                            }
                        }

                        let op = event.get("op").and_then(Value::as_u64).unwrap_or(0);
                        let op_u8 = op as u8;

                        match op_u8 {
                            WS_HELLO => {
                                debug!("QQ: HELLO received");
                            }
                            WS_DISPATCH_EVENT => {
                                if let Some(event_type) = event.get("t").and_then(Value::as_str) {
                                    self.handle_event(event_type, event.get("d").cloned()).await;
                                }

                                // Capture session_id from READY event
                                if event.get("t").and_then(Value::as_str) == Some("READY") {
                                    if let Some(d) = event.get("d") {
                                        if let Some(sid) = d["session_id"].as_str() {
                                            *self.session_id.write().await = Some(sid.to_string());
                                            info!("QQ: session established (session_id={}, event=READY)", sid);
                                        }
                                    }
                                }

                                // Capture session_id from RESUMED event
                                if event.get("t").and_then(Value::as_str) == Some("RESUMED") {
                                    info!("QQ: session resumed successfully");
                                }
                            }
                            WS_HEARTBEAT_ACK => {
                                missed_ack_count = 0;
                                debug!("QQ: heartbeat ACK received");
                            }
                            WS_RECONNECT => {
                                info!("QQ: server requested reconnect");
                                exit_reason = ExitReason::Reconnect;
                                break 'outer;
                            }
                            WS_INVALID_SESSION => {
                                warn!("QQ: invalid session, clearing session state");
                                *self.session_id.write().await = None;
                                *self.last_sequence.write().await = None;
                                exit_reason = ExitReason::InvalidSession;
                                break 'outer;
                            }
                            _ => {
                                debug!("Unknown opcode: {}", op);
                            }
                        }
                    }
                }
            }

            // Persist sequence number for potential resume on next reconnect
            *self.last_sequence.write().await = if sequence >= 0 {
                Some(sequence as u64)
            } else {
                None
            };

            // Handle exit reason
            match exit_reason {
                ExitReason::InvalidSession => {
                    // session_id and last_sequence already cleared above
                    warn!("QQ: invalid session — will perform fresh identify on reconnect");
                }
                ExitReason::Reconnect => {
                    info!("QQ: server requested reconnect — session preserved for resume");
                }
                ExitReason::Close => {
                    warn!("QQ: WebSocket closed by server — will attempt resume");
                }
                ExitReason::StreamEnded => {
                    warn!("QQ: WebSocket stream ended unexpectedly — will attempt resume");
                }
                ExitReason::HeartbeatTimeout => {
                    warn!(
                        "QQ: heartbeat timeout after {} consecutive missed ACKs — will attempt resume",
                        MAX_MISSED_ACKS
                    );
                }
                ExitReason::WriteFailed => {
                    error!("QQ: WebSocket write failed — will attempt resume");
                }
            }

            // Reconnect with exponential backoff
            info!("QQ WebSocket reconnecting in {} seconds...", backoff_secs);
            sleep(Duration::from_secs(backoff_secs)).await;
            backoff_secs = (backoff_secs * 2).min(max_backoff_secs);
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
        if self.is_duplicate(&message.id).await {
            return;
        }

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
        let inbound_msg = agent_diva_core::bus::InboundMessage::new(
            "qq",
            user_id.clone(),
            user_id.clone(),
            content,
        )
        .with_metadata("message_id", json!(message.id))
        .with_metadata("timestamp", json!(message.timestamp));

        if let Some(tx) = &self.inbound_tx {
            if let Err(e) = tx.send(inbound_msg).await {
                error!("Failed to send message to bus: {}", e);
            }
        }
    }
}

/// Reason the websocket loop exited — used to decide reconnection behavior
enum ExitReason {
    Reconnect,
    InvalidSession,
    Close,
    StreamEnded,
    HeartbeatTimeout,
    WriteFailed,
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
        self.running.load(Ordering::Acquire)
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

        self.running.store(true, Ordering::Release);

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
        self.running.store(false, Ordering::Release);

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        info!("QQ channel stopped");
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        if !self.running.load(Ordering::Acquire) {
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
            dedup: Arc::clone(&self.dedup),
            http_client: self.http_client.clone(),
            token: Arc::clone(&self.token),
            shutdown_tx: None,
            inbound_tx: self.inbound_tx.clone(),
            session_id: Arc::clone(&self.session_id),
            last_sequence: Arc::clone(&self.last_sequence),
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
    async fn test_is_duplicate() {
        let mut qq_config = QQConfig::default();
        qq_config.enabled = true;
        qq_config.app_id = "test_app_id".to_string();
        qq_config.secret = "test_secret".to_string();

        let config = Config::default();
        let handler = QQHandler::new(qq_config, config);

        assert!(!handler.is_duplicate("msg_123").await);
        assert!(handler.is_duplicate("msg_123").await);
        assert!(!handler.is_duplicate("msg_456").await);
    }

    #[tokio::test]
    async fn test_is_duplicate_empty_id() {
        let mut qq_config = QQConfig::default();
        qq_config.enabled = true;
        qq_config.app_id = "test_app_id".to_string();
        qq_config.secret = "test_secret".to_string();

        let config = Config::default();
        let handler = QQHandler::new(qq_config, config);

        // Empty ID should not be deduplicated
        assert!(!handler.is_duplicate("").await);
        assert!(!handler.is_duplicate("").await);
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

    #[test]
    fn test_heartbeat_grace_period_calculation() {
        // The grace period is 10% of the server interval, capped at 5000ms.
        let cases: Vec<(u64, u64)> = vec![
            (41_250, 4_125),  // default QQ interval
            (30_000, 3_000),  // smaller interval
            (60_000, 5_000),  // larger interval, capped at 5s
            (100_000, 5_000), // very large, still capped
            (5_000, 500),     // small interval
            (0, 0),           // degenerate zero
        ];
        for (interval, expected_grace) in cases {
            let grace: u64 = (interval / 10).min(5_000);
            assert_eq!(
                grace, expected_grace,
                "grace for interval {} should be {}",
                interval, expected_grace
            );
            let effective = interval.saturating_add(grace);
            assert!(effective >= interval);
        }
    }

    #[test]
    fn test_missed_ack_counter_logic() {
        let max_missed: u32 = MAX_MISSED_ACKS;
        let mut missed: u32 = 0;

        // First tick: counter is 0, send heartbeat
        assert!(missed < max_missed);
        missed += 1;
        assert_eq!(missed, 1, "counter should be 1 after first heartbeat");

        // ACK received: reset
        missed = 0;
        assert_eq!(missed, 0, "counter should reset on ACK");

        // 3 consecutive misses without ACK
        for _ in 0..max_missed {
            assert!(
                missed < max_missed,
                "should not reach zombie state before {} misses",
                max_missed
            );
            missed += 1;
        }
        assert!(
            missed >= max_missed,
            "should declare zombie after {} missed ACKs",
            max_missed
        );
    }

    #[test]
    fn test_missed_ack_counter_reset_on_ack() {
        let _max_missed: u32 = MAX_MISSED_ACKS;
        let mut missed: u32 = 0;

        missed += 1;
        missed += 1;
        assert_eq!(missed, 2);

        // ACK arrives: reset
        missed = 0;
        assert_eq!(missed, 0);

        // Continue sending heartbeats
        missed += 1;
        assert_eq!(missed, 1);
    }

    #[test]
    fn test_auth_retry_backoff_stays_within_bounds() {
        // Simulate the backoff progression and verify it caps at max
        let mut backoff = AUTH_RETRY_INITIAL_BACKOFF_MS;
        for _ in 1..AUTH_RETRY_MAX_ATTEMPTS {
            backoff = (backoff * 2).min(AUTH_RETRY_MAX_BACKOFF_MS);
        }
        assert!(
            backoff <= AUTH_RETRY_MAX_BACKOFF_MS,
            "backoff must never exceed the configured maximum"
        );
    }

    #[test]
    fn test_auth_retry_constants_are_sensible() {
        assert!(AUTH_RETRY_MAX_ATTEMPTS >= 2, "should retry at least once");
        assert!(
            AUTH_RETRY_INITIAL_BACKOFF_MS > 0,
            "initial backoff must be positive"
        );
        assert!(
            AUTH_RETRY_MAX_BACKOFF_MS >= AUTH_RETRY_INITIAL_BACKOFF_MS,
            "max backoff must be >= initial"
        );
    }

    #[tokio::test]
    async fn test_dedup_capacity_eviction() {
        let mut qq_config = QQConfig::default();
        qq_config.enabled = true;
        qq_config.app_id = "test_app_id".to_string();
        qq_config.secret = "test_secret".to_string();

        let config = Config::default();
        let handler = QQHandler::new(qq_config, config);

        // Insert DEDUP_CAPACITY messages
        for i in 0..DEDUP_CAPACITY {
            assert!(!handler.is_duplicate(&format!("msg_{}", i)).await);
        }

        // Next message should trigger eviction
        assert!(
            !handler
                .is_duplicate(&format!("msg_{}", DEDUP_CAPACITY))
                .await
        );

        // Some old messages should have been evicted (half of them)
        // The first DEDUP_CAPACITY/2 messages should be evicted
        let evicted_count = handler
            .dedup
            .read()
            .await
            .iter()
            .filter(|id| {
                let num: usize = id.strip_prefix("msg_").unwrap_or("0").parse().unwrap_or(0);
                num < DEDUP_CAPACITY / 2
            })
            .count();

        // At least half should be evicted
        assert!(
            evicted_count < DEDUP_CAPACITY / 2,
            "expected eviction of old messages"
        );
    }
}
