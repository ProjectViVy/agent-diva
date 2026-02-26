//! Generic pipe channel — a WebSocket **server** for third-party integrations.
//!
//! Protocol:
//!   Client → Diva:  {"pipe":"msg",   "id":"…", "sender":"…", "chat":"…", "content":"…", "meta":{}}
//!   Diva → Client:  {"pipe":"delta", "id":"…", "reply_to":"…", "chat":"…", "content":"…"}
//!   Diva → Client:  {"pipe":"reply", "id":"…", "reply_to":"…", "chat":"…", "content":"…"}

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, RwLock};
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tracing::{error, info, warn};

use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::GenericPipeConfig;

use crate::base::{ChannelError, ChannelHandler, Result};

// ── Wire protocol types ─────────────────────────────────────────────

/// Incoming message from a pipe client.
#[derive(Debug, Clone, Deserialize)]
struct PipeMsg {
    #[allow(dead_code)]
    pipe: String, // always "msg"
    id: String,
    sender: String,
    chat: String,
    content: String,
    #[serde(default)]
    meta: HashMap<String, serde_json::Value>,
}

/// Outgoing frame (delta or reply) to a pipe client.
#[derive(Debug, Clone, Serialize)]
struct PipeFrame {
    pipe: String, // "delta" or "reply"
    id: String,
    reply_to: String,
    chat: String,
    content: String,
}

// Type alias for the server-side WS sink.
type WsSink = futures::stream::SplitSink<
    tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    tokio_tungstenite::tungstenite::Message,
>;

/// Map from chat-id → WS write-half, so we can route replies back.
type ClientMap = Arc<RwLock<HashMap<String, WsSink>>>;

// ── Handler ─────────────────────────────────────────────────────────

/// WebSocket **server** channel for generic pipe integrations.
pub struct GenericPipeHandler {
    name: String,
    host: String,
    port: u16,
    allow_from: Vec<String>,
    running: bool,
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    clients: ClientMap,
    task_handle: Option<JoinHandle<()>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl GenericPipeHandler {
    /// Create a new generic pipe handler from config.
    pub fn new(config: GenericPipeConfig) -> Self {
        Self {
            name: "generic_pipe".to_string(),
            host: config.host,
            port: config.port,
            allow_from: config.allow_from,
            running: false,
            inbound_tx: None,
            clients: Arc::new(RwLock::new(HashMap::new())),
            task_handle: None,
            shutdown_tx: None,
        }
    }

    /// Set the inbound message sender.
    pub fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    /// Check if a sender is allowed.
    fn is_allowed_internal(&self, sender_id: &str) -> bool {
        if self.allow_from.is_empty() {
            return true;
        }
        self.allow_from.contains(&sender_id.to_string())
    }

    /// Get a clone of the client map (for sending replies).
    pub fn clients(&self) -> ClientMap {
        self.clients.clone()
    }

    /// Accept loop: binds TCP, upgrades each connection to WS, spawns a reader task.
    async fn accept_loop(
        listener: TcpListener,
        allow_from: Vec<String>,
        clients: ClientMap,
        inbound_tx: Option<mpsc::Sender<InboundMessage>>,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                accept = listener.accept() => {
                    match accept {
                        Ok((stream, addr)) => {
                            info!("generic_pipe: new connection from {}", addr);
                            let clients = clients.clone();
                            let inbound_tx = inbound_tx.clone();
                            let allow_from = allow_from.clone();
                            tokio::spawn(async move {
                                Self::handle_connection(
                                    stream, allow_from, clients, inbound_tx,
                                ).await;
                            });
                        }
                        Err(e) => {
                            error!("generic_pipe: accept error: {}", e);
                        }
                    }
                }
                _ = shutdown_rx.recv() => {
                    info!("generic_pipe: shutdown signal received");
                    break;
                }
            }
        }
    }
}

