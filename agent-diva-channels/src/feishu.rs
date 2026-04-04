//! Feishu/Lark channel integration using WebSocket long connection
//!
//! This implementation uses Feishu Open Platform WebSocket API with protobuf (pbbp2.proto)
//! frame encoding for real-time message reception and HTTP API for sending messages.
//!
//! Key features:
//! - WebSocket long connection with protobuf frame encoding (no public IP required)
//! - App ID + App Secret authentication
//! - Message deduplication with event_id and message_id
//! - Allowlist-based access control
//! - Interactive card message support with markdown and tables
//! - Heartbeat mechanism with configurable ping interval
//! - Message fragment reassembly for large payloads
//!
//! References:
//! - zeroclaw implementation: .workspace/zeroclaw/src/channels/lark.rs
//! - Feishu API: https://open.feishu.cn/document/home/index

use crate::base::{BaseChannel, ChannelError, ChannelHandler, Result};
use agent_diva_core::bus::OutboundMessage;
use agent_diva_core::config::schema::FeishuConfig;
use async_trait::async_trait;
use base64::Engine;
use futures::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use prost_derive::Message as ProstDeriveMessage;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{debug, error, info, warn};

// Feishu OpenAPI endpoints
const FEISHU_API_BASE: &str = "https://open.feishu.cn/open-apis";
const FEISHU_WS_BASE: &str = "https://open.feishu.cn";

// Heartbeat timeout
const WS_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(300);
const EVENT_DEDUP_TTL: Duration = Duration::from_secs(600);
const EVENT_DEDUP_CLEANUP_INTERVAL: Duration = Duration::from_secs(300);

// Message type display mapping
const MSG_TYPE_MAP: &[(&str, &str)] = &[
    ("image", "[image]"),
    ("audio", "[audio]"),
    ("file", "[file]"),
    ("sticker", "[sticker]"),
];

// ─────────────────────────────────────────────────────────────────────────────
// Feishu WebSocket long-connection: pbbp2.proto frame codec
// ─────────────────────────────────────────────────────────────────────────────

/// Protobuf header key-value pair
#[derive(Clone, PartialEq, ProstDeriveMessage)]
struct PbHeader {
    #[prost(string, tag = "1")]
    pub key: String,
    #[prost(string, tag = "2")]
    pub value: String,
}

/// Feishu WS frame (pbbp2.proto).
/// method=0 → CONTROL (ping/pong)  method=1 → DATA (events)
#[derive(Clone, PartialEq, ProstDeriveMessage)]
struct PbFrame {
    #[prost(uint64, tag = "1")]
    pub seq_id: u64,
    #[prost(uint64, tag = "2")]
    pub log_id: u64,
    #[prost(int32, tag = "3")]
    pub service: i32,
    #[prost(int32, tag = "4")]
    pub method: i32,
    #[prost(message, repeated, tag = "5")]
    pub headers: Vec<PbHeader>,
    #[prost(bytes = "vec", optional, tag = "8")]
    pub payload: Option<Vec<u8>>,
}

impl PbFrame {
    fn header_value<'a>(&'a self, key: &str) -> &'a str {
        self.headers
            .iter()
            .find(|h| h.key == key)
            .map(|h| h.value.as_str())
            .unwrap_or("")
    }
}

/// Server-sent client config (parsed from pong payload)
#[derive(Debug, Deserialize, Default, Clone)]
struct WsClientConfig {
    #[serde(rename = "PingInterval")]
    ping_interval: Option<u64>,
}

/// POST /callback/ws/endpoint response
#[derive(Debug, Deserialize)]
struct WsEndpointResp {
    code: i32,
    #[serde(default)]
    msg: Option<String>,
    #[serde(default)]
    data: Option<WsEndpoint>,
}

#[derive(Debug, Deserialize)]
struct WsEndpoint {
    #[serde(rename = "URL")]
    url: String,
    #[serde(rename = "ClientConfig")]
    client_config: Option<WsClientConfig>,
}

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

