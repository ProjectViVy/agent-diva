//! Native QQ Bot Open Platform v2 adapter.
//!
//! QQ uses an outbound WebSocket gateway for ingress and the official REST
//! API for token exchange and replies.  This module intentionally does not
//! call the legacy `QQHandler`; all transport state, deduplication, admission,
//! and receipts are owned by the C5 adapter contract.

use crate::adapter::{
    accepted_receipt, execution_error, external_message_envelope, is_sender_allowed,
    AdapterContext, AdapterError, AdapterServices, ChannelAdapter,
};
use agent_diva_core::channel::{
    ChannelAddress, ChannelCapabilities, ChannelCapability, ChannelCommand, ChannelDirection,
    ChannelEnvelopeV1, ChannelHealth, ChannelHealthStatus, ChannelId, ChannelOrigin,
    ChannelPayloadV1, ContentPart, Correlation, DeliveryReceipt, FabricAdmissionError, StreamPhase,
};
use agent_diva_core::config::schema::QQConfig;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use reqwest::StatusCode;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex as AsyncMutex, RwLock};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tokio_util::sync::CancellationToken;

const CHANNEL: &str = "qq";
const TOKEN_URL: &str = "https://bots.qq.com/app/getAppAccessToken";
const API_BASE: &str = "https://api.sgroup.qq.com";
const GATEWAY_PATH: &str = "/gateway";
const MAX_MESSAGE_CHARS: usize = 4_000;
const TOKEN_REFRESH_MARGIN: Duration = Duration::from_secs(60);
const HEARTBEAT_FALLBACK: Duration = Duration::from_secs(41);
const RECONNECT_BASE: Duration = Duration::from_secs(5);
const RECONNECT_MAX: Duration = Duration::from_secs(60);
const INGRESS_ADMISSION_DEADLINE: Duration = Duration::from_secs(2);
const MIN_SEND_INTERVAL: Duration = Duration::from_millis(100);
// Keep DIVA's previously deployed intent mask until D-013 is resolved with
// an official event-delivery fixture.  The group parser is implemented, but a
// capability claim cannot silently change the production subscription bits.
const INTENTS: u32 = (1 << 25) | (1 << 12);

#[derive(Debug, Clone)]
struct QqEndpoint {
    api_base: String,
    token_url: String,
}

impl QqEndpoint {
    fn production() -> Self {
        Self {
            api_base: API_BASE.to_string(),
            token_url: TOKEN_URL.to_string(),
        }
    }

    #[cfg(test)]
    fn test(base: &str) -> Self {
        let base = base.trim_end_matches('/').to_string();
        Self {
            token_url: format!("{base}/app/getAppAccessToken"),
            api_base: base,
        }
    }
}

#[derive(Debug, Default)]
struct TokenState {
    token: Option<String>,
    expires_at: Option<Instant>,
    refreshing: bool,
}

