//! IRC channel handler with TLS support, SASL authentication, and reconnection.

use async_trait::async_trait;
use agent_diva_core::bus::{InboundMessage, OutboundMessage};
use agent_diva_core::config::schema::IrcConfig;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::base::{ChannelError, ChannelHandler, Result};

/// Maximum IRC line length (RFC 2812: 512 bytes including CRLF)
const MAX_IRC_LINE: usize = 510;

/// Parsed IRC message
#[derive(Debug, Clone)]
struct IrcMessage {
    prefix: Option<String>,
    command: String,
    params: Vec<String>,
}

impl IrcMessage {
    /// Parse a raw IRC line into an IrcMessage.
    fn parse(line: &str) -> Option<Self> {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            return None;
        }

        let mut rest = line;
        let prefix = if rest.starts_with(':') {
            let end = rest.find(' ')?;
            let p = rest[1..end].to_string();
            rest = &rest[end + 1..];
            Some(p)
        } else {
            None
        };

        // Split command from params
        let (command, params_str) = if let Some(idx) = rest.find(' ') {
            (&rest[..idx], &rest[idx + 1..])
        } else {
            (rest, "")
        };

        let command = command.to_uppercase();
        let mut params = Vec::new();

        if !params_str.is_empty() {
            let mut remaining = params_str;
            while !remaining.is_empty() {
                if let Some(stripped) = remaining.strip_prefix(':') {
                    params.push(stripped.to_string());
                    break;
                }
                if let Some(idx) = remaining.find(' ') {
                    params.push(remaining[..idx].to_string());
                    remaining = &remaining[idx + 1..];
                } else {
                    params.push(remaining.to_string());
                    break;
                }
            }
        }

        Some(IrcMessage {
            prefix,
            command,
            params,
        })
    }

    /// Extract the nickname from the prefix (nick!user@host).
    fn nick(&self) -> Option<&str> {
        self.prefix.as_ref().and_then(|p| {
            p.find('!').map(|idx| &p[..idx]).or(Some(p.as_str()))
        })
    }
}

/// Split a message into IRC-safe chunks (max ~400 bytes to leave room for PRIVMSG overhead).
fn split_message(text: &str, max_payload: usize) -> Vec<String> {
    if text.len() <= max_payload {
        return vec![text.to_string()];
    }

    let mut chunks = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        if remaining.len() <= max_payload {
            chunks.push(remaining.to_string());
            break;
        }

        // Find a safe split point (UTF-8 boundary, prefer space/newline)
        let mut split_at = max_payload;
        // Back up to a char boundary
        while split_at > 0 && !remaining.is_char_boundary(split_at) {
            split_at -= 1;
        }
        // Try to find a space or newline to split at
        if let Some(pos) = remaining[..split_at].rfind([' ', '\n']) {
            split_at = pos + 1;
        }
        if split_at == 0 {
            split_at = max_payload.min(remaining.len());
            while split_at < remaining.len() && !remaining.is_char_boundary(split_at) {
                split_at += 1;
            }
        }

        chunks.push(remaining[..split_at].trim_end().to_string());
        remaining = &remaining[split_at..];
    }

    chunks
}

/// Encode SASL PLAIN credentials: base64("\0{username}\0{password}")
fn encode_sasl_plain(username: &str, password: &str) -> String {
    use base64::Engine;
    let plain = format!("\0{}\0{}", username, password);
    base64::engine::general_purpose::STANDARD.encode(plain.as_bytes())
}

/// Type alias for the write half of an IRC connection (plain or TLS).
type IrcWriter = Arc<Mutex<Box<dyn tokio::io::AsyncWrite + Send + Unpin>>>;

/// IRC channel handler
pub struct IrcHandler {
    config: IrcConfig,
    allow_from: Vec<String>,
    running: Arc<RwLock<bool>>,
    inbound_tx: Option<mpsc::Sender<InboundMessage>>,
    task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
    writer: Arc<Mutex<Option<IrcWriter>>>,
}

