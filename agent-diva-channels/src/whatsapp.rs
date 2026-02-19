﻿//! WhatsApp channel integration
//!
//! Connects to a Node.js bridge via WebSocket to communicate with WhatsApp Web.
//! The bridge uses @whiskeysockets/baileys library.
//!
//! Python reference: agent-diva/channels/whatsapp.py
//! Bridge reference: bridge/src/whatsapp.ts

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use agent_diva_providers::transcription::TranscriptionService;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tracing::{error, info, warn};

use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::WhatsAppConfig;

use crate::base::{ChannelError, ChannelHandler, Result};

// Type alias for WebSocket sink to simplify type signatures
type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Message,
>;

/// Bridge message types received from the Node.js bridge
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum BridgeMessage {
    #[serde(rename = "message")]
    Message {
        #[serde(default)]
        id: String,
        #[serde(default)]
        sender: String,
        #[serde(default)]
        pn: String,
        #[serde(default)]
        content: String,
        #[serde(default)]
        timestamp: Option<i64>,
        #[serde(default, alias = "isGroup")]
        is_group: bool,
        #[serde(default, alias = "protocolVersion")]
        protocol_version: Option<u8>,
        #[serde(default)]
        media: Option<BridgeMedia>,
    },
    #[serde(rename = "status")]
    Status { status: String },
    #[serde(rename = "qr")]
    Qr {
        #[allow(dead_code)]
        qr: String,
    },
    #[serde(rename = "error")]
    Error { error: String },
    #[serde(rename = "sent")]
    Sent { to: String },
}

/// Optional media payload forwarded by the bridge (v2 protocol).
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct BridgeMedia {
    #[serde(default)]
    media_type: String,
    #[serde(default)]
    mime: Option<String>,
    #[serde(default)]
    media_key: Option<String>,
    #[serde(default)]
    direct_path: Option<String>,
    #[serde(default)]
    file_sha256: Option<String>,
    #[serde(default)]
    file_enc_sha256: Option<String>,
    #[serde(default)]
    file_length: Option<u64>,
    #[serde(default)]
    local_path: Option<String>,
    #[serde(default)]
    file_name: Option<String>,
}

/// Send command to the bridge
#[derive(Debug, Clone, Serialize)]
struct SendCommand {
    #[serde(rename = "type")]
    msg_type: String,
    to: String,
    text: String,
}

impl SendCommand {
    fn new(to: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            msg_type: "send".to_string(),
            to: to.into(),
            text: text.into(),
        }
    }
}

