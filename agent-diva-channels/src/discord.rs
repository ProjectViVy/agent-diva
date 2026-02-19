//! Discord channel integration using Gateway WebSocket
//!
//! Implements Discord bot functionality using the Discord Gateway API
//! for real-time message receiving and REST API for sending messages.

use crate::base::{ChannelError, ChannelHandler, Result};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::DiscordConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";
const MAX_ATTACHMENT_BYTES: usize = 20 * 1024 * 1024; // 20MB

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
    /// Channel name
    name: String,
    /// Bot token
    token: String,
    /// Allowed senders
    allow_from: Vec<String>,
    /// Gateway URL
    gateway_url: String,
    /// Gateway intents
    intents: u64,
    /// Running state
    running: bool,
    /// Inbound message sender
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    /// WebSocket connection
    ws: Option<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
    /// HTTP client
    http: reqwest::Client,
    /// Sequence number for heartbeats
    seq: Arc<Mutex<Option<u64>>>,
    /// Heartbeat task
    heartbeat_task: Option<JoinHandle<()>>,
    /// Typing indicator tasks
    typing_tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    /// Session ID for resuming
    session_id: Arc<Mutex<Option<String>>>,
}

impl DiscordHandler {
    /// Create a new Discord handler from config
    pub fn new(config: &DiscordConfig) -> Self {
        Self {
            name: "discord".to_string(),
            token: config.token.clone(),
            allow_from: config.allow_from.clone(),
            gateway_url: config.gateway_url.clone(),
            intents: config.intents,
            running: false,
            inbound_tx: None,
            ws: None,
            http: reqwest::Client::new(),
            seq: Arc::new(Mutex::new(None)),
            heartbeat_task: None,
            typing_tasks: Arc::new(Mutex::new(HashMap::new())),
            session_id: Arc::new(Mutex::new(None)),
        }
    }

    /// Set the inbound message sender
    pub fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    /// Check if a sender is allowed
    fn is_allowed(&self, sender_id: &str) -> bool {
        if self.allow_from.is_empty() {
            return true;
        }

        if self.allow_from.contains(&sender_id.to_string()) {
            return true;
        }

        // Handle compound IDs
        if sender_id.contains('|') {
            for part in sender_id.split('|') {
                if !part.is_empty() && self.allow_from.contains(&part.to_string()) {
                    return true;
                }
            }
        }

        false
    }

    /// Send identify payload
    async fn send_identify(&mut self) -> Result<()> {
        let identify = serde_json::json!({
            "op": 2,
            "d": {
                "token": self.token,
                "intents": self.intents,
                "properties": {
                    "os": "agent-diva",
                    "browser": "agent-diva",
                    "device": "agent-diva"
                }
            }
        });

        self.send_ws_message(identify).await
    }

    /// Send WebSocket message
    async fn send_ws_message(&mut self, payload: serde_json::Value) -> Result<()> {
        if let Some(ws) = &mut self.ws {
            let msg = tokio_tungstenite::tungstenite::Message::Text(payload.to_string());
            ws.send(msg)
                .await
                .map_err(|e| ChannelError::ApiError(format!("WebSocket send failed: {}", e)))?;
            Ok(())
        } else {
            Err(ChannelError::NotRunning(
                "WebSocket not connected".to_string(),
            ))
        }
    }

    /// Start heartbeat loop
    fn start_heartbeat(&mut self, interval_ms: u64) {
        let seq = self.seq.clone();
        let mut ws = self.ws.take();

        let handle = tokio::spawn(async move {
            let mut ticker = interval(Duration::from_millis(interval_ms));
            ticker.tick().await; // First tick is immediate

            loop {
                ticker.tick().await;

                let seq_num = *seq.lock().await;
                let heartbeat = serde_json::json!({
                    "op": 1,
                    "d": seq_num
                });

                if let Some(ref mut ws) = ws {
                    let msg = tokio_tungstenite::tungstenite::Message::Text(heartbeat.to_string());
                    if let Err(e) = ws.send(msg).await {
                        tracing::warn!("Discord heartbeat failed: {}", e);
                        break;
                    }
                } else {
                    break;
                }
            }
        });

        self.heartbeat_task = Some(handle);
    }

