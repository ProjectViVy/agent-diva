//! Native Discord Gateway/REST adapter for the C5 channel contract.
//!
//! The implementation deliberately uses the existing HTTP/WebSocket stack and
//! does not depend on Serenity or the legacy `DiscordHandler`.  Gateway state
//! is kept local to this adapter; the C2 supervisor remains responsible for
//! pacing, retry scheduling, and restart policy.

use crate::adapter::{
    accepted_receipt, execution_error, external_message_envelope, is_sender_allowed,
    validate_attachment_reference, AdapterContext, AdapterError, AdapterServices, ChannelAdapter,
    IngressAttachment,
};
use agent_diva_core::channel::{
    ChannelAddress, ChannelCapabilities, ChannelCapability, ChannelCommand, ChannelHealth,
    ChannelHealthStatus, ChannelId, ChannelPayloadV1, ContentPart, Correlation, DeliveryReceipt,
    ReactionOperation, TypingState,
};
use agent_diva_core::config::schema::DiscordConfig;
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{
        protocol::{frame::coding::CloseCode, CloseFrame},
        Message as WsMessage,
    },
};
use tokio_util::sync::CancellationToken;

const CHANNEL: &str = "discord";
const API_BASE: &str = "https://discord.com/api/v10";
const MAX_MESSAGE_CHARS: usize = 2_000;
const MAX_ATTACHMENT_BYTES: u64 = 25 * 1024 * 1024;
const MAX_RESPONSE_BYTES: u64 = 64 * 1024;
const MAX_DEDUP_ENTRIES: usize = 1_000;
const DEDUP_TTL: Duration = Duration::from_secs(60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const GATEWAY_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const GATEWAY_HELLO_TIMEOUT: Duration = Duration::from_secs(30);
const GATEWAY_INVALID_SESSION_COOLDOWN: Duration = Duration::from_secs(5);
const GATEWAY_CLOSE_TIMEOUT: Duration = Duration::from_millis(250);
const INGRESS_ADMISSION_DEADLINE: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
struct DiscordEndpoint {
    api_base: String,
    gateway_url: String,
}

impl DiscordEndpoint {
    fn production(config: &DiscordConfig) -> Self {
        Self {
            api_base: API_BASE.to_string(),
            gateway_url: config.gateway_url.clone(),
        }
    }

    #[cfg(test)]
    fn test(base: &str) -> Self {
        Self {
            api_base: base.trim_end_matches('/').to_string(),
            gateway_url: base.trim_end_matches('/').to_string(),
        }
    }
}

#[derive(Debug, Default)]
struct GatewayState {
    sequence: Option<u64>,
    session_id: Option<String>,
    resume_gateway_url: Option<String>,
    bot_user_id: Option<String>,
    heartbeat_interval: Option<Duration>,
    awaiting_heartbeat_ack: bool,
}

#[derive(Debug, Default)]
struct MessageDedup {
    committed: HashMap<String, Instant>,
    order: VecDeque<(String, Instant)>,
    pending: HashSet<String>,
}

impl MessageDedup {
    fn prune(&mut self, now: Instant) {
        while self
            .order
            .front()
            .is_some_and(|(_, expires_at)| *expires_at <= now)
        {
            if let Some((id, expires_at)) = self.order.pop_front() {
                if self
                    .committed
                    .get(&id)
                    .is_some_and(|value| *value == expires_at)
                {
                    self.committed.remove(&id);
                }
            }
        }
    }

    fn reserve(&mut self, id: &str, now: Instant) -> bool {
        if id.trim().is_empty() {
            return true;
        }
        self.prune(now);
        if self.committed.contains_key(id) || !self.pending.insert(id.to_string()) {
            return false;
        }
        true
    }

    fn commit(&mut self, id: String, now: Instant) {
        if id.trim().is_empty() {
            return;
        }
        self.pending.remove(&id);
        let expires_at = now + DEDUP_TTL;
        self.committed.insert(id.clone(), expires_at);
        self.order.push_back((id, expires_at));
        while self.committed.len() > MAX_DEDUP_ENTRIES {
            let Some((old_id, old_expires_at)) = self.order.pop_front() else {
                break;
            };
            if self
                .committed
                .get(&old_id)
                .is_some_and(|value| *value == old_expires_at)
            {
                self.committed.remove(&old_id);
            }
        }
    }

    fn release(&mut self, id: &str) {
        self.pending.remove(id);
    }
}

#[derive(Debug, Deserialize)]
struct GatewayEnvelope {
    op: u8,
    #[serde(default)]
    s: Option<u64>,
    #[serde(default)]
    t: Option<String>,
    #[serde(default)]
    d: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct CreateMessageResponse {
    id: String,
}

#[derive(Debug, Deserialize)]
struct GatewayDiscovery {
    url: String,
}

#[derive(Debug, Deserialize)]
struct RateLimitBody {
    #[serde(default)]
    retry_after: Option<f64>,
}

#[derive(Debug, Clone)]
struct IncomingMessage {
    id: String,
    channel_id: String,
    guild_id: Option<String>,
    thread_id: Option<String>,
    author_id: String,
    author_bot: bool,
    content: String,
    reply_to: Option<String>,
    attachments: Vec<IncomingAttachment>,
    mention_ids: Vec<String>,
}

#[derive(Debug, Clone)]
struct IncomingAttachment {
    filename: String,
    content_type: Option<String>,
    size: u64,
    url: String,
}

/// Native Discord adapter.  The concrete type is public for the future C6
/// registry; construction remains crate-visible so endpoint injection cannot
/// become a product configuration surface.
pub struct DiscordAdapter {
    config: DiscordConfig,
    services: AdapterServices,
    endpoint: DiscordEndpoint,
    http: reqwest::Client,
    state: Arc<tokio::sync::Mutex<GatewayState>>,
    seen: Arc<tokio::sync::Mutex<MessageDedup>>,
    health: Arc<Mutex<ChannelHealth>>,
    running: Arc<AtomicBool>,
    cancel: CancellationToken,
    invalid_session_cooldown: Duration,
}

impl DiscordAdapter {
    pub(crate) fn new(config: DiscordConfig, services: AdapterServices) -> Self {
        Self::from_endpoint(
            config.clone(),
            services,
            DiscordEndpoint::production(&config),
        )
    }

    fn from_endpoint(
        config: DiscordConfig,
        services: AdapterServices,
        endpoint: DiscordEndpoint,
    ) -> Self {
        Self::from_endpoint_with_options(
            config,
            services,
            endpoint,
            REQUEST_TIMEOUT,
            GATEWAY_INVALID_SESSION_COOLDOWN,
        )
    }

    fn from_endpoint_with_options(
        config: DiscordConfig,
        services: AdapterServices,
        endpoint: DiscordEndpoint,
        request_timeout: Duration,
        invalid_session_cooldown: Duration,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(request_timeout)
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            services,
            endpoint,
            http,
            state: Arc::new(tokio::sync::Mutex::new(GatewayState::default())),
            seen: Arc::new(tokio::sync::Mutex::new(MessageDedup::default())),
            health: Arc::new(Mutex::new(ChannelHealth::new(ChannelHealthStatus::Unknown))),
            running: Arc::new(AtomicBool::new(false)),
            cancel: CancellationToken::new(),
            invalid_session_cooldown,
        }
    }

    #[cfg(test)]
    fn with_test_endpoint(config: DiscordConfig, services: AdapterServices, base: &str) -> Self {
        Self::from_endpoint(config, services, DiscordEndpoint::test(base))
    }

    #[cfg(test)]
    fn with_test_endpoint_and_timeout(
        config: DiscordConfig,
        services: AdapterServices,
        base: &str,
        request_timeout: Duration,
        invalid_session_cooldown: Duration,
    ) -> Self {
        Self::from_endpoint_with_options(
            config,
            services,
            DiscordEndpoint::test(base),
            request_timeout,
            invalid_session_cooldown,
        )
    }

    fn static_capabilities() -> ChannelCapabilities {
        let mut caps = ChannelCapabilities::new([
            ChannelCapability::IngressText,
            ChannelCapability::IngressMarkdown,
            ChannelCapability::IngressThread,
            ChannelCapability::IngressGroup,
            ChannelCapability::IngressDirect,
            ChannelCapability::IngressTypedAttachments,
            ChannelCapability::IngressDedupId,
            ChannelCapability::EgressText,
            ChannelCapability::EgressMarkdown,
            ChannelCapability::EgressChunking,
            ChannelCapability::EgressReply,
            ChannelCapability::EgressImage,
            ChannelCapability::EgressAudio,
            ChannelCapability::EgressVideo,
            ChannelCapability::EgressFile,
            ChannelCapability::EgressCard,
            ChannelCapability::InteractionTyping,
            ChannelCapability::InteractionEdit,
            ChannelCapability::InteractionDelete,
            ChannelCapability::InteractionReaction,
            ChannelCapability::InteractionStreamFinalize,
            ChannelCapability::ReliabilityHealth,
            ChannelCapability::ReliabilityHeartbeat,
            ChannelCapability::ReliabilityPacing,
            ChannelCapability::ReliabilitySupervisedRestart,
        ]);
        caps.limits.max_text_chars = Some(MAX_MESSAGE_CHARS);
        caps.limits.max_attachment_bytes = Some(MAX_ATTACHMENT_BYTES);
        caps.limits.supported_mime_types = BTreeSet::from([
            "image/*".to_string(),
            "audio/*".to_string(),
            "video/*".to_string(),
            "application/octet-stream".to_string(),
        ]);
        caps.limits.rate_limit_hint_ms = Some(250);
        caps
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
                "Discord channel is disabled",
                None,
                false,
            ));
        }
        if self.config.token.trim().is_empty() {
            return Err(execution_error(
                "invalid_config",
                "Discord bot token is empty",
                None,
                false,
            ));
        }
        Ok(())
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

    async fn send_request(
        &self,
        request: reqwest::RequestBuilder,
        context_cancel: Option<&CancellationToken>,
        timeout_code: &'static str,
        transport_code: &'static str,
    ) -> Result<reqwest::Response, AdapterError> {
        let send = request.send();
        tokio::pin!(send);
        let result = match context_cancel {
            Some(context_cancel) => {
                tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => return Err(AdapterError::Stopped),
                    _ = context_cancel.cancelled() => return Err(AdapterError::Stopped),
                    result = &mut send => result,
                }
            }
            None => {
                tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => return Err(AdapterError::Stopped),
                    result = &mut send => result,
                }
            }
        };
        result.map_err(|error| {
            if error.is_timeout() {
                execution_error(timeout_code, error.to_string(), None, true)
            } else {
                execution_error(transport_code, error.to_string(), None, true)
            }
        })
    }

    async fn read_response_bytes(
        &self,
        response: reqwest::Response,
        context_cancel: Option<&CancellationToken>,
        max_bytes: u64,
        too_large_code: &'static str,
    ) -> Result<Vec<u8>, AdapterError> {
        let mut stream = response.bytes_stream();
        let mut bytes = Vec::new();
        while let Some(chunk) = match context_cancel {
            Some(context_cancel) => {
                tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => return Err(AdapterError::Stopped),
                    _ = context_cancel.cancelled() => return Err(AdapterError::Stopped),
                    chunk = stream.next() => chunk,
                }
            }
            None => {
                tokio::select! {
                    biased;
                    _ = self.cancel.cancelled() => return Err(AdapterError::Stopped),
                    chunk = stream.next() => chunk,
                }
            }
        } {
            let chunk = chunk
                .map_err(|error| execution_error("response_read", error.to_string(), None, true))?;
            if bytes.len() as u64 + chunk.len() as u64 > max_bytes {
                return Err(execution_error(
                    too_large_code,
                    format!("Discord response exceeded {max_bytes} bytes"),
                    None,
                    false,
                ));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }

    async fn discover_gateway(
        &self,
        context: Option<&AdapterContext>,
    ) -> Result<String, AdapterError> {
        let url = format!("{}/gateway/bot", self.endpoint.api_base);
        let response = self
            .send_request(
                self.http
                    .get(url)
                    .header("Authorization", format!("Bot {}", self.config.token)),
                context.map(|value| &value.cancel),
                "gateway_discovery_timeout",
                "gateway_discovery",
            )
            .await?;
        if !response.status().is_success() {
            if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(AdapterError::RateLimited {
                    retry_after: self
                        .parse_rate_limit(response, context.map(|value| &value.cancel))
                        .await?,
                });
            }
            let status = response.status();
            let body = self
                .read_response_bytes(
                    response,
                    context.map(|value| &value.cancel),
                    MAX_RESPONSE_BYTES,
                    "gateway_discovery_body",
                )
                .await?;
            return Err(rest_error(status, &body, "gateway_discovery_http"));
        }
        let body = self
            .read_response_bytes(
                response,
                context.map(|value| &value.cancel),
                MAX_RESPONSE_BYTES,
                "gateway_discovery_body",
            )
            .await?;
        let discovered = serde_json::from_slice::<GatewayDiscovery>(&body).map_err(|error| {
            execution_error("gateway_discovery_body", error.to_string(), None, false)
        })?;
        if discovered.url.trim().is_empty() {
            return Err(execution_error(
                "gateway_discovery_body",
                "Discord gateway discovery returned an empty URL",
                None,
                false,
            ));
        }
        Ok(normalize_gateway_url(&discovered.url))
    }

    async fn next_gateway_url(&self, context: &AdapterContext) -> Result<String, AdapterError> {
        let resume_url = {
            let state = self.state.lock().await;
            state
                .session_id
                .as_ref()
                .and(state.resume_gateway_url.as_deref())
                .map(str::to_string)
        };
        if let Some(resume_url) = resume_url {
            return Ok(normalize_gateway_url(&resume_url));
        }
        self.discover_gateway(Some(context)).await
    }

    async fn run_gateway(&self, context: &AdapterContext) -> Result<(), AdapterError> {
        let mut reconnect_delay = Duration::from_secs(1);
        loop {
            if context.cancel.is_cancelled() || self.cancel.is_cancelled() {
                return Ok(());
            }
            let gateway = match self.next_gateway_url(context).await {
                Ok(url) => url,
                Err(error) => {
                    if matches!(error, AdapterError::Stopped)
                        || context.cancel.is_cancelled()
                        || self.cancel.is_cancelled()
                    {
                        return Ok(());
                    }
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                    if !error.is_retryable() {
                        return Err(error);
                    }
                    if self.endpoint.gateway_url.trim().is_empty() {
                        if !sleep_or_cancel(reconnect_delay, context, &self.cancel).await {
                            return Ok(());
                        }
                        reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(60));
                        continue;
                    }
                    normalize_gateway_url(&self.endpoint.gateway_url)
                }
            };
            let connected = tokio::select! {
                biased;
                _ = context.cancel.cancelled() => return Ok(()),
                _ = self.cancel.cancelled() => return Ok(()),
                result = timeout(GATEWAY_CONNECT_TIMEOUT, connect_async(gateway)) => result,
            };
            let retry_after = match connected {
                Ok(Ok((stream, _))) => {
                    reconnect_delay = Duration::from_secs(1);
                    match self.consume_gateway(stream, context).await {
                        Ok(()) if context.cancel.is_cancelled() || self.cancel.is_cancelled() => {
                            return Ok(())
                        }
                        Ok(()) => None,
                        Err(error) => {
                            if matches!(error, AdapterError::Stopped)
                                && (context.cancel.is_cancelled() || self.cancel.is_cancelled())
                            {
                                return Ok(());
                            }
                            self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                            if !error.is_retryable() {
                                return Err(error);
                            }
                            error.retry_after()
                        }
                    }
                }
                Ok(Err(error)) => {
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                    None
                }
                Err(_) => {
                    self.set_health(
                        ChannelHealthStatus::Degraded,
                        Some("Discord gateway connect timed out".to_string()),
                    );
                    None
                }
            };
            let delay = retry_after.unwrap_or(reconnect_delay);
            if !sleep_or_cancel(delay, context, &self.cancel).await {
                return Ok(());
            }
            if retry_after.is_none() {
                reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(60));
            } else {
                reconnect_delay = Duration::from_secs(1);
            }
        }
    }

    async fn consume_gateway<S>(
        &self,
        stream: S,
        context: &AdapterContext,
    ) -> Result<(), AdapterError>
    where
        S: futures::Stream<Item = Result<WsMessage, tokio_tungstenite::tungstenite::Error>>
            + futures::Sink<WsMessage, Error = tokio_tungstenite::tungstenite::Error>
            + Unpin,
    {
        let (mut write, mut read) = stream.split();
        let first = tokio::select! {
            biased;
            _ = context.cancel.cancelled() => {
                send_gateway_close(&mut write, CloseCode::Normal, "stopped").await;
                return Ok(());
            }
            _ = self.cancel.cancelled() => {
                send_gateway_close(&mut write, CloseCode::Normal, "stopped").await;
                return Ok(());
            }
            first = timeout(GATEWAY_HELLO_TIMEOUT, read.next()) => first
                .map_err(|_| execution_error("gateway_hello_timeout", "Discord Hello timed out", None, true))?
                .ok_or_else(|| execution_error("gateway_closed", "Discord gateway closed before Hello", None, true))?
                .map_err(|error| execution_error("gateway_read", error.to_string(), None, true))?,
        };
        let hello = match first {
            WsMessage::Close(close) => return Err(gateway_close_error(close)),
            frame => parse_gateway(frame)?,
        };
        if hello.op != 10 {
            return Err(execution_error(
                "gateway_protocol",
                "first Discord frame was not Hello",
                None,
                false,
            ));
        }
        let heartbeat_ms = hello
            .d
            .as_ref()
            .and_then(|value| value.get("heartbeat_interval"))
            .and_then(Value::as_u64)
            .filter(|interval| *interval > 0)
            .ok_or_else(|| {
                execution_error(
                    "gateway_hello",
                    "Discord Hello omitted a positive heartbeat_interval",
                    None,
                    false,
                )
            })?;
        {
            let mut state = self.state.lock().await;
            state.heartbeat_interval = Some(Duration::from_millis(heartbeat_ms));
            state.awaiting_heartbeat_ack = false;
        }
        let resume = {
            let state = self.state.lock().await;
            state.session_id.clone().map(|session_id| {
                json!({
                    "op": 6,
                    "d": {"token": self.config.token, "session_id": session_id, "seq": state.sequence}
                })
            })
        };
        let identify = json!({
            "op": 2,
            "d": {"token": self.config.token, "intents": self.config.intents,
                "properties": {"os": "agent-diva", "browser": "agent-diva", "device": "agent-diva"}}
        });
        let initial = resume.unwrap_or(identify);
        write
            .send(WsMessage::Text(initial.to_string()))
            .await
            .map_err(|error| execution_error("gateway_write", error.to_string(), None, true))?;

        let mut heartbeat = tokio::time::interval(Duration::from_millis(heartbeat_ms));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        heartbeat.tick().await;
        loop {
            tokio::select! {
                biased;
                _ = context.cancel.cancelled() => {
                    send_gateway_close(&mut write, CloseCode::Normal, "stopped").await;
                    return Ok(());
                }
                _ = self.cancel.cancelled() => {
                    send_gateway_close(&mut write, CloseCode::Normal, "stopped").await;
                    return Ok(());
                }
                _ = heartbeat.tick() => {
                    send_gateway_heartbeat(self, &mut write).await?;
                }
                frame = read.next() => {
                    let Some(frame) = frame else { return Err(execution_error("gateway_closed", "Discord gateway stream ended", None, true)); };
                    let frame = frame.map_err(|error| execution_error("gateway_read", error.to_string(), None, true))?;
                    match frame {
                        WsMessage::Ping(payload) => {
                            write.send(WsMessage::Pong(payload)).await
                                .map_err(|error| execution_error("gateway_pong", error.to_string(), None, true))?;
                        }
                        WsMessage::Pong(_) => {}
                        WsMessage::Close(close) => {
                            let code = close.as_ref().map(|frame| u16::from(frame.code));
                            if code.is_some_and(close_requires_new_session) {
                                let mut state = self.state.lock().await;
                                state.session_id = None;
                                state.resume_gateway_url = None;
                                state.sequence = None;
                                state.bot_user_id = None;
                            }
                            return Err(gateway_close_error(close));
                        }
                        WsMessage::Text(_) | WsMessage::Binary(_) => match parse_gateway(frame)? {
                            envelope if envelope.op == 0 => {
                                if let Some(sequence) = envelope.s {
                                    self.state.lock().await.sequence = Some(sequence);
                                }
                                match envelope.t.as_deref() {
                                    Some("READY") => {
                                        let mut state = self.state.lock().await;
                                        if let Some(session_id) = envelope.d.as_ref()
                                            .and_then(|value| value.get("session_id"))
                                            .and_then(Value::as_str)
                                        {
                                            state.session_id = Some(session_id.to_string());
                                        }
                                        if let Some(resume_url) = envelope.d.as_ref()
                                            .and_then(|value| value.get("resume_gateway_url"))
                                            .and_then(Value::as_str)
                                            .filter(|url| !url.trim().is_empty())
                                        {
                                            state.resume_gateway_url = Some(resume_url.to_string());
                                        }
                                        if let Some(bot_user_id) = envelope.d.as_ref()
                                            .and_then(|value| value.get("user"))
                                            .and_then(|value| value.get("id"))
                                            .and_then(Value::as_str)
                                        {
                                            state.bot_user_id = Some(bot_user_id.to_string());
                                        }
                                        state.awaiting_heartbeat_ack = false;
                                        drop(state);
                                        self.set_health(ChannelHealthStatus::Healthy, None);
                                    }
                                    Some("MESSAGE_CREATE") => {
                                        if let Some(message) = envelope.d.as_ref().and_then(parse_incoming_message) {
                                            self.handle_incoming(message, context).await?;
                                        }
                                    }
                                    Some("RESUMED") => self.set_health(ChannelHealthStatus::Healthy, None),
                                    _ => {}
                                }
                            }
                            envelope if envelope.op == 1 => {
                                send_gateway_heartbeat(self, &mut write).await?;
                            }
                            envelope if envelope.op == 11 => {
                                self.state.lock().await.awaiting_heartbeat_ack = false;
                                self.set_health(ChannelHealthStatus::Healthy, None);
                            }
                            envelope if envelope.op == 7 => {
                                send_gateway_close(&mut write, CloseCode::Normal, "reconnect").await;
                                return Err(execution_error(
                                    "gateway_reconnect",
                                    "Discord requested reconnect",
                                    Some(Duration::ZERO),
                                    true,
                                ));
                            }
                            envelope if envelope.op == 9 => {
                                let resumable = envelope.d.as_ref().and_then(Value::as_bool).unwrap_or(false);
                                let mut state = self.state.lock().await;
                                state.awaiting_heartbeat_ack = false;
                                if !resumable {
                                    state.session_id = None;
                                    state.resume_gateway_url = None;
                                    state.sequence = None;
                                    state.bot_user_id = None;
                                }
                                drop(state);
                                send_gateway_close(&mut write, CloseCode::Normal, "invalid session").await;
                                return Err(execution_error(
                                    "gateway_invalid_session",
                                    format!("Discord invalid session; resumable={resumable}"),
                                    Some(self.invalid_session_cooldown),
                                    true,
                                ));
                            }
                            _ => {}
                        },
                        WsMessage::Frame(_) => {
                            return Err(execution_error(
                                "gateway_protocol",
                                "raw Discord frame is unsupported",
                                None,
                                false,
                            ));
                        }
                    }
                }
            }
        }
    }

    async fn handle_incoming(
        &self,
        message: IncomingMessage,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        let bot_user_id = self.state.lock().await.bot_user_id.clone();
        if bot_user_id.as_deref() == Some(message.author_id.as_str()) {
            return Ok(());
        }
        if message.author_bot && !self.config.listen_to_bots {
            return Ok(());
        }
        if !is_sender_allowed(&self.config.allow_from, &message.author_id) {
            return Ok(());
        }
        if let Some(guild_id) = &self.config.guild_id {
            if message
                .guild_id
                .as_deref()
                .is_some_and(|message_guild| message_guild != guild_id)
            {
                return Ok(());
            }
        }
        let mentioned_bot = bot_user_id.as_deref().is_some_and(|bot_user_id| {
            message
                .mention_ids
                .iter()
                .any(|mention_id| mention_id == bot_user_id)
        });
        if message.guild_id.is_some() && self.config.mention_only && !mentioned_bot {
            let allowed = self
                .config
                .group_reply_allowed_sender_ids
                .iter()
                .any(|id| id == &message.author_id);
            if !allowed {
                return Ok(());
            }
        }
        if message.content.trim().is_empty() && message.attachments.is_empty() {
            return Ok(());
        }
        if !self.seen.lock().await.reserve(&message.id, Instant::now()) {
            return Ok(());
        }
        let message_id = message.id.clone();
        let result = self.handle_incoming_reserved(message, context).await;
        let mut seen = self.seen.lock().await;
        match result {
            Ok(()) => {
                seen.commit(message_id, Instant::now());
                Ok(())
            }
            Err(error) => {
                seen.release(&message_id);
                Err(error)
            }
        }
    }

    async fn handle_incoming_reserved(
        &self,
        message: IncomingMessage,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        let mut parts = Vec::new();
        if !message.content.trim().is_empty() {
            parts.push(ContentPart::Markdown {
                markdown: message.content.clone(),
            });
        }
        for attachment in message.attachments {
            let (bytes, response_mime) = self.fetch_attachment(&attachment, context).await?;
            let mime = attachment
                .content_type
                .clone()
                .filter(|value| !value.trim().is_empty())
                .or(response_mime)
                .unwrap_or_else(|| mime_from_filename(&attachment.filename))
                .to_ascii_lowercase();
            let bytes_for_validation = bytes.clone();
            let reference = self
                .services
                .attachments
                .put(IngressAttachment {
                    source_channel: CHANNEL.to_string(),
                    platform_message_id: Some(message.id.clone()),
                    sender_id: Some(message.author_id.clone()),
                    file_name: Some(attachment.filename.clone()),
                    declared_mime: Some(mime.clone()),
                    bytes,
                })
                .await
                .map_err(|error| {
                    execution_error("attachment_store", error.to_string(), None, false)
                })?;
            validate_attachment_reference(&reference, &bytes_for_validation).map_err(|error| {
                execution_error("attachment_store", error.to_string(), None, false)
            })?;
            parts.push(if mime.starts_with("image/") {
                ContentPart::Image {
                    attachment: reference,
                }
            } else if mime.starts_with("audio/") {
                ContentPart::Audio {
                    attachment: reference,
                    transcript: None,
                }
            } else if mime.starts_with("video/") {
                ContentPart::Video {
                    attachment: reference,
                }
            } else {
                ContentPart::File {
                    attachment: reference,
                }
            });
        }
        let mut address = ChannelAddress::new(CHANNEL, message.channel_id.clone());
        address.sender_id = Some(message.author_id.clone());
        address.thread_id = message.thread_id.clone();
        let mut correlation = Correlation::new(format!("{CHANNEL}:{}", message.channel_id));
        correlation.message_id = Some(message.id.clone());
        correlation.reply_to = message.reply_to;
        let envelope = external_message_envelope(address, correlation, parts, None, None);
        context
            .fabric
            .admit_ingress(envelope, INGRESS_ADMISSION_DEADLINE, &context.cancel)
            .await
            .map_err(|error| match error {
                agent_diva_core::channel::FabricAdmissionError::Busy { retry_after, .. } => {
                    execution_error("fabric_busy", error.to_string(), Some(retry_after), true)
                }
                agent_diva_core::channel::FabricAdmissionError::Cancelled { .. } => {
                    AdapterError::Stopped
                }
                agent_diva_core::channel::FabricAdmissionError::Closed { .. } => {
                    execution_error("fabric_closed", error.to_string(), None, true)
                }
                agent_diva_core::channel::FabricAdmissionError::InvalidEnvelope(_) => {
                    execution_error("invalid_envelope", error.to_string(), None, false)
                }
            })?;
        Ok(())
    }

    async fn fetch_attachment(
        &self,
        attachment: &IncomingAttachment,
        context: &AdapterContext,
    ) -> Result<(Vec<u8>, Option<String>), AdapterError> {
        if attachment.size > MAX_ATTACHMENT_BYTES {
            return Err(execution_error(
                "attachment_too_large",
                format!(
                    "Discord attachment declares {} bytes, above {}",
                    attachment.size, MAX_ATTACHMENT_BYTES
                ),
                None,
                false,
            ));
        }
        let response = self
            .send_request(
                self.http.get(&attachment.url),
                Some(&context.cancel),
                "attachment_timeout",
                "attachment_fetch",
            )
            .await?;
        let response_mime = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AdapterError::RateLimited {
                retry_after: self.parse_rate_limit(response, None).await?,
            });
        }
        if !response.status().is_success() {
            let status = response.status();
            let body = self
                .read_response_bytes(
                    response,
                    Some(&context.cancel),
                    MAX_RESPONSE_BYTES,
                    "attachment_response_body",
                )
                .await?;
            return Err(rest_error(status, &body, "attachment_http"));
        }
        if response
            .content_length()
            .is_some_and(|length| length > MAX_ATTACHMENT_BYTES)
        {
            return Err(execution_error(
                "attachment_too_large",
                format!("Discord attachment Content-Length exceeds {MAX_ATTACHMENT_BYTES}"),
                None,
                false,
            ));
        }
        let bytes = self
            .read_response_bytes(
                response,
                Some(&context.cancel),
                MAX_ATTACHMENT_BYTES,
                "attachment_too_large",
            )
            .await?;
        if bytes.len() as u64 != attachment.size {
            return Err(execution_error(
                "attachment_size_mismatch",
                format!(
                    "Discord attachment declared {} bytes but returned {}",
                    attachment.size,
                    bytes.len()
                ),
                None,
                false,
            ));
        }
        Ok((bytes, response_mime))
    }

    async fn send_message(
        &self,
        envelope: &agent_diva_core::channel::ChannelEnvelopeV1,
    ) -> Result<DeliveryReceipt, AdapterError> {
        self.validate_config()?;
        let (address, correlation, parts) = (
            &envelope.address,
            &envelope.correlation,
            match &envelope.payload {
                ChannelPayloadV1::Message { parts, .. }
                | ChannelPayloadV1::Stream { parts, .. } => parts,
                _ => {
                    return Err(execution_error(
                        "invalid_payload",
                        "Discord send requires message/stream payload",
                        None,
                        false,
                    ))
                }
            },
        );
        if address.chat_id.trim().is_empty() {
            return Err(execution_error(
                "invalid_recipient",
                "Discord channel id is empty",
                None,
                false,
            ));
        }
        let mut text_parts = Vec::new();
        let mut attachments = Vec::new();
        let mut embed = None;
        for part in parts {
            match part {
                ContentPart::Text { text } | ContentPart::Markdown { markdown: text } => {
                    text_parts.push(text.clone())
                }
                ContentPart::Card { body, .. } => embed = Some(discord_embed(body)),
                ContentPart::Image { attachment }
                | ContentPart::Audio { attachment, .. }
                | ContentPart::Video { attachment }
                | ContentPart::File { attachment } => attachments.push(attachment.clone()),
                ContentPart::Location {
                    latitude,
                    longitude,
                    ..
                } => text_parts.push(format!("{latitude},{longitude}")),
                ContentPart::Reference { uri, .. } => text_parts.push(uri.clone()),
            }
        }
        let chunks = split_chunks(&text_parts.join("\n"), MAX_MESSAGE_CHARS);
        if chunks.is_empty() && attachments.is_empty() && embed.is_none() {
            return Err(execution_error(
                "empty_message",
                "Discord message has no sendable content",
                None,
                false,
            ));
        }
        let mut last_id = None;
        if !attachments.is_empty() {
            let first_chunk = chunks.first().map(String::as_str).unwrap_or("");
            let payload =
                message_payload(first_chunk, embed.as_ref(), correlation.reply_to.as_deref());
            let message_id = self.send_attachments(address, attachments, payload).await?;
            last_id = Some(message_id);
            for chunk in chunks.iter().skip(1) {
                last_id = Some(
                    self.post_json(
                        &format!(
                            "{}/channels/{}/messages",
                            self.endpoint.api_base, address.chat_id
                        ),
                        &message_payload(chunk, None, None),
                    )
                    .await?,
                );
            }
        } else {
            for (index, chunk) in chunks.iter().enumerate() {
                last_id = Some(
                    self.post_json(
                        &format!(
                            "{}/channels/{}/messages",
                            self.endpoint.api_base, address.chat_id
                        ),
                        &message_payload(
                            chunk,
                            (index == 0).then_some(embed.as_ref()).flatten(),
                            (index == 0)
                                .then_some(correlation.reply_to.as_deref())
                                .flatten(),
                        ),
                    )
                    .await?,
                );
            }
            if chunks.is_empty() && embed.is_some() {
                last_id = Some(
                    self.post_json(
                        &format!(
                            "{}/channels/{}/messages",
                            self.endpoint.api_base, address.chat_id
                        ),
                        &message_payload("", embed.as_ref(), correlation.reply_to.as_deref()),
                    )
                    .await?,
                );
            }
        }
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id.clone(),
            last_id,
            address.thread_id.clone(),
        ))
    }

    async fn send_attachments(
        &self,
        address: &ChannelAddress,
        refs: Vec<agent_diva_core::channel::AttachmentRef>,
        payload: Value,
    ) -> Result<String, AdapterError> {
        let mut payload = payload;
        let mut attachment_descriptors = Vec::with_capacity(refs.len());
        let mut form = Form::new();
        for (index, reference) in refs.iter().enumerate() {
            if reference.size_bytes > MAX_ATTACHMENT_BYTES {
                return Err(execution_error(
                    "attachment_too_large",
                    format!("Discord attachment exceeds {MAX_ATTACHMENT_BYTES} bytes"),
                    None,
                    false,
                ));
            }
            let stored = self
                .services
                .attachments
                .get(reference)
                .await
                .map_err(|error| {
                    execution_error("attachment_read", error.to_string(), None, false)
                })?;
            stored.validate().map_err(|error| {
                execution_error("attachment_read", error.to_string(), None, false)
            })?;
            let part = Part::bytes(stored.bytes)
                .file_name(
                    reference
                        .file_name
                        .clone()
                        .unwrap_or_else(|| format!("attachment-{index}")),
                )
                .mime_str(&reference.media_type)
                .map_err(|error| {
                    execution_error("attachment_mime", error.to_string(), None, false)
                })?;
            attachment_descriptors.push(json!({
                "id": index,
                "filename": reference
                    .file_name
                    .clone()
                    .unwrap_or_else(|| format!("attachment-{index}")),
            }));
            form = form.part(format!("files[{index}]"), part);
        }
        payload["attachments"] = Value::Array(attachment_descriptors);
        form = form.text("payload_json", payload.to_string());
        let response = self
            .send_request(
                self.http
                    .post(format!(
                        "{}/channels/{}/messages",
                        self.endpoint.api_base, address.chat_id
                    ))
                    .header("Authorization", format!("Bot {}", self.config.token))
                    .multipart(form),
                None,
                "attachment_timeout",
                "attachment_send",
            )
            .await?;
        self.parse_response(response).await
    }

    async fn post_json(&self, url: &str, payload: &Value) -> Result<String, AdapterError> {
        let response = self
            .send_request(
                self.http
                    .post(url)
                    .header("Authorization", format!("Bot {}", self.config.token))
                    .json(payload),
                None,
                "http_timeout",
                "http_send",
            )
            .await?;
        self.parse_response(response).await
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<String, AdapterError> {
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AdapterError::RateLimited {
                retry_after: self.parse_rate_limit(response, None).await?,
            });
        }
        let status = response.status();
        let body = self
            .read_response_bytes(response, None, MAX_RESPONSE_BYTES, "discord_response_body")
            .await?;
        if !status.is_success() {
            return Err(rest_error(status, &body, "discord_http"));
        }
        let body = serde_json::from_slice::<CreateMessageResponse>(&body)
            .map_err(|error| execution_error("discord_response", error.to_string(), None, false))?;
        if body.id.trim().is_empty() {
            return Err(execution_error(
                "discord_response",
                "Discord response contained an empty message id",
                None,
                false,
            ));
        }
        Ok(body.id)
    }

    async fn parse_rate_limit(
        &self,
        response: reqwest::Response,
        context_cancel: Option<&CancellationToken>,
    ) -> Result<Duration, AdapterError> {
        let retry_header = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(parse_retry_after);
        let body = self
            .read_response_bytes(
                response,
                context_cancel,
                MAX_RESPONSE_BYTES,
                "discord_response_body",
            )
            .await?;
        let retry_body = serde_json::from_slice::<RateLimitBody>(&body)
            .ok()
            .and_then(|body| body.retry_after)
            .and_then(seconds_to_duration);
        Ok(retry_body
            .or(retry_header)
            .unwrap_or(Duration::from_secs(1)))
    }

    async fn execute_edit(
        &self,
        address: ChannelAddress,
        target: String,
        parts: Vec<ContentPart>,
    ) -> Result<DeliveryReceipt, AdapterError> {
        self.validate_config()?;
        let text = content_to_text(&parts);
        let payload = json!({"content": truncate(&text, MAX_MESSAGE_CHARS)});
        let id = self
            .send_request(
                self.http
                    .patch(format!(
                        "{}/channels/{}/messages/{}",
                        self.endpoint.api_base, address.chat_id, target
                    ))
                    .header("Authorization", format!("Bot {}", self.config.token))
                    .json(&payload),
                None,
                "edit_timeout",
                "edit_send",
            )
            .await?;
        let id = self.parse_response(id).await?;
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id,
            Some(id),
            address.thread_id,
        ))
    }

    async fn execute_delete(
        &self,
        address: ChannelAddress,
        target: String,
    ) -> Result<DeliveryReceipt, AdapterError> {
        self.validate_config()?;
        let response = self
            .send_request(
                self.http
                    .delete(format!(
                        "{}/channels/{}/messages/{}",
                        self.endpoint.api_base, address.chat_id, target
                    ))
                    .header("Authorization", format!("Bot {}", self.config.token)),
                None,
                "delete_timeout",
                "delete_send",
            )
            .await?;
        self.parse_status_response(response, "delete_http").await?;
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id,
            Some(target),
            address.thread_id,
        ))
    }

    async fn execute_reaction(
        &self,
        address: ChannelAddress,
        target: String,
        operation: ReactionOperation,
        emoji: String,
    ) -> Result<DeliveryReceipt, AdapterError> {
        self.validate_config()?;
        let encoded = percent_encode(&emoji);
        let url = format!(
            "{}/channels/{}/messages/{}/reactions/{}/@me",
            self.endpoint.api_base, address.chat_id, target, encoded
        );
        let request = match operation {
            ReactionOperation::Add => self.http.put(url),
            ReactionOperation::Remove => self.http.delete(url),
        }
        .header("Authorization", format!("Bot {}", self.config.token));
        let response = self
            .send_request(request, None, "reaction_timeout", "reaction_send")
            .await?;
        self.parse_status_response(response, "reaction_http")
            .await?;
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id,
            Some(target),
            address.thread_id,
        ))
    }

    async fn execute_typing(
        &self,
        address: ChannelAddress,
    ) -> Result<DeliveryReceipt, AdapterError> {
        self.validate_config()?;
        let response = self
            .send_request(
                self.http
                    .post(format!(
                        "{}/channels/{}/typing",
                        self.endpoint.api_base, address.chat_id
                    ))
                    .header("Authorization", format!("Bot {}", self.config.token)),
                None,
                "typing_timeout",
                "typing_send",
            )
            .await?;
        self.parse_status_response(response, "typing_http").await?;
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id,
            None,
            address.thread_id,
        ))
    }

    async fn probe_health(&self) -> Result<DeliveryReceipt, AdapterError> {
        self.validate_config()?;
        let response = self
            .send_request(
                self.http
                    .get(format!("{}/gateway/bot", self.endpoint.api_base))
                    .header("Authorization", format!("Bot {}", self.config.token)),
                None,
                "health_timeout",
                "health_probe",
            )
            .await;
        let response = match response {
            Ok(response) => response,
            Err(error) => {
                if !matches!(error, AdapterError::Stopped) {
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                }
                return Err(error);
            }
        };
        if !response.status().is_success() {
            let status = response.status();
            let error = if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                AdapterError::RateLimited {
                    retry_after: self.parse_rate_limit(response, None).await?,
                }
            } else {
                let body = self
                    .read_response_bytes(response, None, MAX_RESPONSE_BYTES, "health_response_body")
                    .await?;
                rest_error(status, &body, "health_http")
            };
            self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
            return Err(error);
        }
        let body = match self
            .read_response_bytes(response, None, MAX_RESPONSE_BYTES, "health_response_body")
            .await
        {
            Ok(body) => body,
            Err(error) => {
                if !matches!(error, AdapterError::Stopped) {
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                }
                return Err(error);
            }
        };
        let discovered = match serde_json::from_slice::<GatewayDiscovery>(&body) {
            Ok(discovered) => discovered,
            Err(error) => {
                let error = execution_error("health_response", error.to_string(), None, false);
                self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                return Err(error);
            }
        };
        if discovered.url.trim().is_empty() {
            let error = execution_error(
                "health_response",
                "Discord health response returned an empty gateway URL",
                None,
                false,
            );
            self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
            return Err(error);
        }
        self.set_health(ChannelHealthStatus::Healthy, None);
        Ok(accepted_receipt(CHANNEL, "health", None, None))
    }

    async fn parse_status_response(
        &self,
        response: reqwest::Response,
        code: &'static str,
    ) -> Result<(), AdapterError> {
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            return Err(AdapterError::RateLimited {
                retry_after: self.parse_rate_limit(response, None).await?,
            });
        }
        if response.status().is_success() {
            return Ok(());
        }
        let status = response.status();
        let body = self
            .read_response_bytes(response, None, MAX_RESPONSE_BYTES, "discord_response_body")
            .await?;
        Err(rest_error(status, &body, code))
    }
}