/// WhatsApp channel handler
///
/// Connects to a Node.js bridge via WebSocket to communicate with WhatsApp Web.
/// The bridge handles the actual WhatsApp protocol using @whiskeysockets/baileys.
pub struct WhatsAppHandler {
    name: String,
    bridge_url: String,
    allow_from: Vec<String>,
    running: bool,
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    /// WebSocket write stream
    ws_tx: Arc<RwLock<Option<WsSink>>>,
    /// Connection state
    connected: Arc<RwLock<bool>>,
    /// Background task handle
    task_handle: Option<JoinHandle<()>>,
    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl WhatsAppHandler {
    /// Create a new WhatsApp handler
    pub fn new(config: WhatsAppConfig) -> Self {
        Self {
            name: "whatsapp".to_string(),
            bridge_url: config.bridge_url,
            allow_from: config.allow_from,
            running: false,
            inbound_tx: None,
            ws_tx: Arc::new(RwLock::new(None)),
            connected: Arc::new(RwLock::new(false)),
            task_handle: None,
            shutdown_tx: None,
        }
    }

    /// Set the inbound message sender
    pub fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    /// Check if a sender is allowed
    fn is_allowed_internal(&self, sender_id: &str) -> bool {
        if self.allow_from.is_empty() {
            return true;
        }

        let sender_str = sender_id.to_string();
        if self.allow_from.contains(&sender_str) {
            return true;
        }

        // Handle compound IDs (e.g., "12345|username")
        if sender_str.contains('|') {
            for part in sender_str.split('|') {
                if !part.is_empty() && self.allow_from.contains(&part.to_string()) {
                    return true;
                }
            }
        }

        false
    }

    /// Handle an incoming message from the bridge
    async fn handle_bridge_message(&self, raw: &str) -> Result<()> {
        let data: BridgeMessage = match serde_json::from_str(raw) {
            Ok(msg) => msg,
            Err(e) => {
                warn!(
                    "Invalid JSON from bridge: {} (raw: {})",
                    e,
                    &agent_diva_core::utils::truncate(raw, 100)
                );
                return Ok(());
            }
        };

        match data {
            BridgeMessage::Message {
                id,
                sender,
                pn,
                mut content,
                timestamp,
                is_group,
                protocol_version,
                media,
            } => {
                // Extract sender ID from phone number or sender JID
                let user_id = if !pn.is_empty() {
                    pn.clone()
                } else {
                    sender.clone()
                };
                let sender_id = user_id.split('@').next().unwrap_or(&user_id).to_string();

                info!("Received message from sender: {}", sender_id);

                if !self.is_allowed_internal(&sender_id) {
                    info!(
                        "Dropping WhatsApp message from non-allowlisted sender: {}",
                        sender_id
                    );
                    return Ok(());
                }

                let mut transcription_status: Option<&'static str> = None;
                if let Some(media_info) = media.as_ref() {
                    if media_info.media_type == "audio" {
                        match self.transcribe_voice_message(media_info).await {
                            Some(transcribed) => {
                                content = transcribed;
                                transcription_status = Some("ok");
                            }
                            None => {
                                transcription_status = Some("failed");
                                if content.trim().is_empty() || content == "[Voice Message]" {
                                    content =
                                        "[Voice Message: Transcription unavailable]".to_string();
                                }
                            }
                        }
                    }
                } else if content == "[Voice Message]" {
                    content = "[Voice Message: Transcription unavailable]".to_string();
                    transcription_status = Some("missing_media");
                }

                // Build metadata
                let mut metadata = serde_json::Map::new();
                if !id.is_empty() {
                    metadata.insert("message_id".to_string(), id.into());
                }
                if let Some(ts) = timestamp {
                    metadata.insert("timestamp".to_string(), ts.into());
                }
                metadata.insert("is_group".to_string(), is_group.into());
                if let Some(v) = protocol_version {
                    metadata.insert("protocol_version".to_string(), v.into());
                }
                if let Some(media_info) = media {
                    if let Ok(media_json) = serde_json::to_value(media_info) {
                        metadata.insert("media".to_string(), media_json);
                    }
                }
                if let Some(status) = transcription_status {
                    metadata.insert("transcription_status".to_string(), status.into());
                }

                // Send to inbound channel
                if let Some(tx) = &self.inbound_tx {
                    let mut msg = InboundMessage::new(
                        self.name.clone(),
                        sender_id,
                        sender, // Use full JID for replies
                        content,
                    )
                    .with_metadata(
                        "message_id",
                        metadata.get("message_id").cloned().unwrap_or_default(),
                    )
                    .with_metadata(
                        "timestamp",
                        metadata.get("timestamp").cloned().unwrap_or_default(),
                    )
                    .with_metadata("is_group", is_group);
                    if let Some(v) = metadata.get("protocol_version") {
                        msg = msg.with_metadata("protocol_version", v.clone());
                    }
                    if let Some(v) = metadata.get("media") {
                        msg = msg.with_metadata("media", v.clone());
                    }
                    if let Some(v) = metadata.get("transcription_status") {
                        msg = msg.with_metadata("transcription_status", v.clone());
                    }

                    if let Err(e) = tx.send(msg).await {
                        error!("Failed to send inbound message: {}", e);
                    }
                }
            }
            BridgeMessage::Status { status } => {
                info!("WhatsApp status: {}", status);
                let mut connected = self.connected.write().await;
                *connected = status == "connected";
            }
            BridgeMessage::Qr { .. } => {
                info!("QR code received - scan with WhatsApp mobile app");
            }
            BridgeMessage::Error { error } => {
                error!("WhatsApp bridge error: {}", error);
            }
            BridgeMessage::Sent { to } => {
                info!("Message sent to: {}", to);
            }
        }

        Ok(())
    }

    async fn transcribe_voice_message(&self, media: &BridgeMedia) -> Option<String> {
        let local_path = media.local_path.as_deref()?;
        let service = TranscriptionService::new(None);
        if !service.is_configured() {
            warn!("WhatsApp voice transcription skipped because GROQ_API_KEY is not configured");
            return None;
        }

        let text = service.transcribe_safe(local_path).await;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }

        Some(trimmed.to_string())
    }