    /// Handle incoming Discord message
    async fn handle_message_create(&self, payload: serde_json::Value) -> Result<()> {
        let msg: DiscordMessage = serde_json::from_value(payload)
            .map_err(|e| ChannelError::Error(format!("Failed to parse message: {}", e)))?;

        // Ignore bot messages
        if msg.author.bot {
            return Ok(());
        }

        let sender_id = msg.author.id.clone();
        let channel_id = msg.channel_id.clone();

        // Check permissions
        if !self.is_allowed(&sender_id) {
            tracing::warn!(
                "Access denied for sender {} on channel {}",
                sender_id,
                self.name
            );
            return Ok(());
        }

        // Build content with attachments
        let mut content_parts = Vec::new();
        if !msg.content.is_empty() {
            content_parts.push(msg.content.clone());
        }

        // Download attachments
        let mut media_paths = Vec::new();
        for attachment in &msg.attachments {
            if attachment.size > MAX_ATTACHMENT_BYTES {
                content_parts.push(format!("[attachment: {} - too large]", attachment.filename));
                continue;
            }

            match self.download_attachment(attachment).await {
                Ok(path) => {
                    media_paths.push(path.clone());
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

        // Start typing indicator
        self.start_typing(channel_id.clone()).await;

        // Send to inbound channel
        if let Some(tx) = &self.inbound_tx {
            let reply_to = msg.reply_to.as_ref().map(|m| m.id.clone());
            let inbound_msg =
                InboundMessage::new(self.name.clone(), sender_id, channel_id.clone(), content)
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

    /// Download attachment
    async fn download_attachment(&self, attachment: &DiscordAttachment) -> Result<String> {
        let media_dir = dirs::home_dir()
            .map(|h: PathBuf| h.join(".agent-diva").join("media"))
            .unwrap_or_else(|| PathBuf::from(".agent-diva/media"));

        tokio::fs::create_dir_all(&media_dir)
            .await
            .map_err(|e| ChannelError::Error(format!("Failed to create media dir: {}", e)))?;

        let safe_filename = attachment.filename.replace('/', "_");
        let file_path = media_dir.join(format!("{}_{}", attachment.id, safe_filename));

        let response = self
            .http
            .get(&attachment.url)
            .send()
            .await
            .map_err(|e| ChannelError::ApiError(format!("Download failed: {}", e)))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ChannelError::ApiError(format!("Read failed: {}", e)))?;

        tokio::fs::write(&file_path, bytes)
            .await
            .map_err(|e| ChannelError::Error(format!("Write failed: {}", e)))?;

        Ok(file_path.to_string_lossy().to_string())
    }

    /// Start typing indicator
    async fn start_typing(&self, channel_id: String) {
        self.stop_typing(&channel_id).await;

        let token = self.token.clone();
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

    /// Gateway connection loop
    async fn gateway_loop(&mut self) -> Result<()> {
        use tokio_tungstenite::tungstenite::Message;

        while self.running {
            if let Some(ws) = &mut self.ws {
                match ws.next().await {
                    Some(Ok(Message::Text(text))) => {
                        let payload: GatewayPayload = match serde_json::from_str(&text) {
                            Ok(p) => p,
                            Err(e) => {
                                tracing::warn!("Invalid JSON from Discord: {}", e);
                                continue;
                            }
                        };

                        // Update sequence number
                        if let Some(s) = payload.s {
                            *self.seq.lock().await = Some(s);
                        }

                        match GatewayOp::from_u8(payload.op) {
                            Some(GatewayOp::Hello) => {
                                // Start heartbeat and identify
                                if let Some(d) = payload.d {
                                    let interval_ms = d
                                        .get("heartbeat_interval")
                                        .and_then(|v| v.as_u64())
                                        .unwrap_or(45000);
                                    self.start_heartbeat(interval_ms);
                                    self.send_identify().await?;
                                }
                            }
                            Some(GatewayOp::Dispatch) => {
                                if payload.t.as_deref() == Some("MESSAGE_CREATE") {
                                    if let Some(d) = payload.d {
                                        if let Err(e) = self.handle_message_create(d).await {
                                            tracing::error!("Error handling message: {}", e);
                                        }
                                    }
                                } else if payload.t.as_deref() == Some("READY") {
                                    tracing::info!("Discord gateway READY");
                                    if let Some(d) = payload.d {
                                        if let Some(session_id) =
                                            d.get("session_id").and_then(|v| v.as_str())
                                        {
                                            *self.session_id.lock().await =
                                                Some(session_id.to_string());
                                        }
                                    }
                                }
                            }
                            Some(GatewayOp::Reconnect) => {
                                tracing::info!("Discord requested reconnect");
                                break;
                            }
                            Some(GatewayOp::InvalidSession) => {
                                tracing::warn!("Discord invalid session");
                                *self.session_id.lock().await = None;
                                break;
                            }
                            _ => {}
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::info!("Discord WebSocket closed");
                        break;
                    }
                    Some(Err(e)) => {
                        tracing::warn!("Discord WebSocket error: {}", e);
                        break;
                    }
                    None => {
                        tracing::warn!("Discord WebSocket stream ended");
                        break;
                    }
                    _ => {}
                }
            } else {
                break;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl ChannelHandler for DiscordHandler {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_running(&self) -> bool {
        self.running
    }

    async fn start(&mut self) -> Result<()> {
        if self.token.is_empty() {
            return Err(ChannelError::NotConfigured(
                "Discord token not configured".to_string(),
            ));
        }

        if self.running {
            return Ok(());
        }

        tracing::info!("Starting Discord bot...");

        // Connect to gateway
        let (ws, _) = tokio_tungstenite::connect_async(&self.gateway_url)
            .await
            .map_err(|e| ChannelError::ApiError(format!("Failed to connect: {}", e)))?;

        self.ws = Some(ws);
        self.running = true;

        // Run gateway loop with reconnect
        let token = self.token.clone();
        let gateway_url = self.gateway_url.clone();
        let intents = self.intents;
        let allow_from = self.allow_from.clone();
        let inbound_tx = self.inbound_tx.clone();

        tokio::spawn(async move {
            let mut reconnect_delay = 5;

            loop {
                // Create new handler for reconnection
                let mut handler = DiscordHandler {
                    name: "discord".to_string(),
                    token: token.clone(),
                    allow_from: allow_from.clone(),
                    gateway_url: gateway_url.clone(),
                    intents,
                    running: true,
                    inbound_tx: inbound_tx.clone(),
                    ws: None,
                    http: reqwest::Client::new(),
                    seq: Arc::new(Mutex::new(None)),
                    heartbeat_task: None,
                    typing_tasks: Arc::new(Mutex::new(HashMap::new())),
                    session_id: Arc::new(Mutex::new(None)),
                };

                // Connect
                match tokio_tungstenite::connect_async(&gateway_url).await {
                    Ok((ws_stream, _)) => {
                        handler.ws = Some(ws_stream);
                        reconnect_delay = 5; // Reset delay on successful connection

                        if let Err(e) = handler.gateway_loop().await {
                            tracing::warn!("Discord gateway error: {}", e);
                        }
                    }
                    Err(e) => {
                        tracing::warn!("Discord connection failed: {}", e);
                    }
                }

                // Reconnect delay
                tracing::info!("Reconnecting to Discord in {} seconds...", reconnect_delay);
                tokio::time::sleep(Duration::from_secs(reconnect_delay)).await;
                reconnect_delay = (reconnect_delay * 2).min(60); // Exponential backoff
            }
        });

        tracing::info!("Discord bot started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        tracing::info!("Stopping Discord bot...");
        self.running = false;

        // Stop heartbeat
        if let Some(handle) = self.heartbeat_task.take() {
            handle.abort();
        }

        // Stop typing indicators
        let mut tasks = self.typing_tasks.lock().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
        }

        // Close WebSocket
        if let Some(ws) = &mut self.ws {
            let _ = ws
                .close(None)
                .await
                .map_err(|e| ChannelError::ApiError(format!("Close failed: {}", e)));
        }

        tracing::info!("Discord bot stopped");
        Ok(())
    }

    async fn send(&self, message: OutboundMessage) -> Result<()> {
        // Stop typing for this channel
        self.stop_typing(&message.chat_id).await;

        let url = format!("{}/channels/{}/messages", DISCORD_API_BASE, message.chat_id);

        let mut payload = serde_json::json!({
            "content": message.content
        });

        // Add reply reference if available
        if let Some(reply_to) = message.metadata.get("reply_to").and_then(|v| v.as_str()) {
            if !reply_to.is_empty() {
                payload["message_reference"] = serde_json::json!({
                    "message_id": reply_to
                });
                payload["allowed_mentions"] = serde_json::json!({
                    "replied_user": false
                });
            }
        }

        // Send with retry
        for attempt in 0..3 {
            let response = self
                .http
                .post(&url)
                .header("Authorization", format!("Bot {}", self.token))
                .header("Content-Type", "application/json")
                .json(&payload)
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

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.is_allowed(sender_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_discord_handler_new() {
        let config = DiscordConfig {
            enabled: true,
            token: "test_token".to_string(),
            allow_from: vec!["user1".to_string()],
            gateway_url: "wss://gateway.discord.gg".to_string(),
            intents: 37377,
        };

        let handler = DiscordHandler::new(&config);
        assert_eq!(handler.name, "discord");
        assert_eq!(handler.token, "test_token");
        assert_eq!(handler.allow_from, vec!["user1".to_string()]);
        assert!(!handler.running);
    }

    #[test]
    fn test_discord_handler_is_allowed() {
        let config = DiscordConfig {
            enabled: true,
            token: "test_token".to_string(),
            allow_from: vec!["12345".to_string(), "67890".to_string()],
            gateway_url: "wss://gateway.discord.gg".to_string(),
            intents: 37377,
        };

        let handler = DiscordHandler::new(&config);
        assert!(handler.is_allowed("12345"));
        assert!(handler.is_allowed("67890"));
        assert!(!handler.is_allowed("99999"));
    }

    #[test]
    fn test_discord_handler_is_allowed_empty() {
        let config = DiscordConfig {
            enabled: true,
            token: "test_token".to_string(),
            allow_from: vec![],
            gateway_url: "wss://gateway.discord.gg".to_string(),
            intents: 37377,
        };

        let handler = DiscordHandler::new(&config);
        assert!(handler.is_allowed("anyone"));
        assert!(handler.is_allowed("12345"));
    }

    #[test]
    fn test_gateway_op_from_u8() {
        assert_eq!(GatewayOp::from_u8(0), Some(GatewayOp::Dispatch));
        assert_eq!(GatewayOp::from_u8(1), Some(GatewayOp::Heartbeat));
        assert_eq!(GatewayOp::from_u8(2), Some(GatewayOp::Identify));
        assert_eq!(GatewayOp::from_u8(10), Some(GatewayOp::Hello));
        assert_eq!(GatewayOp::from_u8(255), None);
    }
}