#[async_trait]
impl ChannelAdapter for DiscordAdapter {
    fn name(&self) -> ChannelId {
        ChannelId::new(CHANNEL).expect("constant Discord channel id")
    }

    fn capabilities(&self) -> ChannelCapabilities {
        Self::static_capabilities()
    }

    async fn start(&self, context: AdapterContext) -> Result<(), AdapterError> {
        if context.cancel.is_cancelled() {
            self.set_health(
                ChannelHealthStatus::Down,
                Some("Discord adapter stopped before start".to_string()),
            );
            return Ok(());
        }
        self.validate_config()?;
        if self.running.swap(true, Ordering::AcqRel) {
            return Err(execution_error(
                "already_running",
                "Discord adapter is already running",
                None,
                false,
            ));
        }
        let result = self.run_gateway(&context).await;
        self.running.store(false, Ordering::Release);
        if context.cancel.is_cancelled() || self.cancel.is_cancelled() {
            self.set_health(
                ChannelHealthStatus::Down,
                Some("Discord adapter stopped".to_string()),
            );
            Ok(())
        } else {
            result
        }
    }

    async fn execute(&self, command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError> {
        self.ensure_supported(&command)?;
        match command {
            ChannelCommand::Send { envelope, .. } => self.send_message(&envelope).await,
            ChannelCommand::Typing { address, state, .. } => match state {
                TypingState::Started => self.execute_typing(address).await,
                TypingState::Stopped => Ok(accepted_receipt(
                    CHANNEL,
                    address.chat_id,
                    None,
                    address.thread_id,
                )),
                TypingState::Listening => Err(AdapterError::UnsupportedCapability {
                    capability: ChannelCapability::InteractionListening,
                }),
            },
            ChannelCommand::Edit {
                address,
                target_message_id,
                parts,
                ..
            } => self.execute_edit(address, target_message_id, parts).await,
            ChannelCommand::Delete {
                address,
                target_message_id,
                ..
            } => self.execute_delete(address, target_message_id).await,
            ChannelCommand::React {
                address,
                target_message_id,
                operation,
                emoji,
                ..
            } => {
                self.execute_reaction(address, target_message_id, operation, emoji)
                    .await
            }
            ChannelCommand::FinalizeStream {
                address,
                correlation,
                parts,
                ..
            } => {
                if let Some(target) = correlation.message_id {
                    self.execute_edit(address, target, parts).await
                } else {
                    self.send_message(&agent_diva_core::channel::ChannelEnvelopeV1::new(
                        agent_diva_core::channel::ChannelDirection::Egress,
                        address,
                        correlation,
                        agent_diva_core::channel::ChannelOrigin::Runtime,
                        ChannelPayloadV1::Stream {
                            phase: agent_diva_core::channel::StreamPhase::Finalized,
                            parts,
                        },
                    ))
                    .await
                }
            }
            ChannelCommand::ProbeHealth { .. } => self.probe_health().await,
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
            Some("Discord adapter stopped".to_string()),
        );
        Ok(())
    }
}

async fn send_gateway_close<S>(
    write: &mut futures::stream::SplitSink<S, WsMessage>,
    code: CloseCode,
    reason: &str,
) where
    S: futures::Sink<WsMessage, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let frame = WsMessage::Close(Some(CloseFrame {
        code,
        reason: reason.to_string().into(),
    }));
    let _ = timeout(GATEWAY_CLOSE_TIMEOUT, write.send(frame)).await;
}

