//! QQ channel implementation using WebSocket
//!
//! This implementation is based on the QQ Bot OpenAPI (botpy SDK).
//! It uses WebSocket for real-time message reception and HTTP API for sending messages.
//!
//! Key features:
//! - WebSocket gateway connection with automatic reconnection
//! - Token-based authentication with automatic refresh
//! - Heartbeat mechanism with ACK timeout tolerance
//! - C2C (user-to-bot) private message support
//! - Message deduplication
//! - Allowlist-based access control

use agent_diva_core::bus::OutboundMessage;
use agent_diva_core::config::schema::QQConfig;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use reqwest_qq::Client as HttpClient;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, sleep, Instant};
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
const DEFAULT_HEARTBEAT_INTERVAL_MS: u64 = 41_250;
const MAX_MISSED_HEARTBEAT_ACKS: u32 = 3;
const DEFAULT_RECONNECT_BACKOFF_SECS: u64 = 5;
const INVALID_SESSION_BACKOFF_SECS: [u64; 4] = [5, 15, 30, 60];
const INVALID_SESSION_COOLDOWN_THRESHOLD: u32 = 5;
const INVALID_SESSION_COOLDOWN_SECS: u64 = 300;

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
        if let Ok(token) = std::env::var("QQ_ACCESS_TOKEN_OVERRIDE") {
            self.access_token = Some(token);
            self.expires_at = u64::MAX;
            return Ok(());
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if self.access_token.is_some() && now < self.expires_at {
            return Ok(());
        }

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

            self.expires_at = now + expires_in - 60;

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

#[derive(Debug, Default)]
struct SessionState {
    session_id: Option<String>,
    last_sequence: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AttemptMode {
    Identify,
    Resume,
}

impl AttemptMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Identify => "identify",
            Self::Resume => "resume",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitReason {
    ConnectFailed,
    Reconnect,
    InvalidSession,
    Close,
    StreamEnded,
    HeartbeatTimeout,
    WriteFailed,
    Shutdown,
}

impl ExitReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::ConnectFailed => "connect_failed",
            Self::Reconnect => "reconnect",
            Self::InvalidSession => "invalid_session",
            Self::Close => "close",
            Self::StreamEnded => "stream_ended",
            Self::HeartbeatTimeout => "heartbeat_timeout",
            Self::WriteFailed => "write_failed",
            Self::Shutdown => "shutdown",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ConnectionOutcome {
    exit_reason: ExitReason,
    session_established: bool,
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
    session_state: Arc<RwLock<SessionState>>,
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
            session_state: Arc::new(RwLock::new(SessionState::default())),
            shutdown_tx: None,
            inbound_tx: None,
        }
    }

    fn api_base() -> String {
        std::env::var("QQ_API_BASE_OVERRIDE").unwrap_or_else(|_| API_BASE.to_string())
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
        if let Ok(url) = std::env::var("QQ_GATEWAY_URL_OVERRIDE") {
            return Ok(GatewayInfo {
                url,
                shards: 1,
                session_start_limit: SessionStartLimit {
                    total: 1,
                    remaining: 1,
                    reset_after: 0,
                    max_concurrency: 1,
                },
            });
        }

        let url = format!("{}/gateway/bot", Self::api_base());
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

    fn build_identify_payload(token: &Token) -> WsPayload {
        WsPayload {
            op: WS_IDENTITY,
            d: Some(json!({
                "token": token.get_string(),
                "intents": 1 << 25 | 1 << 12,
                "properties": {
                    "os": "windows",
                    "browser": "agent-diva",
                    "device": "agent-diva",
                },
            })),
            s: None,
            t: None,
        }
    }

    fn build_resume_payload(token: &Token, session_id: &str, seq: u64) -> WsPayload {
        WsPayload {
            op: WS_RESUME,
            d: Some(json!({
                "token": token.get_string(),
                "session_id": session_id,
                "seq": seq,
            })),
            s: None,
            t: None,
        }
    }

    fn reconnect_backoff() -> Duration {
        Self::duration_from_env_ms("QQ_WS_TEST_RECONNECT_DELAY_MS")
            .unwrap_or_else(|| Duration::from_secs(DEFAULT_RECONNECT_BACKOFF_SECS))
    }

    fn invalid_session_backoff(streak: u32) -> Duration {
        if let Some(overrides) =
            Self::duration_list_from_env_ms("QQ_WS_TEST_INVALID_SESSION_BACKOFF_MS")
        {
            let index = streak.saturating_sub(1) as usize;
            return overrides
                .get(index)
                .copied()
                .or_else(|| overrides.last().copied())
                .unwrap_or_else(Self::reconnect_backoff);
        }

        let index = streak.saturating_sub(1) as usize;
        let secs = INVALID_SESSION_BACKOFF_SECS
            .get(index)
            .copied()
            .unwrap_or(*INVALID_SESSION_BACKOFF_SECS.last().unwrap());
        Duration::from_secs(secs)
    }

    fn invalid_session_cooldown() -> Duration {
        Self::duration_from_env_ms("QQ_WS_TEST_INVALID_SESSION_COOLDOWN_MS")
            .unwrap_or_else(|| Duration::from_secs(INVALID_SESSION_COOLDOWN_SECS))
    }

    fn duration_from_env_ms(var: &str) -> Option<Duration> {
        std::env::var(var)
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_millis)
    }

    fn duration_list_from_env_ms(var: &str) -> Option<Vec<Duration>> {
        let value = std::env::var(var).ok()?;
        let parsed: Vec<Duration> = value
            .split(',')
            .filter_map(|part| part.trim().parse::<u64>().ok())
            .map(Duration::from_millis)
            .collect();
        if parsed.is_empty() {
            None
        } else {
            Some(parsed)
        }
    }

    async fn sleep_or_shutdown(
        shutdown_rx: &mut mpsc::Receiver<()>,
        duration: Duration,
    ) -> ExitReason {
        tokio::select! {
            _ = sleep(duration) => ExitReason::Reconnect,
            _ = shutdown_rx.recv() => ExitReason::Shutdown,
        }
    }

    async fn run_websocket_once(
        &self,
        gateway_url: &str,
        shutdown_rx: &mut mpsc::Receiver<()>,
        invalid_session_streak: u32,
    ) -> ConnectionOutcome {
        let (ws_stream, _) = match connect_async(gateway_url).await {
            Ok(result) => result,
            Err(e) => {
                error!("QQ WebSocket connection failed: {}", e);
                return ConnectionOutcome {
                    exit_reason: ExitReason::ConnectFailed,
                    session_established: false,
                };
            }
        };

        info!("QQ WebSocket connected");
        let (mut write, mut read) = ws_stream.split();

        let hello_text = match read.next().await {
            Some(Ok(WsMessage::Text(text))) => text,
            Some(Ok(WsMessage::Ping(payload))) => {
                if let Err(e) = write.send(WsMessage::Pong(payload)).await {
                    error!("Failed to respond to QQ gateway ping before HELLO: {}", e);
                    return ConnectionOutcome {
                        exit_reason: ExitReason::WriteFailed,
                        session_established: false,
                    };
                }
                match read.next().await {
                    Some(Ok(WsMessage::Text(text))) => text,
                    Some(Ok(other)) => {
                        warn!("Unexpected frame while waiting for QQ HELLO: {:?}", other);
                        return ConnectionOutcome {
                            exit_reason: ExitReason::StreamEnded,
                            session_established: false,
                        };
                    }
                    Some(Err(e)) => {
                        error!("Failed to receive QQ HELLO after ping: {}", e);
                        return ConnectionOutcome {
                            exit_reason: ExitReason::StreamEnded,
                            session_established: false,
                        };
                    }
                    None => {
                        warn!("QQ gateway closed before HELLO");
                        return ConnectionOutcome {
                            exit_reason: ExitReason::StreamEnded,
                            session_established: false,
                        };
                    }
                }
            }
            Some(Ok(other)) => {
                warn!("Unexpected first QQ frame: {:?}", other);
                return ConnectionOutcome {
                    exit_reason: ExitReason::StreamEnded,
                    session_established: false,
                };
            }
            Some(Err(e)) => {
                error!("Failed to receive QQ HELLO: {}", e);
                return ConnectionOutcome {
                    exit_reason: ExitReason::StreamEnded,
                    session_established: false,
                };
            }
            None => {
                warn!("QQ gateway closed before HELLO");
                return ConnectionOutcome {
                    exit_reason: ExitReason::StreamEnded,
                    session_established: false,
                };
            }
        };

        let heartbeat_interval_ms = serde_json::from_str::<WsPayload>(&hello_text)
            .ok()
            .filter(|payload| payload.op == WS_HELLO)
            .and_then(|payload| payload.d)
            .and_then(|data| data.get("heartbeat_interval").and_then(Value::as_u64))
            .unwrap_or(DEFAULT_HEARTBEAT_INTERVAL_MS);
        let heartbeat_grace_ms = (heartbeat_interval_ms / 10).min(5_000);
        let effective_heartbeat_ms = heartbeat_interval_ms.saturating_add(heartbeat_grace_ms);

        let (stored_session_id, stored_last_seq) = {
            let state = self.session_state.read().await;
            (state.session_id.clone(), state.last_sequence)
        };
        let has_session = stored_session_id.is_some();
        let has_seq = stored_last_seq.is_some();

        let token = self.token.read().await;
        let (attempt_mode, outbound) = if let (Some(session_id), Some(seq)) =
            (stored_session_id.as_deref(), stored_last_seq)
        {
            (
                AttemptMode::Resume,
                Self::build_resume_payload(&token, session_id, seq),
            )
        } else {
            (AttemptMode::Identify, Self::build_identify_payload(&token))
        };
        drop(token);

        info!(
            attempt_mode = attempt_mode.as_str(),
            has_session, has_seq, invalid_session_streak, "QQ gateway handshake attempt"
        );

        if let Err(e) = write
            .send(WsMessage::Text(serde_json::to_string(&outbound).unwrap()))
            .await
        {
            error!(
                attempt_mode = attempt_mode.as_str(),
                "Failed to send QQ identify/resume: {}", e
            );
            return ConnectionOutcome {
                exit_reason: ExitReason::WriteFailed,
                session_established: false,
            };
        }

        let mut last_seq = stored_last_seq.unwrap_or(0);
        let mut heartbeat_ticker = interval(Duration::from_millis(effective_heartbeat_ms));
        let mut missed_heartbeat_acks = 0;
        let mut session_established = false;

        loop {
            tokio::select! {
                _ = heartbeat_ticker.tick() => {
                    let heartbeat = WsPayload {
                        op: WS_HEARTBEAT,
                        d: Some(json!(if last_seq > 0 { last_seq } else { 0 })),
                        s: None,
                        t: None,
                    };

                    if let Err(e) = write
                        .send(WsMessage::Text(serde_json::to_string(&heartbeat).unwrap()))
                        .await
                    {
                        error!("QQ heartbeat failed: {}", e);
                        return ConnectionOutcome {
                            exit_reason: ExitReason::WriteFailed,
                            session_established,
                        };
                    }
                    missed_heartbeat_acks += 1;
                }
                _ = shutdown_rx.recv() => {
                    info!("QQ WebSocket shutting down");
                    return ConnectionOutcome {
                        exit_reason: ExitReason::Shutdown,
                        session_established,
                    };
                }
                msg = read.next() => {
                    let msg = match msg {
                        Some(msg) => msg,
                        None => {
                            warn!("QQ WebSocket stream ended unexpectedly");
                            return ConnectionOutcome {
                                exit_reason: ExitReason::StreamEnded,
                                session_established,
                            };
                        }
                    };

                    match msg {
                        Ok(WsMessage::Text(text)) => {
                            let payload = match serde_json::from_str::<WsPayload>(&text) {
                                Ok(payload) => payload,
                                Err(e) => {
                                    debug!("Ignoring QQ payload parse failure: {}", e);
                                    continue;
                                }
                            };

                            if let Some(seq) = payload.s.filter(|seq| *seq > 0) {
                                last_seq = seq;
                                self.session_state.write().await.last_sequence = Some(seq);
                            }

                            match payload.op {
                                WS_DISPATCH_EVENT => {
                                    if let Some(event_type) = payload.t.as_deref() {
                                        if matches!(event_type, "READY" | "RESUMED") {
                                            session_established = true;
                                            if let Some(session_id) = payload
                                                .d
                                                .as_ref()
                                                .and_then(|d| d.get("session_id"))
                                                .and_then(Value::as_str)
                                            {
                                                let mut state = self.session_state.write().await;
                                                state.session_id = Some(session_id.to_string());
                                                state.last_sequence = Some(last_seq);
                                                info!(
                                                    session_id,
                                                    sequence = last_seq,
                                                    invalid_session_streak,
                                                    "QQ session established via {} and invalid session streak reset",
                                                    event_type
                                                );
                                            } else {
                                                warn!(
                                                    event_type,
                                                    "QQ session event missing session_id"
                                                );
                                            }
                                        }

                                        self.handle_event(event_type, payload.d.clone()).await;
                                    }
                                }
                                WS_HEARTBEAT => {
                                    let heartbeat = WsPayload {
                                        op: WS_HEARTBEAT,
                                        d: Some(json!(if last_seq > 0 { last_seq } else { 0 })),
                                        s: None,
                                        t: None,
                                    };

                                    if let Err(e) = write
                                        .send(WsMessage::Text(
                                            serde_json::to_string(&heartbeat).unwrap(),
                                        ))
                                        .await
                                    {
                                        error!("QQ immediate heartbeat failed: {}", e);
                                        return ConnectionOutcome {
                                            exit_reason: ExitReason::WriteFailed,
                                            session_established,
                                        };
                                    }
                                    missed_heartbeat_acks += 1;
                                }
                                WS_HEARTBEAT_ACK => {
                                    missed_heartbeat_acks = 0;
                                    debug!("QQ heartbeat ACK received");
                                }
                                WS_RECONNECT => {
                                    info!("QQ server requested reconnect");
                                    return ConnectionOutcome {
                                        exit_reason: ExitReason::Reconnect,
                                        session_established,
                                    };
                                }
                                WS_INVALID_SESSION => {
                                    let next_backoff = Self::invalid_session_backoff(
                                        invalid_session_streak.saturating_add(1),
                                    );
                                    let cooldown_entered =
                                        invalid_session_streak.saturating_add(1)
                                            >= INVALID_SESSION_COOLDOWN_THRESHOLD;
                                    warn!(
                                        streak = invalid_session_streak.saturating_add(1),
                                        next_backoff_secs = next_backoff.as_secs(),
                                        cooldown_entered,
                                        "QQ invalid session, clearing session state"
                                    );
                                    let mut state = self.session_state.write().await;
                                    state.session_id = None;
                                    state.last_sequence = None;
                                    return ConnectionOutcome {
                                        exit_reason: ExitReason::InvalidSession,
                                        session_established,
                                    };
                                }
                                WS_HELLO => {
                                    debug!("Ignoring duplicate QQ HELLO");
                                }
                                _ => {
                                    debug!("Unknown opcode: {}", payload.op);
                                }
                            }
                        }
                        Ok(WsMessage::Ping(payload)) => {
                            if let Err(e) = write.send(WsMessage::Pong(payload)).await {
                                error!("Failed to send QQ pong: {}", e);
                                return ConnectionOutcome {
                                    exit_reason: ExitReason::WriteFailed,
                                    session_established,
                                };
                            }
                        }
                        Ok(WsMessage::Pong(_)) => {}
                        Ok(WsMessage::Close(_)) => {
                            info!("QQ WebSocket closed by server");
                            return ConnectionOutcome {
                                exit_reason: ExitReason::Close,
                                session_established,
                            };
                        }
                        Err(e) => {
                            error!("QQ WebSocket error: {}", e);
                            return ConnectionOutcome {
                                exit_reason: ExitReason::StreamEnded,
                                session_established,
                            };
                        }
                        _ => {}
                    }
                }
            }

            if missed_heartbeat_acks >= MAX_MISSED_HEARTBEAT_ACKS {
                warn!(
                    "QQ heartbeat ACK timeout after {} consecutive misses",
                    MAX_MISSED_HEARTBEAT_ACKS
                );
                return ConnectionOutcome {
                    exit_reason: ExitReason::HeartbeatTimeout,
                    session_established,
                };
            }
        }
    }

    /// Run WebSocket connection
    async fn run_websocket(
        &self,
        gateway_url: String,
        mut shutdown_rx: mpsc::Receiver<()>,
    ) -> Result<()> {
        let mut consecutive_invalid_sessions = 0u32;
        let mut cooldown_until: Option<Instant> = None;

        loop {
            if shutdown_rx.try_recv().is_ok() {
                info!("QQ WebSocket shutting down");
                break;
            }

            if let Some(until) = cooldown_until {
                let now = Instant::now();
                if until > now {
                    let remaining = until.duration_since(now);
                    info!(
                        cooldown_secs = remaining.as_secs(),
                        "QQ invalid session cooldown active before reconnect"
                    );
                    if Self::sleep_or_shutdown(&mut shutdown_rx, remaining).await
                        == ExitReason::Shutdown
                    {
                        info!("QQ WebSocket shutting down");
                        break;
                    }
                }
                cooldown_until = None;
                consecutive_invalid_sessions = 0;
            }

            let outcome = self
                .run_websocket_once(&gateway_url, &mut shutdown_rx, consecutive_invalid_sessions)
                .await;

            if outcome.session_established && consecutive_invalid_sessions > 0 {
                info!(
                    previous_invalid_session_streak = consecutive_invalid_sessions,
                    "QQ gateway session recovered, resetting invalid session streak"
                );
                consecutive_invalid_sessions = 0;
            }

            match outcome.exit_reason {
                ExitReason::Shutdown => break,
                ExitReason::InvalidSession => {
                    consecutive_invalid_sessions = consecutive_invalid_sessions.saturating_add(1);
                    let next_backoff = Self::invalid_session_backoff(consecutive_invalid_sessions);

                    if consecutive_invalid_sessions >= INVALID_SESSION_COOLDOWN_THRESHOLD {
                        let cooldown = Self::invalid_session_cooldown();
                        cooldown_until = Some(Instant::now() + cooldown);
                        error!(
                            streak = consecutive_invalid_sessions,
                            cooldown_secs = cooldown.as_secs(),
                            "QQ invalid session storm detected, entering cooldown"
                        );
                        continue;
                    }

                    warn!(
                        reason = outcome.exit_reason.as_str(),
                        streak = consecutive_invalid_sessions,
                        next_backoff_secs = next_backoff.as_secs(),
                        "QQ WebSocket scheduling invalid-session reconnect"
                    );
                    if Self::sleep_or_shutdown(&mut shutdown_rx, next_backoff).await
                        == ExitReason::Shutdown
                    {
                        info!("QQ WebSocket shutting down");
                        break;
                    }
                }
                reason => {
                    let backoff = Self::reconnect_backoff();
                    info!(
                        reason = reason.as_str(),
                        reconnect_delay_secs = backoff.as_secs(),
                        "QQ WebSocket reconnect scheduled"
                    );
                    if Self::sleep_or_shutdown(&mut shutdown_rx, backoff).await
                        == ExitReason::Shutdown
                    {
                        info!("QQ WebSocket shutting down");
                        break;
                    }
                }
            }
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
            "resumed" => {
                info!("QQ Bot session resumed");
            }
            _ => {
                debug!("Unhandled event type: {}", event_type);
            }
        }
    }

    /// Handle C2C (user-to-bot) message
    async fn handle_c2c_message(&self, data: Value) {
        let message: C2CMessage = match serde_json::from_value(data) {
            Ok(msg) => msg,
            Err(e) => {
                error!("Failed to parse C2C message: {}", e);
                return;
            }
        };

        if self.is_processed_qq(&message.id).await {
            return;
        }
        self.mark_processed_qq(message.id.clone()).await;

        let user_id = message
            .author
            .user_openid
            .or(message.author.id)
            .unwrap_or_else(|| "unknown".to_string());

        if !self.is_allowed(&user_id) {
            debug!("User {} not in allowlist", user_id);
            return;
        }

        let content = match message.content {
            Some(c) if !c.trim().is_empty() => c.trim().to_string(),
            _ => {
                debug!("Empty message content");
                return;
            }
        };

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

        {
            let mut token = self.token.write().await;
            token.check_token(&self.http_client).await?;
        }

        let gateway = self.get_gateway().await?;
        info!("QQ Gateway URL: {}", gateway.url);

        *self.running.write().await = true;

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

        {
            let mut token = self.token.write().await;
            token.check_token(&self.http_client).await?;
        }

        let url = format!("{}/v2/users/{}/messages", Self::api_base(), msg.chat_id);
        let token = self.token.read().await;

        let mut body = json!({
            "msg_type": 0,
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
            session_state: Arc::clone(&self.session_state),
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

    #[test]
    fn test_invalid_session_backoff_progression() {
        assert_eq!(
            QQHandler::invalid_session_backoff(1),
            Duration::from_secs(5)
        );
        assert_eq!(
            QQHandler::invalid_session_backoff(2),
            Duration::from_secs(15)
        );
        assert_eq!(
            QQHandler::invalid_session_backoff(3),
            Duration::from_secs(30)
        );
        assert_eq!(
            QQHandler::invalid_session_backoff(4),
            Duration::from_secs(60)
        );
        assert_eq!(
            QQHandler::invalid_session_backoff(10),
            Duration::from_secs(60)
        );
    }

    #[test]
    fn test_reconnect_backoff_default() {
        std::env::remove_var("QQ_WS_TEST_RECONNECT_DELAY_MS");
        assert_eq!(
            QQHandler::reconnect_backoff(),
            Duration::from_secs(DEFAULT_RECONNECT_BACKOFF_SECS)
        );
    }
}