impl GenericPipeHandler {
    /// Handle a single WS connection: read pipe:msg frames, forward to bus.
    async fn handle_connection(
        stream: tokio::net::TcpStream,
        allow_from: Vec<String>,
        clients: ClientMap,
        inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    ) {
        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                error!("generic_pipe: WS handshake failed: {}", e);
                return;
            }
        };

        let (write, mut read) = ws_stream.split();
        // We'll track which chat this connection belongs to so we can clean up.
        let mut connection_chat: Option<String> = None;

        // Temporarily store the write half; it gets moved into the client map
        // on the first message so we know the chat id.
        let write = Arc::new(RwLock::new(Some(write)));

        while let Some(frame) = read.next().await {
            let text = match frame {
                Ok(WsMessage::Text(t)) => t,
                Ok(WsMessage::Close(_)) => break,
                Err(e) => {
                    warn!("generic_pipe: read error: {}", e);
                    break;
                }
                _ => continue,
            };

            let msg: PipeMsg = match serde_json::from_str(&text) {
                Ok(m) => m,
                Err(e) => {
                    warn!("generic_pipe: bad JSON: {}", e);
                    continue;
                }
            };

            // Allow-list check
            if !allow_from.is_empty() && !allow_from.contains(&msg.sender) {
                warn!("generic_pipe: sender {} not in allow_from", msg.sender);
                continue;
            }

            // Register the write half under this chat id (first time only).
            if connection_chat.is_none() {
                connection_chat = Some(msg.chat.clone());
                let mut map = clients.write().await;
                if let Some(sink) = write.write().await.take() {
                    map.insert(msg.chat.clone(), sink);
                }
            }

            // Build InboundMessage and forward to bus.
            if let Some(ref tx) = inbound_tx {
                let mut inbound =
                    InboundMessage::new("generic_pipe", &msg.sender, &msg.chat, &msg.content);
                inbound = inbound.with_metadata("pipe_msg_id", msg.id.clone());
                for (k, v) in &msg.meta {
                    inbound = inbound.with_metadata(k.clone(), v.clone());
                }
                if let Err(e) = tx.send(inbound).await {
                    error!("generic_pipe: failed to forward inbound: {}", e);
                }
            }
        }

        // Clean up client entry on disconnect.
        if let Some(chat) = connection_chat {
            let mut map = clients.write().await;
            map.remove(&chat);
            info!("generic_pipe: client for chat '{}' disconnected", chat);
        }
    }
}

// ── ChannelHandler trait ────────────────────────────────────────────

#[async_trait]
impl ChannelHandler for GenericPipeHandler {
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

        let addr = format!("{}:{}", self.host, self.port);
        let listener = TcpListener::bind(&addr).await.map_err(|e| {
            ChannelError::ConnectionFailed(format!("generic_pipe: failed to bind {}: {}", addr, e))
        })?;
        info!("generic_pipe: listening on {}", addr);

        let (shutdown_tx, shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);

        let clients = self.clients.clone();
        let inbound_tx = self.inbound_tx.clone();
        let allow_from = self.allow_from.clone();

        self.task_handle = Some(tokio::spawn(async move {
            Self::accept_loop(listener, allow_from, clients, inbound_tx, shutdown_rx).await;
        }));

        self.running = true;
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.running {
            return Ok(());
        }
        info!("generic_pipe: stopping...");

        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }
        if let Some(handle) = self.task_handle.take() {
            handle.abort();
        }

        // Close all client sinks.
        {
            let mut map = self.clients.write().await;
            for (_, mut sink) in map.drain() {
                let _ = sink.close().await;
            }
        }

        self.running = false;
        info!("generic_pipe: stopped");
        Ok(())
    }

    async fn send(&self, msg: OutboundMessage) -> Result<()> {
        let frame = PipeFrame {
            pipe: "reply".to_string(),
            id: uuid_v4(),
            reply_to: msg.reply_to.clone().unwrap_or_default(),
            chat: msg.chat_id.clone(),
            content: msg.content.clone(),
        };
        let payload = serde_json::to_string(&frame)
            .map_err(|e| ChannelError::SendError(format!("serialize: {}", e)))?;

        let mut map = self.clients.write().await;
        if let Some(sink) = map.get_mut(&msg.chat_id) {
            sink.send(WsMessage::Text(payload))
                .await
                .map_err(|e| ChannelError::SendError(format!("ws send: {}", e)))?;
            Ok(())
        } else {
            Err(ChannelError::NotRunning(format!(
                "no pipe client for chat '{}'",
                msg.chat_id
            )))
        }
    }

    fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        self.is_allowed_internal(sender_id)
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