#[derive(Debug, Default)]
struct SessionState {
    session_id: Option<String>,
    sequence: Option<u64>,
    heartbeat_interval: Option<Duration>,
    awaiting_heartbeat_ack: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChatKind {
    Direct,
    Group,
}

#[derive(Debug, Deserialize)]
struct GatewayResponse {
    url: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    expires_in: Option<FlexibleU64>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum FlexibleU64 {
    Number(u64),
    Text(String),
}

impl FlexibleU64 {
    fn value(&self) -> Option<u64> {
        match self {
            Self::Number(value) => Some(*value),
            Self::Text(value) => value.parse().ok(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GatewayFrame {
    op: u8,
    #[serde(default)]
    d: Option<Value>,
    #[serde(default)]
    s: Option<u64>,
    #[serde(default)]
    t: Option<String>,
}

/// Native QQ Bot adapter.  Group text and C2C text are supported; QQ media
/// upload and interactive commands remain explicitly unsupported until an
/// official request/response fixture is added to the migration evidence.
pub struct QqAdapter {
    config: QQConfig,
    endpoint: QqEndpoint,
    http: reqwest::Client,
    token: Arc<AsyncMutex<TokenState>>,
    session: Arc<AsyncMutex<SessionState>>,
    seen: Arc<RwLock<HashSet<String>>>,
    health: Arc<Mutex<ChannelHealth>>,
    running: Arc<AtomicBool>,
    cancel: CancellationToken,
    next_sequence: AtomicU64,
    last_send: AsyncMutex<Option<Instant>>,
}

impl QqAdapter {
    pub(crate) fn new(config: QQConfig, services: AdapterServices) -> Self {
        Self::from_endpoint(config, services, QqEndpoint::production())
    }

    fn from_endpoint(config: QQConfig, services: AdapterServices, endpoint: QqEndpoint) -> Self {
        // QQ media upload is intentionally not advertised until the official
        // file-upload request/response fixture is landed. Keep the common
        // service seam in the constructor for C6 assembly compatibility.
        let _ = services;
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            endpoint,
            http,
            token: Arc::new(AsyncMutex::new(TokenState::default())),
            session: Arc::new(AsyncMutex::new(SessionState::default())),
            seen: Arc::new(RwLock::new(HashSet::new())),
            health: Arc::new(Mutex::new(ChannelHealth::new(ChannelHealthStatus::Unknown))),
            running: Arc::new(AtomicBool::new(false)),
            cancel: CancellationToken::new(),
            next_sequence: AtomicU64::new(1),
            last_send: AsyncMutex::new(None),
        }
    }

    #[cfg(test)]
    fn with_test_endpoint(config: QQConfig, services: AdapterServices, base: &str) -> Self {
        Self::from_endpoint(config, services, QqEndpoint::test(base))
    }

    fn static_capabilities() -> ChannelCapabilities {
        let mut capabilities = ChannelCapabilities::new([
            ChannelCapability::IngressText,
            ChannelCapability::IngressGroup,
            ChannelCapability::IngressDirect,
            ChannelCapability::IngressDedupId,
            ChannelCapability::EgressText,
            ChannelCapability::EgressChunking,
            ChannelCapability::EgressReply,
            ChannelCapability::ReliabilityHealth,
            ChannelCapability::ReliabilityHeartbeat,
            ChannelCapability::ReliabilityResume,
            ChannelCapability::ReliabilityTokenRefresh,
            ChannelCapability::ReliabilityPacing,
            ChannelCapability::ReliabilitySupervisedRestart,
        ]);
        capabilities.limits.max_text_chars = Some(MAX_MESSAGE_CHARS);
        capabilities.limits.rate_limit_hint_ms = Some(MIN_SEND_INTERVAL.as_millis() as u64);
        capabilities.limits.supported_mime_types = BTreeSet::new();
        capabilities
    }

    fn set_health(&self, status: ChannelHealthStatus, diagnosis: Option<String>) {
        if let Ok(mut health) = self.health.lock() {
            health.status = status;
            health.diagnosis = diagnosis;
            health.checked_at = chrono::Utc::now();
        }
    }

    fn validate_config(&self) -> Result<(), AdapterError> {
        if !self.config.enabled {
            return Err(execution_error(
                "not_configured",
                "QQ channel is disabled",
                None,
                false,
            ));
        }
        let mut missing = Vec::new();
        if self.config.app_id.trim().is_empty() {
            missing.push("app_id");
        }
        if self.config.secret.trim().is_empty() {
            missing.push("secret");
        }
        if missing.is_empty() {
            Ok(())
        } else {
            Err(execution_error(
                "invalid_config",
                format!("missing QQ fields: {}", missing.join(", ")),
                None,
                false,
            ))
        }
    }

    fn ensure_supported(&self, command: &ChannelCommand) -> Result<(), AdapterError> {
        let target = command
            .target_channel()
            .map_err(|error| execution_error("invalid_target", error.to_string(), None, false))?;
        if target.as_str() != CHANNEL {
            return Err(execution_error(
                "wrong_channel",
                format!("command targets {}, adapter is {CHANNEL}", target.as_str()),
                None,
                false,
            ));
        }
        if let Some(capability) = command
            .required_capabilities()
            .into_iter()
            .find(|capability| !self.capabilities().supports(*capability))
        {
            return Err(AdapterError::UnsupportedCapability { capability });
        }
        Ok(())
    }

    async fn access_token(&self) -> Result<String, AdapterError> {
        {
            let state = self.token.lock().await;
            if let (Some(token), Some(expires_at)) = (&state.token, state.expires_at) {
                if expires_at > Instant::now() + TOKEN_REFRESH_MARGIN {
                    return Ok(token.clone());
                }
            }
            if state.refreshing {
                drop(state);
                for _ in 0..50 {
                    sleep(Duration::from_millis(20)).await;
                    let state = self.token.lock().await;
                    if let Some(token) = &state.token {
                        if state
                            .expires_at
                            .is_some_and(|expiry| expiry > Instant::now())
                        {
                            return Ok(token.clone());
                        }
                    }
                }
                return Err(execution_error(
                    "token_refresh_timeout",
                    "QQ token refresh did not complete",
                    None,
                    true,
                ));
            }
        }

        {
            let mut state = self.token.lock().await;
            if let (Some(token), Some(expires_at)) = (&state.token, state.expires_at) {
                if expires_at > Instant::now() + TOKEN_REFRESH_MARGIN {
                    return Ok(token.clone());
                }
            }
            state.refreshing = true;
        }

        let request = self
            .http
            .post(&self.endpoint.token_url)
            .json(&json!({"appId": self.config.app_id, "clientSecret": self.config.secret}))
            .send()
            .await;
        let result = match request {
            Ok(response) => {
                let status = response.status();
                if status == StatusCode::TOO_MANY_REQUESTS {
                    let retry_after = response
                        .headers()
                        .get("retry-after")
                        .and_then(|value| value.to_str().ok())
                        .and_then(|value| value.parse::<u64>().ok())
                        .unwrap_or(1);
                    let _ = response.bytes().await;
                    Err(AdapterError::RateLimited {
                        retry_after: Duration::from_secs(retry_after),
                    })
                } else {
                    match response.json::<TokenResponse>().await {
                        Ok(parsed)
                            if status.is_success() && !parsed.access_token.trim().is_empty() =>
                        {
                            let expires_in = parsed
                                .expires_in
                                .as_ref()
                                .and_then(FlexibleU64::value)
                                .unwrap_or(7_200);
                            let safe_expiry = Duration::from_secs(
                                expires_in.saturating_sub(TOKEN_REFRESH_MARGIN.as_secs()),
                            );
                            let token = parsed.access_token;
                            let mut state = self.token.lock().await;
                            state.token = Some(token.clone());
                            state.expires_at = Some(Instant::now() + safe_expiry);
                            state.refreshing = false;
                            Ok(token)
                        }
                        Ok(_) => Err(execution_error(
                            "token_refresh",
                            format!("QQ token endpoint returned {status}"),
                            None,
                            status.is_server_error(),
                        )),
                        Err(error) => Err(execution_error(
                            "token_response",
                            error.to_string(),
                            None,
                            true,
                        )),
                    }
                }
            }
            Err(error) => Err(execution_error(
                "token_transport",
                error.to_string(),
                None,
                true,
            )),
        };
        if result.is_err() {
            self.token.lock().await.refreshing = false;
        }
        result
    }

    async fn fetch_gateway_url(&self, token: &str) -> Result<String, AdapterError> {
        let response = self
            .http
            .get(format!("{}{}", self.endpoint.api_base, GATEWAY_PATH))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .map_err(|error| execution_error("gateway_discovery", error.to_string(), None, true))?;
        let status = response.status();
        let body = response
            .json::<GatewayResponse>()
            .await
            .map_err(|error| execution_error("gateway_response", error.to_string(), None, true))?;
        if !status.is_success() || body.url.trim().is_empty() {
            return Err(execution_error(
                "gateway_discovery",
                format!("QQ gateway returned {status}"),
                None,
                status.is_server_error(),
            ));
        }
        Ok(body.url)
    }

    async fn pace(&self) {
        let mut last_send = self.last_send.lock().await;
        if let Some(previous) = *last_send {
            let elapsed = previous.elapsed();
            if elapsed < MIN_SEND_INTERVAL {
                sleep(MIN_SEND_INTERVAL - elapsed).await;
            }
        }
        *last_send = Some(Instant::now());
    }

    async fn post_message(
        &self,
        kind: ChatKind,
        chat_id: &str,
        content: &str,
        reply_to: Option<&str>,
    ) -> Result<Option<String>, AdapterError> {
        let token = self.access_token().await?;
        let endpoint = match kind {
            ChatKind::Direct => format!("{}/v2/users/{chat_id}/messages", self.endpoint.api_base),
            ChatKind::Group => format!("{}/v2/groups/{chat_id}/messages", self.endpoint.api_base),
        };
        self.pace().await;
        let mut body = json!({
            "content": content,
            "msg_type": 0,
            "msg_seq": self.next_sequence.fetch_add(1, Ordering::Relaxed),
        });
        if let Some(message_id) = reply_to {
            body["msg_id"] = json!(message_id);
        }
        let response = self
            .http
            .post(endpoint)
            .header("Authorization", format!("QQBot {token}"))
            .json(&body)
            .send()
            .await
            .map_err(|error| execution_error("qq_transport", error.to_string(), None, true))?;
        let status = response.status();
        if status == StatusCode::TOO_MANY_REQUESTS {
            let retry_after = response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(|value| value.parse::<u64>().ok())
                .unwrap_or(1);
            return Err(AdapterError::RateLimited {
                retry_after: Duration::from_secs(retry_after),
            });
        }
        let body = response
            .json::<Value>()
            .await
            .map_err(|error| execution_error("qq_response", error.to_string(), None, true))?;
        if !status.is_success() {
            return Err(execution_error(
                "qq_api",
                body.to_string(),
                None,
                status.is_server_error(),
            ));
        }
        Ok(message_id_from_response(&body))
    }

    async fn send_envelope(
        &self,
        envelope: ChannelEnvelopeV1,
    ) -> Result<DeliveryReceipt, AdapterError> {
        let kind = chat_kind_from_extensions(&envelope.extensions);
        let ChannelEnvelopeV1 {
            payload,
            address,
            correlation,
            ..
        } = envelope;
        let parts = match payload {
            ChannelPayloadV1::Message { parts, .. } | ChannelPayloadV1::Stream { parts, .. } => {
                parts
            }
            _ => {
                return Err(execution_error(
                    "invalid_payload",
                    "QQ send requires message/stream payload",
                    None,
                    false,
                ))
            }
        };
        if address.chat_id.trim().is_empty() {
            return Err(execution_error(
                "invalid_recipient",
                "QQ chat id is empty",
                None,
                false,
            ));
        }
        let text = parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.clone()),
                ContentPart::Location {
                    latitude,
                    longitude,
                    ..
                } => Some(format!("{latitude},{longitude}")),
                ContentPart::Reference { uri, .. } => Some(uri.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        if text.trim().is_empty() {
            return Err(execution_error(
                "empty_message",
                "QQ message has no sendable text",
                None,
                false,
            ));
        }
        let chunks = split_chunks(&text, MAX_MESSAGE_CHARS);
        let mut last_id = None;
        for (index, chunk) in chunks.iter().enumerate() {
            let reply = (index == 0)
                .then_some(correlation.reply_to.as_deref())
                .flatten();
            let response_id = self
                .post_message(kind, &address.chat_id, chunk, reply)
                .await?;
            // Do not reuse an earlier chunk's identifier when the platform
            // omits the current response ID. A receipt must remain truthful.
            last_id = match response_id {
                Some(id) => Some(id),
                None => None,
            };
        }
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id,
            last_id,
            address.thread_id,
        ))
    }

    async fn process_frame(
        &self,
        frame: GatewayFrame,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        if let Some(sequence) = frame.s {
            self.session.lock().await.sequence = Some(sequence);
        }
        match frame.op {
            10 => {
                let interval = frame
                    .d
                    .as_ref()
                    .and_then(|value| value.get("heartbeat_interval"))
                    .and_then(Value::as_u64)
                    .map(Duration::from_millis)
                    .unwrap_or(HEARTBEAT_FALLBACK);
                self.session.lock().await.heartbeat_interval = Some(interval);
            }
            0 => {
                let event = frame.t.as_deref().unwrap_or_default();
                match event {
                    "READY" => {
                        if let Some(session_id) = frame
                            .d
                            .as_ref()
                            .and_then(|value| value.get("session_id"))
                            .and_then(Value::as_str)
                        {
                            self.session.lock().await.session_id = Some(session_id.to_string());
                        }
                        self.set_health(ChannelHealthStatus::Healthy, None);
                    }
                    "RESUMED" => self.set_health(ChannelHealthStatus::Healthy, None),
                    "C2C_MESSAGE_CREATE" | "GROUP_AT_MESSAGE_CREATE" | "AT_MESSAGE_CREATE" => {
                        if let Some(data) = frame.d {
                            self.process_message_event(event, data, context).await?;
                        }
                    }
                    _ => {}
                }
            }
            7 => {
                return Err(execution_error(
                    "gateway_reconnect",
                    "QQ gateway requested reconnect",
                    None,
                    true,
                ))
            }
            9 => {
                let resumable = frame.d.and_then(|value| value.as_bool()).unwrap_or(false);
                if !resumable {
                    let mut session = self.session.lock().await;
                    session.session_id = None;
                    session.sequence = None;
                }
                return Err(execution_error(
                    "gateway_invalid_session",
                    format!("resumable={resumable}"),
                    None,
                    true,
                ));
            }
            _ => {}
        }
        Ok(())
    }

    async fn process_message_event(
        &self,
        event: &str,
        data: Value,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        let message_id = data
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        if message_id.is_empty() {
            return Ok(());
        }
        {
            if self.seen.read().await.contains(&message_id) {
                return Ok(());
            }
        }
        let Some((kind, chat_id, sender_id)) = event_identity(event, &data) else {
            return Ok(());
        };
        if !is_sender_allowed(&self.config.allow_from, &sender_id) {
            self.seen.write().await.insert(message_id);
            return Ok(());
        }
        let content = data
            .get("content")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if content.is_empty() {
            self.seen.write().await.insert(message_id);
            return Ok(());
        }
        let mut address = ChannelAddress::new(CHANNEL, chat_id.clone());
        address.sender_id = Some(sender_id.clone());
        let mut correlation = Correlation::new(format!("{CHANNEL}:{chat_id}"));
        correlation.message_id = Some(message_id.clone());
        let mut envelope = external_message_envelope(
            address,
            correlation,
            vec![ContentPart::Text { text: content }],
            None,
            None,
        );
        envelope
            .extensions
            .insert("qq.event_type".to_string(), json!(event));
        envelope.extensions.insert(
            "qq.chat_kind".to_string(),
            json!(match kind {
                ChatKind::Direct => "direct",
                ChatKind::Group => "group",
            }),
        );
        context
            .fabric
            .admit_ingress(envelope, INGRESS_ADMISSION_DEADLINE, &context.cancel)
            .await
            .map_err(map_fabric_error)?;
        self.seen.write().await.insert(message_id);
        Ok(())
    }

    async fn run_connection(
        &self,
        gateway_url: &str,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        let (socket, _) = connect_async(gateway_url)
            .await
            .map_err(|error| execution_error("gateway_connect", error.to_string(), None, true))?;
        let (mut write, mut read) = socket.split();
        let mut heartbeat_interval = HEARTBEAT_FALLBACK;
        let mut heartbeat = tokio::time::interval(heartbeat_interval);
        heartbeat.tick().await;
        loop {
            if context.cancel.is_cancelled() || self.cancel.is_cancelled() {
                return Ok(());
            }
            tokio::select! {
                _ = heartbeat.tick() => {
                    let sequence = {
                        let mut session = self.session.lock().await;
                        if session.awaiting_heartbeat_ack {
                            return Err(execution_error(
                                "gateway_heartbeat_timeout",
                                "QQ gateway heartbeat acknowledgement was not received",
                                None,
                                true,
                            ));
                        }
                        session.awaiting_heartbeat_ack = true;
                        session.sequence.unwrap_or(0)
                    };
                    write.send(WsMessage::Text(json!({"op": 1, "d": sequence}).to_string())).await.map_err(|error| execution_error("gateway_heartbeat", error.to_string(), None, true))?;
                }
                message = read.next() => {
                    let Some(message) = message else { return Err(execution_error("gateway_closed", "QQ gateway stream ended", None, true)); };
                    match message.map_err(|error| execution_error("gateway_read", error.to_string(), None, true))? {
                        WsMessage::Text(text) => {
                            let frame = serde_json::from_str::<GatewayFrame>(&text).map_err(|error| execution_error("gateway_frame", error.to_string(), None, false))?;
                            if frame.op == 10 {
                                heartbeat_interval = frame.d.as_ref().and_then(|value| value.get("heartbeat_interval")).and_then(Value::as_u64).map(Duration::from_millis).unwrap_or(HEARTBEAT_FALLBACK);
                                heartbeat = tokio::time::interval(heartbeat_interval);
                                let token = self.access_token().await?;
                                let session = self.session.lock().await;
                                let payload = if let Some(session_id) = &session.session_id {
                                    json!({"op": 6, "d": {"token": format!("QQBot {token}"), "session_id": session_id, "seq": session.sequence.unwrap_or(0)}})
                                } else {
                                    json!({"op": 2, "d": {"token": format!("QQBot {token}"), "intents": INTENTS, "shard": [0, 1]}})
                                };
                                drop(session);
                                write.send(WsMessage::Text(payload.to_string())).await.map_err(|error| execution_error("gateway_identify", error.to_string(), None, true))?;
                            } else if frame.op == 11 {
                                self.session.lock().await.awaiting_heartbeat_ack = false;
                                self.set_health(ChannelHealthStatus::Healthy, None);
                            } else {
                                self.process_frame(frame, context).await?;
                            }
                        }
                        WsMessage::Ping(payload) => { write.send(WsMessage::Pong(payload)).await.map_err(|error| execution_error("gateway_pong", error.to_string(), None, true))?; }
                        WsMessage::Close(_) => return Err(execution_error("gateway_closed", "QQ gateway closed the connection", None, true)),
                        _ => {}
                    }
                }
            }
        }
    }

    async fn probe_health(&self) -> Result<DeliveryReceipt, AdapterError> {
        let token = self.access_token().await?;
        let response = self
            .http
            .get(format!("{}{}", self.endpoint.api_base, GATEWAY_PATH))
            .header("Authorization", format!("Bearer {token}"))
            .send()
            .await
            .map_err(|error| execution_error("health_transport", error.to_string(), None, true))?;
        if response.status().is_success() {
            self.set_health(ChannelHealthStatus::Healthy, None);
            Ok(accepted_receipt(CHANNEL, "health", None, None))
        } else {
            let status = response.status();
            self.set_health(
                ChannelHealthStatus::Degraded,
                Some(format!("QQ gateway returned {status}")),
            );
            Err(execution_error(
                "health_probe",
                format!("QQ gateway returned {status}"),
                None,
                status.is_server_error(),
            ))
        }
    }
}

#[async_trait]
impl ChannelAdapter for QqAdapter {
    fn name(&self) -> ChannelId {
        ChannelId::new(CHANNEL).expect("static QQ channel id")
    }
    fn capabilities(&self) -> ChannelCapabilities {
        Self::static_capabilities()
    }

