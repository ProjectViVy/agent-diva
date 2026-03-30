//! Discord channel integration using Gateway WebSocket
//!
//! Implements Discord bot functionality using the Discord Gateway API
//! for real-time message receiving and REST API for sending messages.

use crate::base::{BaseChannel, ChannelError, ChannelHandler, Result};
use crate::common::{create_http_client, download_file};
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::{Config, DiscordConfig};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tokio_tungstenite::tungstenite::Message as WsMessage;

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";
const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024; // 20MB
/// Discord's maximum message length for regular messages.
const DISCORD_MAX_MESSAGE_LENGTH: usize = 2000;

const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Discord Gateway message opcodes
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
enum GatewayOp {
    Dispatch = 0,
    Heartbeat = 1,
    Identify = 2,
    PresenceUpdate = 3,
    VoiceStateUpdate = 4,
    Resume = 6,
    Reconnect = 7,
    RequestGuildMembers = 8,
    InvalidSession = 9,
    Hello = 10,
    HeartbeatAck = 11,
}

impl GatewayOp {
    fn from_u8(op: u8) -> Option<Self> {
        match op {
            0 => Some(GatewayOp::Dispatch),
            1 => Some(GatewayOp::Heartbeat),
            2 => Some(GatewayOp::Identify),
            3 => Some(GatewayOp::PresenceUpdate),
            4 => Some(GatewayOp::VoiceStateUpdate),
            6 => Some(GatewayOp::Resume),
            7 => Some(GatewayOp::Reconnect),
            8 => Some(GatewayOp::RequestGuildMembers),
            9 => Some(GatewayOp::InvalidSession),
            10 => Some(GatewayOp::Hello),
            11 => Some(GatewayOp::HeartbeatAck),
            _ => None,
        }
    }
}

/// Discord Gateway payload
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GatewayPayload {
    op: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    d: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    s: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    t: Option<String>,
}

/// Discord message author
#[derive(Debug, Clone, Deserialize)]
struct DiscordAuthor {
    id: String,
    username: String,
    #[serde(default)]
    bot: bool,
}

/// Discord attachment
#[derive(Debug, Clone, Deserialize)]
struct DiscordAttachment {
    id: String,
    filename: String,
    url: String,
    #[serde(default)]
    size: usize,
}

/// Discord message
#[derive(Debug, Clone, Deserialize)]
struct DiscordMessage {
    id: String,
    channel_id: String,
    #[serde(default)]
    content: String,
    author: DiscordAuthor,
    #[serde(default)]
    attachments: Vec<DiscordAttachment>,
    #[serde(rename = "referenced_message")]
    #[serde(default)]
    reply_to: Option<Box<DiscordMessage>>,
    #[serde(default)]
    guild_id: Option<String>,
}

/// Discord channel handler
pub struct DiscordHandler {
    config: DiscordConfig,
    base: BaseChannel,
    /// Running state (shared across tasks)
    running: Arc<AtomicBool>,
    /// Inbound message sender
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    /// HTTP client
    http: reqwest::Client,
    /// Sequence number for heartbeats
    seq: Arc<Mutex<Option<u64>>>,
    /// Typing indicator tasks
    typing_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    /// Session ID for resuming
    session_id: Arc<Mutex<Option<String>>>,
    /// Shutdown signal sender
    shutdown_tx: Option<mpsc::Sender<()>>,
    /// Bot application user id (first segment of token, base64)
    bot_user_id: String,
    /// Normalized `group_reply_allowed_sender_ids` from config
    group_reply_allowed_sender_ids: Vec<String>,
}

