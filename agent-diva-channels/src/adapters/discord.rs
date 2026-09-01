//! Native Discord Gateway/REST adapter for the C5 channel contract.
//!
//! The implementation deliberately uses the existing HTTP/WebSocket stack and
//! does not depend on Serenity or the legacy `DiscordHandler`.  Gateway state
//! is kept local to this adapter; the C2 supervisor remains responsible for
//! pacing, retry scheduling, and restart policy.

use crate::adapter::{
    accepted_receipt, execution_error, external_message_envelope, is_sender_allowed,
    AdapterContext, AdapterError, AdapterServices, ChannelAdapter, IngressAttachment,
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
use std::collections::{BTreeSet, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout};
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tokio_util::sync::CancellationToken;

const CHANNEL: &str = "discord";
const API_BASE: &str = "https://discord.com/api/v10";
const MAX_MESSAGE_CHARS: usize = 2_000;
const MAX_ATTACHMENT_BYTES: u64 = 25 * 1024 * 1024;
const GATEWAY_CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
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
    heartbeat_interval: Option<Duration>,
    awaiting_heartbeat_ack: bool,
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
    mentioned_bot: bool,
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
    seen: Arc<RwLock<HashSet<String>>>,
    health: Arc<Mutex<ChannelHealth>>,
    running: Arc<AtomicBool>,
    cancel: CancellationToken,
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
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            services,
            endpoint,
            http,
            state: Arc::new(tokio::sync::Mutex::new(GatewayState::default())),
            seen: Arc::new(RwLock::new(HashSet::new())),
            health: Arc::new(Mutex::new(ChannelHealth::new(ChannelHealthStatus::Unknown))),
            running: Arc::new(AtomicBool::new(false)),
            cancel: CancellationToken::new(),
        }
    }

    #[cfg(test)]
    fn with_test_endpoint(config: DiscordConfig, services: AdapterServices, base: &str) -> Self {
        Self::from_endpoint(config, services, DiscordEndpoint::test(base))
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

    async fn discover_gateway(&self) -> Result<String, AdapterError> {
        let url = format!("{}/gateway/bot", self.endpoint.api_base);
        let response = self
            .http
            .get(url)
            .header("Authorization", format!("Bot {}", self.config.token))
            .send()
            .await
            .map_err(|error| execution_error("gateway_discovery", error.to_string(), None, true))?;
        if !response.status().is_success() {
            return Err(execution_error(
                "gateway_discovery_http",
                format!("Discord gateway discovery returned {}", response.status()),
                None,
                response.status().is_server_error(),
            ));
        }
        let discovered = response.json::<GatewayDiscovery>().await.map_err(|error| {
            execution_error("gateway_discovery_body", error.to_string(), None, true)
        })?;
        Ok(normalize_gateway_url(&discovered.url))
    }

    async fn run_gateway(&self, context: &AdapterContext) -> Result<(), AdapterError> {
        let mut reconnect_delay = Duration::from_secs(1);
        loop {
            if context.cancel.is_cancelled() || self.cancel.is_cancelled() {
                return Ok(());
            }
            let gateway = match self.discover_gateway().await {
                Ok(url) => url,
                Err(error) => {
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
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
            match timeout(GATEWAY_CONNECT_TIMEOUT, connect_async(gateway)).await {
                Ok(Ok((stream, _))) => {
                    reconnect_delay = Duration::from_secs(1);
                    self.set_health(ChannelHealthStatus::Healthy, None);
                    if let Err(error) = self.consume_gateway(stream, context).await {
                        self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                    }
                }
                Ok(Err(error)) => {
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                }
                Err(_) => {
                    self.set_health(
                        ChannelHealthStatus::Degraded,
                        Some("Discord gateway connect timed out".to_string()),
                    );
                }
            }
            if !sleep_or_cancel(reconnect_delay, context, &self.cancel).await {
                return Ok(());
            }
            reconnect_delay = (reconnect_delay * 2).min(Duration::from_secs(60));
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
        let first = timeout(Duration::from_secs(30), read.next())
            .await
            .map_err(|_| {
                execution_error(
                    "gateway_hello_timeout",
                    "Discord Hello timed out",
                    None,
                    true,
                )
            })?
            .ok_or_else(|| {
                execution_error(
                    "gateway_closed",
                    "Discord gateway closed before Hello",
                    None,
                    true,
                )
            })?
            .map_err(|error| execution_error("gateway_read", error.to_string(), None, true))?;
        let hello = parse_gateway(first)?;
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
            .unwrap_or(45_000);
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
                _ = context.cancel.cancelled() => return Ok(()),
                _ = self.cancel.cancelled() => return Ok(()),
                _ = heartbeat.tick() => {
                    let seq = {
                        let mut state = self.state.lock().await;
                        if state.awaiting_heartbeat_ack {
                            return Err(execution_error(
                                "heartbeat_ack_timeout",
                                "Discord heartbeat acknowledgement was not received",
                                None,
                                true,
                            ));
                        }
                        state.awaiting_heartbeat_ack = true;
                        state.sequence
                    };
                    write.send(WsMessage::Text(json!({"op": 1, "d": seq}).to_string())).await
                        .map_err(|error| execution_error("heartbeat_write", error.to_string(), None, true))?;
                }
                frame = read.next() => {
                    let Some(frame) = frame else { return Err(execution_error("gateway_closed", "Discord gateway stream ended", None, true)); };
                    let frame = frame.map_err(|error| execution_error("gateway_read", error.to_string(), None, true))?;
                    match parse_gateway(frame)? {
                        envelope if envelope.op == 0 => {
                            if let Some(sequence) = envelope.s { self.state.lock().await.sequence = Some(sequence); }
                            match envelope.t.as_deref() {
                                Some("READY") => {
                                    if let Some(session_id) = envelope.d.as_ref().and_then(|d| d.get("session_id")).and_then(Value::as_str) {
                                        self.state.lock().await.session_id = Some(session_id.to_string());
                                    }
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
                        envelope if envelope.op == 11 => {
                            self.state.lock().await.awaiting_heartbeat_ack = false;
                            self.set_health(ChannelHealthStatus::Healthy, None);
                        }
                        envelope if envelope.op == 7 => return Err(execution_error("gateway_reconnect", "Discord requested reconnect", None, true)),
                        envelope if envelope.op == 9 => {
                            let mut state = self.state.lock().await;
                            state.session_id = None;
                            state.sequence = None;
                            return Err(execution_error("gateway_invalid_session", "Discord invalid session", None, true));
                        }
                        _ => {}
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
        if self.seen.read().await.contains(&message.id) {
            return Ok(());
        }
        if message.author_bot && !self.config.listen_to_bots {
            return Ok(());
        }
        if !is_sender_allowed(&self.config.allow_from, &message.author_id) {
            return Ok(());
        }
        if let Some(guild_id) = &self.config.guild_id {
            if message.guild_id.as_deref() != Some(guild_id.as_str()) {
                return Ok(());
            }
        }
        if message.guild_id.is_some() && self.config.mention_only && !message.mentioned_bot {
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
        let mut parts = Vec::new();
        if !message.content.trim().is_empty() {
            parts.push(ContentPart::Markdown {
                markdown: message.content.clone(),
            });
        }
        for attachment in message.attachments {
            if attachment.size > MAX_ATTACHMENT_BYTES {
                continue;
            }
            let response = self
                .http
                .get(&attachment.url)
                .send()
                .await
                .map_err(|error| {
                    execution_error("attachment_fetch", error.to_string(), None, true)
                })?;
            let bytes = response.bytes().await.map_err(|error| {
                execution_error("attachment_read", error.to_string(), None, true)
            })?;
            if bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
                continue;
            }
            let reference = self
                .services
                .attachments
                .put(IngressAttachment {
                    source_channel: CHANNEL.to_string(),
                    platform_message_id: Some(message.id.clone()),
                    sender_id: Some(message.author_id.clone()),
                    file_name: Some(attachment.filename.clone()),
                    declared_mime: attachment.content_type.clone(),
                    bytes: bytes.to_vec(),
                })
                .await
                .map_err(|error| {
                    execution_error("attachment_store", error.to_string(), None, false)
                })?;
            let part = attachment.content_type.as_deref().unwrap_or("");
            parts.push(if part.starts_with("image/") {
                ContentPart::Image {
                    attachment: reference,
                }
            } else if part.starts_with("audio/") {
                ContentPart::Audio {
                    attachment: reference,
                    transcript: None,
                }
            } else if part.starts_with("video/") {
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
                _ => execution_error("fabric_admission", error.to_string(), None, true),
            })?;
        self.seen.write().await.insert(message.id);
        Ok(())
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
        for (index, chunk) in chunks.iter().enumerate() {
            let mut payload = json!({"content": chunk});
            if index == 0 {
                if let Some(reply) = &correlation.reply_to {
                    payload["message_reference"] = json!({"message_id": reply});
                    payload["allowed_mentions"] = json!({"replied_user": false});
                }
                if let Some(embed) = &embed {
                    payload["embeds"] = json!([embed]);
                }
            }
            last_id = Some(
                self.post_json(
                    &format!(
                        "{}/channels/{}/messages",
                        self.endpoint.api_base, address.chat_id
                    ),
                    &payload,
                )
                .await?,
            );
        }
        if chunks.is_empty() && embed.is_some() {
            let mut payload = json!({"content": ""});
            if let Some(embed) = &embed {
                payload["embeds"] = json!([embed]);
            }
            if let Some(reply) = &correlation.reply_to {
                payload["message_reference"] = json!({"message_id": reply});
                payload["allowed_mentions"] = json!({"replied_user": false});
            }
            last_id = Some(
                self.post_json(
                    &format!(
                        "{}/channels/{}/messages",
                        self.endpoint.api_base, address.chat_id
                    ),
                    &payload,
                )
                .await?,
            );
        }
        if !attachments.is_empty() {
            let message_id = self
                .send_attachments(address, correlation, attachments)
                .await?;
            last_id = Some(message_id);
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
        correlation: &Correlation,
        refs: Vec<agent_diva_core::channel::AttachmentRef>,
    ) -> Result<String, AdapterError> {
        let mut form = Form::new().text("payload_json", json!({"content":""}).to_string());
        for (index, reference) in refs.iter().enumerate() {
            let stored = self
                .services
                .attachments
                .get(reference)
                .await
                .map_err(|error| {
                    execution_error("attachment_read", error.to_string(), None, false)
                })?;
            let part = Part::bytes(stored.bytes).file_name(
                reference
                    .file_name
                    .clone()
                    .unwrap_or_else(|| format!("attachment-{index}")),
            );
            form = form.part(format!("files[{index}]"), part);
        }
        let mut request = self
            .http
            .post(format!(
                "{}/channels/{}/messages",
                self.endpoint.api_base, address.chat_id
            ))
            .header("Authorization", format!("Bot {}", self.config.token))
            .multipart(form);
        if let Some(reply) = &correlation.reply_to {
            request = request.header("X-Reply-To", reply);
        }
        let response = request
            .send()
            .await
            .map_err(|error| execution_error("attachment_send", error.to_string(), None, true))?;
        self.parse_response(response).await
    }

    async fn post_json(&self, url: &str, payload: &Value) -> Result<String, AdapterError> {
        let response = self
            .http
            .post(url)
            .header("Authorization", format!("Bot {}", self.config.token))
            .json(payload)
            .send()
            .await
            .map_err(|error| execution_error("http_send", error.to_string(), None, true))?;
        self.parse_response(response).await
    }

    async fn parse_response(&self, response: reqwest::Response) -> Result<String, AdapterError> {
        if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
            let retry_header = response
                .headers()
                .get("retry-after")
                .and_then(|value| value.to_str().ok())
                .and_then(parse_retry_after);
            let body = response
                .json::<RateLimitBody>()
                .await
                .ok()
                .and_then(|body| body.retry_after)
                .and_then(seconds_to_duration);
            let retry = body.or(retry_header).unwrap_or(Duration::from_secs(1));
            return Err(AdapterError::RateLimited { retry_after: retry });
        }
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(execution_error(
                "discord_http",
                format!("Discord HTTP {status}: {}", truncate(&body, 256)),
                None,
                status.is_server_error(),
            ));
        }
        let body = response
            .json::<CreateMessageResponse>()
            .await
            .map_err(|error| execution_error("discord_response", error.to_string(), None, true))?;
        Ok(body.id)
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
            .http
            .patch(format!(
                "{}/channels/{}/messages/{}",
                self.endpoint.api_base, address.chat_id, target
            ))
            .header("Authorization", format!("Bot {}", self.config.token))
            .json(&payload)
            .send()
            .await
            .map_err(|error| execution_error("edit_send", error.to_string(), None, true))?;
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
            .http
            .delete(format!(
                "{}/channels/{}/messages/{}",
                self.endpoint.api_base, address.chat_id, target
            ))
            .header("Authorization", format!("Bot {}", self.config.token))
            .send()
            .await
            .map_err(|error| execution_error("delete_send", error.to_string(), None, true))?;
        if !response.status().is_success() {
            return Err(execution_error(
                "delete_http",
                format!("Discord HTTP {}", response.status()),
                None,
                response.status().is_server_error(),
            ));
        }
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
        let response = match operation {
            ReactionOperation::Add => self.http.put(url),
            ReactionOperation::Remove => self.http.delete(url),
        }
        .header("Authorization", format!("Bot {}", self.config.token))
        .send()
        .await
        .map_err(|error| execution_error("reaction_send", error.to_string(), None, true))?;
        if !response.status().is_success() {
            return Err(execution_error(
                "reaction_http",
                format!("Discord HTTP {}", response.status()),
                None,
                response.status().is_server_error(),
            ));
        }
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
            .http
            .post(format!(
                "{}/channels/{}/typing",
                self.endpoint.api_base, address.chat_id
            ))
            .header("Authorization", format!("Bot {}", self.config.token))
            .send()
            .await
            .map_err(|error| execution_error("typing_send", error.to_string(), None, true))?;
        if !response.status().is_success() {
            return Err(execution_error(
                "typing_http",
                format!("Discord HTTP {}", response.status()),
                None,
                response.status().is_server_error(),
            ));
        }
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
            .http
            .get(format!("{}/gateway/bot", self.endpoint.api_base))
            .header("Authorization", format!("Bot {}", self.config.token))
            .send()
            .await
            .map_err(|error| execution_error("health_probe", error.to_string(), None, true))?;
        if response.status().is_success() {
            self.set_health(ChannelHealthStatus::Healthy, None);
            Ok(accepted_receipt(CHANNEL, "health", None, None))
        } else {
            self.set_health(
                ChannelHealthStatus::Degraded,
                Some(format!("Discord health HTTP {}", response.status())),
            );
            Err(execution_error(
                "health_http",
                format!("Discord HTTP {}", response.status()),
                None,
                response.status().is_server_error(),
            ))
        }
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
            return Ok(());
        }
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
        result
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
    let mentioned_bot = value
        .get("mentions")
        .and_then(Value::as_array)
        .is_some_and(|mentions| {
            mentions
                .iter()
                .any(|mention| mention.get("bot").and_then(Value::as_bool).unwrap_or(false))
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
        mentioned_bot,
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
    Some(Duration::from_secs_f64(seconds))
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
                bytes: Vec::new(),
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

        let attachment = AttachmentRef {
            uri: "sha256:fixture".to_string(),
            media_type: "image/png".to_string(),
            size_bytes: 0,
            sha256: "fixture".to_string(),
            file_name: Some("image.png".to_string()),
        };
        let attachment_receipt = adapter
            .execute(ChannelCommand::Send {
                envelope: external_message_envelope(
                    ChannelAddress::new(CHANNEL, "channel-1"),
                    Correlation::new("discord:channel-1"),
                    vec![ContentPart::Image { attachment }],
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
        assert!(requests[5].contains("files[0]"));
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
        assert!(gateway_frames.iter().any(|frame| frame["op"] == 2));
        assert!(gateway_frames.iter().any(|frame| frame["op"] == 1));
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
                    json!({"op":10,"d":{"heartbeat_interval":50}}).to_string(),
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
                if opcode == 2 {
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
            frames
        });
        (url, handle)
    }
}
