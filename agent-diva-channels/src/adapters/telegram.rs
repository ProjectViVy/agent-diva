//! Native Telegram Bot API adapter for the Super Channel Fabric.
//!
//! This module is intentionally independent from the legacy `TelegramHandler`.
//! It keeps the DIVA command/allowlist semantics while using explicit Bot API
//! requests, bounded polling, typed attachment storage, and truthful receipts.

use crate::adapter::{
    accepted_receipt, execution_error, external_message_envelope, is_sender_allowed,
    AdapterContext, AdapterError, AdapterServices, ChannelAdapter, IngressAttachment,
};
use agent_diva_core::channel::{
    ChannelAddress, ChannelCapabilities, ChannelCapability, ChannelCommand, ChannelDirection,
    ChannelEnvelopeV1, ChannelHealth, ChannelHealthStatus, ChannelId, ChannelOrigin,
    ChannelPayloadV1, ContentPart, Correlation, DeliveryReceipt, TypingState,
};
use agent_diva_core::config::schema::TelegramConfig;
use async_trait::async_trait;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashSet};
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;

const CHANNEL: &str = "telegram";
const API_BASE: &str = "https://api.telegram.org";
const MAX_MESSAGE_CHARS: usize = 4_096;
const MAX_CAPTION_CHARS: usize = 1_024;
const MAX_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;
const POLL_TIMEOUT_SECONDS: u64 = 25;
const INGRESS_ADMISSION_DEADLINE: Duration = Duration::from_secs(2);

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
}