async fn send_gateway_heartbeat<S>(
    adapter: &DiscordAdapter,
    write: &mut futures::stream::SplitSink<S, WsMessage>,
) -> Result<(), AdapterError>
where
    S: futures::Sink<WsMessage, Error = tokio_tungstenite::tungstenite::Error> + Unpin,
{
    let sequence = {
        let mut state = adapter.state.lock().await;
        if state.awaiting_heartbeat_ack {
            drop(state);
            send_gateway_close(write, CloseCode::Away, "heartbeat ACK timeout").await;
            return Err(execution_error(
                "heartbeat_ack_timeout",
                "Discord heartbeat was not acknowledged before the next heartbeat",
                None,
                true,
            ));
        }
        state.awaiting_heartbeat_ack = true;
        state.sequence
    };
    write
        .send(WsMessage::Text(json!({"op": 1, "d": sequence}).to_string()))
        .await
        .map_err(|error| execution_error("gateway_heartbeat", error.to_string(), None, true))
}

fn close_requires_new_session(code: u16) -> bool {
    matches!(code, 4004 | 4007 | 4009 | 4010 | 4011 | 4012 | 4013 | 4014)
}

fn gateway_close_error(close: Option<CloseFrame<'_>>) -> AdapterError {
    let (code, reason) = close
        .map(|frame| (u16::from(frame.code), frame.reason.to_string()))
        .unwrap_or((
            1006,
            "Discord gateway stream ended without a close frame".to_string(),
        ));
    let retryable = !matches!(code, 4004 | 4010 | 4011 | 4012 | 4013 | 4014);
    execution_error(
        "gateway_closed",
        format!("Discord gateway closed with code {code}: {reason}"),
        None,
        retryable,
    )
}

