//! Native Telegram Bot API adapter for the Super Channel Fabric.
//!
//! This module is intentionally independent from the legacy `TelegramHandler`.
//! It keeps the DIVA command/allowlist semantics while using explicit Bot API
//! requests, bounded polling, typed attachment storage, and truthful receipts.

use crate::adapter::{
    accepted_receipt, delivery_receipt, execution_error, external_message_envelope,
    is_sender_allowed, AdapterContext, AdapterError, AdapterServices, ChannelAdapter,
    IngressAttachment,
};
use agent_diva_core::channel::{
    ChannelAddress, ChannelCapabilities, ChannelCapability, ChannelCommand, ChannelDirection,
    ChannelEnvelopeV1, ChannelHealth, ChannelHealthStatus, ChannelId, ChannelOrigin,
    ChannelPayloadV1, ContentPart, Correlation, DeliveryReceipt, DeliveryStatus, TypingState,
};
use agent_diva_core::config::schema::TelegramConfig;
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout, Instant};
use tokio_util::sync::CancellationToken;

const CHANNEL: &str = "telegram";
const API_BASE: &str = "https://api.telegram.org";
const MAX_MESSAGE_CHARS: usize = 4_096;
const MAX_CAPTION_CHARS: usize = 1_024;
const MAX_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;
const POLL_TIMEOUT_SECONDS: u64 = 25;
const INGRESS_ADMISSION_DEADLINE: Duration = Duration::from_secs(2);
const MEDIA_DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(30);
const TYPING_REFRESH_INTERVAL: Duration = Duration::from_secs(4);
const BASE_RECONNECT_DELAY: Duration = Duration::from_secs(5);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(60);
const DEDUP_CAPACITY: usize = 1_000;
const DEDUP_TTL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone)]
struct TelegramEndpoint {
    api_base: String,
}

impl TelegramEndpoint {
    fn production() -> Self {
        Self {
            api_base: API_BASE.to_string(),
        }
    }