fn normalize_group_reply_allowed_sender_ids(sender_ids: Vec<String>) -> Vec<String> {
    let mut normalized = sender_ids
        .into_iter()
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

#[allow(clippy::cast_possible_truncation)]
fn base64_decode(input: &str) -> Option<String> {
    let padded = match input.len() % 4 {
        2 => format!("{input}=="),
        3 => format!("{input}="),
        _ => input.to_string(),
    };

    let mut bytes = Vec::new();
    let chars: Vec<u8> = padded.bytes().collect();

    for chunk in chars.chunks(4) {
        if chunk.len() < 4 {
            break;
        }

        let mut v = [0usize; 4];
        for (i, &b) in chunk.iter().enumerate() {
            if b == b'=' {
                v[i] = 0;
            } else {
                v[i] = BASE64_ALPHABET.iter().position(|&a| a == b)?;
            }
        }

        bytes.push(((v[0] << 2) | (v[1] >> 4)) as u8);
        if chunk[2] != b'=' {
            bytes.push((((v[1] & 0xF) << 4) | (v[2] >> 2)) as u8);
        }
        if chunk[3] != b'=' {
            bytes.push((((v[2] & 0x3) << 6) | v[3]) as u8);
        }
    }

    String::from_utf8(bytes).ok()
}

fn bot_user_id_from_token(token: &str) -> Option<String> {
    let part = token.split('.').next()?;
    base64_decode(part)
}

fn mention_tags(bot_user_id: &str) -> [String; 2] {
    [format!("<@{bot_user_id}>"), format!("<@!{bot_user_id}>")]
}

fn contains_bot_mention(content: &str, bot_user_id: &str) -> bool {
    let tags = mention_tags(bot_user_id);
    content.contains(&tags[0]) || content.contains(&tags[1])
}

fn normalize_incoming_content(
    content: &str,
    require_mention: bool,
    bot_user_id: &str,
) -> Option<String> {
    if content.is_empty() {
        return None;
    }

    if require_mention && !contains_bot_mention(content, bot_user_id) {
        return None;
    }

    let mut normalized = content.to_string();
    if require_mention {
        for tag in mention_tags(bot_user_id) {
            normalized = normalized.replace(&tag, " ");
        }
    }

    let normalized = normalized.trim().to_string();
    if normalized.is_empty() {
        return None;
    }

    Some(normalized)
}

fn heartbeat_d_value(seq: Option<u64>) -> serde_json::Value {
    match seq {
        Some(s) => serde_json::Value::from(s),
        None => serde_json::Value::Null,
    }
}

/// Append Discord Gateway v10 JSON query to the URL returned by `GET /gateway/bot`.
fn discord_ws_url_from_api_base(base_url: &str) -> String {
    let base = base_url.trim_end_matches('/');
    format!("{base}/?v=10&encoding=json")
}

/// Split a message into chunks that respect Discord's 2000-character limit.
fn split_message_for_discord(message: &str) -> Vec<String> {
    if message.chars().count() <= DISCORD_MAX_MESSAGE_LENGTH {
        return vec![message.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = message;

    while !remaining.is_empty() {
        let hard_split = remaining
            .char_indices()
            .nth(DISCORD_MAX_MESSAGE_LENGTH)
            .map_or(remaining.len(), |(idx, _)| idx);

        let chunk_end = if hard_split == remaining.len() {
            hard_split
        } else {
            let search_area = &remaining[..hard_split];

            if let Some(pos) = search_area.rfind('\n') {
                if search_area[..pos].chars().count() >= DISCORD_MAX_MESSAGE_LENGTH / 2 {
                    pos + 1
                } else {
                    search_area.rfind(' ').map_or(hard_split, |space| space + 1)
                }
            } else if let Some(pos) = search_area.rfind(' ') {
                pos + 1
            } else {
                hard_split
            }
        };

        chunks.push(remaining[..chunk_end].to_string());
        remaining = &remaining[chunk_end..];
    }

    chunks
}

fn resolve_reply_message_id(message: &OutboundMessage) -> Option<String> {
    message
        .reply_to
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .or_else(|| {
            message
                .metadata
                .get("reply_to")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        })
}

impl DiscordHandler {
    /// Create a new Discord handler from config
    pub fn new(config: &DiscordConfig, base_config: Config) -> Self {
        let allow_from = config.allow_from.clone();
        let base = BaseChannel::new("discord", base_config, allow_from);
        let bot_user_id = bot_user_id_from_token(&config.token).unwrap_or_default();
        let group_reply_allowed_sender_ids =
            normalize_group_reply_allowed_sender_ids(config.group_reply_allowed_sender_ids.clone());

        Self {
            config: config.clone(),
            base,
            running: Arc::new(AtomicBool::new(false)),
            inbound_tx: None,
            http: create_http_client().expect("Failed to create HTTP client"),
            seq: Arc::new(Mutex::new(None)),
            typing_tasks: Arc::new(Mutex::new(HashMap::new())),
            session_id: Arc::new(Mutex::new(None)),
            shutdown_tx: None,
            bot_user_id,
            group_reply_allowed_sender_ids,
        }
    }

    fn is_group_sender_trigger_enabled(&self, sender_id: &str) -> bool {
        let sender_id = sender_id.trim();
        if sender_id.is_empty() {
            return false;
        }
        self.group_reply_allowed_sender_ids
            .iter()
            .any(|entry| entry == "*" || entry == sender_id)
    }

    /// Check if a sender is allowed
    fn is_allowed(&self, sender_id: &str) -> bool {
        self.base.is_allowed(sender_id)
    }

    /// Handle incoming Discord message
    async fn handle_message_create(&self, payload: serde_json::Value) -> Result<()> {
        let msg: DiscordMessage = serde_json::from_value(payload)
            .map_err(|e| ChannelError::Error(format!("Failed to parse message: {}", e)))?;

        let author_id = msg.author.id.as_str();

        if author_id == self.bot_user_id {
            return Ok(());
        }

        if msg.author.bot && !self.config.listen_to_bots {
            return Ok(());
        }

        let sender_id = msg.author.id.clone();
        let channel_id = msg.channel_id.clone();

        if !self.is_allowed(&sender_id) {
            tracing::warn!(
                "Access denied for sender {} on channel {}",
                sender_id,
                self.base.name
            );
            return Ok(());
        }

        if let Some(ref expected_gid) = self.config.guild_id {
            if let Some(ref g) = msg.guild_id {
                if g != expected_gid {
                    return Ok(());
                }
            }
        }

        let is_group_message = msg.guild_id.is_some();
        let allow_sender_without_mention =
            is_group_message && self.is_group_sender_trigger_enabled(&sender_id);
        let require_mention =
            self.config.mention_only && is_group_message && !allow_sender_without_mention;
        let Some(text_content) =
            normalize_incoming_content(&msg.content, require_mention, &self.bot_user_id)
        else {
            return Ok(());
        };

        // Build content with attachments
        let mut content_parts = Vec::new();
        if !text_content.is_empty() {
            content_parts.push(text_content);
        }

        // Download attachments
        for attachment in &msg.attachments {
            if attachment.size > MAX_ATTACHMENT_BYTES {
                content_parts.push(format!("[attachment: {} - too large]", attachment.filename));
                continue;
            }

            match download_file(
                &self.http,
                &attachment.url,
                &attachment.filename,
                &attachment.id,
            )
            .await
            {
                Ok(path) => {
                    content_parts.push(format!("[attachment: {}]", path));
                }
                Err(e) => {
                    tracing::warn!("Failed to download attachment: {}", e);
                    content_parts.push(format!(
                        "[attachment: {} - download failed]",
                        attachment.filename
                    ));
                }
            }
        }

        let content = if content_parts.is_empty() {
            "[empty message]".to_string()
        } else {
            content_parts.join("\n")
        };

        self.start_typing(channel_id.clone()).await;

        if let Some(tx) = &self.inbound_tx {
            let reply_to = msg.reply_to.as_ref().map(|m| m.id.clone());
            let inbound_msg = InboundMessage::new(
                self.base.name.clone(),
                sender_id,
                channel_id.clone(),
                content,
            )
            .with_metadata("message_id", msg.id)
            .with_metadata("username", msg.author.username)
            .with_metadata("guild_id", msg.guild_id.unwrap_or_default())
            .with_metadata("reply_to", reply_to.unwrap_or_default());

            tx.send(inbound_msg)
                .await
                .map_err(|e| ChannelError::SendError(e.to_string()))?;
        }

        Ok(())
    }

    /// Start typing indicator
    async fn start_typing(&self, channel_id: String) {
        self.stop_typing(&channel_id).await;

        let token = self.config.token.clone();
        let http = self.http.clone();
        let url = format!("{}/channels/{}/typing", DISCORD_API_BASE, channel_id);

        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(8));
            loop {
                let _ = http
                    .post(&url)
                    .header("Authorization", format!("Bot {}", token))
                    .send()
                    .await;
                ticker.tick().await;
            }
        });

        let mut tasks = self.typing_tasks.lock().await;
        tasks.insert(channel_id, handle);
    }

    /// Stop typing indicator
    async fn stop_typing(&self, channel_id: &str) {
        let mut tasks = self.typing_tasks.lock().await;
        if let Some(handle) = tasks.remove(channel_id) {
            handle.abort();
        }
    }

    /// Clone for async task
    fn clone_for_task(&self) -> Self {
        Self {
            config: self.config.clone(),
            base: BaseChannel::new(
                self.base.name.clone(),
                self.base.config.clone(),
                self.base.allow_from.clone(),
            ),
            running: Arc::clone(&self.running),
            inbound_tx: self.inbound_tx.clone(),
            http: self.http.clone(),
            seq: Arc::clone(&self.seq),
            typing_tasks: Arc::clone(&self.typing_tasks),
            session_id: Arc::clone(&self.session_id),
            shutdown_tx: None,
            bot_user_id: self.bot_user_id.clone(),
            group_reply_allowed_sender_ids: self.group_reply_allowed_sender_ids.clone(),
        }
    }

    async fn fetch_gateway_ws_url(&self) -> String {
        let url = format!("{DISCORD_API_BASE}/gateway/bot");
        match self
            .http
            .get(&url)
            .header("Authorization", format!("Bot {}", self.config.token))
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                if let Ok(v) = resp.json::<serde_json::Value>().await {
                    if let Some(base) = v.get("url").and_then(|u| u.as_str()) {
                        return discord_ws_url_from_api_base(base);
                    }
                }
            }
            Ok(resp) => {
                tracing::warn!(
                    status = %resp.status(),
                    "Discord gateway/bot request failed, using configured gateway_url"
                );
            }
            Err(e) => {
                tracing::warn!(error = %e, "Discord gateway/bot request error, using configured gateway_url");
            }
        }
        self.config.gateway_url.clone()
    }

    /// Run the gateway connection
    async fn run_gateway(&self, mut shutdown_rx: mpsc::Receiver<()>) -> Result<()> {
        let mut reconnect_delay = 5;

        loop {
            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            tracing::info!("Connecting to Discord gateway...");
            let ws_url = self.fetch_gateway_ws_url().await;

            match tokio_tungstenite::connect_async(ws_url.as_str()).await {
                Ok((ws_stream, _)) => {
                    tracing::info!("Connected to Discord gateway");
                    reconnect_delay = 5;

                    let (mut write, mut read) = ws_stream.split();

                    let (tx, mut rx) = mpsc::channel::<String>(32);

                    let writer_handle = tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                            if let Err(e) = write.send(WsMessage::Text(msg)).await {
                                tracing::warn!("WebSocket write failed: {}", e);
                                break;
                            }
                        }
                    });

                    loop {
                        tokio::select! {
                            _ = shutdown_rx.recv() => {
                                tracing::info!("Shutdown signal received");
                                break;
                            }
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(WsMessage::Text(text))) => {
                                        if let Err(e) = self.handle_gateway_message(&text, &tx).await {
                                            tracing::error!("Error handling gateway message: {}", e);
                                        }
                                    }
                                    Some(Ok(WsMessage::Close(_))) => {
                                        tracing::warn!("Discord WebSocket closed");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        tracing::error!("Discord WebSocket error: {}", e);
                                        break;
                                    }
                                    None => {
                                        tracing::warn!("Discord WebSocket stream ended");
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }

                    writer_handle.abort();
                }
                Err(e) => {
                    tracing::warn!("Discord connection failed: {}", e);
                }
            }

            if !self.running.load(Ordering::Acquire) {
                break;
            }

            tracing::info!("Reconnecting to Discord in {} seconds...", reconnect_delay);
            tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
            reconnect_delay = (reconnect_delay * 2).min(60);
        }

        Ok(())
    }

    async fn handle_gateway_message(&self, text: &str, tx: &mpsc::Sender<String>) -> Result<()> {
        let payload: GatewayPayload = serde_json::from_str(text)
            .map_err(|e| ChannelError::Error(format!("Failed to parse payload: {}", e)))?;

        if let Some(s) = payload.s {
            *self.seq.lock().await = Some(s);
        }

        match GatewayOp::from_u8(payload.op) {
            Some(GatewayOp::Hello) => {
                if let Some(d) = payload.d {
                    let interval_ms = d
                        .get("heartbeat_interval")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(45000);

                    let tx_hb = tx.clone();
                    let seq_hb = self.seq.clone();
                    tokio::spawn(async move {
                        let mut tick = interval(Duration::from_millis(interval_ms));
                        tick.tick().await;

                        loop {
                            tick.tick().await;
                            let seq = *seq_hb.lock().await;
                            let heartbeat = json!({
                                "op": 1,
                                "d": heartbeat_d_value(seq)
                            });
                            if tx_hb.send(heartbeat.to_string()).await.is_err() {
                                break;
                            }
                        }
                    });

                    let identify = json!({
                        "op": 2,
                        "d": {
                            "token": self.config.token,
                            "intents": self.config.intents,
                            "properties": {
                                "os": "agent-diva",
                                "browser": "agent-diva",
                                "device": "agent-diva"
                            }
                        }
                    });
                    tx.send(identify.to_string())
                        .await
                        .map_err(|e| ChannelError::Error(e.to_string()))?;
                }
            }
            Some(GatewayOp::Heartbeat) => {
                let seq = *self.seq.lock().await;
                let heartbeat = json!({
                    "op": 1,
                    "d": heartbeat_d_value(seq)
                });
                tx.send(heartbeat.to_string())
                    .await
                    .map_err(|e| ChannelError::Error(e.to_string()))?;
            }
            Some(GatewayOp::Dispatch) => {
                if payload.t.as_deref() == Some("MESSAGE_CREATE") {
                    if let Some(d) = payload.d {
                        self.handle_message_create(d).await?;
                    }
                } else if payload.t.as_deref() == Some("READY") {
                    tracing::info!("Discord gateway READY");
                    if let Some(d) = payload.d {
                        if let Some(session_id) = d.get("session_id").and_then(|v| v.as_str()) {
                            *self.session_id.lock().await = Some(session_id.to_string());
                        }
                    }
                }
            }
            Some(GatewayOp::Reconnect) => {
                tracing::info!("Discord requested reconnect");
                return Err(ChannelError::ConnectionError(
                    "Reconnect requested".to_string(),
                ));
            }
            Some(GatewayOp::InvalidSession) => {
                tracing::warn!("Discord invalid session");
                *self.session_id.lock().await = None;
                return Err(ChannelError::ConnectionError("Invalid session".to_string()));
            }
            _ => {}
        }
        Ok(())
    }

    async fn post_message_with_retries(
        &self,
        url: &str,
        payload: &serde_json::Value,
    ) -> Result<()> {
        for attempt in 0..3 {
            let response = self
                .http
                .post(url)
                .header("Authorization", format!("Bot {}", self.config.token))
                .header("Content-Type", "application/json")
                .json(payload)
                .send()
                .await
                .map_err(|e| ChannelError::ApiError(format!("Request failed: {}", e)))?;

            let status = response.status();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                let retry_after = response
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<f64>().ok())
                    .unwrap_or(1.0);
                tracing::warn!("Discord rate limited, retrying in {}s", retry_after);
                tokio::time::sleep(Duration::from_secs_f64(retry_after)).await;
                continue;
            }

            if status.is_success() {
                return Ok(());
            }

            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());

            if attempt == 2 {
                return Err(ChannelError::ApiError(format!(
                    "Discord API error: {} - {}",
                    status, error_text
                )));
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        Ok(())
    }
}