    /// WebSocket connection loop with reconnection
    async fn connection_loop(
        bridge_url: String,
        allow_from: Vec<String>,
        ws_tx: Arc<RwLock<Option<WsSink>>>,
        connected: Arc<RwLock<bool>>,
        inbound_tx: Option<mpsc::Sender<InboundMessage>>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) {
        let handler = Self {
            name: "whatsapp".to_string(),
            bridge_url: bridge_url.clone(),
            allow_from,
            running: true,
            inbound_tx,
            ws_tx: ws_tx.clone(),
            connected: connected.clone(),
            task_handle: None,
            shutdown_tx: None,
        };

        let mut running = true;

        while running {
            info!("Connecting to WhatsApp bridge at {}...", bridge_url);

            match connect_async(&bridge_url).await {
                Ok((ws_stream, _)) => {
                    info!("Connected to WhatsApp bridge");
                    let (write, mut read) = ws_stream.split();

                    // Store the write stream
                    {
                        let mut tx = ws_tx.write().await;
                        *tx = Some(write);
                    }

                    // Set connected flag
                    {
                        let mut conn = connected.write().await;
                        *conn = true;
                    }

                    // Message processing loop
                    loop {
                        tokio::select! {
                            msg = read.next() => {
                                match msg {
                                    Some(Ok(WsMessage::Text(text))) => {
                                        if let Err(e) = handler.handle_bridge_message(&text).await {
                                            error!("Error handling bridge message: {}", e);
                                        }
                                    }
                                    Some(Ok(WsMessage::Close(_))) => {
                                        info!("WebSocket closed by server");
                                        break;
                                    }
                                    Some(Err(e)) => {
                                        error!("WebSocket error: {}", e);
                                        break;
                                    }
                                    _ => {}
                                }
                            }
                            _ = shutdown_rx.recv() => {
                                info!("Shutdown signal received");
                                running = false;
                                break;
                            }
                        }
                    }

                    // Set disconnected flag
                    {
                        let mut conn = connected.write().await;
                        *conn = false;
                    }

                    // Clear write stream
                    {
                        let mut tx = ws_tx.write().await;
                        *tx = None;
                    }
                }
                Err(e) => {
                    error!("Failed to connect to WhatsApp bridge: {}", e);
                }
            }

            if running {
                info!("Reconnecting in 5 seconds...");
                tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
            }
        }

        info!("WhatsApp connection loop ended");
    }
}

#[async_trait]
impl ChannelHandler for WhatsAppHandler {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_running(&self) -> bool {
        self.running
    }

    async fn start(&mut self) -> Result<()> {
        if self.running {
            return Ok(());
        }

        info!("Starting WhatsApp channel...");
        info!("Bridge URL: {}", self.bridge_url);

        // Create shutdown channel
        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Start connection loop in background
        let bridge_url = self.bridge_url.clone();
        let ws_tx = self.ws_tx.clone();
        let connected = self.connected.clone();
        let inbound_tx = self.inbound_tx.clone();
        let allow_from = self.allow_from.clone();

        let handle = tokio::spawn(async move {
            Self::connection_loop(
                bridge_url,
                allow_from,
                ws_tx,
                connected,
                inbound_tx,
                shutdown_rx,
            )
            .await;
        });

        self.task_handle = Some(handle);
        self.running = true;

        info!("WhatsApp channel started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }

        info!("Stopping WhatsApp channel...");

        // Send shutdown signal
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        // Abort background task
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }

        // Close WebSocket connection
        {
            let mut tx = self.ws_tx.write().await;
            if let Some(mut write) = tx.take() {
                let _ = write.close().await;
            }
        }

        // Set disconnected
        {
            let mut conn = self.connected.write().await;
            *conn = false;
        }

        self.running = false;
        info!("WhatsApp channel stopped");

        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        let connected = *self.connected.read().await;
        if !connected {
            return Err(ChannelError::NotRunning(
                "WhatsApp bridge not connected".to_string(),
            ));
        }

        let cmd = SendCommand::new(&msg.chat_id, &msg.content);
        let payload = serde_json::to_string(&cmd)
            .map_err(|e| ChannelError::SendError(format!("Failed to serialize message: {}", e)))?;

        let mut tx = self.ws_tx.write().await;
        if let Some(ref mut write) = *tx {
            write
                .send(WsMessage::Text(payload))
                .await
                .map_err(|e| ChannelError::SendError(format!("Failed to send: {}", e)))?;
            Ok(())
        } else {
            Err(ChannelError::NotRunning(
                "WebSocket not initialized".to_string(),
            ))
        }
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.is_allowed_internal(sender_id)
    }
}