    #[cfg(test)]
    fn test(base: &str) -> Self {
        Self {
            api_base: base.trim_end_matches('/').to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(bound(deserialize = "T: Deserialize<'de>"))]
struct ApiResponse<T> {
    ok: bool,
    #[serde(default)]
    result: Option<T>,
    #[serde(default)]
    error_code: Option<u16>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    parameters: Option<ApiParameters>,
}

#[derive(Debug, Deserialize)]
struct ApiParameters {
    #[serde(default)]
    retry_after: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TelegramFile {
    file_path: Option<String>,
    #[serde(default)]
    file_size: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
struct TelegramUser {
    id: i64,
    #[serde(default)]
    username: Option<String>,
    #[serde(default)]
    is_bot: bool,
}

#[derive(Debug, Deserialize)]
struct TelegramChat {
    id: i64,
    #[serde(default)]
    kind: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramMessage {
    message_id: i64,
    #[serde(default)]
    from: Option<TelegramUser>,
    chat: TelegramChat,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    caption: Option<String>,
    #[serde(default)]
    message_thread_id: Option<i64>,
    #[serde(default)]
    reply_to_message: Option<Box<TelegramMessage>>,
    #[serde(default)]
    photo: Option<Vec<TelegramPhotoSize>>,
    #[serde(default)]
    voice: Option<TelegramVoice>,
    #[serde(default)]
    audio: Option<TelegramAudio>,
    #[serde(default)]
    video: Option<TelegramVideo>,
    #[serde(default)]
    document: Option<TelegramDocument>,
}

#[derive(Debug, Deserialize)]
struct TelegramPhotoSize {
    file_id: String,
    #[serde(default)]
    file_size: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TelegramVoice {
    file_id: String,
    #[serde(default)]
    file_size: Option<u64>,
    #[serde(default)]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramAudio {
    file_id: String,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    file_size: Option<u64>,
    #[serde(default)]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramVideo {
    file_id: String,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    file_size: Option<u64>,
    #[serde(default)]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramDocument {
    file_id: String,
    #[serde(default)]
    file_name: Option<String>,
    #[serde(default)]
    file_size: Option<u64>,
    #[serde(default)]
    mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramCallbackQuery {
    id: String,
    from: TelegramUser,
    message: Option<TelegramMessage>,
    #[serde(default)]
    data: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TelegramUpdate {
    update_id: i64,
    #[serde(default)]
    message: Option<TelegramMessage>,
    #[serde(default)]
    callback_query: Option<TelegramCallbackQuery>,
}

struct TelegramAttachmentInput<'a> {
    file_id: &'a str,
    size: Option<u64>,
    mime: Option<&'a str>,
    file_name: Option<&'a str>,
    kind: &'a str,
    message_id: i64,
    sender_id: &'a str,
}

#[derive(Debug, Default, Deserialize)]
struct SentMessage {
    message_id: i64,
}

/// Native Telegram adapter.  The production constructor is crate-visible so
/// only the C6 factory can assemble it; tests use a private endpoint seam.
pub struct TelegramAdapter {
    config: TelegramConfig,
    services: AdapterServices,
    endpoint: TelegramEndpoint,
    http: reqwest::Client,
    seen: Arc<RwLock<HashMap<i64, Instant>>>,
    offset: AtomicI64,
    health: Arc<Mutex<ChannelHealth>>,
    running: Arc<AtomicBool>,
    cancel: CancellationToken,
    bot_username: Arc<Mutex<Option<String>>>,
    typing_tasks: Arc<Mutex<HashMap<String, CancellationToken>>>,
    media_timeout: Duration,
    typing_refresh_interval: Duration,
}

impl TelegramAdapter {
    pub(crate) fn new(config: TelegramConfig, services: AdapterServices) -> Self {
        Self::from_endpoint(config, services, TelegramEndpoint::production())
    }

    fn from_endpoint(
        config: TelegramConfig,
        services: AdapterServices,
        endpoint: TelegramEndpoint,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(35))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            services,
            endpoint,
            http,
            seen: Arc::new(RwLock::new(HashMap::new())),
            offset: AtomicI64::new(0),
            health: Arc::new(Mutex::new(ChannelHealth::new(ChannelHealthStatus::Unknown))),
            running: Arc::new(AtomicBool::new(false)),
            cancel: CancellationToken::new(),
            bot_username: Arc::new(Mutex::new(None)),
            typing_tasks: Arc::new(Mutex::new(HashMap::new())),
            media_timeout: MEDIA_DOWNLOAD_TIMEOUT,
            typing_refresh_interval: TYPING_REFRESH_INTERVAL,
        }
    }

    #[cfg(test)]
    fn with_test_endpoint(config: TelegramConfig, services: AdapterServices, base: &str) -> Self {
        Self::from_endpoint(config, services, TelegramEndpoint::test(base))
    }

    #[cfg(test)]
    fn with_bot_username(self, username: &str) -> Self {
        if let Ok(mut bot_username) = self.bot_username.lock() {
            *bot_username = Some(username.to_string());
        }
        self
    }

    #[cfg(test)]
    fn with_media_timeout(mut self, duration: Duration) -> Self {
        self.media_timeout = duration;
        self
    }

    #[cfg(test)]
    fn with_typing_refresh_interval(mut self, duration: Duration) -> Self {
        self.typing_refresh_interval = duration;
        self
    }

    fn static_capabilities() -> ChannelCapabilities {
        let mut capabilities = ChannelCapabilities::new([
            ChannelCapability::IngressText,
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
            ChannelCapability::InteractionTyping,
            ChannelCapability::InteractionListening,
            ChannelCapability::InteractionEdit,
            ChannelCapability::InteractionDelete,
            ChannelCapability::InteractionStreamFinalize,
            ChannelCapability::ReliabilityHealth,
            ChannelCapability::ReliabilityPacing,
            ChannelCapability::ReliabilitySupervisedRestart,
        ]);
        capabilities.limits.max_text_chars = Some(MAX_MESSAGE_CHARS);
        capabilities.limits.max_attachment_bytes = Some(MAX_ATTACHMENT_BYTES);
        capabilities.limits.supported_mime_types = BTreeSet::from([
            "image/*".to_string(),
            "audio/*".to_string(),
            "video/*".to_string(),
            "application/octet-stream".to_string(),
        ]);
        capabilities.limits.rate_limit_hint_ms = Some(250);
        capabilities
    }

    fn set_health(&self, status: ChannelHealthStatus, diagnosis: Option<String>) {
        if let Ok(mut health) = self.health.lock() {
            health.status = status;
            health.diagnosis = diagnosis;
            health.checked_at = chrono::Utc::now();
            if status == ChannelHealthStatus::Healthy {
                health.consecutive_failures = 0;
            }
        }
    }

    fn record_health_failure(&self, diagnosis: String) {
        if let Ok(mut health) = self.health.lock() {
            health.status = ChannelHealthStatus::Degraded;
            health.diagnosis = Some(diagnosis);
            health.consecutive_failures = health.consecutive_failures.saturating_add(1);
            health.checked_at = chrono::Utc::now();
        }
    }

    fn validate_config(&self) -> Result<(), AdapterError> {
        if !self.config.enabled {
            return Err(execution_error(
                "not_configured",
                "Telegram channel is disabled",
                None,
                false,
            ));
        }
        if self.config.token.trim().is_empty() {
            return Err(execution_error(
                "invalid_config",
                "Telegram bot token is empty",
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
                format!("command targets {}", target),
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

    fn method_url(&self, method: &str) -> String {
        format!(
            "{}/bot{}/{}",
            self.endpoint.api_base, self.config.token, method
        )
    }

    async fn call_json<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        body: Value,
    ) -> Result<T, AdapterError> {
        self.validate_config()?;
        let response = self
            .http
            .post(self.method_url(method))
            .json(&body)
            .send()
            .await
            .map_err(|error| {
                execution_error("telegram_transport", error.to_string(), None, true)
            })?;
        let status = response.status();
        let retry_after_header = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok());
        let parsed = response
            .json::<ApiResponse<T>>()
            .await
            .map_err(|error| execution_error("telegram_response", error.to_string(), None, true))?;
        if !parsed.ok {
            let diagnosis = parsed
                .description
                .unwrap_or_else(|| format!("Telegram API returned {status}"));
            if parsed.error_code == Some(429) || status.as_u16() == 429 {
                return Err(AdapterError::RateLimited {
                    retry_after: Duration::from_secs(
                        retry_after_header
                            .or_else(|| parsed.parameters.and_then(|p| p.retry_after))
                            .unwrap_or(1),
                    ),
                });
            }
            let code = if parsed.error_code == Some(400)
                && diagnosis.to_ascii_lowercase().contains("parse")
            {
                "telegram_parse_error"
            } else {
                "telegram_api"
            };
            return Err(execution_error(
                code,
                diagnosis,
                None,
                status.is_server_error(),
            ));
        }
        parsed.result.ok_or_else(|| {
            execution_error(
                "telegram_response",
                "Telegram response omitted result",
                None,
                true,
            )
        })
    }

    async fn call_multipart(&self, method: &str, form: Form) -> Result<SentMessage, AdapterError> {
        self.validate_config()?;
        let response = self
            .http
            .post(self.method_url(method))
            .multipart(form)
            .send()
            .await
            .map_err(|error| {
                execution_error("telegram_transport", error.to_string(), None, true)
            })?;
        let status = response.status();
        let retry_after_header = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.trim().parse::<u64>().ok());
        let parsed = response
            .json::<ApiResponse<SentMessage>>()
            .await
            .map_err(|error| execution_error("telegram_response", error.to_string(), None, true))?;
        if !parsed.ok {
            let diagnosis = parsed
                .description
                .unwrap_or_else(|| format!("Telegram API returned {status}"));
            if parsed.error_code == Some(429) || status.as_u16() == 429 {
                return Err(AdapterError::RateLimited {
                    retry_after: Duration::from_secs(
                        retry_after_header
                            .or_else(|| parsed.parameters.and_then(|p| p.retry_after))
                            .unwrap_or(1),
                    ),
                });
            }
            return Err(execution_error(
                "telegram_api",
                diagnosis,
                None,
                status.is_server_error(),
            ));
        }
        parsed.result.ok_or_else(|| {
            execution_error(
                "telegram_response",
                "Telegram response omitted message",
                None,
                true,
            )
        })
    }

    async fn get_updates(&self) -> Result<Vec<TelegramUpdate>, AdapterError> {
        let offset = self.offset.load(Ordering::Acquire);
        self.call_json(
            "getUpdates",
            json!({
                "offset": offset,
                "timeout": POLL_TIMEOUT_SECONDS,
                "allowed_updates": ["message", "callback_query"]
            }),
        )
        .await
    }

    #[cfg(test)]
    async fn process_update(
        &self,
        update: TelegramUpdate,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        self.process_batch(vec![update], context).await
    }

    async fn process_batch(
        &self,
        updates: Vec<TelegramUpdate>,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        let mut processed_ids = Vec::new();
        for update in updates {
            let update_id = update.update_id;
            if self.is_update_seen(update_id).await {
                continue;
            }
            if let Err(error) = self.process_update_inner(update, context).await {
                for processed_id in processed_ids {
                    self.forget_update(processed_id).await;
                }
                return Err(error);
            }
            processed_ids.push(update_id);
        }
        if let Some(max_update_id) = processed_ids.iter().copied().max() {
            self.remember_updates(&processed_ids).await;
            self.offset
                .fetch_max(max_update_id.saturating_add(1), Ordering::AcqRel);
        }
        Ok(())
    }

    async fn process_update_inner(
        &self,
        update: TelegramUpdate,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        if let Some(callback) = update.callback_query {
            self.process_callback(callback, context).await?;
        } else if let Some(message) = update.message {
            self.process_message(message, context).await?;
        }
        Ok(())
    }

    async fn is_update_seen(&self, update_id: i64) -> bool {
        let now = Instant::now();
        let mut seen = self.seen.write().await;
        seen.retain(|_, seen_at| now.duration_since(*seen_at) <= DEDUP_TTL);
        seen.contains_key(&update_id)
    }

    async fn remember_updates(&self, update_ids: &[i64]) {
        let now = Instant::now();
        let mut seen = self.seen.write().await;
        seen.retain(|_, seen_at| now.duration_since(*seen_at) <= DEDUP_TTL);
        for update_id in update_ids {
            seen.insert(*update_id, now);
        }
        while seen.len() > DEDUP_CAPACITY {
            let oldest = seen
                .iter()
                .min_by_key(|(_, seen_at)| **seen_at)
                .map(|(update_id, _)| *update_id);
            if let Some(update_id) = oldest {
                seen.remove(&update_id);
            } else {
                break;
            }
        }
    }

    async fn forget_update(&self, update_id: i64) {
        self.seen.write().await.remove(&update_id);
    }

    async fn process_callback(
        &self,
        callback: TelegramCallbackQuery,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        let _: bool = self
            .call_json(
                "answerCallbackQuery",
                json!({"callback_query_id": callback.id}),
            )
            .await?;
        let Some(message) = callback.message else {
            return Ok(());
        };
        if !is_sender_allowed(&self.config.allow_from, &callback.from.id.to_string()) {
            return Ok(());
        }
        let chat_id = message.chat.id.to_string();
        let mut address = ChannelAddress::new(CHANNEL, chat_id.clone());
        address.sender_id = Some(callback.from.id.to_string());
        address.thread_id = message.message_thread_id.map(|id| id.to_string());
        let mut correlation = Correlation::new(format!("{CHANNEL}:{chat_id}"));
        correlation.message_id = Some(message.message_id.to_string());
        let mut envelope = external_message_envelope(
            address,
            correlation,
            vec![ContentPart::Text {
                text: callback.data.clone().unwrap_or_default(),
            }],
            None,
            None,
        );
        envelope.extensions.insert(
            "telegram.callback_id".to_string(),
            json!(callback.id.clone()),
        );
        envelope
            .extensions
            .insert("telegram.callback_data".to_string(), json!(callback.data));
        context
            .fabric
            .admit_ingress(envelope, INGRESS_ADMISSION_DEADLINE, &context.cancel)
            .await
            .map_err(map_fabric_error)?;
        Ok(())
    }

    async fn process_message(
        &self,
        message: TelegramMessage,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        let sender_id = message
            .from
            .as_ref()
            .map(|user| user.id.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let username = message.from.as_ref().and_then(|user| user.username.clone());
        if !is_sender_allowed(&self.config.allow_from, &sender_id)
            && !username
                .as_deref()
                .is_some_and(|name| is_sender_allowed(&self.config.allow_from, name))
        {
            return Ok(());
        }
        let mut text = message
            .text
            .clone()
            .or(message.caption.clone())
            .unwrap_or_default();
        let is_group = is_group_chat(message.chat.kind.as_deref());
        let is_reply_to_bot = message
            .reply_to_message
            .as_ref()
            .and_then(|reply| reply.from.as_ref())
            .is_some_and(|sender| sender.is_bot);
        let bot_username = self
            .bot_username
            .lock()
            .ok()
            .and_then(|username| username.clone());
        if !should_respond_in_group(&text, is_group, is_reply_to_bot, bot_username.as_deref()) {
            return Ok(());
        }
        if is_group {
            text = strip_bot_mention(&text, bot_username.as_deref());
        }

        let mut parts = Vec::new();
        if !text.trim().is_empty() {
            parts.push(ContentPart::Text {
                text: truncate(&text, MAX_CAPTION_CHARS),
            });
        }
        if let Some(photo) = message.photo.as_ref().and_then(|items| items.last()) {
            parts.push(
                self.fetch_attachment(
                    TelegramAttachmentInput {
                        file_id: &photo.file_id,
                        size: photo.file_size,
                        mime: None,
                        file_name: None,
                        kind: "image",
                        message_id: message.message_id,
                        sender_id: &sender_id,
                    },
                    &context.cancel,
                )
                .await?,
            );
        }
        if let Some(voice) = &message.voice {
            parts.push(
                self.fetch_attachment(
                    TelegramAttachmentInput {
                        file_id: &voice.file_id,
                        size: voice.file_size,
                        mime: voice.mime_type.as_deref(),
                        file_name: None,
                        kind: "audio",
                        message_id: message.message_id,
                        sender_id: &sender_id,
                    },
                    &context.cancel,
                )
                .await?,
            );
        }
        if let Some(audio) = &message.audio {
            parts.push(
                self.fetch_attachment(
                    TelegramAttachmentInput {
                        file_id: &audio.file_id,
                        size: audio.file_size,
                        mime: audio.mime_type.as_deref(),
                        file_name: audio.file_name.as_deref(),
                        kind: "audio",
                        message_id: message.message_id,
                        sender_id: &sender_id,
                    },
                    &context.cancel,
                )
                .await?,
            );
        }
        if let Some(video) = &message.video {
            parts.push(
                self.fetch_attachment(
                    TelegramAttachmentInput {
                        file_id: &video.file_id,
                        size: video.file_size,
                        mime: video.mime_type.as_deref(),
                        file_name: video.file_name.as_deref(),
                        kind: "video",
                        message_id: message.message_id,
                        sender_id: &sender_id,
                    },
                    &context.cancel,
                )
                .await?,
            );
        }
        if let Some(document) = &message.document {
            parts.push(
                self.fetch_attachment(
                    TelegramAttachmentInput {
                        file_id: &document.file_id,
                        size: document.file_size,
                        mime: document.mime_type.as_deref(),
                        file_name: document.file_name.as_deref(),
                        kind: "file",
                        message_id: message.message_id,
                        sender_id: &sender_id,
                    },
                    &context.cancel,
                )
                .await?,
            );
        }
        if parts.is_empty() {
            return Ok(());
        }
        let chat_id = message.chat.id.to_string();
        let mut address = ChannelAddress::new(CHANNEL, chat_id.clone());
        address.sender_id = Some(sender_id.clone());
        address.thread_id = message.message_thread_id.map(|id| id.to_string());
        let mut correlation = Correlation::new(format!("{CHANNEL}:{chat_id}"));
        correlation.message_id = Some(message.message_id.to_string());
        correlation.reply_to = message
            .reply_to_message
            .as_ref()
            .map(|reply| reply.message_id.to_string());
        let mut envelope = external_message_envelope(address, correlation, parts, None, None);
        if let Some(kind) = message.chat.kind {
            envelope
                .extensions
                .insert("telegram.chat_kind".to_string(), json!(kind));
        }
        context
            .fabric
            .admit_ingress(envelope, INGRESS_ADMISSION_DEADLINE, &context.cancel)
            .await
            .map_err(map_fabric_error)?;
        Ok(())
    }

    async fn fetch_attachment(
        &self,
        input: TelegramAttachmentInput<'_>,
        cancel: &CancellationToken,
    ) -> Result<ContentPart, AdapterError> {
        let operation = async {
            if let Some(size_bytes) = input.size.filter(|size| *size > MAX_ATTACHMENT_BYTES) {
                return Err(media_too_large(size_bytes));
            }
            let file: TelegramFile = self
                .call_json("getFile", json!({"file_id": input.file_id}))
                .await?;
            if let Some(size_bytes) = file.file_size.filter(|size| *size > MAX_ATTACHMENT_BYTES) {
                return Err(media_too_large(size_bytes));
            }
            let path = file.file_path.ok_or_else(|| {
                execution_error(
                    "telegram_media_missing_path",
                    format!("Telegram getFile omitted file_path for {}", input.file_id),
                    None,
                    false,
                )
            })?;
            let url = format!(
                "{}/file/bot{}/{}",
                self.endpoint.api_base, self.config.token, path
            );
            let response = self.http.get(url).send().await.map_err(|error| {
                execution_error("telegram_media", error.to_string(), None, true)
            })?;
            let status = response.status();
            if !status.is_success() {
                return Err(execution_error(
                    "telegram_media_http",
                    format!("Telegram media download returned {status}"),
                    None,
                    status.is_server_error(),
                ));
            }
            if response
                .content_length()
                .is_some_and(|size_bytes| size_bytes > MAX_ATTACHMENT_BYTES)
            {
                return Err(media_too_large(
                    response
                        .content_length()
                        .unwrap_or(MAX_ATTACHMENT_BYTES + 1),
                ));
            }
            let response_mime = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .and_then(normalize_mime);
            let effective_mime =
                resolve_media_mime(input.kind, input.mime, response_mime.as_deref())?;
            let mut stream = response.bytes_stream();
            let mut bytes = Vec::new();
            while let Some(chunk) = stream.next().await {
                let chunk = chunk.map_err(|error| {
                    execution_error("telegram_media", error.to_string(), None, true)
                })?;
                let next_size = bytes.len() as u64 + chunk.len() as u64;
                if next_size > MAX_ATTACHMENT_BYTES {
                    return Err(media_too_large(next_size));
                }
                bytes.extend_from_slice(&chunk);
            }
            let attachment_name = input
                .file_name
                .filter(|name| !name.trim().is_empty())
                .map(str::to_string)
                .or_else(|| path.rsplit('/').next().map(str::to_string));
            let attachment_input = IngressAttachment {
                source_channel: CHANNEL.to_string(),
                platform_message_id: Some(input.message_id.to_string()),
                sender_id: Some(input.sender_id.to_string()),
                file_name: attachment_name,
                declared_mime: Some(effective_mime),
                bytes,
            };
            attachment_input.validate().map_err(|error| {
                execution_error("attachment_store", error.to_string(), None, false)
            })?;
            let reference = self
                .services
                .attachments
                .put(attachment_input)
                .await
                .map_err(|error| {
                    execution_error("attachment_store", error.to_string(), None, false)
                })?;
            let stored = self
                .services
                .attachments
                .get(&reference)
                .await
                .map_err(|error| {
                    execution_error("attachment_store_corrupt", error.to_string(), None, false)
                })?;
            stored.validate().map_err(|error| {
                execution_error("attachment_store_corrupt", error.to_string(), None, false)
            })?;
            Ok(content_part_for_kind(input.kind, reference))
        };
        tokio::select! {
            _ = cancel.cancelled() => Err(AdapterError::Stopped),
            _ = self.cancel.cancelled() => Err(AdapterError::Stopped),
            result = timeout(self.media_timeout, operation) => result.unwrap_or_else(|_| {
                Err(execution_error(
                    "telegram_media_timeout",
                    format!("Telegram media operation exceeded {:?}", self.media_timeout),
                    None,
                    true,
                ))
            }),
        }
    }

    async fn send_text_message(&self, mut body: Value) -> Result<SentMessage, AdapterError> {
        let fallback_text = body.get("text").and_then(Value::as_str).map(html_to_plain);
        match self.call_json("sendMessage", body.clone()).await {
            Err(AdapterError::Execution { code, .. })
                if code == "telegram_parse_error" && body.get("parse_mode").is_some() =>
            {
                if let Some(object) = body.as_object_mut() {
                    object.remove("parse_mode");
                    if let Some(fallback_text) = fallback_text {
                        object.insert("text".to_string(), Value::String(fallback_text));
                    }
                }
                self.call_json("sendMessage", body).await
            }
            result => result,
        }
    }

    async fn execute_send(
        &self,
        envelope: ChannelEnvelopeV1,
    ) -> Result<DeliveryReceipt, AdapterError> {
        let (parts, address, correlation) = match envelope.payload {
            ChannelPayloadV1::Message { parts, .. } | ChannelPayloadV1::Stream { parts, .. } => {
                (parts, envelope.address, envelope.correlation)
            }
            _ => {
                return Err(execution_error(
                    "invalid_payload",
                    "Telegram send requires message/stream payload",
                    None,
                    false,
                ))
            }
        };
        if address.chat_id.trim().is_empty() {
            return Err(execution_error(
                "invalid_recipient",
                "Telegram chat id is empty",
                None,
                false,
            ));
        }
        let reply_to = correlation
            .reply_to
            .as_deref()
            .map(parse_message_id)
            .transpose()?;
        let text = parts
            .iter()
            .filter_map(|part| match part {
                ContentPart::Text { text } => Some(text.clone()),
                ContentPart::Markdown { markdown } => Some(markdown_to_html(markdown)),
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
        let has_media = parts.iter().any(|part| {
            matches!(
                part,
                ContentPart::Image { .. }
                    | ContentPart::Audio { .. }
                    | ContentPart::Video { .. }
                    | ContentPart::File { .. }
            )
        });
        let plain_caption = html_to_plain(&text);
        let caption = truncate(&plain_caption, MAX_CAPTION_CHARS);
        let caption_remainder = plain_caption
            .chars()
            .skip(MAX_CAPTION_CHARS)
            .collect::<String>();
        let text_chunks = if has_media {
            Vec::new()
        } else {
            split_chunks(&text, MAX_MESSAGE_CHARS)
        };
        let media_count = parts
            .iter()
            .filter(|part| {
                matches!(
                    part,
                    ContentPart::Image { .. }
                        | ContentPart::Audio { .. }
                        | ContentPart::Video { .. }
                        | ContentPart::File { .. }
                )
            })
            .count();
        let remainder_chunks = if has_media {
            split_chunks(&caption_remainder, MAX_MESSAGE_CHARS)
        } else {
            Vec::new()
        };
        let total_units = text_chunks.len() + media_count + remainder_chunks.len();
        let mut last_id = None;
        let mut delivered_units = 0;
        for (index, chunk) in text_chunks.into_iter().enumerate() {
            let mut body = json!({
                "chat_id": address.chat_id,
                "text": chunk,
                "parse_mode": "HTML"
            });
            if index == 0 {
                if let Some(reply_to) = reply_to {
                    body["reply_parameters"] = json!({"message_id": reply_to});
                }
            }
            match self.send_text_message(body).await {
                Ok(sent) => {
                    last_id = Some(sent.message_id.to_string());
                    delivered_units += 1;
                }
                Err(error) if delivered_units > 0 => {
                    return Ok(partial_delivery_receipt(
                        &address,
                        last_id,
                        delivered_units,
                        total_units,
                        &error,
                    ));
                }
                Err(error) => return Err(error),
            }
        }
        let mut media_index = 0;
        for part in parts {
            let (method, field, reference) = match part {
                ContentPart::Image { attachment } => ("sendPhoto", "photo", attachment),
                ContentPart::Audio { attachment, .. } => {
                    let method = audio_method(&attachment);
                    (method.0, method.1, attachment)
                }
                ContentPart::Video { attachment } => ("sendVideo", "video", attachment),
                ContentPart::File { attachment } => ("sendDocument", "document", attachment),
                _ => continue,
            };
            let stored = self
                .services
                .attachments
                .get(&reference)
                .await
                .map_err(|error| {
                    execution_error("attachment_read", error.to_string(), None, false)
                })?;
            stored.validate().map_err(|error| {
                execution_error("attachment_read_corrupt", error.to_string(), None, false)
            })?;
            if stored.bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
                return Err(media_too_large(stored.bytes.len() as u64));
            }
            let file_name = reference
                .file_name
                .clone()
                .unwrap_or_else(|| "attachment.bin".to_string());
            let part = Part::bytes(stored.bytes)
                .file_name(file_name)
                .mime_str(&reference.media_type)
                .map_err(|error| {
                    execution_error("attachment_mime", error.to_string(), None, false)
                })?;
            let mut form = Form::new()
                .text("chat_id", address.chat_id.clone())
                .part(field, part);
            if media_index == 0 {
                if let Some(reply_to) = reply_to {
                    form = form.text(
                        "reply_parameters",
                        json!({"message_id": reply_to}).to_string(),
                    );
                }
                if !caption.is_empty() {
                    form = form.text("caption", caption.clone());
                }
            }
            match self.call_multipart(method, form).await {
                Ok(sent) => {
                    last_id = Some(sent.message_id.to_string());
                    delivered_units += 1;
                    media_index += 1;
                }
                Err(error) if delivered_units > 0 => {
                    return Ok(partial_delivery_receipt(
                        &address,
                        last_id,
                        delivered_units,
                        total_units,
                        &error,
                    ));
                }
                Err(error) => return Err(error),
            }
        }
        for chunk in remainder_chunks {
            let sent: SentMessage = match self
                .send_text_message(json!({
                    "chat_id": address.chat_id,
                    "text": chunk,
                }))
                .await
            {
                Ok(sent) => sent,
                Err(error) if delivered_units > 0 => {
                    return Ok(partial_delivery_receipt(
                        &address,
                        last_id,
                        delivered_units,
                        total_units,
                        &error,
                    ));
                }
                Err(error) => return Err(error),
            };
            last_id = Some(sent.message_id.to_string());
            delivered_units += 1;
        }
        if delivered_units == 0 {
            return Err(execution_error(
                "empty_message",
                "Telegram message has no sendable content",
                None,
                false,
            ));
        }
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id,
            last_id,
            address.thread_id,
        ))
    }

    async fn execute_edit(
        &self,
        address: ChannelAddress,
        target: String,
        parts: Vec<ContentPart>,
    ) -> Result<DeliveryReceipt, AdapterError> {
        let target_id = parse_message_id(&target)?;
        let text = content_to_text(&parts);
        let _: SentMessage = self
            .call_json(
                "editMessageText",
                json!({
                    "chat_id": address.chat_id,
                    "message_id": target_id,
                    "text": truncate(&text, MAX_MESSAGE_CHARS),
                    "parse_mode": "HTML"
                }),
            )
            .await?;
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id,
            Some(target),
            address.thread_id,
        ))
    }

    async fn execute_delete(
        &self,
        address: ChannelAddress,
        target: String,
    ) -> Result<DeliveryReceipt, AdapterError> {
        let target_id = parse_message_id(&target)?;
        let _: bool = self
            .call_json(
                "deleteMessage",
                json!({"chat_id": address.chat_id, "message_id": target_id}),
            )
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
        state: TypingState,
    ) -> Result<DeliveryReceipt, AdapterError> {
        let action = match state {
            TypingState::Started => "typing",
            TypingState::Listening => "record_audio",
            TypingState::Stopped => {
                self.cancel_typing(&address.chat_id);
                return Ok(accepted_receipt(
                    CHANNEL,
                    address.chat_id,
                    None,
                    address.thread_id,
                ));
            }
        };
        self.cancel_typing(&address.chat_id);
        let _: Value = self
            .call_json(
                "sendChatAction",
                json!({"chat_id": address.chat_id, "action": action}),
            )
            .await?;
        let refresh_cancel = CancellationToken::new();
        if let Ok(mut tasks) = self.typing_tasks.lock() {
            tasks.insert(address.chat_id.clone(), refresh_cancel.clone());
        }
        let http = self.http.clone();
        let api_base = self.endpoint.api_base.clone();
        let token = self.config.token.clone();
        let chat_id = address.chat_id.clone();
        let refresh_interval = self.typing_refresh_interval;
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = refresh_cancel.cancelled() => break,
                    _ = sleep(refresh_interval) => {}
                }
                if refresh_chat_action(&http, &api_base, &token, &chat_id, action)
                    .await
                    .is_err()
                {
                    break;
                }
            }
        });
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id,
            None,
            address.thread_id,
        ))
    }

    async fn load_bot_identity(&self) -> Result<(), AdapterError> {
        let user: TelegramUser = self.call_json("getMe", json!({})).await?;
        if let Ok(mut username) = self.bot_username.lock() {
            *username = user.username;
        }
        Ok(())
    }

    fn cancel_typing(&self, chat_id: &str) {
        if let Ok(mut tasks) = self.typing_tasks.lock() {
            if let Some(cancel) = tasks.remove(chat_id) {
                cancel.cancel();
            }
        }
    }

    async fn probe_health(&self) -> Result<DeliveryReceipt, AdapterError> {
        match self.load_bot_identity().await {
            Ok(()) => {
                self.set_health(ChannelHealthStatus::Healthy, None);
                Ok(accepted_receipt(CHANNEL, "health", None, None))
            }
            Err(error) => {
                self.set_health(ChannelHealthStatus::Down, Some(error.to_string()));
                Err(error)
            }
        }
    }
}