#[derive(Debug, Deserialize)]
struct TelegramUser {
    id: i64,
    #[serde(default)]
    username: Option<String>,
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
    seen: Arc<RwLock<HashSet<i64>>>,
    offset: AtomicI64,
    health: Arc<Mutex<ChannelHealth>>,
    running: Arc<AtomicBool>,
    cancel: CancellationToken,
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
            seen: Arc::new(RwLock::new(HashSet::new())),
            offset: AtomicI64::new(0),
            health: Arc::new(Mutex::new(ChannelHealth::new(ChannelHealthStatus::Unknown))),
            running: Arc::new(AtomicBool::new(false)),
            cancel: CancellationToken::new(),
        }
    }

    #[cfg(test)]
    fn with_test_endpoint(config: TelegramConfig, services: AdapterServices, base: &str) -> Self {
        Self::from_endpoint(config, services, TelegramEndpoint::test(base))
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
        let parsed = response
            .json::<ApiResponse<T>>()
            .await
            .map_err(|error| execution_error("telegram_response", error.to_string(), None, true))?;
        if !parsed.ok {
            let diagnosis = parsed
                .description
                .unwrap_or_else(|| format!("Telegram API returned {status}"));
            if parsed.error_code == Some(429) {
                return Err(AdapterError::RateLimited {
                    retry_after: Duration::from_secs(
                        parsed.parameters.and_then(|p| p.retry_after).unwrap_or(1),
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
        let parsed = response
            .json::<ApiResponse<SentMessage>>()
            .await
            .map_err(|error| execution_error("telegram_response", error.to_string(), None, true))?;
        if !parsed.ok {
            let diagnosis = parsed
                .description
                .unwrap_or_else(|| format!("Telegram API returned {status}"));
            if parsed.error_code == Some(429) {
                return Err(AdapterError::RateLimited {
                    retry_after: Duration::from_secs(
                        parsed.parameters.and_then(|p| p.retry_after).unwrap_or(1),
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
        self.call_json("getUpdates", json!({"offset": offset, "timeout": POLL_TIMEOUT_SECONDS, "allowed_updates": ["message", "callback_query"]})).await
    }

    async fn process_update(
        &self,
        update: TelegramUpdate,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        if self.seen.read().await.contains(&update.update_id) {
            return Ok(());
        }
        if let Some(callback) = update.callback_query {
            self.process_callback(callback, context).await?;
        } else if let Some(message) = update.message {
            self.process_message(message, context).await?;
        }
        self.seen.write().await.insert(update.update_id);
        self.offset
            .store(update.update_id.saturating_add(1), Ordering::Release);
        Ok(())
    }

    async fn process_callback(
        &self,
        callback: TelegramCallbackQuery,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
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
        let _: Value = self
            .call_json(
                "answerCallbackQuery",
                json!({"callback_query_id": callback.id}),
            )
            .await?;
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
        let mut parts = Vec::new();
        let text = message
            .text
            .clone()
            .or(message.caption.clone())
            .unwrap_or_default();
        if !text.trim().is_empty() {
            parts.push(ContentPart::Text {
                text: truncate(&text, MAX_CAPTION_CHARS),
            });
        }
        if let Some(photo) = message.photo.as_ref().and_then(|items| items.last()) {
            if let Some(part) = self
                .fetch_attachment(&photo.file_id, photo.file_size, None, None, "image")
                .await?
            {
                parts.push(part);
            }
        }
        if let Some(voice) = &message.voice {
            if let Some(part) = self
                .fetch_attachment(
                    &voice.file_id,
                    voice.file_size,
                    voice.mime_type.as_deref(),
                    None,
                    "audio",
                )
                .await?
            {
                parts.push(part);
            }
        }
        if let Some(audio) = &message.audio {
            if let Some(part) = self
                .fetch_attachment(
                    &audio.file_id,
                    audio.file_size,
                    audio.mime_type.as_deref(),
                    audio.file_name.as_deref(),
                    "audio",
                )
                .await?
            {
                parts.push(part);
            }
        }
        if let Some(video) = &message.video {
            if let Some(part) = self
                .fetch_attachment(
                    &video.file_id,
                    video.file_size,
                    video.mime_type.as_deref(),
                    video.file_name.as_deref(),
                    "video",
                )
                .await?
            {
                parts.push(part);
            }
        }
        if let Some(document) = &message.document {
            if let Some(part) = self
                .fetch_attachment(
                    &document.file_id,
                    document.file_size,
                    document.mime_type.as_deref(),
                    document.file_name.as_deref(),
                    "file",
                )
                .await?
            {
                parts.push(part);
            }
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
        file_id: &str,
        size: Option<u64>,
        mime: Option<&str>,
        file_name: Option<&str>,
        kind: &str,
    ) -> Result<Option<ContentPart>, AdapterError> {
        if size.is_some_and(|size| size > MAX_ATTACHMENT_BYTES) {
            return Ok(None);
        }
        let file: TelegramFile = self
            .call_json("getFile", json!({"file_id": file_id}))
            .await?;
        let Some(path) = file.file_path else {
            return Ok(None);
        };
        let url = format!(
            "{}/file/bot{}/{}",
            self.endpoint.api_base, self.config.token, path
        );
        let response =
            self.http.get(url).send().await.map_err(|error| {
                execution_error("telegram_media", error.to_string(), None, true)
            })?;
        if !response.status().is_success() {
            let status = response.status();
            return Err(execution_error(
                "telegram_media_http",
                format!("Telegram media download returned {status}"),
                None,
                status.is_server_error(),
            ));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|error| execution_error("telegram_media", error.to_string(), None, true))?;
        if bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
            return Ok(None);
        }
        let attachment_name = file_name
            .filter(|name| !name.trim().is_empty())
            .map(str::to_string)
            .or_else(|| path.rsplit('/').next().map(str::to_string));
        let reference = self
            .services
            .attachments
            .put(IngressAttachment {
                source_channel: CHANNEL.to_string(),
                platform_message_id: Some(file_id.to_string()),
                sender_id: None,
                file_name: attachment_name,
                declared_mime: mime.map(str::to_string),
                bytes: bytes.to_vec(),
            })
            .await
            .map_err(|error| execution_error("attachment_store", error.to_string(), None, false))?;
        Ok(Some(match kind {
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
        }))
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
        let mut last_id = None;
        if !has_media {
            for (index, chunk) in split_chunks(&text, MAX_MESSAGE_CHARS)
                .into_iter()
                .enumerate()
            {
                let mut body =
                    json!({"chat_id": address.chat_id, "text": chunk, "parse_mode": "HTML"});
                if index == 0 {
                    if let Some(reply) = &correlation.reply_to {
                        body["reply_parameters"] = json!({"message_id": reply});
                    }
                }
                let sent: SentMessage = self.send_text_message(body).await?;
                last_id = Some(sent.message_id.to_string());
            }
        }
        let mut media_index = 0;
        for part in parts {
            let (method, field, reference) = match part {
                ContentPart::Image { attachment } => ("sendPhoto", "photo", attachment),
                ContentPart::Audio { attachment, .. } => ("sendAudio", "audio", attachment),
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
            let mut form = Form::new().text("chat_id", address.chat_id.clone()).part(
                field,
                Part::bytes(stored.bytes).file_name(
                    reference
                        .file_name
                        .unwrap_or_else(|| "attachment.bin".to_string()),
                ),
            );
            if media_index == 0 {
                if let Some(reply) = &correlation.reply_to {
                    form = form.text("reply_parameters", json!({"message_id": reply}).to_string());
                }
                if !caption.is_empty() {
                    form = form.text("caption", caption.clone());
                }
            }
            let sent = self.call_multipart(method, form).await?;
            last_id = Some(sent.message_id.to_string());
            media_index += 1;
        }
        if has_media && !caption_remainder.is_empty() {
            for chunk in split_chunks(&caption_remainder, MAX_MESSAGE_CHARS) {
                let sent: SentMessage = self
                    .send_text_message(json!({
                        "chat_id": address.chat_id,
                        "text": chunk,
                    }))
                    .await?;
                last_id = Some(sent.message_id.to_string());
            }
        }
        if last_id.is_none() {
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
        let text = content_to_text(&parts);
        let _: SentMessage = self.call_json("editMessageText", json!({"chat_id": address.chat_id, "message_id": target, "text": truncate(&text, MAX_MESSAGE_CHARS), "parse_mode": "HTML"})).await?;
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
        let _: Value = self
            .call_json(
                "deleteMessage",
                json!({"chat_id": address.chat_id, "message_id": target}),
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
                return Ok(accepted_receipt(
                    CHANNEL,
                    address.chat_id,
                    None,
                    address.thread_id,
                ))
            }
        };
        let _: Value = self
            .call_json(
                "sendChatAction",
                json!({"chat_id": address.chat_id, "action": action}),
            )
            .await?;
        Ok(accepted_receipt(
            CHANNEL,
            address.chat_id,
            None,
            address.thread_id,
        ))
    }

    async fn probe_health(&self) -> Result<DeliveryReceipt, AdapterError> {
        let _: Value = self.call_json("getMe", json!({})).await?;
        self.set_health(ChannelHealthStatus::Healthy, None);
        Ok(accepted_receipt(CHANNEL, "health", None, None))
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
                    self.set_health(ChannelHealthStatus::Healthy, None);
                    for update in updates {
                        if let Err(error) = self.process_update(update, &context).await {
                            self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                        }
                    }
                }
                Err(error) => {
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                    if !sleep_or_cancel(Duration::from_secs(2), &context, &self.cancel).await {
                        break;
                    }
                }
            }
        }
        self.running.store(false, Ordering::Release);
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
        self.running.store(false, Ordering::Release);
        self.set_health(
            ChannelHealthStatus::Down,
            Some("Telegram adapter stopped".to_string()),
        );
        Ok(())
    }
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
    use crate::adapter::{ChannelAttachmentStore, StoredAttachment};
    use agent_diva_core::channel::{AttachmentRef, FabricKernel};
    use agent_diva_core::config::Config;
    use async_trait::async_trait;
    use sha2::Digest;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

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
    fn adapter() -> TelegramAdapter {
        adapter_at("http://127.0.0.1:1")
    }

    fn adapter_at(base: &str) -> TelegramAdapter {
        let mut config = Config::default().channels.telegram;
        config.enabled = true;
        config.token = "test-token".into();
        TelegramAdapter::with_test_endpoint(config, AdapterServices::new(Arc::new(Store)), base)
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
                    && matches!(&parts[1], ContentPart::Image { attachment } if attachment.media_type == "application/octet-stream")
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
        assert!(requests[0].contains("\"reply_parameters\":{\"message_id\":\"91\"}"));
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
        let adapter = adapter_at(&base);
        let envelope = external_message_envelope(
            ChannelAddress::new(CHANNEL, "42"),
            Correlation::new("telegram:42"),
            vec![
                ContentPart::Text {
                    text: "caption".to_string(),
                },
                ContentPart::Image {
                    attachment: AttachmentRef {
                        uri: "sha256:fixture".to_string(),
                        media_type: "image/png".to_string(),
                        size_bytes: 14,
                        sha256: "fixture".to_string(),
                        file_name: Some("photo.png".to_string()),
                    },
                },
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
                    400 => "Bad Request",
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
}