    async fn start(&self, context: AdapterContext) -> Result<(), AdapterError> {
        self.validate_config()?;
        if context.cancel.is_cancelled() || self.cancel.is_cancelled() {
            return Ok(());
        }
        if self.running.swap(true, Ordering::AcqRel) {
            return Err(execution_error(
                "already_running",
                "QQ adapter is already running",
                None,
                false,
            ));
        }
        let mut attempt = 0u32;
        let result = loop {
            if context.cancel.is_cancelled() || self.cancel.is_cancelled() {
                break Ok(());
            }
            let connection = tokio::select! {
                biased;
                _ = context.cancel.cancelled() => break Ok(()),
                _ = self.cancel.cancelled() => break Ok(()),
                result = async {
                    let token = self.access_token().await?;
                    let gateway = self.fetch_gateway_url(&token).await?;
                    self.run_connection(&gateway, &context).await
                } => result,
            };
            match connection {
                Ok(()) => break Ok(()),
                Err(error) => {
                    attempt = attempt.saturating_add(1);
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                    let shift = attempt.saturating_sub(1).min(4);
                    let delay = RECONNECT_BASE
                        .saturating_mul(1u32 << shift)
                        .min(RECONNECT_MAX);
                    if !sleep_or_cancel(delay, &context, &self.cancel).await {
                        break Ok(());
                    }
                }
            }
        };
        self.running.store(false, Ordering::Release);
        result
    }