#[async_trait]
impl ChannelAdapter for TelegramAdapter {
    fn name(&self) -> ChannelId {
        ChannelId::new(CHANNEL).expect("static Telegram channel id")
    }
    fn capabilities(&self) -> ChannelCapabilities {
        Self::static_capabilities()
    }

    async fn start(&self, context: AdapterContext) -> Result<(), AdapterError> {
        if context.cancel.is_cancelled() {
            return Ok(());
        }
        self.validate_config()?;
        if self.running.swap(true, Ordering::AcqRel) {
            return Err(execution_error(
                "already_running",
                "Telegram adapter is already running",
                None,
                false,
            ));
        }
        if let Err(error) = self.load_bot_identity().await {
            self.set_health(ChannelHealthStatus::Down, Some(error.to_string()));
            self.running.store(false, Ordering::Release);
            return Err(error);
        }
        let mut consecutive_failures = 0_u32;
        loop {
            if context.cancel.is_cancelled() || self.cancel.is_cancelled() {
                break;
            }
            let updates = tokio::select! {
                biased;
                _ = context.cancel.cancelled() => break,
                _ = self.cancel.cancelled() => break,
                result = self.get_updates() => result,
            };
            match updates {
                Ok(updates) => {
                    if let Err(error) = self.process_batch(updates, &context).await {
                        consecutive_failures = consecutive_failures.saturating_add(1);
                        self.record_health_failure(error.to_string());
                        let delay = reconnect_delay(consecutive_failures);
                        if !sleep_or_cancel(delay, &context, &self.cancel).await {
                            break;
                        }
                    } else {
                        consecutive_failures = 0;
                        self.set_health(ChannelHealthStatus::Healthy, None);
                    }
                }
                Err(error) => {
                    consecutive_failures = consecutive_failures.saturating_add(1);
                    self.record_health_failure(error.to_string());
                    if !sleep_or_cancel(
                        reconnect_delay(consecutive_failures),
                        &context,
                        &self.cancel,
                    )
                    .await
                    {
                        break;
                    }
                }
            }
        }
        self.running.store(false, Ordering::Release);
        if context.cancel.is_cancelled() || self.cancel.is_cancelled() {
            self.set_health(
                ChannelHealthStatus::Down,
                Some("Telegram adapter listener stopped".to_string()),
            );
        }
        Ok(())
    }