impl IrcHandler {
    pub fn new(config: IrcConfig) -> Self {
        Self {
            allow_from: config.allow_from.clone(),
            config,
            running: Arc::new(RwLock::new(false)),
            inbound_tx: None,
            task_handle: Arc::new(Mutex::new(None)),
            writer: Arc::new(Mutex::new(None)),
        }
    }
}

/// Send a raw IRC line over the writer.
async fn send_raw(writer: &IrcWriter, line: &str) -> std::result::Result<(), String> {
    let mut w = writer.lock().await;
    let data = format!("{}\r\n", line);
    w.write_all(data.as_bytes())
        .await
        .map_err(|e| format!("IRC write error: {}", e))
}

#[async_trait]
impl ChannelHandler for IrcHandler {
    fn name(&self) -> &str {
        "irc"
    }

    fn is_running(&self) -> bool {
        self.running.try_read().map(|r| *r).unwrap_or(false)
    }

    fn set_inbound_sender(&mut self, tx: mpsc::Sender<InboundMessage>) {
        self.inbound_tx = Some(tx);
    }

    fn is_allowed(&self, sender_id: &str) -> bool {
        if self.allow_from.is_empty() {
            return true;
        }
        self.allow_from.contains(&sender_id.to_string())
    }

    async fn start(&mut self) -> Result<()> {
        if *self.running.read().await {
            return Ok(());
        }

        let tx = self
            .inbound_tx
            .clone()
            .ok_or_else(|| ChannelError::Error("Inbound sender not set".into()))?;

        let running = self.running.clone();
        let writer_slot = self.writer.clone();
        let config = self.config.clone();
        let allow_from = self.allow_from.clone();

        *running.write().await = true;

        let handle = tokio::spawn(async move {
            let mut backoff_secs: u64 = 1;
            let max_backoff: u64 = 60;

            loop {
                if !*running.read().await {
                    break;
                }

                info!("IRC: connecting to {}:{}", config.server, config.port);

                match irc_connect_and_run(
                    &config,
                    &allow_from,
                    &tx,
                    &running,
                    &writer_slot,
                )
                .await
                {
                    Ok(()) => {
                        info!("IRC: connection closed cleanly");
                    }
                    Err(e) => {
                        warn!("IRC: connection error: {}", e);
                    }
                }

                // Clear writer on disconnect
                *writer_slot.lock().await = None;

                if !*running.read().await {
                    break;
                }

                info!("IRC: reconnecting in {} seconds", backoff_secs);
                tokio::time::sleep(std::time::Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(max_backoff);
            }

            info!("IRC task stopped");
        });

        *self.task_handle.lock().await = Some(handle);
        info!("IRC channel started");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        *self.running.write().await = false;

        // Try to send QUIT
        if let Some(ref w) = *self.writer.lock().await {
            let _ = send_raw(w, "QUIT :Shutting down").await;
        }
        *self.writer.lock().await = None;

        if let Some(handle) = self.task_handle.lock().await.take() {
            handle.abort();
        }
        info!("IRC channel stopped");
        Ok(())
    }

    async fn send(&self, message: OutboundMessage) -> Result<()> {
        let writer_guard = self.writer.lock().await;
        let writer = writer_guard
            .as_ref()
            .ok_or_else(|| ChannelError::NotRunning("IRC not connected".into()))?;

        let target = &message.chat_id;
        // Leave room for "PRIVMSG <target> :" prefix
        let overhead = format!("PRIVMSG {} :", target).len();
        let max_payload = MAX_IRC_LINE.saturating_sub(overhead);

        let chunks = split_message(&message.content, max_payload);
        for chunk in chunks {
            let line = format!("PRIVMSG {} :{}", target, chunk);
            send_raw(writer, &line)
                .await
                .map_err(ChannelError::SendFailed)?;
        }

        Ok(())
    }

    async fn test_connection(&self) -> Result<()> {
        // Try a TLS/TCP connect and immediately disconnect
        let addr = format!("{}:{}", self.config.server, self.config.port);
        let tcp = tokio::net::TcpStream::connect(&addr)
            .await
            .map_err(|e| ChannelError::ConnectionFailed(format!("TCP connect failed: {}", e)))?;

        if self.config.use_tls {
            let connector = native_tls::TlsConnector::builder()
                .danger_accept_invalid_certs(!self.config.verify_tls)
                .build()
                .map_err(|e| ChannelError::ConnectionFailed(format!("TLS setup failed: {}", e)))?;
            let connector = tokio_native_tls::TlsConnector::from(connector);
            let mut tls = connector
                .connect(&self.config.server, tcp)
                .await
                .map_err(|e| ChannelError::ConnectionFailed(format!("TLS handshake failed: {}", e)))?;
            let _ = tls.shutdown().await;
        } else {
            drop(tcp);
        }

        Ok(())
    }
}

/// Establish an IRC connection, perform registration, and run the message loop.
async fn irc_connect_and_run(
    config: &IrcConfig,
    allow_from: &[String],
    tx: &mpsc::Sender<InboundMessage>,
    running: &Arc<RwLock<bool>>,
    writer_slot: &Arc<Mutex<Option<IrcWriter>>>,
) -> std::result::Result<(), String> {
    let addr = format!("{}:{}", config.server, config.port);
    let tcp = tokio::net::TcpStream::connect(&addr)
        .await
        .map_err(|e| format!("TCP connect failed: {}", e))?;

    if config.use_tls {
        let connector = native_tls::TlsConnector::builder()
            .danger_accept_invalid_certs(!config.verify_tls)
            .build()
            .map_err(|e| format!("TLS setup failed: {}", e))?;
        let connector = tokio_native_tls::TlsConnector::from(connector);
        let tls_stream = connector
            .connect(&config.server, tcp)
            .await
            .map_err(|e| format!("TLS handshake failed: {}", e))?;

        let (reader, writer) = tokio::io::split(tls_stream);
        let writer: IrcWriter = Arc::new(Mutex::new(Box::new(writer)));
        *writer_slot.lock().await = Some(writer.clone());

        irc_register_and_loop(config, allow_from, tx, running, BufReader::new(reader), &writer).await
    } else {
        let (reader, writer) = tokio::io::split(tcp);
        let writer: IrcWriter = Arc::new(Mutex::new(Box::new(writer)));
        *writer_slot.lock().await = Some(writer.clone());

        irc_register_and_loop(config, allow_from, tx, running, BufReader::new(reader), &writer).await
    }
}

/// Perform IRC registration (PASS, CAP/SASL, NICK, USER) and run the message loop.
async fn irc_register_and_loop<R: tokio::io::AsyncBufRead + Unpin>(
    config: &IrcConfig,
    allow_from: &[String],
    tx: &mpsc::Sender<InboundMessage>,
    running: &Arc<RwLock<bool>>,
    mut reader: R,
    writer: &IrcWriter,
) -> std::result::Result<(), String> {
    // Send PASS if server password is set
    if let Some(ref pass) = config.server_password {
        if !pass.is_empty() {
            send_raw(writer, &format!("PASS {}", pass)).await?;
        }
    }

    // Request SASL if sasl_password is set
    let use_sasl = config.sasl_password.as_ref().is_some_and(|p| !p.is_empty());
    if use_sasl {
        send_raw(writer, "CAP REQ :sasl").await?;
    }

    // NICK and USER
    let nick = if config.nickname.is_empty() {
        "diva-bot"
    } else {
        &config.nickname
    };
    let user = if config.username.is_empty() {
        nick
    } else {
        &config.username
    };

    send_raw(writer, &format!("NICK {}", nick)).await?;
    send_raw(writer, &format!("USER {} 0 * :agent-diva IRC bot", user)).await?;

    let mut line_buf = String::new();
    let mut registered = false;
    let mut _sasl_done = !use_sasl;

    loop {
        if !*running.read().await {
            break;
        }

        line_buf.clear();
        let n = tokio::time::timeout(
            std::time::Duration::from_secs(300),
            reader.read_line(&mut line_buf),
        )
        .await
        .map_err(|_| "IRC read timeout".to_string())?
        .map_err(|e| format!("IRC read error: {}", e))?;

        if n == 0 {
            return Err("IRC connection closed by server".into());
        }

        let msg = match IrcMessage::parse(&line_buf) {
            Some(m) => m,
            None => continue,
        };

        debug!("IRC recv: {} {}", msg.command, msg.params.join(" "));

        match msg.command.as_str() {
            "PING" => {
                let payload = msg.params.first().map(|s| s.as_str()).unwrap_or("");
                send_raw(writer, &format!("PONG :{}", payload)).await?;
            }
            "CAP" => {
                // Handle SASL capability negotiation
                if msg.params.len() >= 3 && msg.params[1] == "ACK" {
                    let caps = &msg.params[2];
                    if caps.contains("sasl") {
                        send_raw(writer, "AUTHENTICATE PLAIN").await?;
                    }
                }
            }
            "AUTHENTICATE" => {
                if msg.params.first().map(|s| s.as_str()) == Some("+") {
                    if let Some(ref sasl_pass) = config.sasl_password {
                        let encoded = encode_sasl_plain(user, sasl_pass);
                        send_raw(writer, &format!("AUTHENTICATE {}", encoded)).await?;
                    }
                }
            }
            // SASL success
            "903" => {
                info!("IRC: SASL authentication successful");
                _sasl_done = true;
                send_raw(writer, "CAP END").await?;
            }
            // SASL failure
            "904" | "905" | "906" | "907" => {
                warn!("IRC: SASL authentication failed ({})", msg.command);
                _sasl_done = true;
                send_raw(writer, "CAP END").await?;
            }
            // RPL_WELCOME (001) — registration complete
            "001" => {
                info!("IRC: registered as {}", nick);
                registered = true;

                // Identify with NickServ if configured
                if let Some(ref ns_pass) = config.nickserv_password {
                    if !ns_pass.is_empty() {
                        send_raw(writer, &format!("PRIVMSG NickServ :IDENTIFY {}", ns_pass))
                            .await?;
                    }
                }

                // Join channels
                for channel in &config.channels {
                    if !channel.is_empty() {
                        send_raw(writer, &format!("JOIN {}", channel)).await?;
                        info!("IRC: joining {}", channel);
                    }
                }
            }
            "PRIVMSG" => {
                if !registered {
                    continue;
                }
                handle_privmsg(&msg, nick, allow_from, tx).await;
            }
            // Nickname in use
            "433" => {
                let alt = format!("{}_", nick);
                warn!("IRC: nickname in use, trying {}", alt);
                send_raw(writer, &format!("NICK {}", alt)).await?;
            }
            // Error
            "ERROR" => {
                let reason = msg.params.first().map(|s| s.as_str()).unwrap_or("unknown");
                return Err(format!("IRC ERROR: {}", reason));
            }
            _ => {}
        }
    }

    Ok(())
}

/// Handle an incoming PRIVMSG and forward it as an InboundMessage.
async fn handle_privmsg(
    msg: &IrcMessage,
    my_nick: &str,
    allow_from: &[String],
    tx: &mpsc::Sender<InboundMessage>,
) {
    let sender_nick = match msg.nick() {
        Some(n) => n,
        None => return,
    };

    // Allowlist check
    if !allow_from.is_empty() && !allow_from.contains(&sender_nick.to_string()) {
        debug!("IRC: ignoring message from non-allowed user {}", sender_nick);
        return;
    }

    // params[0] = target (channel or our nick for DM), params[1] = message text
    if msg.params.len() < 2 {
        return;
    }

    let target = &msg.params[0];
    let content = &msg.params[1];

    if content.is_empty() {
        return;
    }

    // Determine chat_id: if target is our nick, it's a DM — reply to sender
    let chat_id = if target.eq_ignore_ascii_case(my_nick) {
        sender_nick.to_string()
    } else {
        target.clone()
    };

    let inbound = InboundMessage::new("irc", sender_nick, &chat_id, content);

    if let Err(e) = tx.send(inbound).await {
        error!("IRC: failed to send inbound message: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_privmsg() {
        let msg = IrcMessage::parse(":nick!user@host PRIVMSG #channel :Hello world").unwrap();
        assert_eq!(msg.command, "PRIVMSG");
        assert_eq!(msg.nick(), Some("nick"));
        assert_eq!(msg.params[0], "#channel");
        assert_eq!(msg.params[1], "Hello world");
    }

    #[test]
    fn test_parse_ping() {
        let msg = IrcMessage::parse("PING :server.example.com").unwrap();
        assert_eq!(msg.command, "PING");
        assert_eq!(msg.params[0], "server.example.com");
        assert!(msg.prefix.is_none());
    }

    #[test]
    fn test_parse_numeric() {
        let msg = IrcMessage::parse(":server 001 botnick :Welcome to IRC").unwrap();
        assert_eq!(msg.command, "001");
        assert_eq!(msg.prefix, Some("server".to_string()));
        assert_eq!(msg.params[0], "botnick");
        assert_eq!(msg.params[1], "Welcome to IRC");
    }

    #[test]
    fn test_parse_empty() {
        assert!(IrcMessage::parse("").is_none());
        assert!(IrcMessage::parse("\r\n").is_none());
    }

    #[test]
    fn test_parse_no_trailing() {
        let msg = IrcMessage::parse(":nick!u@h JOIN #channel").unwrap();
        assert_eq!(msg.command, "JOIN");
        assert_eq!(msg.params[0], "#channel");
    }

    #[test]
    fn test_nick_extraction() {
        let msg = IrcMessage::parse(":alice!alice@host PRIVMSG #ch :hi").unwrap();
        assert_eq!(msg.nick(), Some("alice"));

        let msg2 = IrcMessage::parse(":server 001 bot :Welcome").unwrap();
        assert_eq!(msg2.nick(), Some("server"));
    }

    #[test]
    fn test_split_message_short() {
        let chunks = split_message("hello", 400);
        assert_eq!(chunks, vec!["hello"]);
    }

    #[test]
    fn test_split_message_long() {
        let long = "a".repeat(500);
        let chunks = split_message(&long, 200);
        assert!(chunks.len() >= 3);
        for chunk in &chunks {
            assert!(chunk.len() <= 200);
        }
    }

    #[test]
    fn test_split_message_prefers_space() {
        let text = "hello world this is a test message";
        let chunks = split_message(text, 15);
        // Should split at word boundaries
        assert!(chunks.len() > 1);
        for chunk in &chunks {
            assert!(chunk.len() <= 15);
        }
    }

    #[test]
    fn test_encode_sasl_plain() {
        let encoded = encode_sasl_plain("user", "pass");
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(&encoded)
            .unwrap();
        assert_eq!(decoded, b"\0user\0pass");
    }

    #[test]
    fn test_allowlist_empty_allows_all() {
        let config = IrcConfig::default();
        let handler = IrcHandler::new(config);
        assert!(handler.is_allowed("anyone"));
    }

    #[test]
    fn test_allowlist_restricts() {
        let mut config = IrcConfig::default();
        config.allow_from = vec!["alice".to_string()];
        let handler = IrcHandler::new(config);
        assert!(handler.is_allowed("alice"));
        assert!(!handler.is_allowed("bob"));
    }

    #[test]
    fn test_config_defaults() {
        let config = IrcConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.port, 6697);
        assert!(config.use_tls);
        assert!(config.verify_tls);
    }

    #[test]
    fn test_config_deserialize_minimal() {
        let json = r##"{
            "enabled": true,
            "server": "irc.libera.chat",
            "nickname": "divabot",
            "channels": ["#test"]
        }"##;
        let config: IrcConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert_eq!(config.server, "irc.libera.chat");
        assert_eq!(config.port, 6697); // default
        assert!(config.use_tls); // default
    }
}