impl Default for WhatsAppHandler {
    fn default() -> Self {
        Self::new(WhatsAppConfig {
            enabled: false,
            bridge_url: "ws://localhost:3001".to_string(),
            allow_from: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::time::{timeout, Duration};

    #[test]
    fn test_whatsapp_handler_new() {
        let config = WhatsAppConfig {
            enabled: true,
            bridge_url: "ws://localhost:3000".to_string(),
            allow_from: vec![],
        };
        let handler = WhatsAppHandler::new(config);
        assert_eq!(handler.name(), "whatsapp");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_is_allowed() {
        let config = WhatsAppConfig {
            enabled: true,
            bridge_url: "ws://localhost:3000".to_string(),
            allow_from: vec!["1234567890".to_string()],
        };
        let handler = WhatsAppHandler::new(config);
        assert!(handler.is_allowed("1234567890"));
        assert!(!handler.is_allowed("9876543210"));
    }

    #[test]
    fn test_is_allowed_empty_list() {
        let config = WhatsAppConfig {
            enabled: true,
            bridge_url: "ws://localhost:3000".to_string(),
            allow_from: vec![],
        };
        let handler = WhatsAppHandler::new(config);
        assert!(handler.is_allowed("1234567890"));
        assert!(handler.is_allowed("9876543210"));
    }

    #[tokio::test]
    async fn test_start_stop() {
        let config = WhatsAppConfig {
            enabled: true,
            bridge_url: "ws://localhost:3000".to_string(),
            allow_from: vec![],
        };
        let mut handler = WhatsAppHandler::new(config);

        handler.start().await.unwrap();
        assert!(handler.is_running());

        handler.stop().await.unwrap();
        assert!(!handler.is_running());
    }

    #[tokio::test]
    async fn test_bridge_message_maps_is_group_alias() {
        let config = WhatsAppConfig {
            enabled: true,
            bridge_url: "ws://localhost:3000".to_string(),
            allow_from: vec![],
        };
        let mut handler = WhatsAppHandler::new(config);
        let (tx, mut rx) = mpsc::channel(1);
        handler.set_inbound_sender(tx);

        let raw = json!({
            "type": "message",
            "id": "m1",
            "sender": "12345@s.whatsapp.net",
            "pn": "",
            "content": "hello",
            "timestamp": 1700000000,
            "isGroup": true
        })
        .to_string();

        handler.handle_bridge_message(&raw).await.unwrap();
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.sender_id, "12345");
        assert_eq!(msg.metadata.get("is_group"), Some(&json!(true)));
    }

    #[tokio::test]
    async fn test_allowlist_blocks_sender() {
        let config = WhatsAppConfig {
            enabled: true,
            bridge_url: "ws://localhost:3000".to_string(),
            allow_from: vec!["allow-me".to_string()],
        };
        let mut handler = WhatsAppHandler::new(config);
        let (tx, mut rx) = mpsc::channel(1);
        handler.set_inbound_sender(tx);

        let raw = json!({
            "type": "message",
            "id": "m2",
            "sender": "blocked@s.whatsapp.net",
            "pn": "",
            "content": "blocked content",
            "timestamp": 1700000000,
            "isGroup": false
        })
        .to_string();

        handler.handle_bridge_message(&raw).await.unwrap();
        let received = timeout(Duration::from_millis(200), rx.recv()).await;
        assert!(received.is_err(), "blocked sender should not be forwarded");
    }

    #[tokio::test]
    async fn test_bridge_v2_audio_metadata_and_fallback_content() {
        let config = WhatsAppConfig {
            enabled: true,
            bridge_url: "ws://localhost:3000".to_string(),
            allow_from: vec![],
        };
        let mut handler = WhatsAppHandler::new(config);
        let (tx, mut rx) = mpsc::channel(1);
        handler.set_inbound_sender(tx);

        let raw = json!({
            "type": "message",
            "id": "m3",
            "sender": "12345@s.whatsapp.net",
            "pn": "",
            "content": "[Voice Message]",
            "timestamp": 1700000001,
            "isGroup": false,
            "protocolVersion": 2,
            "media": {
                "mediaType": "audio",
                "mime": "audio/ogg",
                "localPath": "C:/nonexistent/audio.ogg",
                "fileName": "audio.ogg"
            }
        })
        .to_string();

        handler.handle_bridge_message(&raw).await.unwrap();
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.content, "[Voice Message: Transcription unavailable]");
        assert_eq!(msg.metadata.get("protocol_version"), Some(&json!(2)));
        assert_eq!(
            msg.metadata
                .get("media")
                .and_then(|m| m.get("mediaType"))
                .cloned(),
            Some(json!("audio"))
        );
        assert_eq!(
            msg.metadata.get("transcription_status"),
            Some(&json!("failed"))
        );
    }
}