/// Minimal v4-style UUID without pulling in the `uuid` crate.
fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:032x}", t)
}

/// Send a streaming delta frame to the pipe client for a given chat.
/// Called from the agent loop when `AgentEvent::AssistantDelta` fires.
pub async fn send_delta(
    clients: &ClientMap,
    chat_id: &str,
    reply_to: &str,
    content: &str,
) -> Result<()> {
    let frame = PipeFrame {
        pipe: "delta".to_string(),
        id: uuid_v4(),
        reply_to: reply_to.to_string(),
        chat: chat_id.to_string(),
        content: content.to_string(),
    };
    let payload = serde_json::to_string(&frame)
        .map_err(|e| ChannelError::SendError(format!("serialize: {}", e)))?;

    let mut map = clients.write().await;
    if let Some(sink) = map.get_mut(chat_id) {
        sink.send(WsMessage::Text(payload))
            .await
            .map_err(|e| ChannelError::SendError(format!("ws send delta: {}", e)))?;
    }
    Ok(())
}

impl Default for GenericPipeHandler {
    fn default() -> Self {
        Self::new(GenericPipeConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handler_new() {
        let handler = GenericPipeHandler::new(GenericPipeConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 9100,
            allow_from: vec![],
        });
        assert_eq!(handler.name(), "generic_pipe");
        assert!(!handler.is_running());
    }

    #[test]
    fn test_is_allowed_empty() {
        let handler = GenericPipeHandler::default();
        assert!(handler.is_allowed("anyone"));
    }

    #[test]
    fn test_is_allowed_restricted() {
        let handler = GenericPipeHandler::new(GenericPipeConfig {
            enabled: true,
            host: "0.0.0.0".to_string(),
            port: 9100,
            allow_from: vec!["vtuber-1".to_string()],
        });
        assert!(handler.is_allowed("vtuber-1"));
        assert!(!handler.is_allowed("unknown"));
    }

    #[test]
    fn test_pipe_msg_deserialize() {
        let raw = r#"{"pipe":"msg","id":"a1","sender":"s1","chat":"c1","content":"hi","meta":{}}"#;
        let msg: PipeMsg = serde_json::from_str(raw).unwrap();
        assert_eq!(msg.id, "a1");
        assert_eq!(msg.content, "hi");
    }

    #[test]
    fn test_pipe_frame_serialize() {
        let frame = PipeFrame {
            pipe: "reply".to_string(),
            id: "r1".to_string(),
            reply_to: "a1".to_string(),
            chat: "c1".to_string(),
            content: "hello".to_string(),
        };
        let json = serde_json::to_string(&frame).unwrap();
        assert!(json.contains(r#""pipe":"reply""#));
        assert!(json.contains(r#""reply_to":"a1""#));
    }

    #[tokio::test]
    async fn test_start_stop() {
        let mut handler = GenericPipeHandler::new(GenericPipeConfig {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 0, // OS picks a free port — but TcpListener::bind("127.0.0.1:0") works
            allow_from: vec![],
        });
        // Port 0 won't work with our addr format, so use a high port
        handler.port = 19876;
        handler.host = "127.0.0.1".to_string();
        handler.start().await.unwrap();
        assert!(handler.is_running());
        handler.stop().await.unwrap();
        assert!(!handler.is_running());
    }
}
