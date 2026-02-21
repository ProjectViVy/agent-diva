//! Discord channel integration using Gateway WebSocket
//!
//! Implements Discord bot functionality using the Discord Gateway API
//! for real-time message receiving and REST API for sending messages.

use crate::base::{BaseChannel, ChannelError, ChannelHandler, Result};
use crate::common::{create_http_client, download_file};
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::{Config, DiscordConfig};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex, RwLock};
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
    config: DiscordConfig,
    base: BaseChannel,
    /// Running state (shared across tasks)
    running: Arc<RwLock<bool>>,
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
}

impl DiscordHandler {
    /// Create a new Discord handler from config
    pub fn new(config: &DiscordConfig, base_config: Config) -> Self {
        let allow_from = config.allow_from.clone();
        let base = BaseChannel::new("discord", base_config, allow_from);

        Self {
            config: config.clone(),
            base,
            running: Arc::new(RwLock::new(false)),
            inbound_tx: None,
            http: create_http_client().expect("Failed to create HTTP client"),
            seq: Arc::new(Mutex::new(None)),
            typing_tasks: Arc::new(Mutex::new(HashMap::new())),
            session_id: Arc::new(Mutex::new(None)),
            shutdown_tx: None,
        }
    }

    /// Set the inbound message sender
    pub fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    /// Check if a sender is allowed
    fn is_allowed(&self, sender_id: &str) -> bool {
        self.base.is_allowed(sender_id)
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
                self.base.name
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

            match download_file(&self.http, &attachment.url, &attachment.filename, &attachment.id).await {
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
                InboundMessage::new(self.base.name.clone(), sender_id, channel_id.clone(), content)
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
        }
    }

    /// Run the gateway connection
    async fn run_gateway(
        &self, 
        mut shutdown_rx: mpsc::Receiver<()>
    ) -> Result<()> {
        let mut reconnect_delay = 5;

        loop {
            // Check shutdown
            if shutdown_rx.try_recv().is_ok() {
                break;
            }

            tracing::info!("Connecting to Discord gateway...");
            
            // Connect
            match tokio_tungstenite::connect_async(&self.config.gateway_url).await {
                Ok((ws_stream, _)) => {
                    tracing::info!("Connected to Discord gateway");
                    reconnect_delay = 5; // Reset delay
                    
                    let (mut write, mut read) = ws_stream.split();
                    
                    // We need a channel to send messages to the write sink from other tasks (heartbeat)
                    let (tx, mut rx) = mpsc::channel::<String>(32);
                    
                    // Spawn writer task
                    let writer_handle = tokio::spawn(async move {
                        while let Some(msg) = rx.recv().await {
                             if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Text(msg)).await {
                                 tracing::warn!("WebSocket write failed: {}", e);
                                 break;
                             }
                        }
                    });

                    // Main read loop
                    loop {
                         tokio::select! {
                            _ = shutdown_rx.recv() => {
                                tracing::info!("Shutdown signal received");
                                break;
                            }
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(text))) => {
                                        if let Err(e) = self.handle_gateway_message(&text, &tx).await {
                                            tracing::error!("Error handling gateway message: {}", e);
                                        }
                                    }
                                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => {
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

            // Check running state
            if !*self.running.read().await {
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

        // Update sequence
        if let Some(s) = payload.s {
            *self.seq.lock().await = Some(s);
        }

        match GatewayOp::from_u8(payload.op) {
            Some(GatewayOp::Hello) => {
                if let Some(d) = payload.d {
                    let interval_ms = d.get("heartbeat_interval").and_then(|v| v.as_u64()).unwrap_or(45000);
                    
                    // Spawn heartbeat task
                    let tx_hb = tx.clone();
                    let seq_hb = self.seq.clone();
                    // Note: This is a simplified heartbeat that spawns a detached task.
                    // Ideally we should manage its lifecycle better.
                    tokio::spawn(async move {
                        let mut interval = interval(Duration::from_millis(interval_ms));
                        interval.tick().await; 
                        
                        loop {
                            interval.tick().await;
                            let seq = *seq_hb.lock().await;
                            let heartbeat = serde_json::json!({
                                "op": 1,
                                "d": seq
                            });
                            if tx_hb.send(heartbeat.to_string()).await.is_err() {
                                break;
                            }
                        }
                    });
                    
                    // Identify
                    let identify = serde_json::json!({
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
                    tx.send(identify.to_string()).await.map_err(|e| ChannelError::Error(e.to_string()))?;
                }
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
                return Err(ChannelError::ConnectionError("Reconnect requested".to_string()));
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
}

#[async_trait]
impl ChannelHandler for DiscordHandler {
    fn name(&self) -> &str {
        &self.base.name
    }

    fn is_running(&self) -> bool {
        *self.running.blocking_read()
    }

    async fn start(&mut self) -> Result<()> {
        if self.config.token.is_empty() {
            return Err(ChannelError::NotConfigured(
                "Discord token not configured".to_string(),
            ));
        }

        if self.is_running() {
            return Ok(());
        }

        tracing::info!("Starting Discord bot...");
        *self.running.write().await = true;

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
        if !self.is_running() {
            return Ok(());
        }

        tracing::info!("Stopping Discord bot...");
        *self.running.write().await = false;

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Stop typing indicators
        let mut tasks = self.typing_tasks.lock().await;
        for (_, handle) in tasks.drain() {
            handle.abort();
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
                .header("Authorization", format!("Bot {}", self.config.token))
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

    #[test]
    fn test_discord_handler_new() {
        let config = DiscordConfig {
            enabled: true,
            token: "test_token".to_string(),
            allow_from: vec!["user1".to_string()],
            gateway_url: "wss://gateway.discord.gg".to_string(),
            intents: 37377,
        };
        let global_config = Config::default();

        let handler = DiscordHandler::new(&config, global_config);
        assert_eq!(handler.base.name, "discord");
        assert_eq!(handler.config.token, "test_token");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_gateway_op_from_u8() {
        assert_eq!(GatewayOp::from_u8(0), Some(GatewayOp::Dispatch));
        assert_eq!(GatewayOp::from_u8(10), Some(GatewayOp::Hello));
        assert_eq!(GatewayOp::from_u8(255), None);
    }
}