    async fn execute(&self, command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError> {
        self.ensure_supported(&command)?;
        match command {
            ChannelCommand::Send { envelope, .. } => self.send_envelope(envelope).await,
            ChannelCommand::FinalizeStream {
                address,
                correlation,
                parts,
                ..
            } => {
                self.send_envelope(ChannelEnvelopeV1::new(
                    ChannelDirection::Egress,
                    address,
                    correlation,
                    ChannelOrigin::Runtime,
                    ChannelPayloadV1::Stream {
                        phase: StreamPhase::Finalized,
                        parts,
                    },
                ))
                .await
            }
            ChannelCommand::ProbeHealth { .. } => self.probe_health().await,
            ChannelCommand::Typing { .. }
            | ChannelCommand::Edit { .. }
            | ChannelCommand::Delete { .. }
            | ChannelCommand::React { .. } => {
                let capability = match command {
                    ChannelCommand::Typing { .. } => ChannelCapability::InteractionTyping,
                    ChannelCommand::Edit { .. } => ChannelCapability::InteractionEdit,
                    ChannelCommand::Delete { .. } => ChannelCapability::InteractionDelete,
                    ChannelCommand::React { .. } => ChannelCapability::InteractionReaction,
                    _ => unreachable!(),
                };
                Err(AdapterError::UnsupportedCapability { capability })
            }
        }
    }

    fn health(&self) -> ChannelHealth {
        self.health
            .lock()
            .map(|health| health.clone())
            .unwrap_or_else(|_| ChannelHealth::new(ChannelHealthStatus::Unknown))
    }
    async fn stop(&self) -> Result<(), AdapterError> {
        self.cancel.cancel();
        self.running.store(false, Ordering::Release);
        self.set_health(
            ChannelHealthStatus::Down,
            Some("QQ adapter stopped".to_string()),
        );
        Ok(())
    }
}

fn map_fabric_error(error: FabricAdmissionError) -> AdapterError {
    let diagnosis = error.to_string();
    match error {
        FabricAdmissionError::Busy { retry_after, .. } => {
            execution_error("fabric_busy", diagnosis, Some(retry_after), true)
        }
        FabricAdmissionError::Cancelled { .. } => AdapterError::Stopped,
        FabricAdmissionError::Closed { .. } => {
            execution_error("fabric_closed", diagnosis, None, true)
        }
        FabricAdmissionError::InvalidEnvelope(_) => {
            execution_error("invalid_envelope", diagnosis, None, false)
        }
    }
}

async fn sleep_or_cancel(
    duration: Duration,
    context: &AdapterContext,
    local_cancel: &CancellationToken,
) -> bool {
    tokio::select! {
        _ = context.cancel.cancelled() => false,
        _ = local_cancel.cancelled() => false,
        _ = sleep(duration) => true,
    }
}

fn split_chunks(text: &str, limit: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    text.chars()
        .collect::<Vec<_>>()
        .chunks(limit)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn message_id_from_response(body: &Value) -> Option<String> {
    body.get("id")
        .or_else(|| body.get("message_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn event_identity(event: &str, data: &Value) -> Option<(ChatKind, String, String)> {
    if event == "C2C_MESSAGE_CREATE" {
        let sender = data
            .get("author")
            .and_then(|author| author.get("user_openid").or_else(|| author.get("id")))
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        return Some((ChatKind::Direct, sender.clone(), sender));
    }
    let chat = data
        .get("group_openid")
        .or_else(|| data.get("group_id"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if chat.is_empty() {
        return None;
    }
    let sender = data
        .get("author")
        .and_then(|author| author.get("member_openid").or_else(|| author.get("id")))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    Some((ChatKind::Group, chat, sender))
}

fn chat_kind_from_extensions(extensions: &std::collections::BTreeMap<String, Value>) -> ChatKind {
    match extensions.get("qq.chat_kind").and_then(Value::as_str) {
        Some("group") => ChatKind::Group,
        _ => ChatKind::Direct,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{ChannelAttachmentStore, IngressAttachment, StoredAttachment};
    use agent_diva_core::channel::{AttachmentRef, FabricConsumer, FabricIngressItem};
    use agent_diva_core::config::Config;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio_tungstenite::accept_async;

    #[derive(Default)]
    struct Store;
    #[async_trait]
    impl ChannelAttachmentStore for Store {
        async fn put(
            &self,
            input: IngressAttachment,
        ) -> Result<AttachmentRef, crate::adapter::AttachmentStoreError> {
            Ok(AttachmentRef {
                uri: "sha256:test".into(),
                media_type: input
                    .declared_mime
                    .unwrap_or_else(|| "application/octet-stream".into()),
                size_bytes: input.bytes.len() as u64,
                sha256: "test".into(),
                file_name: input.file_name,
            })
        }
        async fn get(
            &self,
            reference: &AttachmentRef,
        ) -> Result<StoredAttachment, crate::adapter::AttachmentStoreError> {
            Ok(StoredAttachment {
                reference: reference.clone(),
                bytes: Vec::new(),
            })
        }
    }

    fn adapter() -> QqAdapter {
        let mut config = Config::default().channels.qq;
        config.enabled = true;
        config.app_id = "app".into();
        config.secret = "secret".into();
        QqAdapter::with_test_endpoint(
            config,
            AdapterServices::new(Arc::new(Store)),
            "http://127.0.0.1:1",
        )
    }

    #[test]
    fn capabilities_keep_unsupported_media_explicit() {
        let capabilities = adapter().capabilities();
        assert!(capabilities.supports(ChannelCapability::IngressGroup));
        assert!(capabilities.supports(ChannelCapability::ReliabilityResume));
        assert!(capabilities.supports(ChannelCapability::ReliabilityTokenRefresh));
        assert!(!capabilities.supports(ChannelCapability::EgressImage));
        assert!(!capabilities.supports(ChannelCapability::InteractionReaction));
    }

    #[test]
    fn response_message_id_is_parsed_truthfully() {
        assert_eq!(
            message_id_from_response(&json!({"id": "m1"})),
            Some("m1".into())
        );
        assert_eq!(
            message_id_from_response(&json!({"message_id": "m2"})),
            Some("m2".into())
        );
        assert_eq!(message_id_from_response(&json!({"code": 0})), None);
    }

    #[test]
    fn event_identity_preserves_c2c_and_group_open_ids() {
        let c2c = event_identity(
            "C2C_MESSAGE_CREATE",
            &json!({"author": {"user_openid": "user-1"}}),
        )
        .unwrap();
        assert_eq!(c2c.0, ChatKind::Direct);
        assert_eq!(c2c.1, "user-1");
        assert_eq!(c2c.2, "user-1");

        let group = event_identity(
            "GROUP_AT_MESSAGE_CREATE",
            &json!({"group_openid": "group-1", "author": {"member_openid": "member-1"}}),
        )
        .unwrap();
        assert_eq!(group.0, ChatKind::Group);
        assert_eq!(group.1, "group-1");
        assert_eq!(group.2, "member-1");
    }

    #[test]
    fn guild_at_event_without_group_open_id_is_not_misclassified_as_group() {
        assert_eq!(
            event_identity(
                "AT_MESSAGE_CREATE",
                &json!({
                    "channel_id": "guild-channel",
                    "author": {"id": "member-1"}
                })
            ),
            None
        );
    }

    #[test]
    fn outbound_group_selection_requires_explicit_address_extension() {
        let mut extensions = std::collections::BTreeMap::new();
        assert_eq!(chat_kind_from_extensions(&extensions), ChatKind::Direct);
        extensions.insert("qq.chat_kind".to_string(), json!("group"));
        assert_eq!(chat_kind_from_extensions(&extensions), ChatKind::Group);
        extensions.insert("qq.chat_kind".to_string(), json!("unknown"));
        assert_eq!(chat_kind_from_extensions(&extensions), ChatKind::Direct);
    }

    #[tokio::test]
    async fn admission_precedes_dedup_commit_for_c2c_event() {
        let adapter = adapter();
        let (fabric, mut consumer) = FabricConsumer::new();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let event = json!({
            "id": "c2c-1",
            "author": {"user_openid": "user-1"},
            "content": "hello"
        });
        adapter
            .process_message_event("C2C_MESSAGE_CREATE", event.clone(), &context)
            .await
            .unwrap();
        let first = consumer.recv_ingress().await;
        assert!(matches!(first, Some(FabricIngressItem::Envelope(_))));
        adapter
            .process_message_event("C2C_MESSAGE_CREATE", event, &context)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), consumer.recv_ingress())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn wire_qq_gateway_admits_c2c_and_group_once_and_handles_heartbeat() {
        let (gateway_url, gateway) = spawn_gateway_fixture().await;
        let (base, discovery) = spawn_http_fixture(vec![
            fixture_response(
                "/app/getAppAccessToken",
                200,
                "application/json",
                include_bytes!("../../tests/fixtures/c5/qq/token-response.json"),
            ),
            fixture_response(
                "/gateway",
                200,
                "application/json",
                format!(r#"{{"url":"{gateway_url}"}}"#).as_bytes(),
            ),
        ])
        .await;
        let adapter = Arc::new(adapter_at(&base));
        let (fabric, mut consumer) = FabricConsumer::new();
        let cancel = CancellationToken::new();
        let task = tokio::spawn({
            let adapter = adapter.clone();
            let cancel = cancel.clone();
            async move { adapter.start(AdapterContext { fabric, cancel }).await }
        });

        let first = tokio::time::timeout(Duration::from_secs(2), consumer.recv_ingress())
            .await
            .unwrap()
            .unwrap()
            .envelope()
            .clone();
        let second = tokio::time::timeout(Duration::from_secs(2), consumer.recv_ingress())
            .await
            .unwrap()
            .unwrap()
            .envelope()
            .clone();

        assert_eq!(first.address.chat_id, "user-openid-1");
        assert_eq!(first.address.sender_id.as_deref(), Some("user-openid-1"));
        assert_eq!(first.extensions["qq.chat_kind"], json!("direct"));
        assert_eq!(
            first.correlation.message_id.as_deref(),
            Some("c2c-message-1")
        );
        assert_eq!(second.address.chat_id, "group-openid-1");
        assert_eq!(second.address.sender_id.as_deref(), Some("member-openid-1"));
        assert_eq!(second.extensions["qq.chat_kind"], json!("group"));
        assert_eq!(
            second.correlation.message_id.as_deref(),
            Some("group-message-1")
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(50), consumer.recv_ingress())
                .await
                .is_err(),
            "the replayed C2C event must not be admitted twice"
        );

        tokio::time::sleep(Duration::from_millis(80)).await;
        cancel.cancel();
        assert!(task.await.unwrap().is_ok());
        assert_eq!(discovery.await.unwrap().len(), 2);
        let frames = gateway.await.unwrap();
        assert!(frames.iter().any(|frame| frame["op"] == 2));
        assert!(frames.iter().any(|frame| frame["op"] == 1));
    }

    #[tokio::test]
    async fn wire_qq_outbound_routes_c2c_and_group_with_seq_reply_and_real_ids() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/app/getAppAccessToken",
                200,
                "application/json",
                include_bytes!("../../tests/fixtures/c5/qq/token-response.json"),
            ),
            fixture_response(
                "/v2/users/user-openid-1/messages",
                200,
                "application/json",
                br#"{"id":"c2c-out-1"}"#,
            ),
            fixture_response(
                "/v2/groups/group-openid-1/messages",
                200,
                "application/json",
                br#"{"message_id":"group-out-1"}"#,
            ),
        ])
        .await;
        let adapter = adapter_at(&base);

        let mut c2c_correlation = Correlation::new("qq:user-openid-1");
        c2c_correlation.reply_to = Some("c2c-message-1".to_string());
        let c2c_receipt = adapter
            .execute(ChannelCommand::Send {
                envelope: external_message_envelope(
                    ChannelAddress::new(CHANNEL, "user-openid-1"),
                    c2c_correlation,
                    vec![ContentPart::Text {
                        text: "reply to c2c".to_string(),
                    }],
                    None,
                    None,
                ),
                idempotency_key: Some("qq-c2c-1".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            c2c_receipt.platform_message_id.as_deref(),
            Some("c2c-out-1")
        );

        let group_address = ChannelAddress::new(CHANNEL, "group-openid-1");
        let mut group_envelope = external_message_envelope(
            group_address,
            Correlation::new("qq:group-openid-1"),
            vec![ContentPart::Text {
                text: "group reply".to_string(),
            }],
            None,
            None,
        );
        group_envelope
            .extensions
            .insert("qq.chat_kind".to_string(), json!("group"));
        let group_receipt = adapter
            .execute(ChannelCommand::Send {
                envelope: group_envelope,
                idempotency_key: Some("qq-group-1".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            group_receipt.platform_message_id.as_deref(),
            Some("group-out-1")
        );

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].contains("clientSecret"));
        assert!(requests[1].contains("\"msg_type\":0"));
        assert!(requests[1].contains("\"msg_id\":\"c2c-message-1\""));
        assert!(requests[1].contains("\"msg_seq\":1"));
        assert!(requests[2].contains("\"msg_type\":0"));
        assert!(requests[2].contains("\"msg_seq\":2"));
    }

    #[test]
    fn chunks_are_unicode_safe_at_qq_limit() {
        let chunks = split_chunks(&"你".repeat(MAX_MESSAGE_CHARS + 1), MAX_MESSAGE_CHARS);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chars().count(), MAX_MESSAGE_CHARS);
    }

    #[test]
    fn unsupported_media_fails_before_transport() {
        let adapter = adapter();
        let command = ChannelCommand::Send {
            envelope: ChannelEnvelopeV1::new(
                ChannelDirection::Egress,
                ChannelAddress::new(CHANNEL, "user"),
                Correlation::new("qq:user"),
                ChannelOrigin::Runtime,
                ChannelPayloadV1::Message {
                    parts: vec![ContentPart::Card {
                        schema: "test.card".into(),
                        body: json!({"title": "unsupported"}),
                    }],
                    subject: None,
                    locale: None,
                    context: None,
                },
            ),
            idempotency_key: None,
        };
        let result = futures::executor::block_on(adapter.execute(command));
        assert!(matches!(
            result,
            Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::EgressCard
            })
        ));
    }

    fn adapter_at(base: &str) -> QqAdapter {
        let mut config = Config::default().channels.qq;
        config.enabled = true;
        config.app_id = "app".into();
        config.secret = "secret".into();
        QqAdapter::with_test_endpoint(config, AdapterServices::new(Arc::new(Store)), base)
    }

    struct HttpFixtureResponse {
        expected_path: String,
        status: u16,
        content_type: String,
        body: Vec<u8>,
    }

    fn fixture_response(
        expected_path: &str,
        status: u16,
        content_type: &str,
        body: &[u8],
    ) -> HttpFixtureResponse {
        HttpFixtureResponse {
            expected_path: expected_path.to_string(),
            status,
            content_type: content_type.to_string(),
            body: body.to_vec(),
        }
    }

    async fn spawn_http_fixture(
        responses: Vec<HttpFixtureResponse>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut requests = Vec::new();
            for expected in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let request = read_http_request(&mut socket).await;
                let request_text = String::from_utf8_lossy(&request).to_string();
                assert!(
                    request_text
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .contains(&expected.expected_path),
                    "request did not contain expected path: {}",
                    expected.expected_path
                );
                requests.push(request_text);
                let reason = match expected.status {
                    200 => "OK",
                    204 => "No Content",
                    429 => "Too Many Requests",
                    _ => "Fixture",
                };
                let header = format!(
                    "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    expected.status,
                    expected.content_type,
                    expected.body.len()
                );
                socket.write_all(header.as_bytes()).await.unwrap();
                socket.write_all(&expected.body).await.unwrap();
            }
            requests
        });
        (format!("http://{address}"), handle)
    }