/// LarkEvent envelope (method=1 / type=event payload)
#[derive(Debug, Deserialize)]
struct LarkEvent {
    header: LarkEventHeader,
    event: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct LarkEventHeader {
    event_type: String,
    event_id: String,
}

#[derive(Debug, Deserialize)]
struct MsgReceivePayload {
    sender: LarkSender,
    message: LarkMessage,
}

#[derive(Debug, Deserialize)]
struct LarkSender {
    sender_id: LarkSenderId,
    #[serde(default)]
    sender_type: String,
}

#[derive(Debug, Deserialize, Default)]
struct LarkSenderId {
    open_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LarkMessage {
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
    running: Arc<AtomicBool>,
    /// Dedup map: event_key -> Instant (when seen)
    recent_event_keys: Arc<RwLock<HashMap<String, Instant>>>,
    /// Last cleanup time for dedup cache
    recent_event_cleanup_at: Arc<RwLock<Instant>>,
    http_client: reqwest::Client,
    token: Arc<RwLock<Option<String>>>,
    token_expiry: Arc<RwLock<Option<Instant>>>,
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
            running: Arc::new(AtomicBool::new(false)),
            recent_event_keys: Arc::new(RwLock::new(HashMap::new())),
            recent_event_cleanup_at: Arc::new(RwLock::new(Instant::now())),
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

    /// Generate dedup key from event_id and message_id
    fn dedupe_event_key(event_id: Option<&str>, message_id: Option<&str>) -> Option<String> {
        let normalized_event = event_id.map(str::trim).filter(|v| !v.is_empty());
        if let Some(event_id) = normalized_event {
            return Some(format!("event:{}", event_id));
        }
        let normalized_message = message_id.map(str::trim).filter(|v| !v.is_empty());
        normalized_message.map(|message_id| format!("message:{}", message_id))
    }

    /// Try to mark event key as seen, returns false if already seen
    async fn try_mark_event_key_seen(&self, dedupe_key: &str) -> bool {
        let now = Instant::now();
        if self.recent_event_keys.read().await.contains_key(dedupe_key) {
            return false;
        }

        let should_cleanup = {
            let last_cleanup = self.recent_event_cleanup_at.read().await;
            now.duration_since(*last_cleanup) >= EVENT_DEDUP_CLEANUP_INTERVAL
        };

        let mut seen = self.recent_event_keys.write().await;
        if seen.contains_key(dedupe_key) {
            return false;
        }

        if should_cleanup {
            seen.retain(|_, t| now.duration_since(*t) < EVENT_DEDUP_TTL);
            let mut last_cleanup = self.recent_event_cleanup_at.write().await;
            *last_cleanup = now;
        }

        seen.insert(dedupe_key.to_string(), now);
        true
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
    async fn get_websocket_url(&self) -> Result<(String, WsClientConfig)> {
        let url = format!("{}/callback/ws/endpoint", FEISHU_WS_BASE);

        let response = self
            .http_client
            .post(&url)
            .json(&json!({
                "AppID": self.config.app_id,
                "AppSecret": self.config.app_secret,
            }))
            .send()
            .await
            .map_err(|e| {
                ChannelError::ConnectionFailed(format!("WebSocket URL request failed: {}", e))
            })?;

        let ws_resp: WsEndpointResp = response.json().await.map_err(|e| {
            ChannelError::ConnectionFailed(format!("WebSocket URL parse failed: {}", e))
        })?;

        if ws_resp.code != 0 {
            return Err(ChannelError::ConnectionFailed(format!(
                "WebSocket URL request failed: {} - {}",
                ws_resp.code,
                ws_resp.msg.as_deref().unwrap_or("(none)")
            )));
        }

        let ws_info = ws_resp.data.ok_or_else(|| {
            ChannelError::ConnectionFailed("No WebSocket info in response".to_string())
        })?;

        let client_config = ws_info.client_config.unwrap_or_default();
        Ok((ws_info.url, client_config))
    }

    /// Run WebSocket connection with protobuf frames
    async fn run_websocket(
        &self,
        ws_url: String,
        client_config: WsClientConfig,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        // Extract service_id from URL query string
        let service_id = ws_url
            .split('?')
            .nth(1)
            .and_then(|qs| {
                qs.split('&')
                    .find(|kv| kv.starts_with("service_id="))
                    .and_then(|kv| kv.split('=').nth(1))
                    .and_then(|v| v.parse::<i32>().ok())
            })
            .unwrap_or(0);

        info!("Feishu WebSocket connecting to {}", ws_url);

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

            info!("Feishu WebSocket connected (service_id={})", service_id);
            let (mut write, mut read) = ws_stream.split();

            // Setup heartbeat
            let mut ping_secs = client_config.ping_interval.unwrap_or(120).max(10);
            let mut hb_interval = tokio::time::interval(Duration::from_secs(ping_secs));
            let mut timeout_check = tokio::time::interval(Duration::from_secs(10));
            hb_interval.tick().await; // consume immediate tick

            let mut seq: u64 = 0;
            let mut last_recv = Instant::now();

            // Send initial ping
            seq = seq.wrapping_add(1);
            let initial_ping = PbFrame {
                seq_id: seq,
                log_id: 0,
                service: service_id,
                method: 0,
                headers: vec![PbHeader {
                    key: "type".into(),
                    value: "ping".into(),
                }],
                payload: None,
            };
            if write
                .send(WsMessage::Binary(initial_ping.encode_to_vec()))
                .await
                .is_err()
            {
                error!("Feishu initial ping failed");
                sleep(Duration::from_secs(5)).await;
                continue;
            }

            // Fragment reassembly cache
            type FragEntry = (Vec<Option<Vec<u8>>>, Instant);
            let mut frag_cache: HashMap<String, FragEntry> = HashMap::new();

            // Message receiving loop
            let mut should_reconnect = true;
            loop {
                tokio::select! {
                    biased;

                    _ = hb_interval.tick() => {
                        seq = seq.wrapping_add(1);
                        let ping = PbFrame {
                            seq_id: seq, log_id: 0, service: service_id, method: 0,
                            headers: vec![PbHeader { key: "type".into(), value: "ping".into() }],
                            payload: None,
                        };
                        if write.send(WsMessage::Binary(ping.encode_to_vec())).await.is_err() {
                            warn!("Feishu ping failed, reconnecting");
                            break;
                        }
                        // GC stale fragments > 5 min
                        let cutoff = Instant::now().checked_sub(Duration::from_secs(300)).unwrap_or(Instant::now());
                        frag_cache.retain(|_, (_, ts)| *ts > cutoff);
                    }

                    _ = timeout_check.tick() => {
                        if last_recv.elapsed() > WS_HEARTBEAT_TIMEOUT {
                            warn!("Feishu heartbeat timeout, reconnecting");
                            break;
                        }
                    }

                    msg = read.next() => {
                        let raw = match msg {
                            Some(Ok(WsMessage::Binary(b))) => {
                                last_recv = Instant::now();
                                b
                            }
                            Some(Ok(WsMessage::Ping(d))) => {
                                let _ = write.send(WsMessage::Pong(d)).await;
                                continue;
                            }
                            Some(Ok(WsMessage::Close(_))) => {
                                info!("Feishu WebSocket closed — reconnecting");
                                break;
                            }
                            Some(Ok(_)) => continue,
                            None => {
                                info!("Feishu WebSocket closed — reconnecting");
                                break;
                            }
                            Some(Err(e)) => {
                                error!("Feishu WebSocket read error: {}", e);
                                break;
                            }
                        };

                        let frame = match PbFrame::decode(&raw[..]) {
                            Ok(f) => f,
                            Err(e) => {
                                error!("Feishu proto decode error: {}", e);
                                continue;
                            }
                        };

                        // CONTROL frame (method=0)
                        if frame.method == 0 {
                            if frame.header_value("type") == "pong" {
                                if let Some(p) = &frame.payload {
                                    if let Ok(cfg) = serde_json::from_slice::<WsClientConfig>(p) {
                                        if let Some(secs) = cfg.ping_interval {
                                            let secs = secs.max(10);
                                            if secs != ping_secs {
                                                ping_secs = secs;
                                                hb_interval = tokio::time::interval(Duration::from_secs(ping_secs));
                                                info!("Feishu ping_interval → {}s", ping_secs);
                                            }
                                        }
                                    }
                                }
                            }
                            continue;
                        }

                        // DATA frame (method=1)
                        let msg_type = frame.header_value("type").to_string();
                        let msg_id = frame.header_value("message_id").to_string();
                        let sum = frame.header_value("sum").parse::<usize>().unwrap_or(1);
                        let seq_num = frame.header_value("seq").parse::<usize>().unwrap_or(0);

                        // ACK immediately (Feishu requires within 3s)
                        {
                            let mut ack = frame.clone();
                            ack.payload = Some(br#"{"code":200,"headers":{},"data":[]}"#.to_vec());
                            ack.headers.push(PbHeader { key: "biz_rt".into(), value: "0".into() });
                            let _ = write.send(WsMessage::Binary(ack.encode_to_vec())).await;
                        }

                        // Fragment reassembly
                        let sum = if sum == 0 { 1 } else { sum };
                        let payload: Vec<u8> = if sum == 1 || msg_id.is_empty() || seq_num >= sum {
                            frame.payload.clone().unwrap_or_default()
                        } else {
                            let entry = frag_cache.entry(msg_id.clone())
                                .or_insert_with(|| (vec![None; sum], Instant::now()));
                            if entry.0.len() != sum { *entry = (vec![None; sum], Instant::now()); }
                            entry.0[seq_num] = frame.payload.clone();
                            if entry.0.iter().all(|s| s.is_some()) {
                                let full: Vec<u8> = entry.0.iter()
                                    .flat_map(|s| s.as_deref().unwrap_or(&[]))
                                    .copied().collect();
                                frag_cache.remove(&msg_id);
                                full
                            } else { continue; }
                        };

                        if msg_type != "event" { continue; }

                        if let Err(e) = self.handle_protobuf_event(&payload).await {
                            warn!("Error handling protobuf event: {}", e);
                        }
                    }

                    _ = shutdown_rx.recv() => {
                        info!("Feishu WebSocket received shutdown signal");
                        should_reconnect = false;
                        break;
                    }
                }

                if !should_reconnect {
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

    /// Handle protobuf-encoded event
    async fn handle_protobuf_event(&self, payload: &[u8]) -> Result<()> {
        let event: LarkEvent = serde_json::from_slice(payload)
            .map_err(|e| ChannelError::Error(format!("Failed to parse event: {}", e)))?;

        // Only handle message receive events
        if event.header.event_type != "im.message.receive_v1" {
            debug!("Ignoring event type: {}", event.header.event_type);
            return Ok(());
        }

        // Deduplication check
        if let Some(dedupe_key) = Self::dedupe_event_key(
            Some(&event.header.event_id),
            None, // message_id will be extracted from event payload
        ) {
            if !self.try_mark_event_key_seen(&dedupe_key).await {
                debug!("Feishu: duplicate event dropped ({})", dedupe_key);
                return Ok(());
            }
        }

        let event_payload = event.event;

        let recv: MsgReceivePayload = match serde_json::from_value(event_payload.clone()) {
            Ok(r) => r,
            Err(e) => {
                warn!("Feishu: payload parse error: {}", e);
                return Ok(());
            }
        };

        // Skip bot messages
        if recv.sender.sender_type == "app" || recv.sender.sender_type == "bot" {
            debug!("Skipping bot message");
            return Ok(());
        }

        let sender_open_id = recv.sender.sender_id.open_id.as_deref().unwrap_or("");
        if !self.is_allowed(sender_open_id) {
            warn!("Feishu WS: ignoring {} (not in allow_from)", sender_open_id);
            return Ok(());
        }

        let lark_msg = &recv.message;

        // Dedup with message_id as fallback
        if let Some(dedupe_key) = Self::dedupe_event_key(None, Some(&lark_msg.message_id)) {
            if !self.try_mark_event_key_seen(&dedupe_key).await {
                debug!("Feishu: duplicate message dropped ({})", dedupe_key);
                return Ok(());
            }
        }

        // Add reaction to indicate "seen"
        let _ = self.add_reaction(&lark_msg.message_id, "THUMBSUP").await;

        // Parse message content
        let content = if lark_msg.message_type == "text" {
            match serde_json::from_str::<Value>(&lark_msg.content) {
                Ok(v) => v
                    .get("text")
                    .and_then(|t| t.as_str())
                    .unwrap_or("")
                    .to_string(),
                Err(_) => lark_msg.content.clone(),
            }
        } else if lark_msg.message_type == "image" {
            // Try to fetch image
            match self
                .fetch_image_marker(&lark_msg.message_id, &lark_msg.content)
                .await
            {
                Ok(marker) => marker,
                Err(e) => {
                    warn!("Failed to fetch image: {}", e);
                    "[image]".to_string()
                }
            }
        } else {
            MSG_TYPE_MAP
                .iter()
                .find(|(k, _)| *k == lark_msg.message_type)
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| format!("[{}]", lark_msg.message_type))
        };

        if content.is_empty() {
            return Ok(());
        }

        // Forward to message bus
        let reply_to = if lark_msg.chat_type == "group" {
            lark_msg.chat_id.clone()
        } else {
            sender_open_id.to_string()
        };

        let inbound_msg = agent_diva_core::bus::InboundMessage::new(
            "feishu",
            sender_open_id.to_string(),
            reply_to,
            content,
        )
        .with_metadata("message_id", json!(lark_msg.message_id))
        .with_metadata("chat_type", json!(lark_msg.chat_type))
        .with_metadata("msg_type", json!(lark_msg.message_type));

        if let Some(tx) = &self.inbound_tx {
            if let Err(e) = tx.send(inbound_msg).await {
                error!("Failed to send message to bus: {}", e);
            }
        }

        Ok(())
    }

    /// Fetch image and convert to data URI marker
    async fn fetch_image_marker(&self, message_id: &str, content: &str) -> Result<String> {
        // Parse image_key from content
        let image_key = match serde_json::from_str::<Value>(content) {
            Ok(v) => v
                .get("image_key")
                .and_then(|k| k.as_str())
                .unwrap_or("")
                .to_string(),
            Err(_) => content.to_string(),
        };

        if image_key.is_empty() {
            return Err(ChannelError::Error("Empty image_key".to_string()));
        }

        let token = self.get_access_token().await?;
        let url = format!(
            "{}/im/v1/messages/{}/resources/{}",
            FEISHU_API_BASE, message_id, image_key
        );

        let response = self
            .http_client
            .get(&url)
            .query(&[("type", "image")])
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| ChannelError::ApiError(format!("Image download failed: {}", e)))?;

        let status = response.status();
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        let body = response
            .bytes()
            .await
            .map_err(|e| ChannelError::ApiError(format!("Image read failed: {}", e)))?;

        if !status.is_success() {
            return Err(ChannelError::ApiError(format!(
                "Image download failed: {}",
                status
            )));
        }

        if body.is_empty() {
            return Err(ChannelError::Error("Image payload is empty".to_string()));
        }

        let media_type = content_type
            .as_deref()
            .and_then(|v| v.split(';').next())
            .map(|s| s.trim())
            .filter(|v| v.starts_with("image/"))
            .unwrap_or("image/png");

        let encoded = base64::engine::general_purpose::STANDARD.encode(&body);
        Ok(format!("[IMAGE:data:{};base64,{}]", media_type, encoded))
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
        self.running.load(Ordering::Acquire)
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
        let (ws_url, client_config) = self.get_websocket_url().await?;
        info!("Feishu WebSocket URL obtained");

        self.running.store(true, Ordering::Release);

        // Start WebSocket connection
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let handler = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = handler
                .run_websocket(ws_url, client_config, shutdown_rx)
                .await
            {
                error!("Feishu WebSocket task failed: {}", e);
            }
        });

        info!("Feishu channel started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        self.running.store(false, Ordering::Release);

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        info!("Feishu channel stopped");
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        if !self.running.load(Ordering::Acquire) {
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
            recent_event_keys: Arc::clone(&self.recent_event_keys),
            recent_event_cleanup_at: Arc::clone(&self.recent_event_cleanup_at),
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
    async fn test_dedupe_event_key() {
        let mut feishu_config = FeishuConfig::default();
        feishu_config.enabled = true;
        feishu_config.app_id = "test_app_id".to_string();
        feishu_config.app_secret = "test_secret".to_string();

        let config = Config::default();
        let handler = FeishuHandler::new(feishu_config, config);

        // Test event_id priority
        let key1 = FeishuHandler::dedupe_event_key(Some("event_123"), Some("msg_456"));
        assert_eq!(key1, Some("event:event_123".to_string()));

        // Test message_id fallback
        let key2 = FeishuHandler::dedupe_event_key(None, Some("msg_456"));
        assert_eq!(key2, Some("message:msg_456".to_string()));

        // Test both None
        let key3 = FeishuHandler::dedupe_event_key(None, None);
        assert_eq!(key3, None);
    }

    #[tokio::test]
    async fn test_try_mark_event_key_seen() {
        let mut feishu_config = FeishuConfig::default();
        feishu_config.enabled = true;
        feishu_config.app_id = "test_app_id".to_string();
        feishu_config.app_secret = "test_secret".to_string();

        let config = Config::default();
        let handler = FeishuHandler::new(feishu_config, config);

        // First time should succeed
        assert!(handler.try_mark_event_key_seen("event:123").await);

        // Second time should fail (duplicate)
        assert!(!handler.try_mark_event_key_seen("event:123").await);

        // Different key should succeed
        assert!(handler.try_mark_event_key_seen("event:456").await);
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