    async fn execute(&self, command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError> {
        self.ensure_supported(&command)?;
        match command {
            ChannelCommand::Send { envelope, .. } => self.execute_send(envelope).await,
            ChannelCommand::Typing { address, state, .. } => {
                self.execute_typing(address, state).await
            }
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
            ChannelCommand::FinalizeStream {
                address,
                correlation,
                parts,
                ..
            } => {
                if let Some(target) = correlation.message_id {
                    self.execute_edit(address, target, parts).await
                } else {
                    self.execute_send(ChannelEnvelopeV1::new(
                        ChannelDirection::Egress,
                        address,
                        correlation,
                        ChannelOrigin::Runtime,
                        ChannelPayloadV1::Stream {
                            phase: agent_diva_core::channel::StreamPhase::Finalized,
                            parts,
                        },
                    ))
                    .await
                }
            }
            ChannelCommand::React { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionReaction,
            }),
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
        if let Ok(mut tasks) = self.typing_tasks.lock() {
            for cancel in tasks.values() {
                cancel.cancel();
            }
            tasks.clear();
        }
        self.running.store(false, Ordering::Release);
        self.set_health(
            ChannelHealthStatus::Down,
            Some("Telegram adapter stopped".to_string()),
        );
        Ok(())
    }
}

fn is_group_chat(kind: Option<&str>) -> bool {
    matches!(kind, Some("group" | "supergroup"))
}

fn should_respond_in_group(
    text: &str,
    is_group: bool,
    is_reply_to_bot: bool,
    bot_username: Option<&str>,
) -> bool {
    if !is_group {
        return true;
    }
    if is_reply_to_bot || text.trim_start().starts_with('/') {
        return true;
    }
    bot_username.is_some_and(|username| text.contains(&format!("@{username}")))
}

fn strip_bot_mention(text: &str, bot_username: Option<&str>) -> String {
    bot_username
        .map(|username| text.replace(&format!("@{username}"), ""))
        .unwrap_or_else(|| text.to_string())
        .trim()
        .to_string()
}

fn normalize_mime(value: &str) -> Option<String> {
    let mime = value.split(';').next()?.trim().to_ascii_lowercase();
    (!mime.is_empty()).then_some(mime)
}

fn resolve_media_mime(
    kind: &str,
    declared: Option<&str>,
    actual: Option<&str>,
) -> Result<String, AdapterError> {
    let declared = declared.and_then(normalize_mime);
    let actual = actual.and_then(normalize_mime);
    if let (Some(declared), Some(actual)) = (&declared, &actual) {
        if actual != "application/octet-stream" && declared != actual {
            return Err(execution_error(
                "telegram_media_mime",
                format!("declared MIME {declared} differs from response MIME {actual}"),
                None,
                false,
            ));
        }
    }
    let effective = declared.or(actual).ok_or_else(|| {
        execution_error(
            "telegram_media_mime",
            "Telegram media response omitted Content-Type and no declared MIME was supplied",
            None,
            false,
        )
    })?;
    let accepted = match kind {
        "image" => effective.starts_with("image/"),
        "audio" => effective.starts_with("audio/"),
        "video" => effective.starts_with("video/"),
        "file" => true,
        _ => false,
    };
    if !accepted {
        return Err(execution_error(
            "telegram_media_mime",
            format!("MIME {effective} is not valid for Telegram {kind} media"),
            None,
            false,
        ));
    }
    Ok(effective)
}

fn media_too_large(size_bytes: u64) -> AdapterError {
    execution_error(
        "telegram_media_too_large",
        format!("Telegram media is {size_bytes} bytes; limit is {MAX_ATTACHMENT_BYTES}"),
        None,
        false,
    )
}

fn content_part_for_kind(
    kind: &str,
    reference: agent_diva_core::channel::AttachmentRef,
) -> ContentPart {
    match kind {
        "image" => ContentPart::Image {
            attachment: reference,
        },
        "audio" => ContentPart::Audio {
            attachment: reference,
            transcript: None,
        },
        "video" => ContentPart::Video {
            attachment: reference,
        },
        _ => ContentPart::File {
            attachment: reference,
        },
    }
}

fn parse_message_id(value: &str) -> Result<i64, AdapterError> {
    value.parse::<i64>().map_err(|error| {
        execution_error(
            "invalid_message_id",
            format!("Telegram message id {value:?} is not an integer: {error}"),
            None,
            false,
        )
    })
}

fn audio_method(
    reference: &agent_diva_core::channel::AttachmentRef,
) -> (&'static str, &'static str) {
    let is_voice = reference
        .file_name
        .as_deref()
        .map(str::to_ascii_lowercase)
        .is_some_and(|name| {
            name.ends_with(".ogg") || name.ends_with(".oga") || name.ends_with(".opus")
        })
        || matches!(
            reference.media_type.as_str(),
            "audio/ogg" | "audio/oga" | "audio/opus"
        );
    if is_voice {
        ("sendVoice", "voice")
    } else {
        ("sendAudio", "audio")
    }
}

fn reconnect_delay(consecutive_failures: u32) -> Duration {
    let exponent = consecutive_failures.saturating_sub(1).min(4);
    std::cmp::min(
        BASE_RECONNECT_DELAY * 2_u32.saturating_pow(exponent),
        MAX_RECONNECT_DELAY,
    )
}

fn partial_delivery_receipt(
    address: &ChannelAddress,
    platform_message_id: Option<String>,
    delivered_units: usize,
    total_units: usize,
    error: &AdapterError,
) -> DeliveryReceipt {
    let mut receipt = delivery_receipt(
        DeliveryStatus::Failed,
        CHANNEL,
        address.chat_id.clone(),
        platform_message_id,
        address.thread_id.clone(),
    );
    receipt.error_code = Some("partial_delivery".to_string());
    receipt.diagnosis = Some(format!(
        "delivered {delivered_units}/{total_units} Telegram send units before failure: {error}"
    ));
    receipt.retry_after_ms = error
        .retry_after()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok());
    receipt
}