    async fn read_http_request(socket: &mut TcpStream) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut chunk = [0_u8; 4096];
        loop {
            let read = socket.read(&mut chunk).await.unwrap();
            assert!(read > 0, "fixture client closed before sending headers");
            bytes.extend_from_slice(&chunk[..read]);
            let Some(header_end) = bytes.windows(4).position(|window| window == b"\r\n\r\n") else {
                continue;
            };
            let header_end = header_end + 4;
            let headers = String::from_utf8_lossy(&bytes[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            while bytes.len() < header_end + content_length {
                let read = socket.read(&mut chunk).await.unwrap();
                assert!(read > 0, "fixture client closed before sending body");
                bytes.extend_from_slice(&chunk[..read]);
            }
            return bytes;
        }
    }

    async fn spawn_gateway_fixture() -> (String, tokio::task::JoinHandle<Vec<Value>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let url = format!("ws://{address}");
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            socket
                .send(WsMessage::Text(
                    json!({"op":10,"d":{"heartbeat_interval":40}}).to_string(),
                ))
                .await
                .unwrap();
            let mut frames = Vec::new();
            while let Ok(Some(Ok(message))) =
                tokio::time::timeout(Duration::from_secs(3), socket.next()).await
            {
                let WsMessage::Text(text) = message else {
                    continue;
                };
                let frame: Value = serde_json::from_str(&text).unwrap();
                let opcode = frame["op"].as_u64().unwrap_or_default();
                frames.push(frame.clone());
                match opcode {
                    1 => {
                        socket
                            .send(WsMessage::Text(json!({"op":11,"d":null}).to_string()))
                            .await
                            .unwrap();
                    }
                    2 => {
                        socket
                            .send(WsMessage::Text(
                                json!({
                                    "op":0,
                                    "s":1,
                                    "t":"READY",
                                    "d":{"session_id":"session-1"}
                                })
                                .to_string(),
                            ))
                            .await
                            .unwrap();
                        let c2c: Value = serde_json::from_str(include_str!(
                            "../../tests/fixtures/c5/qq/c2c-message.json"
                        ))
                        .unwrap();
                        socket
                            .send(WsMessage::Text(
                                json!({"op":0,"s":2,"t":"C2C_MESSAGE_CREATE","d":c2c}).to_string(),
                            ))
                            .await
                            .unwrap();
                        socket
                            .send(WsMessage::Text(
                                json!({"op":0,"s":3,"t":"C2C_MESSAGE_CREATE","d":c2c}).to_string(),
                            ))
                            .await
                            .unwrap();
                        let group: Value = serde_json::from_str(include_str!(
                            "../../tests/fixtures/c5/qq/group-message.json"
                        ))
                        .unwrap();
                        socket
                            .send(WsMessage::Text(
                                json!({"op":0,"s":4,"t":"GROUP_AT_MESSAGE_CREATE","d":group})
                                    .to_string(),
                            ))
                            .await
                            .unwrap();
                    }
                    _ => {}
                }
            }
            frames
        });
        (url, handle)
    }
}