#[async_trait]
impl ChannelHandler for DiscordHandler {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::Acquire)
    }

    async fn start(&mut self) -> Result<()> {
        if self.config.token.is_empty() {
            return Err(ChannelError::NotConfigured(
                "Discord token not configured".to_string(),
            ));
        }

        if self.running.load(Ordering::Acquire) {
            return Ok(());
        }

        tracing::info!("Starting Discord bot...");
        self.running.store(true, Ordering::Release);

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let handler = self.clone_for_task();
        tokio::spawn(async move {
            if let Err(e) = handler.run_gateway(shutdown_rx).await {
                tracing::error!("Discord gateway task failed: {}", e);
            }
        });

        tracing::info!("Discord bot started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.running.load(Ordering::Acquire) {
            return Ok(());
        }

        tracing::info!("Stopping Discord bot...");
        self.running.store(false, Ordering::Release);

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        let mut tasks = self.typing_tasks.lock().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }

        tracing::info!("Discord bot stopped");
        Ok(())
    }

    async fn send(&self, message: OutboundMessage) -> Result<()> {
        self.stop_typing(&message.chat_id).await;

        let url = format!("{}/channels/{}/messages", DISCORD_API_BASE, message.chat_id);
        let reply_id = resolve_reply_message_id(&message);
        let chunks = split_message_for_discord(&message.content);

        for (i, chunk) in chunks.iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(Duration::from_millis(500)).await;
            }

            let mut payload = json!({ "content": chunk });
            if i == 0 {
                if let Some(ref rid) = reply_id {
                    if !rid.is_empty() {
                        payload["message_reference"] = json!({ "message_id": rid });
                        payload["allowed_mentions"] = json!({ "replied_user": false });
                    }
                }
            }

            self.post_message_with_retries(&url, &payload).await?;
        }

        Ok(())
    }

    fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.base.is_allowed(sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_discord_config() -> DiscordConfig {
        DiscordConfig {
            enabled: true,
            token: "test_token".to_string(),
            allow_from: vec!["user1".to_string()],
            gateway_url: "wss://gateway.discord.gg".to_string(),
            intents: 37377,
            ..Default::default()
        }
    }

    #[test]
    fn test_discord_handler_new() {
        let config = sample_discord_config();
        let global_config = Config::default();

        let handler = DiscordHandler::new(&config, global_config);
        assert_eq!(handler.base.name, "discord");
        assert_eq!(handler.config.token, "test_token");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_gateway_op_from_u8() {
        assert_eq!(GatewayOp::from_u8(0), Some(GatewayOp::Dispatch));
        assert_eq!(GatewayOp::from_u8(1), Some(GatewayOp::Heartbeat));
        assert_eq!(GatewayOp::from_u8(10), Some(GatewayOp::Hello));
        assert_eq!(GatewayOp::from_u8(255), None);
    }

    #[test]
    fn heartbeat_d_value_null_without_seq() {
        assert_eq!(heartbeat_d_value(None), serde_json::Value::Null);
    }

    #[test]
    fn heartbeat_d_value_numeric_with_seq() {
        assert_eq!(heartbeat_d_value(Some(42u64)), json!(42));
    }

    #[test]
    fn base64_decode_bot_id() {
        let decoded = base64_decode("MTIzNDU2");
        assert_eq!(decoded, Some("123456".to_string()));
    }

    #[test]
    fn bot_user_id_extraction() {
        let token = "MTIzNDU2.fake.hmac";
        let id = bot_user_id_from_token(token);
        assert_eq!(id, Some("123456".to_string()));
    }

    #[test]
    fn resolve_reply_prefers_outbound_reply_to() {
        let mut m = OutboundMessage::new("discord", "ch1", "hi");
        m.reply_to = Some("snow1".to_string());
        assert_eq!(resolve_reply_message_id(&m).as_deref(), Some("snow1"));
    }

    #[test]
    fn resolve_reply_falls_back_to_metadata() {
        let mut m = OutboundMessage::new("discord", "ch1", "hi");
        m.metadata.insert("reply_to".to_string(), json!("snow2"));
        assert_eq!(resolve_reply_message_id(&m).as_deref(), Some("snow2"));
    }

    #[test]
    fn split_message_under_limit() {
        let msg = "Hello, world!";
        let chunks = split_message_for_discord(msg);
        assert_eq!(chunks, vec![msg]);
    }

    #[test]
    fn split_message_just_over_limit() {
        let msg = "a".repeat(DISCORD_MAX_MESSAGE_LENGTH + 1);
        let chunks = split_message_for_discord(&msg);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chars().count(), DISCORD_MAX_MESSAGE_LENGTH);
        assert_eq!(chunks[1].chars().count(), 1);
    }

    #[test]
    fn normalize_group_reply_allowed_sender_ids_dedupes() {
        let n = normalize_group_reply_allowed_sender_ids(vec![
            " 111 ".to_string(),
            "111".to_string(),
            "222".to_string(),
        ]);
        assert_eq!(n, vec!["111".to_string(), "222".to_string()]);
    }

    #[test]
    fn discord_ws_url_from_api_base_appends_query() {
        assert_eq!(
            discord_ws_url_from_api_base("wss://gateway.discord.gg"),
            "wss://gateway.discord.gg/?v=10&encoding=json"
        );
        assert_eq!(
            discord_ws_url_from_api_base("wss://gateway.discord.gg/"),
            "wss://gateway.discord.gg/?v=10&encoding=json"
        );
    }

    #[test]
    fn normalize_incoming_strips_mention_when_required() {
        let cleaned = normalize_incoming_content("  <@!12345> hi  ", true, "12345");
        assert_eq!(cleaned.as_deref(), Some("hi"));
    }
}