fn rest_error(
    status: reqwest::StatusCode,
    body: &[u8],
    fallback_code: &'static str,
) -> AdapterError {
    let code = if status == reqwest::StatusCode::FORBIDDEN {
        "permission_denied"
    } else {
        fallback_code
    };
    let detail = String::from_utf8_lossy(body);
    let detail = truncate(&detail, 256);
    let message = if detail.trim().is_empty() {
        format!("Discord REST request returned {status}")
    } else {
        format!("Discord REST request returned {status}: {detail}")
    };
    execution_error(code, message, None, status.is_server_error())
}

fn message_payload(content: &str, embed: Option<&Value>, reply_to: Option<&str>) -> Value {
    let mut payload = json!({"content": truncate(content, MAX_MESSAGE_CHARS)});
    if let Some(embed) = embed {
        payload["embeds"] = json!([embed]);
    }
    if let Some(reply_to) = reply_to.filter(|value| !value.trim().is_empty()) {
        payload["message_reference"] = json!({"message_id": reply_to});
        payload["allowed_mentions"] = json!({"replied_user": false});
    }
    payload
}

fn mime_from_filename(filename: &str) -> String {
    let extension = filename
        .rsplit_once('.')
        .map(|(_, extension)| extension.to_ascii_lowercase());
    match extension.as_deref() {
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        Some("wav") => "audio/wav",
        Some("mp3") => "audio/mpeg",
        Some("ogg" | "oga") => "audio/ogg",
        Some("m4a") => "audio/mp4",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("mov") => "video/quicktime",
        Some("mkv") => "video/x-matroska",
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn parse_gateway(message: WsMessage) -> Result<GatewayEnvelope, AdapterError> {
    let text = match message {
        WsMessage::Text(text) => text,
        WsMessage::Binary(bytes) => String::from_utf8(bytes)
            .map_err(|error| execution_error("gateway_utf8", error.to_string(), None, false))?,
        WsMessage::Ping(_) | WsMessage::Pong(_) => {
            return Err(execution_error(
                "gateway_protocol",
                "unexpected ping/pong before JSON gateway frame",
                None,
                true,
            ))
        }
        WsMessage::Close(_) => {
            return Err(execution_error(
                "gateway_closed",
                "Discord gateway closed",
                None,
                true,
            ))
        }
        WsMessage::Frame(_) => {
            return Err(execution_error(
                "gateway_protocol",
                "raw Discord frame is unsupported",
                None,
                false,
            ))
        }
    };
    serde_json::from_str(&text)
        .map_err(|error| execution_error("gateway_json", error.to_string(), None, false))
}

fn parse_incoming_message(value: &Value) -> Option<IncomingMessage> {
    let id = value.get("id")?.as_str()?.to_string();
    let channel_id = value.get("channel_id")?.as_str()?.to_string();
    let author = value.get("author")?;
    let author_id = author.get("id")?.as_str()?.to_string();
    if id.trim().is_empty() || channel_id.trim().is_empty() || author_id.trim().is_empty() {
        return None;
    }
    let guild_id = value
        .get("guild_id")
        .and_then(Value::as_str)
        .map(str::to_string);
    let thread_id = value
        .get("thread")
        .and_then(|thread| thread.get("id"))
        .and_then(Value::as_str)
        .or_else(|| {
            value
                .get("type")
                .and_then(Value::as_u64)
                .filter(|kind| (10..=12).contains(kind))
                .map(|_| channel_id.as_str())
        })
        .map(str::to_string);
    let reply_to = value
        .get("message_reference")
        .and_then(|reference| reference.get("message_id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let content = value
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let attachments = value
        .get("attachments")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(IncomingAttachment {
                        filename: item.get("filename")?.as_str()?.to_string(),
                        content_type: item
                            .get("content_type")
                            .and_then(Value::as_str)
                            .map(str::to_string),
                        size: item.get("size").and_then(Value::as_u64).unwrap_or_default(),
                        url: item.get("url")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let mention_ids = value
        .get("mentions")
        .and_then(Value::as_array)
        .map(|mentions| {
            mentions
                .iter()
                .filter_map(|mention| {
                    mention
                        .get("id")
                        .and_then(Value::as_str)
                        .filter(|id| !id.trim().is_empty())
                        .map(str::to_string)
                })
                .collect()
        });
    Some(IncomingMessage {
        id,
        channel_id,
        guild_id,
        thread_id,
        author_id,
        author_bot: author.get("bot").and_then(Value::as_bool).unwrap_or(false),
        content,
        reply_to,
        attachments,
        mention_ids: mention_ids.unwrap_or_default(),
    })
}

fn normalize_gateway_url(base: &str) -> String {
    let mut url = base.trim_end_matches('/').to_string();
    if !url.contains("?") {
        if url
            .split_once("://")
            .is_some_and(|(_, authority_and_path)| !authority_and_path.contains('/'))
        {
            url.push('/');
        }
        url.push_str("?v=10&encoding=json");
    }
    url
}

fn split_chunks(text: &str, limit: usize) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }
    let chars: Vec<char> = text.chars().collect();
    chars
        .chunks(limit)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn content_to_text(parts: &[ContentPart]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text { text } => Some(text.as_str()),
            ContentPart::Markdown { markdown } => Some(markdown.as_str()),
            ContentPart::Location { .. } => Some("location"),
            ContentPart::Reference { uri, .. } => Some(uri.as_str()),
            ContentPart::Card { .. }
            | ContentPart::Image { .. }
            | ContentPart::Audio { .. }
            | ContentPart::Video { .. }
            | ContentPart::File { .. } => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn discord_embed(body: &Value) -> Value {
    let mut embed = json!({});
    if let Some(title) = body.get("title").and_then(Value::as_str) {
        embed["title"] = json!(truncate(title, 256));
    }
    if let Some(description) = body.get("description").and_then(Value::as_str) {
        embed["description"] = json!(truncate(description, 4096));
    }
    if let Some(color) = body.get("color").and_then(Value::as_u64) {
        embed["color"] = json!(color.min(0xFF_FFFF));
    }
    if let Some(fields) = body.get("fields").and_then(Value::as_array) {
        embed["fields"] = json!(fields.iter().take(25).cloned().collect::<Vec<_>>());
    }
    embed
}

fn parse_retry_after(value: &str) -> Option<Duration> {
    value.parse::<f64>().ok().and_then(seconds_to_duration)
}

fn seconds_to_duration(seconds: f64) -> Option<Duration> {
    if seconds.is_sign_negative() || !seconds.is_finite() {
        return None;
    }
    Duration::try_from_secs_f64(seconds).ok()
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

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn truncate(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::StoredAttachment;
    use crate::adapter::{AdapterServices, ChannelAttachmentStore};
    use agent_diva_core::channel::{AttachmentRef, FabricKernel};
    use agent_diva_core::config::Config;
    use async_trait::async_trait;
    use sha2::Digest;
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
            let digest = format!("{:x}", sha2::Sha256::digest(&input.bytes));
            Ok(AttachmentRef {
                uri: format!("sha256:{digest}"),
                media_type: input
                    .declared_mime
                    .unwrap_or_else(|| "application/octet-stream".into()),
                size_bytes: input.bytes.len() as u64,
                sha256: digest,
                file_name: input.file_name,
            })
        }
        async fn get(
            &self,
            reference: &AttachmentRef,
        ) -> Result<StoredAttachment, crate::adapter::AttachmentStoreError> {
            Ok(StoredAttachment {
                reference: reference.clone(),
                bytes: b"fixture".to_vec(),
            })
        }
    }

    fn adapter() -> DiscordAdapter {
        adapter_at("http://127.0.0.1:1")
    }

    fn adapter_at(base: &str) -> DiscordAdapter {
        let mut config = Config::default().channels.discord;
        config.enabled = true;
        config.token = "token".to_string();
        DiscordAdapter::with_test_endpoint(config, AdapterServices::new(Arc::new(Store)), base)
    }

    #[test]
    fn capabilities_match_frozen_matrix() {
        let capabilities = adapter().capabilities();
        assert!(capabilities.supports(ChannelCapability::IngressThread));
        assert!(capabilities.supports(ChannelCapability::EgressCard));
        assert!(!capabilities.supports(ChannelCapability::ReliabilityResume));
        assert!(!capabilities.supports(ChannelCapability::InteractionListening));
        assert_eq!(capabilities.limits.max_text_chars, Some(MAX_MESSAGE_CHARS));
    }

    #[test]
    fn gateway_url_is_normalized() {
        assert_eq!(
            normalize_gateway_url("wss://gateway.discord.gg"),
            "wss://gateway.discord.gg/?v=10&encoding=json"
        );
        assert_eq!(
            normalize_gateway_url("wss://gateway.discord.gg/?v=10"),
            "wss://gateway.discord.gg/?v=10"
        );
    }

    #[test]
    fn chunks_are_unicode_safe() {
        let chunks = split_chunks("你好世界", 2);
        assert_eq!(chunks, vec!["你好", "世界"]);
    }

    #[test]
    fn incoming_message_preserves_thread_and_reply() {
        let message = parse_incoming_message(&json!({
            "id":"m1", "channel_id":"c1", "guild_id":"g1", "content":"hi",
            "author":{"id":"u1","bot":false}, "thread":{"id":"t1"},
            "message_reference":{"message_id":"root"}, "mentions":[]
        }))
        .unwrap();
        assert_eq!(message.thread_id.as_deref(), Some("t1"));
        assert_eq!(message.reply_to.as_deref(), Some("root"));
    }

    #[test]
    fn channel_thread_types_use_channel_id_without_marking_dms_as_threads() {
        let thread = parse_incoming_message(&json!({
            "id":"m2", "channel_id":"thread-c", "type":11, "content":"hi",
            "author":{"id":"u1"}, "attachments":[]
        }))
        .unwrap();
        assert_eq!(thread.thread_id.as_deref(), Some("thread-c"));

        let dm = parse_incoming_message(&json!({
            "id":"m3", "channel_id":"dm-c", "type":1, "content":"hi",
            "author":{"id":"u1"}, "attachments":[]
        }))
        .unwrap();
        assert_eq!(dm.thread_id, None);
    }

    #[test]
    fn frozen_default_config_keeps_channel_disabled() {
        assert!(!Config::default().channels.discord.enabled);
    }

    #[test]
    fn c5_fixture_shapes_match_official_discord_wire_fields() {
        let gateway: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/discord/gateway-frames.json"
        ))
        .unwrap();
        let frames = gateway.as_array().unwrap();
        assert_eq!(frames[0]["op"], 10);
        assert!(frames[0]["d"]["heartbeat_interval"].as_u64().unwrap() > 0);
        assert_eq!(frames[1]["op"], 2);
        assert!(frames[1]["d"]["token"].is_string());
        assert_eq!(frames[1]["d"]["properties"]["os"], "agent-diva");
        assert_eq!(frames[2]["t"], "READY");
        assert!(frames[2]["d"]["session_id"].is_string());
        assert!(frames[2]["d"]["resume_gateway_url"].is_string());
        assert_eq!(frames[4]["op"], 1);
        assert_eq!(frames[5]["op"], 11);
        assert_eq!(frames[6]["op"], 6);
        assert_eq!(frames[7]["t"], "RESUMED");
        assert_eq!(frames[8]["op"], 7);
        assert_eq!(frames[9]["op"], 9);

        let rest: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/discord/rest-responses.json"
        ))
        .unwrap();
        assert!(rest["send"]["id"].is_string());
        assert_eq!(rest["rate_limited"]["retry_after"], 1.25);
        assert_eq!(rest["permission_error"]["code"], 50013);
        assert!(rest["multipart"]["payload_json"]["message_reference"].is_object());
        assert_eq!(rest["multipart"]["files"][0], "files[0]");
    }

    #[tokio::test]
    async fn ingress_policy_handles_dm_guild_mentions_bots_and_atomic_dedup() {
        let mut config = Config::default().channels.discord;
        config.enabled = true;
        config.token = "token".to_string();
        config.guild_id = Some("guild-1".to_string());
        config.mention_only = true;
        config.group_reply_allowed_sender_ids = vec!["trusted-user".to_string()];
        let adapter = DiscordAdapter::with_test_endpoint(
            config,
            AdapterServices::new(Arc::new(Store)),
            "http://127.0.0.1:1",
        );
        adapter.state.lock().await.bot_user_id = Some("bot-1".to_string());
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };

        let dm = parse_incoming_message(&json!({
            "id":"dm-1", "channel_id":"dm-channel", "content":"direct",
            "author":{"id":"user-1","bot":false}, "mentions":[], "attachments":[]
        }))
        .unwrap();
        adapter.handle_incoming(dm, &context).await.unwrap();
        let dm_envelope = tokio::time::timeout(Duration::from_secs(1), consumer.recv_ingress())
            .await
            .unwrap()
            .unwrap()
            .envelope()
            .clone();
        assert_eq!(dm_envelope.address.chat_id, "dm-channel");
        assert_eq!(dm_envelope.address.thread_id, None);

        let wrong_guild = parse_incoming_message(&json!({
            "id":"guild-wrong", "channel_id":"guild-channel", "guild_id":"guild-2",
            "content":"wrong guild", "author":{"id":"user-1","bot":false},
            "mentions":[{"id":"bot-1","bot":true}], "attachments":[]
        }))
        .unwrap();
        adapter
            .handle_incoming(wrong_guild, &context)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(30), consumer.recv_ingress())
                .await
                .is_err()
        );

        let wrong_mention = parse_incoming_message(&json!({
            "id":"mention-wrong", "channel_id":"guild-channel", "guild_id":"guild-1",
            "content":"not for this bot", "author":{"id":"user-1","bot":false},
            "mentions":[{"id":"other-bot","bot":true}], "attachments":[]
        }))
        .unwrap();
        adapter
            .handle_incoming(wrong_mention, &context)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(30), consumer.recv_ingress())
                .await
                .is_err()
        );

        let trusted = parse_incoming_message(&json!({
            "id":"trusted", "channel_id":"guild-channel", "guild_id":"guild-1",
            "content":"trusted exception", "author":{"id":"trusted-user","bot":false},
            "mentions":[], "attachments":[]
        }))
        .unwrap();
        adapter.handle_incoming(trusted, &context).await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), consumer.recv_ingress())
                .await
                .unwrap()
                .unwrap()
                .envelope()
                .address
                .chat_id,
            "guild-channel"
        );

        let self_message = parse_incoming_message(&json!({
            "id":"self", "channel_id":"guild-channel", "guild_id":"guild-1",
            "content":"self", "author":{"id":"bot-1","bot":true},
            "mentions":[], "attachments":[]
        }))
        .unwrap();
        adapter
            .handle_incoming(self_message, &context)
            .await
            .unwrap();
        let other_bot = parse_incoming_message(&json!({
            "id":"other-bot", "channel_id":"guild-channel", "guild_id":"guild-1",
            "content":"bot", "author":{"id":"bot-2","bot":true},
            "mentions":[{"id":"bot-1","bot":true}], "attachments":[]
        }))
        .unwrap();
        adapter.handle_incoming(other_bot, &context).await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(30), consumer.recv_ingress())
                .await
                .is_err()
        );

        let mentioned = parse_incoming_message(&json!({
            "id":"mentioned", "channel_id":"thread-channel", "guild_id":"guild-1",
            "content":"for this bot", "author":{"id":"user-2","bot":false},
            "mentions":[{"id":"bot-1","bot":true}], "attachments":[]
        }))
        .unwrap();
        let (first, second) = tokio::join!(
            adapter.handle_incoming(mentioned.clone(), &context),
            adapter.handle_incoming(mentioned, &context)
        );
        assert!(first.is_ok());
        assert!(second.is_ok());
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(1), consumer.recv_ingress())
                .await
                .unwrap()
                .unwrap()
                .envelope()
                .correlation
                .message_id
                .as_deref(),
            Some("mentioned")
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(30), consumer.recv_ingress())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn wire_discord_ingress_downloads_typed_attachments_before_admission() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response("/media/image.png", 200, "image/png", b"fixture"),
            fixture_response("/media/audio.mp3", 200, "audio/mpeg", b"fixture"),
            fixture_response("/media/video.mp4", 200, "video/mp4", b"fixture"),
            fixture_response(
                "/media/file.bin",
                200,
                "application/octet-stream",
                b"fixture",
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let message = parse_incoming_message(&json!({
            "id":"media-message", "channel_id":"channel-1", "content":"",
            "author":{"id":"user-1","bot":false}, "attachments":[
                {"filename":"image.png","content_type":"image/png","size":7,"url":format!("{base}/media/image.png")},
                {"filename":"audio.mp3","content_type":"audio/mpeg","size":7,"url":format!("{base}/media/audio.mp3")},
                {"filename":"video.mp4","content_type":"video/mp4","size":7,"url":format!("{base}/media/video.mp4")},
                {"filename":"file.bin","content_type":"application/octet-stream","size":7,"url":format!("{base}/media/file.bin")}
            ]
        }))
        .unwrap();
        adapter.handle_incoming(message, &context).await.unwrap();
        let envelope = tokio::time::timeout(Duration::from_secs(1), consumer.recv_ingress())
            .await
            .unwrap()
            .unwrap()
            .envelope()
            .clone();
        let ChannelPayloadV1::Message { parts, .. } = envelope.payload else {
            panic!("Discord ingress did not produce a message payload");
        };
        assert_eq!(parts.len(), 4);
        assert!(matches!(parts[0], ContentPart::Image { .. }));
        assert!(matches!(parts[1], ContentPart::Audio { .. }));
        assert!(matches!(parts[2], ContentPart::Video { .. }));
        assert!(matches!(parts[3], ContentPart::File { .. }));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[0].contains("GET /media/image.png"));
        assert!(requests[1].contains("GET /media/audio.mp3"));
        assert!(requests[2].contains("GET /media/video.mp4"));
        assert!(requests[3].contains("GET /media/file.bin"));
    }

    #[tokio::test]
    async fn wire_discord_rest_preserves_reply_embed_and_attachment_receipts() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/channels/channel-1/messages",
                200,
                "application/json",
                br#"{"id":"snowflake-1"}"#,
            ),
            fixture_response(
                "/channels/channel-1/messages/snowflake-1",
                200,
                "application/json",
                br#"{"id":"snowflake-2"}"#,
            ),
            fixture_response("/channels/channel-1/messages/snowflake-2", 204, "", b""),
            fixture_response(
                "/channels/channel-1/messages/snowflake-2/reactions/%F0%9F%91%8D/@me",
                204,
                "",
                b"",
            ),
            fixture_response("/channels/channel-1/typing", 204, "", b""),
            fixture_response(
                "/channels/channel-1/messages",
                200,
                "application/json",
                br#"{"id":"snowflake-attachment"}"#,
            ),
        ])
        .await;
        let adapter = adapter_at(&base);

        let mut correlation = Correlation::new("discord:channel-1");
        correlation.reply_to = Some("snowflake-parent".to_string());
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope: external_message_envelope(
                    ChannelAddress::new(CHANNEL, "channel-1"),
                    correlation.clone(),
                    vec![
                        ContentPart::Markdown {
                            markdown: "**hello**".to_string(),
                        },
                        ContentPart::Card {
                            schema: "discord.embed".to_string(),
                            body: json!({"title":"fixture card"}),
                        },
                    ],
                    None,
                    None,
                ),
                idempotency_key: Some("discord-send-1".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(receipt.platform_message_id.as_deref(), Some("snowflake-1"));

        let address = ChannelAddress::new(CHANNEL, "channel-1");
        let edit = adapter
            .execute(ChannelCommand::Edit {
                address: address.clone(),
                correlation: correlation.clone(),
                target_message_id: "snowflake-1".to_string(),
                parts: vec![ContentPart::Text {
                    text: "edited".to_string(),
                }],
                idempotency_key: Some("discord-edit-1".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(edit.platform_message_id.as_deref(), Some("snowflake-2"));
        adapter
            .execute(ChannelCommand::Delete {
                address: address.clone(),
                correlation: correlation.clone(),
                target_message_id: "snowflake-2".to_string(),
                idempotency_key: None,
            })
            .await
            .unwrap();
        adapter
            .execute(ChannelCommand::React {
                address: address.clone(),
                correlation: correlation.clone(),
                target_message_id: "snowflake-2".to_string(),
                operation: agent_diva_core::channel::ReactionOperation::Add,
                emoji: "👍".to_string(),
                idempotency_key: None,
            })
            .await
            .unwrap();
        adapter
            .execute(ChannelCommand::Typing {
                address,
                correlation,
                state: TypingState::Started,
                idempotency_key: None,
            })
            .await
            .unwrap();

        let attachment_bytes = b"fixture";
        let attachment_digest = format!("{:x}", sha2::Sha256::digest(attachment_bytes));
        let attachment = |media_type: &str, file_name: &str| AttachmentRef {
            uri: format!("sha256:{attachment_digest}"),
            media_type: media_type.to_string(),
            size_bytes: attachment_bytes.len() as u64,
            sha256: attachment_digest.clone(),
            file_name: Some(file_name.to_string()),
        };
        let image = attachment("image/png", "image.png");
        let audio = attachment("audio/mpeg", "sound.mp3");
        let video = attachment("video/mp4", "clip.mp4");
        let file = attachment("application/octet-stream", "document.bin");
        let mut attachment_correlation = Correlation::new("discord:channel-1");
        attachment_correlation.reply_to = Some("snowflake-parent".to_string());
        let attachment_receipt = adapter
            .execute(ChannelCommand::Send {
                envelope: external_message_envelope(
                    ChannelAddress::new(CHANNEL, "channel-1"),
                    attachment_correlation,
                    vec![
                        ContentPart::Image { attachment: image },
                        ContentPart::Audio {
                            attachment: audio,
                            transcript: None,
                        },
                        ContentPart::Video { attachment: video },
                        ContentPart::File { attachment: file },
                    ],
                    None,
                    None,
                ),
                idempotency_key: None,
            })
            .await
            .unwrap();
        assert_eq!(
            attachment_receipt.platform_message_id.as_deref(),
            Some("snowflake-attachment")
        );

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 6);
        assert!(requests[0].contains("\"message_reference\":{\"message_id\":\"snowflake-parent\"}"));
        assert!(requests[0].contains("\"embeds\":["));
        assert!(requests[1].contains("\"content\":\"edited\""));
        assert!(requests[5].contains("image.png"));
        assert!(requests[5].contains("sound.mp3"));
        assert!(requests[5].contains("clip.mp4"));
        assert!(requests[5].contains("document.bin"));
        assert!(requests[5].contains("files[0]"));
        assert!(requests[5].contains("files[1]"));
        assert!(requests[5].contains("files[2]"));
        assert!(requests[5].contains("files[3]"));
        assert!(requests[5].contains("image/png"));
        assert!(requests[5].contains("audio/mpeg"));
        assert!(requests[5].contains("video/mp4"));
        assert!(requests[5].contains("application/octet-stream"));
        assert!(requests[5].contains("message_reference"));
    }

    #[tokio::test]
    async fn wire_discord_rate_limit_and_unsupported_listening_are_explicit() {
        let (base, server) = spawn_http_fixture(vec![fixture_response_with_headers(
            "/channels/channel-1/messages",
            429,
            "application/json",
            br#"{"retry_after":1.25}"#,
            "retry-after: 9\r\n",
        )])
        .await;
        let adapter = adapter_at(&base);
        let result = adapter
            .execute(ChannelCommand::Send {
                envelope: external_message_envelope(
                    ChannelAddress::new(CHANNEL, "channel-1"),
                    Correlation::new("discord:channel-1"),
                    vec![ContentPart::Text {
                        text: "rate limit".to_string(),
                    }],
                    None,
                    None,
                ),
                idempotency_key: None,
            })
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::RateLimited { retry_after })
                if retry_after == Duration::from_millis(1_250)
        ));
        let address = ChannelAddress::new(CHANNEL, "channel-1");
        let result = adapter
            .execute(ChannelCommand::Typing {
                address,
                correlation: Correlation::new("discord:channel-1"),
                state: TypingState::Listening,
                idempotency_key: None,
            })
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionListening
            })
        ));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1);
    }

    #[tokio::test]
    async fn wire_discord_permission_and_malformed_responses_are_typed() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/channels/channel-1/messages",
                403,
                "application/json",
                br#"{"code":50013,"message":"Missing Permissions"}"#,
            ),
            fixture_response(
                "/channels/channel-1/messages",
                200,
                "application/json",
                br#"not-json"#,
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let send = |text: &str| ChannelCommand::Send {
            envelope: external_message_envelope(
                ChannelAddress::new(CHANNEL, "channel-1"),
                Correlation::new("discord:channel-1"),
                vec![ContentPart::Text {
                    text: text.to_string(),
                }],
                None,
                None,
            ),
            idempotency_key: None,
        };
        let permission = adapter.execute(send("permission")).await.unwrap_err();
        assert!(matches!(
            permission,
            AdapterError::Execution {
                ref code,
                retryable: false,
                ..
            } if code == "permission_denied"
        ));
        let malformed = adapter.execute(send("malformed")).await.unwrap_err();
        assert!(matches!(
            malformed,
            AdapterError::Execution {
                ref code,
                retryable: false,
                ..
            } if code == "discord_response"
        ));
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn wire_discord_health_marks_success_and_malformed_probe() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/gateway/bot",
                200,
                "application/json",
                br#"{"url":"wss://gateway.discord.gg"}"#,
            ),
            fixture_response("/gateway/bot", 200, "application/json", br#"not-json"#),
        ])
        .await;
        let adapter = adapter_at(&base);
        let channel = ChannelId::new(CHANNEL).unwrap();
        let receipt = adapter
            .execute(ChannelCommand::ProbeHealth {
                channel: channel.clone(),
            })
            .await
            .unwrap();
        assert_eq!(receipt.channel, CHANNEL);
        assert_eq!(adapter.health().status, ChannelHealthStatus::Healthy);
        let error = adapter
            .execute(ChannelCommand::ProbeHealth { channel })
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            AdapterError::Execution {
                ref code,
                retryable: false,
                ..
            } if code == "health_response"
        ));
        assert_eq!(adapter.health().status, ChannelHealthStatus::Degraded);
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn wire_discord_timeout_is_typed_and_retryable() {
        let (base, server) = spawn_slow_http_fixture().await;
        let mut config = Config::default().channels.discord;
        config.enabled = true;
        config.token = "token".to_string();
        let adapter = DiscordAdapter::with_test_endpoint_and_timeout(
            config,
            AdapterServices::new(Arc::new(Store)),
            &base,
            Duration::from_millis(20),
            Duration::from_millis(5),
        );
        let result = adapter
            .execute(ChannelCommand::Send {
                envelope: external_message_envelope(
                    ChannelAddress::new(CHANNEL, "channel-1"),
                    Correlation::new("discord:channel-1"),
                    vec![ContentPart::Text {
                        text: "timeout".to_string(),
                    }],
                    None,
                    None,
                ),
                idempotency_key: Some("timeout".to_string()),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            result,
            AdapterError::Execution {
                ref code,
                retryable: true,
                ..
            } if code == "http_timeout"
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn wire_discord_gateway_handles_hello_identify_ready_heartbeat_and_cancel() {
        let (gateway_url, gateway) = spawn_gateway_fixture().await;
        let (base, discovery) = spawn_http_fixture(vec![fixture_response(
            "/gateway/bot",
            200,
            "application/json",
            format!(r#"{{"url":"{gateway_url}"}}"#).as_bytes(),
        )])
        .await;
        let adapter = Arc::new(adapter_at(&base));
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let cancel = CancellationToken::new();
        let task_adapter = adapter.clone();
        let task_cancel = cancel.clone();
        let task = tokio::spawn(async move {
            task_adapter
                .start(AdapterContext {
                    fabric,
                    cancel: task_cancel,
                })
                .await
        });
        let envelope = tokio::time::timeout(Duration::from_secs(2), consumer.recv_ingress())
            .await
            .unwrap()
            .unwrap()
            .envelope()
            .clone();
        assert_eq!(envelope.address.chat_id, "channel-1");
        assert_eq!(envelope.address.thread_id.as_deref(), Some("thread-1"));
        assert_eq!(
            envelope.correlation.message_id.as_deref(),
            Some("snowflake-in")
        );
        assert_eq!(
            envelope.correlation.reply_to.as_deref(),
            Some("snowflake-root")
        );
        tokio::time::sleep(Duration::from_millis(80)).await;
        cancel.cancel();
        assert!(task.await.unwrap().is_ok());
        let discovery_requests = discovery.await.unwrap();
        assert_eq!(discovery_requests.len(), 1);
        let gateway_frames = gateway.await.unwrap();
        let identify = gateway_frames
            .iter()
            .find(|frame| frame["op"] == 2)
            .unwrap();
        assert_eq!(identify["d"]["token"], "token");
        assert_eq!(identify["d"]["intents"], 37377);
        assert_eq!(identify["d"]["properties"]["os"], "agent-diva");
        assert_eq!(identify["d"]["properties"]["browser"], "agent-diva");
        assert_eq!(identify["d"]["properties"]["device"], "agent-diva");
        let heartbeat = gateway_frames
            .iter()
            .find(|frame| frame["op"] == 1)
            .unwrap();
        assert_eq!(heartbeat["d"], 2);
        assert!(gateway_frames
            .iter()
            .any(|frame| frame["event"] == "close" && frame["code"] == 1000));
    }

    #[tokio::test]
    async fn wire_discord_gateway_resumes_after_reconnect_and_stops_cleanly() {
        let (gateway_url, gateway) = spawn_gateway_resume_fixture().await;
        let (base, discovery) = spawn_http_fixture(vec![fixture_response(
            "/gateway/bot",
            200,
            "application/json",
            format!(r#"{{"url":"{gateway_url}"}}"#).as_bytes(),
        )])
        .await;
        let adapter = Arc::new(adapter_at(&base));
        let (fabric, _) = FabricKernel::new().into_parts();
        let cancel = CancellationToken::new();
        let task = {
            let adapter = adapter.clone();
            let cancel = cancel.clone();
            tokio::spawn(async move { adapter.start(AdapterContext { fabric, cancel }).await })
        };
        tokio::time::sleep(Duration::from_millis(150)).await;
        cancel.cancel();
        assert!(task.await.unwrap().is_ok());
        assert_eq!(discovery.await.unwrap().len(), 1);
        let frames = tokio::time::timeout(Duration::from_secs(2), gateway)
            .await
            .unwrap()
            .unwrap();
        let resume = frames.iter().find(|frame| frame["op"] == 6).unwrap();
        assert_eq!(resume["d"]["session_id"], "session-1");
        assert_eq!(resume["d"]["seq"], 1);
        assert!(frames.iter().any(|frame| frame["t"] == "RESUMED"));
        assert!(frames
            .iter()
            .any(|frame| frame["event"] == "close" && frame["code"] == 1000));
    }

    #[tokio::test]
    async fn gateway_heartbeat_timeout_and_invalid_session_have_typed_lifecycle_errors() {
        let (timeout_url, timeout_fixture) = spawn_gateway_no_ack_fixture().await;
        let mut config = Config::default().channels.discord;
        config.enabled = true;
        config.token = "token".to_string();
        let adapter = DiscordAdapter::with_test_endpoint_and_timeout(
            config.clone(),
            AdapterServices::new(Arc::new(Store)),
            "http://127.0.0.1:1",
            Duration::from_secs(1),
            Duration::from_millis(25),
        );
        let stream = connect_async(&timeout_url).await.unwrap().0;
        let (fabric, _) = FabricKernel::new().into_parts();
        let error = adapter
            .consume_gateway(
                stream,
                &AdapterContext {
                    fabric,
                    cancel: CancellationToken::new(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            AdapterError::Execution {
                ref code,
                retryable: true,
                ..
            } if code == "heartbeat_ack_timeout"
        ));
        assert_eq!(timeout_fixture.await.unwrap(), Some(1001));

        let (invalid_url, invalid_fixture) = spawn_gateway_invalid_session_fixture().await;
        let adapter = DiscordAdapter::with_test_endpoint_and_timeout(
            config,
            AdapterServices::new(Arc::new(Store)),
            "http://127.0.0.1:1",
            Duration::from_secs(1),
            Duration::from_millis(25),
        );
        let stream = connect_async(&invalid_url).await.unwrap().0;
        let (fabric, _) = FabricKernel::new().into_parts();
        let error = adapter
            .consume_gateway(
                stream,
                &AdapterContext {
                    fabric,
                    cancel: CancellationToken::new(),
                },
            )
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            AdapterError::Execution {
                ref code,
                retry_after: Some(retry_after),
                retryable: true,
                ..
            } if code == "gateway_invalid_session" && retry_after == Duration::from_millis(25)
        ));
        assert_eq!(invalid_fixture.await.unwrap(), Some(1000));
    }

    struct HttpFixtureResponse {
        expected_path: String,
        status: u16,
        content_type: String,
        extra_headers: String,
        body: Vec<u8>,
    }

    fn fixture_response(
        expected_path: &str,
        status: u16,
        content_type: &str,
        body: &[u8],
    ) -> HttpFixtureResponse {
        fixture_response_with_headers(expected_path, status, content_type, body, "")
    }

    fn fixture_response_with_headers(
        expected_path: &str,
        status: u16,
        content_type: &str,
        body: &[u8],
        extra_headers: &str,
    ) -> HttpFixtureResponse {
        HttpFixtureResponse {
            expected_path: expected_path.to_string(),
            status,
            content_type: content_type.to_string(),
            extra_headers: extra_headers.to_string(),
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
                    "HTTP/1.1 {} {reason}\r\n{}Content-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                    expected.status,
                    expected.extra_headers,
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

    async fn spawn_slow_http_fixture() -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let _ = read_http_request(&mut socket).await;
            tokio::time::sleep(Duration::from_millis(100)).await;
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
        let resume_gateway_url = url.clone();
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            socket
                .send(WsMessage::Text(
                    json!({"op":10,"d":{"heartbeat_interval":50}}).to_string(),
                ))
                .await
                .unwrap();
            let mut frames = Vec::new();
            while let Ok(Some(Ok(message))) =
                tokio::time::timeout(Duration::from_secs(3), socket.next()).await
            {
                match message {
                    WsMessage::Text(text) => {
                        let frame: Value = serde_json::from_str(&text).unwrap();
                        let opcode = frame["op"].as_u64().unwrap_or_default();
                        frames.push(frame.clone());
                        if opcode == 2 {
                            socket
                                .send(WsMessage::Text(
                                    json!({
                                        "op":0,
                                        "s":1,
                                        "t":"READY",
                                        "d":{
                                            "session_id":"session-1",
                                            "resume_gateway_url":resume_gateway_url.clone(),
                                            "user":{"id":"bot-1","bot":true}
                                        }
                                    })
                                    .to_string(),
                                ))
                                .await
                                .unwrap();
                            socket
                                .send(WsMessage::Text(
                                    json!({
                                        "op":0,
                                        "s":2,
                                        "t":"MESSAGE_CREATE",
                                        "d":{
                                            "id":"snowflake-in",
                                            "channel_id":"channel-1",
                                            "guild_id":"guild-1",
                                            "content":"hello from discord",
                                            "author":{"id":"user-1","bot":false},
                                            "thread":{"id":"thread-1"},
                                            "message_reference":{"message_id":"snowflake-root"},
                                            "mentions":[{"id":"bot-1","bot":true}],
                                            "attachments":[]
                                        }
                                    })
                                    .to_string(),
                                ))
                                .await
                                .unwrap();
                        } else if opcode == 1 {
                            socket
                                .send(WsMessage::Text(json!({"op":11,"d":null}).to_string()))
                                .await
                                .unwrap();
                        }
                    }
                    WsMessage::Close(close) => {
                        frames.push(json!({
                            "event":"close",
                            "code":close.as_ref().map(|frame| u16::from(frame.code)).unwrap_or(1000),
                            "reason":close.as_ref().map(|frame| frame.reason.to_string()).unwrap_or_default()
                        }));
                        break;
                    }
                    _ => {}
                }
            }
            frames
        });
        (url, handle)
    }

    async fn spawn_gateway_resume_fixture() -> (String, tokio::task::JoinHandle<Vec<Value>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let url = format!("ws://{address}");
        let resume_gateway_url = url.clone();
        let handle = tokio::spawn(async move {
            let mut frames = Vec::new();
            for connection in 0..2 {
                let (stream, _) = listener.accept().await.unwrap();
                let mut socket = accept_async(stream).await.unwrap();
                socket
                    .send(WsMessage::Text(
                        json!({"op":10,"d":{"heartbeat_interval":1000}}).to_string(),
                    ))
                    .await
                    .unwrap();
                let message = tokio::time::timeout(Duration::from_secs(2), socket.next())
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap();
                let WsMessage::Text(text) = message else {
                    panic!("gateway client did not send an identify/resume payload");
                };
                let frame: Value = serde_json::from_str(&text).unwrap();
                let opcode = frame["op"].as_u64().unwrap_or_default();
                frames.push(frame);
                if connection == 0 {
                    assert_eq!(opcode, 2);
                    socket
                        .send(WsMessage::Text(
                            json!({
                                "op":0,
                                "s":1,
                                "t":"READY",
                                "d":{
                                    "session_id":"session-1",
                                    "resume_gateway_url":resume_gateway_url.clone(),
                                    "user":{"id":"bot-1","bot":true}
                                }
                            })
                            .to_string(),
                        ))
                        .await
                        .unwrap();
                    socket
                        .send(WsMessage::Text(json!({"op":7,"d":null}).to_string()))
                        .await
                        .unwrap();
                } else {
                    assert_eq!(opcode, 6);
                    socket
                        .send(WsMessage::Text(
                            json!({"op":0,"t":"RESUMED","d":{}}).to_string(),
                        ))
                        .await
                        .unwrap();
                    frames.push(json!({"event":"server","t":"RESUMED"}));
                }
                while let Ok(Some(Ok(message))) =
                    tokio::time::timeout(Duration::from_secs(2), socket.next()).await
                {
                    match message {
                        WsMessage::Text(text) => frames.push(serde_json::from_str(&text).unwrap()),
                        WsMessage::Close(close) => {
                            frames.push(json!({
                                "event":"close",
                                "code":close.as_ref().map(|frame| u16::from(frame.code)).unwrap_or(1000),
                                "reason":close.as_ref().map(|frame| frame.reason.to_string()).unwrap_or_default()
                            }));
                            break;
                        }
                        _ => {}
                    }
                }
            }
            frames
        });
        (url, handle)
    }

    async fn spawn_gateway_no_ack_fixture() -> (String, tokio::task::JoinHandle<Option<u16>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let url = format!("ws://{address}");
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            socket
                .send(WsMessage::Text(
                    json!({"op":10,"d":{"heartbeat_interval":20}}).to_string(),
                ))
                .await
                .unwrap();
            loop {
                let Ok(Some(Ok(message))) =
                    tokio::time::timeout(Duration::from_secs(2), socket.next()).await
                else {
                    return None;
                };
                match message {
                    WsMessage::Close(close) => {
                        return Some(
                            close
                                .as_ref()
                                .map(|frame| u16::from(frame.code))
                                .unwrap_or(1000),
                        );
                    }
                    WsMessage::Text(_) => {}
                    _ => {}
                }
            }
        });
        (url, handle)
    }

    async fn spawn_gateway_invalid_session_fixture(
    ) -> (String, tokio::task::JoinHandle<Option<u16>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let url = format!("ws://{address}");
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            socket
                .send(WsMessage::Text(
                    json!({"op":10,"d":{"heartbeat_interval":1000}}).to_string(),
                ))
                .await
                .unwrap();
            let Ok(Some(Ok(WsMessage::Text(_)))) =
                tokio::time::timeout(Duration::from_secs(2), socket.next()).await
            else {
                return None;
            };
            socket
                .send(WsMessage::Text(json!({"op":9,"d":false}).to_string()))
                .await
                .unwrap();
            loop {
                let Ok(Some(Ok(message))) =
                    tokio::time::timeout(Duration::from_secs(2), socket.next()).await
                else {
                    return None;
                };
                if let WsMessage::Close(close) = message {
                    return Some(
                        close
                            .as_ref()
                            .map(|frame| u16::from(frame.code))
                            .unwrap_or(1000),
                    );
                }
            }
        });
        (url, handle)
    }
}