async fn refresh_chat_action(
    http: &reqwest::Client,
    api_base: &str,
    token: &str,
    chat_id: &str,
    action: &str,
) -> Result<(), AdapterError> {
    let response = http
        .post(format!("{api_base}/bot{token}/sendChatAction"))
        .json(&json!({"chat_id": chat_id, "action": action}))
        .send()
        .await
        .map_err(|error| execution_error("telegram_transport", error.to_string(), None, true))?;
    let status = response.status();
    let retry_after_header = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse::<u64>().ok());
    let parsed = response
        .json::<ApiResponse<bool>>()
        .await
        .map_err(|error| execution_error("telegram_response", error.to_string(), None, true))?;
    if !parsed.ok {
        let diagnosis = parsed
            .description
            .unwrap_or_else(|| format!("Telegram API returned {status}"));
        if parsed.error_code == Some(429) || status.as_u16() == 429 {
            return Err(AdapterError::RateLimited {
                retry_after: Duration::from_secs(
                    retry_after_header
                        .or_else(|| {
                            parsed
                                .parameters
                                .and_then(|parameters| parameters.retry_after)
                        })
                        .unwrap_or(1),
                ),
            });
        }
        return Err(execution_error(
            "telegram_api",
            diagnosis,
            None,
            status.is_server_error(),
        ));
    }
    parsed.result.ok_or_else(|| {
        execution_error(
            "telegram_response",
            "Telegram response omitted sendChatAction result",
            None,
            true,
        )
    })?;
    Ok(())
}

fn map_fabric_error(error: agent_diva_core::channel::FabricAdmissionError) -> AdapterError {
    let diagnosis = error.to_string();
    match error {
        agent_diva_core::channel::FabricAdmissionError::Busy { retry_after, .. } => {
            execution_error("fabric_busy", diagnosis, Some(retry_after), true)
        }
        agent_diva_core::channel::FabricAdmissionError::Cancelled { .. } => AdapterError::Stopped,
        agent_diva_core::channel::FabricAdmissionError::Closed { .. } => {
            execution_error("fabric_closed", diagnosis, None, true)
        }
        agent_diva_core::channel::FabricAdmissionError::InvalidEnvelope(_) => {
            execution_error("invalid_envelope", diagnosis, None, false)
        }
    }
}

async fn sleep_or_cancel(
    duration: Duration,
    context: &AdapterContext,
    local_cancel: &CancellationToken,
) -> bool {
    tokio::select! { _ = context.cancel.cancelled() => false, _ = local_cancel.cancelled() => false, _ = sleep(duration) => true }
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

fn truncate(text: &str, limit: usize) -> String {
    text.chars().take(limit).collect()
}

fn content_to_text(parts: &[ContentPart]) -> String {
    parts
        .iter()
        .filter_map(|part| match part {
            ContentPart::Text { text } => Some(text.clone()),
            ContentPart::Markdown { markdown } => Some(markdown_to_html(markdown)),
            ContentPart::Location {
                latitude,
                longitude,
                ..
            } => Some(format!("{latitude},{longitude}")),
            ContentPart::Reference { uri, .. } => Some(uri.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn markdown_to_html(markdown: &str) -> String {
    let mut value = html_escape::encode_text(markdown).to_string();
    for (from, to) in [("**", "<b>"), ("__", "<b>"), ("~~", "<s>")] {
        if value.contains(from) {
            value = value.replace(from, to);
        }
    }
    value
}

fn html_to_plain(html: &str) -> String {
    let mut output = String::with_capacity(html.len());
    let mut in_tag = false;
    for character in html.chars() {
        match character {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => output.push(character),
            _ => {}
        }
    }
    output
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{AttachmentStoreError, ChannelAttachmentStore, StoredAttachment};
    use agent_diva_core::channel::{AttachmentRef, FabricKernel};
    use agent_diva_core::config::Config;
    use async_trait::async_trait;
    use sha2::Digest;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    #[derive(Default)]
    struct Store {
        records: Mutex<HashMap<String, StoredAttachment>>,
        corrupt_on_get: bool,
    }
    #[async_trait]
    impl ChannelAttachmentStore for Store {
        async fn put(
            &self,
            input: IngressAttachment,
        ) -> Result<AttachmentRef, crate::adapter::AttachmentStoreError> {
            input.validate()?;
            let digest = format!("{:x}", sha2::Sha256::digest(&input.bytes));
            let reference = AttachmentRef {
                uri: format!("sha256:{digest}"),
                media_type: input
                    .declared_mime
                    .unwrap_or_else(|| "application/octet-stream".into()),
                size_bytes: input.bytes.len() as u64,
                sha256: digest,
                file_name: input.file_name,
            };
            let stored = StoredAttachment {
                reference: reference.clone(),
                bytes: input.bytes,
            };
            stored.validate()?;
            self.records
                .lock()
                .map_err(|_| AttachmentStoreError::Backend {
                    diagnosis: "test store lock poisoned".to_string(),
                })?
                .insert(reference.uri.clone(), stored);
            Ok(reference)
        }
        async fn get(
            &self,
            reference: &AttachmentRef,
        ) -> Result<StoredAttachment, crate::adapter::AttachmentStoreError> {
            let stored = self
                .records
                .lock()
                .map_err(|_| AttachmentStoreError::Backend {
                    diagnosis: "test store lock poisoned".to_string(),
                })?
                .get(&reference.uri)
                .cloned()
                .ok_or_else(|| AttachmentStoreError::NotFound {
                    uri: reference.uri.clone(),
                })?;
            if stored.reference != *reference {
                return Err(AttachmentStoreError::InvalidReference {
                    diagnosis: "stored metadata differs from requested reference".to_string(),
                });
            }
            if self.corrupt_on_get {
                let mut stored = stored;
                stored.bytes.push(0);
                return Ok(stored);
            }
            stored.validate()?;
            Ok(stored)
        }
    }
    fn adapter() -> TelegramAdapter {
        adapter_at("http://127.0.0.1:1")
    }

    fn adapter_at(base: &str) -> TelegramAdapter {
        adapter_at_with_store(base, Arc::new(Store::default()))
    }

    fn adapter_at_with_store(base: &str, store: Arc<Store>) -> TelegramAdapter {
        let mut config = Config::default().channels.telegram;
        config.enabled = true;
        config.token = "test-token".into();
        TelegramAdapter::with_test_endpoint(config, AdapterServices::new(store), base)
    }

    fn fixture_api(name: &str) -> Vec<u8> {
        let fixtures: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/api-responses.json"
        ))
        .unwrap();
        fixtures[name].to_string().into_bytes()
    }

    fn polling_fixture(name: &str) -> Vec<u8> {
        let fixtures: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/polling-responses.json"
        ))
        .unwrap();
        fixtures[name].to_string().into_bytes()
    }

    fn get_file_fixture(path: &str, size: u64) -> Vec<u8> {
        json!({
            "ok": true,
            "result": {"file_path": path, "file_size": size}
        })
        .to_string()
        .into_bytes()
    }

    fn text_update(update_id: i64, message_id: i64, chat_id: i64, text: &str) -> TelegramUpdate {
        serde_json::from_value(json!({
            "update_id": update_id,
            "message": {
                "message_id": message_id,
                "from": {"id": 42, "username": "fixture-user"},
                "chat": {"id": chat_id, "kind": "private"},
                "text": text
            }
        }))
        .unwrap()
    }

    fn media_update(
        update_id: i64,
        message_id: i64,
        file_id: &str,
        kind: &str,
        file_name: Option<&str>,
        declared_mime: Option<&str>,
        file_size: Option<u64>,
    ) -> TelegramUpdate {
        let mut media = serde_json::Map::new();
        media.insert("file_id".to_string(), json!(file_id));
        if let Some(file_name) = file_name {
            media.insert("file_name".to_string(), json!(file_name));
        }
        if let Some(declared_mime) = declared_mime {
            media.insert("mime_type".to_string(), json!(declared_mime));
        }
        if let Some(file_size) = file_size {
            media.insert("file_size".to_string(), json!(file_size));
        }
        let mut message = serde_json::Map::from_iter([
            ("message_id".to_string(), json!(message_id)),
            (
                "from".to_string(),
                json!({"id": 42, "username": "fixture-user"}),
            ),
            ("chat".to_string(), json!({"id": 42, "kind": "private"})),
        ]);
        message.insert(kind.to_string(), Value::Object(media));
        serde_json::from_value(Value::Object(serde_json::Map::from_iter([
            ("update_id".to_string(), json!(update_id)),
            ("message".to_string(), Value::Object(message)),
        ])))
        .unwrap()
    }

    #[test]
    fn capabilities_match_frozen_matrix() {
        let capabilities = adapter().capabilities();
        assert!(capabilities.supports(ChannelCapability::IngressThread));
        assert!(capabilities.supports(ChannelCapability::InteractionListening));
        assert!(!capabilities.supports(ChannelCapability::EgressCard));
        assert_eq!(capabilities.limits.max_text_chars, Some(MAX_MESSAGE_CHARS));
    }
    #[test]
    fn chunk_and_caption_limits_are_unicode_safe() {
        assert_eq!(split_chunks("你好世界", 2), vec!["你好", "世界"]);
        assert_eq!(truncate("你好世界", 3), "你好世");
    }
    #[test]
    fn markdown_is_escaped_before_html_mode() {
        assert!(markdown_to_html("<x>").contains("&lt;x&gt;"));
    }
    #[test]
    fn unsupported_reaction_is_rejected_before_transport() {
        let adapter = adapter();
        let command = ChannelCommand::React {
            address: ChannelAddress::new(CHANNEL, "chat"),
            correlation: Correlation::new("telegram:chat"),
            target_message_id: "m".into(),
            operation: agent_diva_core::channel::ReactionOperation::Add,
            emoji: "👍".into(),
            idempotency_key: None,
        };
        let result = futures::executor::block_on(adapter.execute(command));
        assert!(matches!(
            result,
            Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionReaction
            })
        ));
    }

    #[tokio::test]
    async fn wire_telegram_ingress_preserves_identity_media_and_callback_ack() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                br#"{"ok":true,"result":{"file_path":"photos/photo.png"}}"#,
            ),
            fixture_response(
                "/file/bottest-token/photos/photo.png",
                200,
                "image/png",
                b"fixture-photo",
            ),
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                br#"{"ok":true,"result":{"file_path":"documents/report.pdf"}}"#,
            ),
            fixture_response(
                "/file/bottest-token/documents/report.pdf",
                200,
                "application/pdf",
                b"fixture-document",
            ),
            fixture_response(
                "/bottest-token/answerCallbackQuery",
                200,
                "application/json",
                br#"{"ok":true,"result":true}"#,
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };

        let text: TelegramUpdate = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/inbound-text.json"
        ))
        .unwrap();
        adapter.process_update(text, &context).await.unwrap();
        let text_envelope = consumer.recv_ingress().await.unwrap().envelope().clone();
        assert_eq!(text_envelope.address.chat_id, "42");
        assert_eq!(text_envelope.correlation.message_id.as_deref(), Some("91"));
        assert!(matches!(
            text_envelope.payload,
            ChannelPayloadV1::Message { ref parts, .. }
                if matches!(parts.first(), Some(ContentPart::Text { text }) if text == "hello from telegram")
        ));

        let media: TelegramUpdate = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/inbound-group-media.json"
        ))
        .unwrap();
        adapter.process_update(media, &context).await.unwrap();
        let media_envelope = consumer.recv_ingress().await.unwrap().envelope().clone();
        assert_eq!(media_envelope.address.chat_id, "-1001234");
        assert_eq!(media_envelope.address.thread_id.as_deref(), Some("17"));
        assert_eq!(media_envelope.correlation.reply_to.as_deref(), Some("88"));
        assert!(matches!(
            media_envelope.payload,
            ChannelPayloadV1::Message { ref parts, .. }
                if parts.len() == 3
                    && matches!(&parts[1], ContentPart::Image { attachment } if attachment.media_type == "image/png")
                    && matches!(&parts[2], ContentPart::File { attachment } if attachment.file_name.as_deref() == Some("report.pdf"))
        ));

        let callback: TelegramUpdate = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/callback-query.json"
        ))
        .unwrap();
        adapter.process_update(callback, &context).await.unwrap();
        let callback_envelope = consumer.recv_ingress().await.unwrap().envelope().clone();
        assert_eq!(
            callback_envelope.extensions["telegram.callback_id"],
            json!("callback-9")
        );
        assert_eq!(
            callback_envelope.extensions["telegram.callback_data"],
            json!("approve:request-9")
        );

        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 5);
        assert!(requests[0].starts_with("POST /bottest-token/getFile"));
        assert!(requests[1].starts_with("GET /file/bottest-token/photos/photo.png"));
        assert!(requests[2].starts_with("POST /bottest-token/getFile"));
        assert!(requests[3].starts_with("GET /file/bottest-token/documents/report.pdf"));
        assert!(requests[4].starts_with("POST /bottest-token/answerCallbackQuery"));
    }

    #[tokio::test]
    async fn wire_telegram_egress_uses_html_reply_real_id_and_plain_fallback() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/sendMessage",
                400,
                "application/json",
                br#"{"ok":false,"error_code":400,"description":"can't parse entities"}"#,
            ),
            fixture_response(
                "/bottest-token/sendMessage",
                200,
                "application/json",
                br#"{"ok":true,"result":{"message_id":93}}"#,
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let address = ChannelAddress::new(CHANNEL, "42");
        let mut correlation = Correlation::new("telegram:42");
        correlation.reply_to = Some("91".to_string());
        let envelope = external_message_envelope(
            address,
            correlation,
            vec![ContentPart::Markdown {
                markdown: "**hello**".to_string(),
            }],
            None,
            None,
        );
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope,
                idempotency_key: Some("telegram-send-1".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            receipt.status,
            agent_diva_core::channel::DeliveryStatus::Accepted
        );
        assert_eq!(receipt.platform_message_id.as_deref(), Some("93"));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].contains("\"parse_mode\":\"HTML\""));
        assert!(requests[0].contains("\"reply_parameters\":{\"message_id\":91}"));
        assert!(!requests[1].contains("parse_mode"));
        assert!(requests[1].contains("\"text\":\"hello\""));
    }

    #[tokio::test]
    async fn wire_telegram_multipart_and_rate_limit_are_truthful() {
        let (base, server) = spawn_http_fixture(vec![fixture_response(
            "/bottest-token/sendPhoto",
            200,
            "application/json",
            br#"{"ok":true,"result":{"message_id":94}}"#,
        )])
        .await;
        let store = Arc::new(Store::default());
        let attachment = store
            .put(IngressAttachment {
                source_channel: CHANNEL.to_string(),
                platform_message_id: Some("message-94".to_string()),
                sender_id: Some("sender-42".to_string()),
                file_name: Some("photo.png".to_string()),
                declared_mime: Some("image/png".to_string()),
                bytes: b"fixture-photo".to_vec(),
            })
            .await
            .unwrap();
        let adapter = adapter_at_with_store(&base, store);
        let envelope = external_message_envelope(
            ChannelAddress::new(CHANNEL, "42"),
            Correlation::new("telegram:42"),
            vec![
                ContentPart::Text {
                    text: "caption".to_string(),
                },
                ContentPart::Image { attachment },
            ],
            None,
            None,
        );
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope,
                idempotency_key: None,
            })
            .await
            .unwrap();
        assert_eq!(receipt.platform_message_id.as_deref(), Some("94"));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].starts_with("POST /bottest-token/sendPhoto"));
        assert!(requests[0].contains("photo.png"));
        assert!(requests[0].contains("name=\"caption\""));
        assert!(requests[0].matches("caption").count() >= 2);

        let (base, server) = spawn_http_fixture(vec![fixture_response(
            "/bottest-token/sendMessage",
            429,
            "application/json",
            br#"{"ok":false,"error_code":429,"description":"Too Many Requests","parameters":{"retry_after":2}}"#,
        )])
        .await;
        let adapter = adapter_at(&base);
        let envelope = external_message_envelope(
            ChannelAddress::new(CHANNEL, "42"),
            Correlation::new("telegram:42"),
            vec![ContentPart::Text {
                text: "rate limited probe".to_string(),
            }],
            None,
            None,
        );
        let result = adapter
            .execute(ChannelCommand::Send {
                envelope,
                idempotency_key: None,
            })
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::RateLimited { retry_after })
                if retry_after == Duration::from_secs(2)
        ));
        let _ = server.await.unwrap();
    }

    #[tokio::test]
    async fn group_policy_accepts_dm_reply_mention_and_command_only() {
        let adapter = adapter_at("http://127.0.0.1:1").with_bot_username("testbot");
        let updates: Vec<TelegramUpdate> = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/group-policy.json"
        ))
        .unwrap();
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };

        adapter.process_batch(updates, &context).await.unwrap();
        let mut texts = Vec::new();
        for _ in 0..4 {
            let item = tokio::time::timeout(Duration::from_secs(1), consumer.recv_ingress())
                .await
                .unwrap()
                .unwrap();
            if let ChannelPayloadV1::Message { parts, .. } = &item.envelope().payload {
                if let Some(ContentPart::Text { text }) = parts.first() {
                    texts.push(text.clone());
                }
            }
        }
        assert_eq!(
            texts,
            vec![
                "direct message",
                "reply to the bot",
                "please summarize",
                "/status now"
            ]
        );
        assert_eq!(adapter.offset.load(Ordering::Acquire), 71006);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), consumer.recv_ingress())
                .await
                .is_err(),
            "ordinary group chatter must be ignored"
        );
    }

    #[tokio::test]
    async fn keyboard_fixture_remains_unsupported_without_reply_markup_side_channel() {
        let card: ContentPart = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/keyboard-unsupported.json"
        ))
        .unwrap();
        let envelope = external_message_envelope(
            ChannelAddress::new(CHANNEL, "42"),
            Correlation::new("telegram:42"),
            vec![card],
            None,
            None,
        );
        let result = adapter()
            .execute(ChannelCommand::Send {
                envelope,
                idempotency_key: None,
            })
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::EgressCard
            })
        ));
    }

    #[tokio::test]
    async fn media_matrix_preserves_typed_parts_mime_and_attachment_store_identity() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                &get_file_fixture("voice/voice.ogg", 3),
            ),
            fixture_response(
                "/file/bottest-token/voice/voice.ogg",
                200,
                "audio/ogg; charset=binary",
                b"voc",
            ),
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                &get_file_fixture("audio/track.mp3", 3),
            ),
            fixture_response(
                "/file/bottest-token/audio/track.mp3",
                200,
                "audio/mpeg",
                b"aud",
            ),
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                &get_file_fixture("video/clip.mp4", 3),
            ),
            fixture_response(
                "/file/bottest-token/video/clip.mp4",
                200,
                "video/mp4",
                b"vid",
            ),
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                &get_file_fixture("documents/report.pdf", 3),
            ),
            fixture_response(
                "/file/bottest-token/documents/report.pdf",
                200,
                "application/pdf",
                b"doc",
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let updates: Vec<TelegramUpdate> = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/media-matrix.json"
        ))
        .unwrap();
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };

        adapter.process_batch(updates, &context).await.unwrap();
        let expected = [
            ("audio/ogg", "voice.ogg", "audio"),
            ("audio/mpeg", "track.mp3", "audio"),
            ("video/mp4", "clip.mp4", "video"),
            ("application/pdf", "report.pdf", "file"),
        ];
        for (media_type, file_name, kind) in expected {
            let item = consumer.recv_ingress().await.unwrap();
            let ChannelPayloadV1::Message { parts, .. } = &item.envelope().payload else {
                panic!("media update must produce a message payload");
            };
            assert_eq!(parts.len(), 1);
            let attachment = match (&parts[0], kind) {
                (ContentPart::Audio { attachment, .. }, "audio") => attachment,
                (ContentPart::Video { attachment }, "video") => attachment,
                (ContentPart::File { attachment }, "file") => attachment,
                other => panic!("unexpected typed media part: {other:?}"),
            };
            assert_eq!(attachment.media_type, media_type);
            assert_eq!(attachment.file_name.as_deref(), Some(file_name));
            assert_eq!(attachment.size_bytes, 3);
            assert!(attachment.uri.starts_with("sha256:"));
        }
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 8);
        assert!(requests[1].starts_with("GET /file/bottest-token/voice/voice.ogg"));
        assert!(requests[3].starts_with("GET /file/bottest-token/audio/track.mp3"));
        assert!(requests[5].starts_with("GET /file/bottest-token/video/clip.mp4"));
        assert!(requests[7].starts_with("GET /file/bottest-token/documents/report.pdf"));
    }

    #[tokio::test]
    async fn media_failures_are_typed_for_size_status_mime_and_missing_path() {
        let adapter = adapter_at("http://127.0.0.1:1");
        let (fabric, _consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let oversized = media_update(
            72101,
            211,
            "too-large",
            "voice",
            Some("voice.ogg"),
            Some("audio/ogg"),
            Some(MAX_ATTACHMENT_BYTES + 1),
        );
        let result = adapter.process_update(oversized, &context).await;
        assert!(matches!(
            result,
            Err(AdapterError::Execution { ref code, .. }) if code == "telegram_media_too_large"
        ));

        let (base, server) = spawn_http_fixture(vec![fixture_response(
            "/bottest-token/getFile",
            200,
            "application/json",
            &fixture_api("get_file_missing_path"),
        )])
        .await;
        let adapter = adapter_at(&base);
        let (fabric, _) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let result = adapter
            .process_update(
                media_update(
                    72102,
                    212,
                    "missing-path",
                    "document",
                    Some("missing.pdf"),
                    Some("application/pdf"),
                    Some(3),
                ),
                &context,
            )
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::Execution { ref code, .. }) if code == "telegram_media_missing_path"
        ));
        assert_eq!(server.await.unwrap().len(), 1);

        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                &get_file_fixture("documents/status.pdf", 3),
            ),
            fixture_response(
                "/file/bottest-token/documents/status.pdf",
                503,
                "application/pdf",
                b"unavailable",
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let (fabric, _consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let result = adapter
            .process_update(
                media_update(
                    72103,
                    213,
                    "status",
                    "document",
                    Some("status.pdf"),
                    Some("application/pdf"),
                    Some(3),
                ),
                &context,
            )
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::Execution { ref code, retryable, .. })
                if code == "telegram_media_http" && retryable
        ));
        assert_eq!(server.await.unwrap().len(), 2);

        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                &get_file_fixture("audio/wrong.ogg", 3),
            ),
            fixture_response(
                "/file/bottest-token/audio/wrong.ogg",
                200,
                "image/png",
                b"not-audio",
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let (fabric, _) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let result = adapter
            .process_update(
                media_update(
                    72104,
                    214,
                    "wrong-mime",
                    "voice",
                    Some("wrong.ogg"),
                    Some("audio/ogg"),
                    Some(3),
                ),
                &context,
            )
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::Execution { ref code, .. }) if code == "telegram_media_mime"
        ));
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn media_timeout_and_cancellation_stop_in_flight_downloads() {
        let (base, server) = spawn_http_fixture(vec![fixture_hold("/bottest-token/getFile")]).await;
        let adapter = adapter_at(&base).with_media_timeout(Duration::from_millis(20));
        let (fabric, _) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let result = adapter
            .process_update(
                media_update(
                    72201,
                    221,
                    "timeout",
                    "document",
                    Some("timeout.pdf"),
                    Some("application/pdf"),
                    Some(3),
                ),
                &context,
            )
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::Execution { ref code, retryable, .. })
                if code == "telegram_media_timeout" && retryable
        ));
        server.abort();
        let _ = server.await;

        let (base, server) = spawn_http_fixture(vec![fixture_hold("/bottest-token/getFile")]).await;
        let adapter = adapter_at(&base).with_media_timeout(Duration::from_secs(5));
        let cancel = CancellationToken::new();
        let (fabric, _) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: cancel.clone(),
        };
        let task = tokio::spawn(async move {
            adapter
                .process_update(
                    media_update(
                        72202,
                        222,
                        "cancel",
                        "document",
                        Some("cancel.pdf"),
                        Some("application/pdf"),
                        Some(3),
                    ),
                    &context,
                )
                .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result, Err(AdapterError::Stopped));
        server.abort();
        let _ = server.await;
    }

    #[tokio::test]
    async fn attachment_store_corruption_is_returned_as_a_typed_error() {
        let store = Arc::new(Store {
            corrupt_on_get: true,
            ..Store::default()
        });
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                &get_file_fixture("documents/corrupt.pdf", 3),
            ),
            fixture_response(
                "/file/bottest-token/documents/corrupt.pdf",
                200,
                "application/pdf",
                b"abc",
            ),
        ])
        .await;
        let adapter = adapter_at_with_store(&base, store);
        let (fabric, _) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let result = adapter
            .process_update(
                media_update(
                    72301,
                    231,
                    "corrupt",
                    "document",
                    Some("corrupt.pdf"),
                    Some("application/pdf"),
                    Some(3),
                ),
                &context,
            )
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::Execution { ref code, .. }) if code == "attachment_store_corrupt"
        ));
        assert_eq!(server.await.unwrap().len(), 2);
    }

    #[tokio::test]
    async fn callback_ack_precedes_admission_and_duplicate_update_is_deduped() {
        let (base, server) = spawn_http_fixture(vec![fixture_response(
            "/bottest-token/answerCallbackQuery",
            200,
            "application/json",
            &fixture_api("callback_ack"),
        )])
        .await;
        let adapter = adapter_at(&base);
        let callback: TelegramUpdate = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/callback-query.json"
        ))
        .unwrap();
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };

        adapter.process_update(callback, &context).await.unwrap();
        let envelope = consumer.recv_ingress().await.unwrap().envelope().clone();
        assert_eq!(
            envelope.extensions["telegram.callback_id"],
            json!("callback-9")
        );
        let callback_again: TelegramUpdate = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/telegram/callback-query.json"
        ))
        .unwrap();
        adapter
            .process_update(callback_again, &context)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), consumer.recv_ingress())
                .await
                .is_err()
        );
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests[0].contains("callback_query_id"));
        assert!(requests[0].contains("callback-9"));
    }

    #[tokio::test]
    async fn egress_media_uses_telegram_typed_methods_and_readback_bytes() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/sendVoice",
                200,
                "application/json",
                &fixture_api("send_voice"),
            ),
            fixture_response(
                "/bottest-token/sendAudio",
                200,
                "application/json",
                &fixture_api("send_audio"),
            ),
            fixture_response(
                "/bottest-token/sendVideo",
                200,
                "application/json",
                &fixture_api("send_video"),
            ),
            fixture_response(
                "/bottest-token/sendDocument",
                200,
                "application/json",
                &fixture_api("send_document"),
            ),
        ])
        .await;
        let store = Arc::new(Store::default());
        let voice = store
            .put(IngressAttachment {
                source_channel: CHANNEL.to_string(),
                platform_message_id: Some("301".to_string()),
                sender_id: Some("42".to_string()),
                file_name: Some("voice.ogg".to_string()),
                declared_mime: Some("audio/ogg".to_string()),
                bytes: b"voice-bytes".to_vec(),
            })
            .await
            .unwrap();
        let audio = store
            .put(IngressAttachment {
                source_channel: CHANNEL.to_string(),
                platform_message_id: Some("302".to_string()),
                sender_id: Some("42".to_string()),
                file_name: Some("track.mp3".to_string()),
                declared_mime: Some("audio/mpeg".to_string()),
                bytes: b"audio-bytes".to_vec(),
            })
            .await
            .unwrap();
        let video = store
            .put(IngressAttachment {
                source_channel: CHANNEL.to_string(),
                platform_message_id: Some("303".to_string()),
                sender_id: Some("42".to_string()),
                file_name: Some("clip.mp4".to_string()),
                declared_mime: Some("video/mp4".to_string()),
                bytes: b"video-bytes".to_vec(),
            })
            .await
            .unwrap();
        let document = store
            .put(IngressAttachment {
                source_channel: CHANNEL.to_string(),
                platform_message_id: Some("304".to_string()),
                sender_id: Some("42".to_string()),
                file_name: Some("report.pdf".to_string()),
                declared_mime: Some("application/pdf".to_string()),
                bytes: b"document-bytes".to_vec(),
            })
            .await
            .unwrap();
        let adapter = adapter_at_with_store(&base, store);
        for part in [
            ContentPart::Audio {
                attachment: voice,
                transcript: None,
            },
            ContentPart::Audio {
                attachment: audio,
                transcript: None,
            },
            ContentPart::Video { attachment: video },
            ContentPart::File {
                attachment: document,
            },
        ] {
            let receipt = adapter
                .execute(ChannelCommand::Send {
                    envelope: external_message_envelope(
                        ChannelAddress::new(CHANNEL, "42"),
                        Correlation::new("telegram:42"),
                        vec![part],
                        None,
                        None,
                    ),
                    idempotency_key: None,
                })
                .await
                .unwrap();
            assert_eq!(receipt.status, DeliveryStatus::Accepted);
        }
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[0].contains("name=\"voice\""));
        assert!(requests[0].contains("voice.ogg"));
        assert!(requests[1].contains("name=\"audio\""));
        assert!(requests[1].contains("track.mp3"));
        assert!(requests[2].contains("name=\"video\""));
        assert!(requests[2].contains("clip.mp4"));
        assert!(requests[3].contains("name=\"document\""));
        assert!(requests[3].contains("report.pdf"));
    }

    #[tokio::test]
    async fn partial_chunk_failure_returns_receipt_and_prefers_retry_after_header() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/sendMessage",
                200,
                "application/json",
                br#"{"ok":true,"result":{"message_id":401}}"#,
            ),
            fixture_response_with_headers(
                "/bottest-token/sendMessage",
                429,
                "application/json",
                &fixture_api("rate_limited"),
                &[("Retry-After", "7")],
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope: external_message_envelope(
                    ChannelAddress::new(CHANNEL, "42"),
                    Correlation::new("telegram:42"),
                    vec![ContentPart::Text {
                        text: "x".repeat(MAX_MESSAGE_CHARS + 1),
                    }],
                    None,
                    None,
                ),
                idempotency_key: Some("partial-401".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(receipt.status, DeliveryStatus::Failed);
        assert_eq!(receipt.error_code.as_deref(), Some("partial_delivery"));
        assert_eq!(receipt.platform_message_id.as_deref(), Some("401"));
        assert_eq!(receipt.retry_after_ms, Some(7_000));
        assert!(receipt
            .diagnosis
            .as_deref()
            .is_some_and(|diagnosis| diagnosis.contains("delivered 1/2")));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
    }

    #[tokio::test]
    async fn typing_refreshes_until_stopped() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/sendChatAction",
                200,
                "application/json",
                &fixture_api("chat_action"),
            ),
            fixture_response(
                "/bottest-token/sendChatAction",
                200,
                "application/json",
                &fixture_api("chat_action"),
            ),
        ])
        .await;
        let adapter = adapter_at(&base).with_typing_refresh_interval(Duration::from_millis(30));
        let address = ChannelAddress::new(CHANNEL, "42");
        adapter
            .execute(ChannelCommand::Typing {
                address: address.clone(),
                correlation: Correlation::new("telegram:42"),
                state: TypingState::Started,
                idempotency_key: None,
            })
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(70)).await;
        adapter
            .execute(ChannelCommand::Typing {
                address: address.clone(),
                correlation: Correlation::new("telegram:42"),
                state: TypingState::Stopped,
                idempotency_key: None,
            })
            .await
            .unwrap();
        let requests = tokio::time::timeout(Duration::from_secs(1), server)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[0].contains("\"action\":\"typing\""));
        assert!(requests[1].contains("\"action\":\"typing\""));
    }

    #[tokio::test]
    async fn edit_delete_and_finalize_use_numeric_message_ids() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/editMessageText",
                200,
                "application/json",
                &fixture_api("edit_message"),
            ),
            fixture_response(
                "/bottest-token/deleteMessage",
                200,
                "application/json",
                &fixture_api("delete_message"),
            ),
            fixture_response(
                "/bottest-token/editMessageText",
                200,
                "application/json",
                &fixture_api("edit_message"),
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let address = ChannelAddress::new(CHANNEL, "42");
        let edit_receipt = adapter
            .execute(ChannelCommand::Edit {
                address: address.clone(),
                correlation: Correlation::new("telegram:42"),
                target_message_id: "93".to_string(),
                parts: vec![ContentPart::Text {
                    text: "edited".to_string(),
                }],
                idempotency_key: None,
            })
            .await
            .unwrap();
        assert_eq!(edit_receipt.platform_message_id.as_deref(), Some("93"));
        let delete_receipt = adapter
            .execute(ChannelCommand::Delete {
                address: address.clone(),
                correlation: Correlation::new("telegram:42"),
                target_message_id: "93".to_string(),
                idempotency_key: None,
            })
            .await
            .unwrap();
        assert_eq!(delete_receipt.platform_message_id.as_deref(), Some("93"));
        let mut finalize_correlation = Correlation::new("telegram:42");
        finalize_correlation.message_id = Some("93".to_string());
        let finalize_receipt = adapter
            .execute(ChannelCommand::FinalizeStream {
                address,
                correlation: finalize_correlation,
                parts: vec![ContentPart::Text {
                    text: "final".to_string(),
                }],
                idempotency_key: None,
            })
            .await
            .unwrap();
        assert_eq!(finalize_receipt.platform_message_id.as_deref(), Some("93"));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].contains("\"message_id\":93"));
        assert!(requests[1].contains("\"message_id\":93"));
        assert!(requests[2].contains("\"message_id\":93"));
    }

    #[tokio::test]
    async fn polling_batch_advances_offset_only_after_a_successful_batch() {
        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/getFile",
                200,
                "application/json",
                &fixture_api("get_file_missing_path"),
            ),
            fixture_response(
                "/bottest-token/getUpdates",
                200,
                "application/json",
                &fixture_api("empty_updates"),
            ),
        ])
        .await;
        let adapter = adapter_at(&base);
        let (fabric, _consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let first = text_update(73001, 301, 42, "first in batch");
        let failed = media_update(
            73002,
            302,
            "missing-path",
            "document",
            Some("missing.pdf"),
            Some("application/pdf"),
            Some(3),
        );
        let result = adapter.process_batch(vec![first, failed], &context).await;
        assert!(matches!(
            result,
            Err(AdapterError::Execution { ref code, .. }) if code == "telegram_media_missing_path"
        ));
        assert_eq!(adapter.offset.load(Ordering::Acquire), 0);

        assert!(adapter.get_updates().await.unwrap().is_empty());
        let first_retry = text_update(73001, 301, 42, "first in batch");
        adapter
            .process_batch(vec![first_retry], &context)
            .await
            .unwrap();
        assert_eq!(adapter.offset.load(Ordering::Acquire), 73002);
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests[1].contains("\"offset\":0"));
        assert!(requests[1].contains("\"timeout\":25"));
        assert!(requests[1].contains("allowed_updates"));
    }

    #[tokio::test(start_paused = true)]
    async fn dedup_has_ttl_capacity_forget_and_bounded_reconnect_backoff() {
        let adapter = adapter();
        adapter.remember_updates(&[1]).await;
        assert!(adapter.is_update_seen(1).await);
        adapter.forget_update(1).await;
        assert!(!adapter.is_update_seen(1).await);
        adapter.remember_updates(&[1]).await;
        tokio::time::advance(DEDUP_TTL + Duration::from_secs(1)).await;
        assert!(!adapter.is_update_seen(1).await);

        let update_ids = (0..(DEDUP_CAPACITY as i64 + 1)).collect::<Vec<_>>();
        adapter.remember_updates(&update_ids).await;
        assert!(adapter.seen.read().await.len() <= DEDUP_CAPACITY);
        assert_eq!(reconnect_delay(1), Duration::from_secs(5));
        assert_eq!(reconnect_delay(2), Duration::from_secs(10));
        assert_eq!(reconnect_delay(3), Duration::from_secs(20));
        assert_eq!(reconnect_delay(4), Duration::from_secs(40));
        assert_eq!(reconnect_delay(5), Duration::from_secs(60));
        assert_eq!(reconnect_delay(9), Duration::from_secs(60));
    }

    #[tokio::test]
    async fn failed_health_probe_reports_down_and_start_cancel_is_lifecycle_safe() {
        let (base, server) = spawn_http_fixture(vec![fixture_response(
            "/bottest-token/getMe",
            401,
            "application/json",
            &polling_fixture("unauthorized"),
        )])
        .await;
        let adapter = adapter_at(&base);
        let result = adapter
            .execute(ChannelCommand::ProbeHealth {
                channel: ChannelId::new(CHANNEL).unwrap(),
            })
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::Execution { ref code, .. }) if code == "telegram_api"
        ));
        assert_eq!(adapter.health().status, ChannelHealthStatus::Down);
        assert_eq!(server.await.unwrap().len(), 1);

        let (base, server) = spawn_http_fixture(vec![
            fixture_response(
                "/bottest-token/getMe",
                200,
                "application/json",
                &polling_fixture("get_me"),
            ),
            fixture_hold("/bottest-token/getUpdates"),
        ])
        .await;
        let adapter = Arc::new(adapter_at(&base));
        let cancel = CancellationToken::new();
        let context = AdapterContext {
            fabric: FabricKernel::new().into_parts().0,
            cancel: cancel.clone(),
        };
        let listener = {
            let adapter = Arc::clone(&adapter);
            tokio::spawn(async move { adapter.start(context).await })
        };
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel.cancel();
        let result = tokio::time::timeout(Duration::from_secs(1), listener)
            .await
            .unwrap()
            .unwrap();
        assert!(result.is_ok());
        assert_eq!(adapter.health().status, ChannelHealthStatus::Down);
        server.abort();
        let _ = server.await;
    }

    struct HttpFixtureResponse {
        expected_path: String,
        status: u16,
        content_type: String,
        body: Vec<u8>,
        headers: Vec<(String, String)>,
        hold_open: bool,
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
            headers: Vec::new(),
            hold_open: false,
        }
    }

    fn fixture_response_with_headers(
        expected_path: &str,
        status: u16,
        content_type: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> HttpFixtureResponse {
        let mut response = fixture_response(expected_path, status, content_type, body);
        response.headers = headers
            .iter()
            .map(|(name, value)| ((*name).to_string(), (*value).to_string()))
            .collect();
        response
    }

    fn fixture_hold(expected_path: &str) -> HttpFixtureResponse {
        let mut response = fixture_response(expected_path, 200, "application/json", b"");
        response.hold_open = true;
        response
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
                if expected.hold_open {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                    return requests;
                }
                let reason = match expected.status {
                    200 => "OK",
                    400 => "Bad Request",
                    429 => "Too Many Requests",
                    _ => "Fixture",
                };
                let extra_headers = expected
                    .headers
                    .iter()
                    .map(|(name, value)| format!("{name}: {value}\r\n"))
                    .collect::<String>();
                let header = format!(
                    "HTTP/1.1 {} {reason}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{extra_headers}Connection: close\r\n\r\n",
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
}
