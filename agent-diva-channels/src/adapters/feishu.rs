//! Native C5 Feishu/Lark adapter.
//!
//! The adapter keeps DIVA's protobuf WebSocket, ACK, heartbeat, card/table and
//! best-effort seen-reaction behavior while porting the endpoint-level media,
//! reply, edit, delete, region and webhook security behavior identified in the
//! Octos `v2.0.3-rc.9` scan.  It intentionally does not depend on the legacy
//! [`crate::feishu::FeishuHandler`] or on an Octos runtime.

use crate::adapter::{
    accepted_receipt, delivered_receipt, execution_error, external_message_envelope,
    is_sender_allowed, AdapterContext, AdapterError, AdapterServices, ChannelAdapter,
    IngressAttachment,
};
use agent_diva_core::channel::{
    ChannelAddress, ChannelCapabilities, ChannelCapability, ChannelCommand, ChannelHealth,
    ChannelHealthStatus, ChannelId, ChannelPayloadV1, ContentPart, Correlation, DeliveryReceipt,
};
use agent_diva_core::config::schema::FeishuConfig;
use async_trait::async_trait;
use base64::Engine;
use futures::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use prost_derive::Message as ProstDeriveMessage;
use reqwest::multipart::{Form, Part};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, RwLock, Semaphore};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tokio_util::sync::CancellationToken;

const FEISHU_CHANNEL: &str = "feishu";
const WS_HEARTBEAT_TIMEOUT: Duration = Duration::from_secs(300);
const INGRESS_ADMISSION_DEADLINE: Duration = Duration::from_secs(2);
const MEDIA_REQUEST_TIMEOUT: Duration = Duration::from_secs(20);
const TOKEN_REFRESH_SKEW: Duration = Duration::from_secs(300);
const DEDUP_TTL: Duration = Duration::from_secs(600);
const DEDUP_CLEANUP_INTERVAL: Duration = Duration::from_secs(300);
const MAX_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;

/// Feishu region/domain pair.  The current shared config has no region field,
/// so production construction defaults to China; C6 may select this value
/// once the shared configuration schema is extended by its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeishuRegion {
    /// `open.feishu.cn` China mainland Open Platform.
    China,
    /// `open.larksuite.com` global Lark Open Platform.
    Global,
    /// Alias for the global Lark domain.
    Lark,
}

impl FeishuRegion {
    #[allow(dead_code)]
    fn api_base(self) -> &'static str {
        match self {
            Self::China => "https://open.feishu.cn/open-apis",
            Self::Global | Self::Lark => "https://open.larksuite.com/open-apis",
        }
    }

    #[allow(dead_code)]
    fn ws_base(self) -> &'static str {
        match self {
            Self::China => "https://open.feishu.cn",
            Self::Global | Self::Lark => "https://open.larksuite.com",
        }
    }
}

#[derive(Debug, Clone)]
struct FeishuEndpoint {
    api_base: String,
    ws_base: String,
}

impl FeishuEndpoint {
    #[allow(dead_code)]
    fn for_region(region: FeishuRegion) -> Self {
        Self {
            api_base: region.api_base().to_string(),
            ws_base: region.ws_base().to_string(),
        }
    }

    #[cfg(test)]
    fn test(base: &str) -> Self {
        Self {
            api_base: base.trim_end_matches('/').to_string(),
            ws_base: base.trim_end_matches('/').to_string(),
        }
    }
}

#[derive(Debug, Clone)]
struct TokenCache {
    value: String,
    expires_at: Instant,
}

#[derive(Debug, Default)]
struct DedupState {
    seen: HashMap<String, Instant>,
    pending: HashSet<String>,
    last_cleanup: Option<Instant>,
}

/// Native Feishu adapter.  It is public for the future C6 registry, while the
/// production constructor remains crate-visible to keep Manager assembly in
/// the C6 owner lane.
pub struct FeishuAdapter {
    config: FeishuConfig,
    services: AdapterServices,
    endpoint: FeishuEndpoint,
    http_client: reqwest::Client,
    token: Arc<RwLock<Option<TokenCache>>>,
    token_refresh: Arc<Mutex<()>>,
    dedup: Arc<Mutex<DedupState>>,
    admission: Arc<Semaphore>,
    running: Arc<AtomicBool>,
    listener_cancel: Arc<Mutex<Option<CancellationToken>>>,
    health: Arc<StdMutex<ChannelHealth>>,
}

impl FeishuAdapter {
    /// Construct a China-region adapter with the shared attachment authority.
    #[allow(dead_code)]
    pub(crate) fn new(config: FeishuConfig, services: AdapterServices) -> Self {
        Self::new_with_region(config, services, FeishuRegion::China)
    }

    /// Construct an adapter for an explicit Feishu/Lark region.
    #[allow(dead_code)]
    pub(crate) fn new_with_region(
        config: FeishuConfig,
        services: AdapterServices,
        region: FeishuRegion,
    ) -> Self {
        Self::from_endpoint(config, services, FeishuEndpoint::for_region(region))
    }

    #[allow(dead_code)]
    fn from_endpoint(
        config: FeishuConfig,
        services: AdapterServices,
        endpoint: FeishuEndpoint,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self {
            config,
            services,
            endpoint,
            http_client,
            token: Arc::new(RwLock::new(None)),
            token_refresh: Arc::new(Mutex::new(())),
            dedup: Arc::new(Mutex::new(DedupState::default())),
            admission: Arc::new(Semaphore::new(1)),
            running: Arc::new(AtomicBool::new(false)),
            listener_cancel: Arc::new(Mutex::new(None)),
            health: Arc::new(StdMutex::new(ChannelHealth::new(
                ChannelHealthStatus::Unknown,
            ))),
        }
    }

    #[cfg(test)]
    fn with_test_endpoint(
        config: FeishuConfig,
        services: AdapterServices,
        base: &str,
        _region: FeishuRegion,
    ) -> Self {
        Self::from_endpoint(config, services, FeishuEndpoint::test(base))
    }

    fn static_capabilities() -> ChannelCapabilities {
        let mut capabilities = ChannelCapabilities::new([
            ChannelCapability::IngressText,
            ChannelCapability::IngressMarkdown,
            ChannelCapability::IngressThread,
            ChannelCapability::IngressGroup,
            ChannelCapability::IngressDirect,
            ChannelCapability::IngressTypedAttachments,
            ChannelCapability::IngressDedupId,
            ChannelCapability::EgressText,
            ChannelCapability::EgressMarkdown,
            ChannelCapability::EgressReply,
            ChannelCapability::EgressImage,
            ChannelCapability::EgressFile,
            ChannelCapability::EgressCard,
            ChannelCapability::InteractionEdit,
            ChannelCapability::InteractionDelete,
            ChannelCapability::InteractionStreamFinalize,
            ChannelCapability::ReliabilityHealth,
            ChannelCapability::ReliabilityHeartbeat,
            ChannelCapability::ReliabilityTokenRefresh,
            ChannelCapability::ReliabilityPacing,
            ChannelCapability::ReliabilitySupervisedRestart,
        ]);
        capabilities.limits.max_text_chars = Some(30_000);
        capabilities.limits.max_attachment_bytes = Some(MAX_ATTACHMENT_BYTES);
        capabilities.limits.supported_mime_types = BTreeSet::from([
            "image/*".to_string(),
            "audio/*".to_string(),
            "video/*".to_string(),
            "application/octet-stream".to_string(),
        ]);
        capabilities.limits.rate_limit_hint_ms = Some(100);
        capabilities
    }

    fn validate_config(&self) -> Result<(), AdapterError> {
        if !self.config.enabled {
            return Err(execution_error(
                "not_configured",
                "Feishu channel is disabled",
                None,
                false,
            ));
        }
        if self.config.app_id.trim().is_empty() {
            return Err(execution_error(
                "invalid_config",
                "Feishu app_id is empty",
                None,
                false,
            ));
        }
        if self.config.app_secret.trim().is_empty() {
            return Err(execution_error(
                "invalid_config",
                "Feishu app_secret is empty",
                None,
                false,
            ));
        }
        Ok(())
    }

    fn set_health(&self, status: ChannelHealthStatus, diagnosis: Option<String>) {
        if let Ok(mut health) = self.health.lock() {
            health.status = status;
            health.diagnosis = diagnosis;
            health.checked_at = chrono::Utc::now();
        }
    }

    async fn get_access_token(&self) -> Result<String, AdapterError> {
        self.validate_config()?;
        {
            let cached = self.token.read().await;
            if let Some(cache) = cached.as_ref() {
                if Instant::now() < cache.expires_at {
                    return Ok(cache.value.clone());
                }
            }
        }

        let _guard = self.token_refresh.lock().await;
        {
            let cached = self.token.read().await;
            if let Some(cache) = cached.as_ref() {
                if Instant::now() < cache.expires_at {
                    return Ok(cache.value.clone());
                }
            }
        }

        let url = format!(
            "{}/auth/v3/tenant_access_token/internal",
            self.endpoint.api_base
        );
        let response = self
            .http_client
            .post(url)
            .json(&json!({
                "app_id": self.config.app_id,
                "app_secret": self.config.app_secret,
            }))
            .send()
            .await
            .map_err(|error| {
                execution_error(
                    "token_transport",
                    format!("Feishu tenant token request failed: {error}"),
                    None,
                    true,
                )
            })?;

        if !response.status().is_success() {
            return Err(map_http_error(response, "token request").await);
        }
        let token_response: TokenResponse = response.json().await.map_err(|error| {
            execution_error(
                "token_malformed",
                format!("Feishu tenant token response was malformed: {error}"),
                None,
                false,
            )
        })?;
        if token_response.code != 0 {
            return Err(execution_error(
                "token_rejected",
                format!(
                    "Feishu tenant token rejected with code {}",
                    token_response.code
                ),
                None,
                false,
            ));
        }
        let value = token_response
            .tenant_access_token
            .filter(|token| !token.trim().is_empty())
            .ok_or_else(|| {
                execution_error(
                    "token_malformed",
                    "Feishu tenant token response omitted tenant_access_token",
                    None,
                    false,
                )
            })?;
        let ttl = Duration::from_secs(token_response.expire.unwrap_or(7200).max(1) as u64);
        let expires_at = Instant::now()
            + ttl
                .saturating_sub(TOKEN_REFRESH_SKEW.min(ttl.saturating_sub(Duration::from_secs(1))));
        self.token.write().await.replace(TokenCache {
            value: value.clone(),
            expires_at,
        });
        Ok(value)
    }

    async fn invalidate_token(&self) {
        self.token.write().await.take();
    }

    async fn get_websocket_url(&self) -> Result<(String, WsClientConfig), AdapterError> {
        self.validate_config()?;
        let url = format!("{}/callback/ws/endpoint", self.endpoint.ws_base);
        let response = self
            .http_client
            .post(url)
            .json(&json!({
                "AppID": self.config.app_id,
                "AppSecret": self.config.app_secret,
            }))
            .send()
            .await
            .map_err(|error| {
                execution_error(
                    "ws_endpoint_transport",
                    format!("Feishu WebSocket endpoint request failed: {error}"),
                    None,
                    true,
                )
            })?;
        if !response.status().is_success() {
            return Err(map_http_error(response, "WebSocket endpoint request").await);
        }
        let endpoint: WsEndpointResponse = response.json().await.map_err(|error| {
            execution_error(
                "ws_endpoint_malformed",
                format!("Feishu WebSocket endpoint response was malformed: {error}"),
                None,
                false,
            )
        })?;
        if endpoint.code != 0 {
            return Err(execution_error(
                "ws_endpoint_rejected",
                format!(
                    "Feishu WebSocket endpoint rejected with code {}",
                    endpoint.code
                ),
                None,
                true,
            ));
        }
        let data = endpoint.data.ok_or_else(|| {
            execution_error(
                "ws_endpoint_malformed",
                "Feishu WebSocket endpoint omitted URL",
                None,
                false,
            )
        })?;
        if data.url.trim().is_empty() {
            return Err(execution_error(
                "ws_endpoint_malformed",
                "Feishu WebSocket endpoint returned an empty URL",
                None,
                false,
            ));
        }
        Ok((data.url, data.client_config.unwrap_or_default()))
    }

    fn ensure_supported(&self, command: &ChannelCommand) -> Result<(), AdapterError> {
        let target = command
            .target_channel()
            .map_err(|error| execution_error("invalid_target", error.to_string(), None, false))?;
        if target.as_str() != FEISHU_CHANNEL {
            return Err(execution_error(
                "wrong_channel",
                format!(
                    "command targets {}, adapter is {FEISHU_CHANNEL}",
                    target.as_str()
                ),
                None,
                false,
            ));
        }
        for capability in command.required_capabilities() {
            if !Self::static_capabilities().supports(capability) {
                return Err(AdapterError::UnsupportedCapability { capability });
            }
        }
        Ok(())
    }

    fn ensure_parts_supported(&self, parts: &[ContentPart]) -> Result<(), AdapterError> {
        for part in parts {
            let capability = match part {
                ContentPart::Text { .. }
                | ContentPart::Location { .. }
                | ContentPart::Reference { .. } => ChannelCapability::EgressText,
                ContentPart::Markdown { .. } => ChannelCapability::EgressMarkdown,
                ContentPart::Image { .. } => ChannelCapability::EgressImage,
                ContentPart::Audio { .. } => ChannelCapability::EgressAudio,
                ContentPart::Video { .. } => ChannelCapability::EgressVideo,
                ContentPart::File { .. } => ChannelCapability::EgressFile,
                ContentPart::Card { .. } => ChannelCapability::EgressCard,
            };
            if !Self::static_capabilities().supports(capability) {
                return Err(AdapterError::UnsupportedCapability { capability });
            }
        }
        Ok(())
    }

    async fn execute_send(
        &self,
        envelope: agent_diva_core::channel::ChannelEnvelopeV1,
        retry_safe: bool,
    ) -> Result<DeliveryReceipt, AdapterError> {
        envelope
            .validate()
            .map_err(|error| execution_error("invalid_envelope", error.to_string(), None, false))?;
        if envelope.address.channel != FEISHU_CHANNEL {
            return Err(execution_error(
                "wrong_channel",
                format!("expected {FEISHU_CHANNEL} target"),
                None,
                false,
            ));
        }
        let parts = match envelope.payload {
            ChannelPayloadV1::Message { parts, .. } | ChannelPayloadV1::Stream { parts, .. } => {
                parts
            }
            _ => {
                return Err(execution_error(
                    "invalid_payload",
                    "Feishu send requires a message or stream payload",
                    None,
                    false,
                ));
            }
        };
        self.ensure_parts_supported(&parts)?;
        self.send_parts(&envelope.address, &envelope.correlation, &parts, retry_safe)
            .await
    }

    async fn send_parts(
        &self,
        address: &ChannelAddress,
        correlation: &Correlation,
        parts: &[ContentPart],
        retry_safe: bool,
    ) -> Result<DeliveryReceipt, AdapterError> {
        if address.chat_id.trim().is_empty() {
            return Err(execution_error(
                "invalid_target",
                "Feishu chat_id is empty",
                None,
                false,
            ));
        }
        if parts.is_empty() {
            return Err(execution_error(
                "empty_message",
                "Feishu message has no content parts",
                None,
                false,
            ));
        }

        let payload = self.build_outbound_payload(parts).await?;
        let receive_id_type = if address.chat_id.starts_with("oc_") {
            "chat_id"
        } else {
            "open_id"
        };
        if let Some(parent) = correlation.reply_to.as_deref() {
            if !valid_path_segment(parent) {
                return Err(execution_error(
                    "invalid_reply_target",
                    "Feishu reply_to contains invalid path characters",
                    None,
                    false,
                ));
            }
        }
        let body = if correlation.reply_to.is_some() {
            json!({
                "msg_type": payload.msg_type,
                "content": payload.content,
            })
        } else {
            json!({
                "receive_id": address.chat_id,
                "msg_type": payload.msg_type,
                "content": payload.content,
            })
        };
        let mut token = self.get_access_token().await?;
        let mut retried = false;
        let response = loop {
            let request = if let Some(parent) = correlation.reply_to.as_deref() {
                self.http_client.post(format!(
                    "{}/im/v1/messages/{parent}/reply",
                    self.endpoint.api_base
                ))
            } else {
                let url = format!("{}/im/v1/messages", self.endpoint.api_base);
                self.http_client
                    .post(url)
                    .query(&[("receive_id_type", receive_id_type)])
            };
            let response = request
                .bearer_auth(&token)
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|error| {
                    execution_error(
                        "send_transport",
                        format!("Feishu send request failed: {error}"),
                        None,
                        correlation.request_id.is_some(),
                    )
                })?;
            if response.status() == reqwest::StatusCode::UNAUTHORIZED && retry_safe && !retried {
                self.invalidate_token().await;
                token = self.get_access_token().await?;
                retried = true;
                continue;
            }
            break response;
        };
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.invalidate_token().await;
        }
        if !response.status().is_success() {
            return Err(map_http_error(response, "Feishu send").await);
        }
        let result: MessageResponse = response.json().await.map_err(|error| {
            execution_error(
                "send_malformed",
                format!("Feishu send response was malformed: {error}"),
                None,
                false,
            )
        })?;
        if result.code != 0 {
            return Err(execution_error(
                "send_rejected",
                format!("Feishu send rejected with code {}", result.code),
                None,
                correlation.request_id.is_some(),
            ));
        }
        let message_id = result.data.and_then(|data| data.message_id);
        let thread_id = address.thread_id.clone();
        Ok(accepted_receipt(
            FEISHU_CHANNEL,
            address.chat_id.clone(),
            message_id,
            thread_id,
        ))
    }

    async fn build_outbound_payload(
        &self,
        parts: &[ContentPart],
    ) -> Result<OutboundPayload, AdapterError> {
        if parts.len() > 1
            && parts
                .iter()
                .any(|part| matches!(part, ContentPart::Image { .. } | ContentPart::File { .. }))
        {
            return Err(execution_error(
                "unsupported_multipart",
                "Feishu media messages accept one media part per API request",
                None,
                false,
            ));
        }
        let first = parts.first().ok_or_else(|| {
            execution_error(
                "empty_message",
                "Feishu message has no content",
                None,
                false,
            )
        })?;
        match first {
            ContentPart::Text { text } => Ok(OutboundPayload {
                msg_type: "text",
                content: json!({ "text": text }).to_string(),
            }),
            ContentPart::Markdown { markdown } => Ok(OutboundPayload {
                msg_type: "interactive",
                content: self.build_card(markdown).to_string(),
            }),
            ContentPart::Card { body, .. } => Ok(OutboundPayload {
                msg_type: "interactive",
                content: body.to_string(),
            }),
            ContentPart::Image { attachment } => {
                let stored = self
                    .services
                    .attachments
                    .get(attachment)
                    .await
                    .map_err(|error| {
                        execution_error(
                            "attachment_read",
                            format!("Feishu image attachment unavailable: {error}"),
                            None,
                            false,
                        )
                    })?;
                let image_key = self
                    .upload_image(&stored.bytes, attachment.file_name.as_deref())
                    .await?;
                Ok(OutboundPayload {
                    msg_type: "image",
                    content: json!({ "image_key": image_key }).to_string(),
                })
            }
            ContentPart::File { attachment } => {
                let stored = self
                    .services
                    .attachments
                    .get(attachment)
                    .await
                    .map_err(|error| {
                        execution_error(
                            "attachment_read",
                            format!("Feishu file attachment unavailable: {error}"),
                            None,
                            false,
                        )
                    })?;
                let file_key = self
                    .upload_file(
                        &stored.bytes,
                        attachment.file_name.as_deref().unwrap_or("attachment.bin"),
                    )
                    .await?;
                Ok(OutboundPayload {
                    msg_type: "file",
                    content: json!({ "file_key": file_key }).to_string(),
                })
            }
            ContentPart::Audio { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::EgressAudio,
            }),
            ContentPart::Video { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::EgressVideo,
            }),
            ContentPart::Location { .. } | ContentPart::Reference { .. } => Err(execution_error(
                "unsupported_content",
                "Feishu does not have a native C5 location/reference message mapping",
                None,
                false,
            )),
        }
    }

    async fn upload_image(
        &self,
        bytes: &[u8],
        file_name: Option<&str>,
    ) -> Result<String, AdapterError> {
        let token = self.get_access_token().await?;
        let part =
            Part::bytes(bytes.to_vec()).file_name(file_name.unwrap_or("image.bin").to_string());
        let part = match file_name.and_then(|name| mime_guess::from_path(name).first_raw()) {
            Some(mime) => match part.mime_str(mime) {
                Ok(candidate) => candidate,
                Err(_) => Part::bytes(bytes.to_vec())
                    .file_name(file_name.unwrap_or("image.bin").to_string()),
            },
            None => part,
        };
        let form = Form::new()
            .text("image_type", "message")
            .part("image", part);
        let response = self
            .http_client
            .post(format!("{}/im/v1/images", self.endpoint.api_base))
            .bearer_auth(token)
            .multipart(form)
            .send()
            .await
            .map_err(|error| {
                execution_error(
                    "image_upload_transport",
                    format!("Feishu image upload failed: {error}"),
                    None,
                    true,
                )
            })?;
        if !response.status().is_success() {
            return Err(map_http_error(response, "Feishu image upload").await);
        }
        let result: UploadImageResponse = response.json().await.map_err(|error| {
            execution_error(
                "image_upload_malformed",
                format!("Feishu image upload response was malformed: {error}"),
                None,
                false,
            )
        })?;
        if result.code != 0 {
            return Err(execution_error(
                "image_upload_rejected",
                format!("Feishu image upload rejected with code {}", result.code),
                None,
                true,
            ));
        }
        result
            .data
            .and_then(|data| data.image_key)
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| {
                execution_error(
                    "image_upload_malformed",
                    "Feishu image upload response omitted image_key",
                    None,
                    false,
                )
            })
    }

    async fn upload_file(&self, bytes: &[u8], file_name: &str) -> Result<String, AdapterError> {
        let token = self.get_access_token().await?;
        let part = Part::bytes(bytes.to_vec()).file_name(file_name.to_string());
        let form = Form::new()
            .text("file_type", "stream")
            .text("file_name", file_name.to_string())
            .part("file", part);
        let response = self
            .http_client
            .post(format!("{}/im/v1/files", self.endpoint.api_base))
            .bearer_auth(token)
            .multipart(form)
            .send()
            .await
            .map_err(|error| {
                execution_error(
                    "file_upload_transport",
                    format!("Feishu file upload failed: {error}"),
                    None,
                    true,
                )
            })?;
        if !response.status().is_success() {
            return Err(map_http_error(response, "Feishu file upload").await);
        }
        let result: UploadFileResponse = response.json().await.map_err(|error| {
            execution_error(
                "file_upload_malformed",
                format!("Feishu file upload response was malformed: {error}"),
                None,
                false,
            )
        })?;
        if result.code != 0 {
            return Err(execution_error(
                "file_upload_rejected",
                format!("Feishu file upload rejected with code {}", result.code),
                None,
                true,
            ));
        }
        result
            .data
            .and_then(|data| data.file_key)
            .filter(|key| !key.trim().is_empty())
            .ok_or_else(|| {
                execution_error(
                    "file_upload_malformed",
                    "Feishu file upload response omitted file_key",
                    None,
                    false,
                )
            })
    }

    async fn execute_edit(
        &self,
        address: ChannelAddress,
        target_message_id: String,
        parts: Vec<ContentPart>,
    ) -> Result<DeliveryReceipt, AdapterError> {
        if !valid_path_segment(&target_message_id) {
            return Err(execution_error(
                "invalid_message_id",
                "Feishu target message ID contains invalid path characters",
                None,
                false,
            ));
        }
        self.ensure_parts_supported(&parts)?;
        if parts.iter().any(|part| {
            matches!(
                part,
                ContentPart::Image { .. }
                    | ContentPart::File { .. }
                    | ContentPart::Audio { .. }
                    | ContentPart::Video { .. }
            )
        }) {
            return Err(execution_error(
                "unsupported_edit_media",
                "Feishu edit supports text, Markdown and card bodies only",
                None,
                false,
            ));
        }
        let payload = self.build_outbound_payload(&parts).await?;
        let token = self.get_access_token().await?;
        let response = self
            .http_client
            .patch(format!(
                "{}/im/v1/messages/{target_message_id}",
                self.endpoint.api_base
            ))
            .bearer_auth(token)
            .json(&json!({
                "msg_type": payload.msg_type,
                "content": payload.content,
            }))
            .send()
            .await
            .map_err(|error| {
                execution_error(
                    "edit_transport",
                    format!("Feishu edit request failed: {error}"),
                    None,
                    true,
                )
            })?;
        if !response.status().is_success() {
            return Err(map_http_error(response, "Feishu edit").await);
        }
        let result: MessageResponse = response.json().await.map_err(|error| {
            execution_error(
                "edit_malformed",
                format!("Feishu edit response was malformed: {error}"),
                None,
                false,
            )
        })?;
        if result.code != 0 {
            return Err(execution_error(
                "edit_rejected",
                format!("Feishu edit rejected with code {}", result.code),
                None,
                true,
            ));
        }
        Ok(accepted_receipt(
            FEISHU_CHANNEL,
            address.chat_id,
            Some(target_message_id),
            address.thread_id,
        ))
    }

    async fn execute_delete(
        &self,
        address: ChannelAddress,
        target_message_id: String,
    ) -> Result<DeliveryReceipt, AdapterError> {
        if !valid_path_segment(&target_message_id) {
            return Err(execution_error(
                "invalid_message_id",
                "Feishu target message ID contains invalid path characters",
                None,
                false,
            ));
        }
        let token = self.get_access_token().await?;
        let response = self
            .http_client
            .delete(format!(
                "{}/im/v1/messages/{target_message_id}",
                self.endpoint.api_base
            ))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|error| {
                execution_error(
                    "delete_transport",
                    format!("Feishu delete request failed: {error}"),
                    None,
                    true,
                )
            })?;
        if !response.status().is_success() {
            return Err(map_http_error(response, "Feishu delete").await);
        }
        Ok(delivered_receipt(
            FEISHU_CHANNEL,
            address.chat_id,
            Some(target_message_id),
            address.thread_id,
        ))
    }

    async fn probe_health(&self) -> Result<DeliveryReceipt, AdapterError> {
        let token = self.get_access_token().await?;
        if token.trim().is_empty() {
            return Err(execution_error(
                "health_auth",
                "Feishu health probe received an empty token",
                None,
                false,
            ));
        }
        self.set_health(ChannelHealthStatus::Healthy, None);
        Ok(accepted_receipt(FEISHU_CHANNEL, "", None, None))
    }

    async fn run_listener(
        &self,
        context: AdapterContext,
        listener_cancel: CancellationToken,
    ) -> Result<(), AdapterError> {
        self.get_access_token().await?;
        let mut reconnect_delay = Duration::from_millis(250);
        loop {
            if context.cancel.is_cancelled() || listener_cancel.is_cancelled() {
                return Ok(());
            }
            let (ws_url, client_config) = self.get_websocket_url().await?;
            match self
                .run_websocket(&ws_url, client_config, &context, &listener_cancel)
                .await
            {
                Ok(()) => {
                    if context.cancel.is_cancelled() || listener_cancel.is_cancelled() {
                        return Ok(());
                    }
                    self.set_health(
                        ChannelHealthStatus::Degraded,
                        Some("Feishu WebSocket ended; reconnecting".to_string()),
                    );
                }
                Err(error) => {
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                }
            }
            tokio::select! {
                biased;
                _ = context.cancel.cancelled() => return Ok(()),
                _ = listener_cancel.cancelled() => return Ok(()),
                _ = sleep(reconnect_delay) => {}
            }
            reconnect_delay = reconnect_delay
                .saturating_mul(2)
                .min(Duration::from_secs(30));
        }
    }

    async fn run_websocket(
        &self,
        ws_url: &str,
        client_config: WsClientConfig,
        context: &AdapterContext,
        listener_cancel: &CancellationToken,
    ) -> Result<(), AdapterError> {
        let connect = tokio::select! {
            biased;
            _ = context.cancel.cancelled() => return Ok(()),
            _ = listener_cancel.cancelled() => return Ok(()),
            result = connect_async(ws_url) => result,
        };
        let (stream, _) = connect.map_err(|error| {
            execution_error(
                "ws_connect",
                format!("Feishu WebSocket connection failed: {error}"),
                None,
                true,
            )
        })?;
        self.set_health(ChannelHealthStatus::Healthy, None);
        let (mut write, mut read) = stream.split();
        let service_id = ws_url
            .split('?')
            .nth(1)
            .and_then(|query| {
                query.split('&').find_map(|part| {
                    let (key, value) = part.split_once('=')?;
                    (key == "service_id")
                        .then(|| value.parse::<i32>().ok())
                        .flatten()
                })
            })
            .unwrap_or(0);
        let mut sequence = 0_u64;
        let ping_interval = Duration::from_secs(client_config.ping_interval.unwrap_or(120).max(10));
        let mut heartbeat = tokio::time::interval(ping_interval);
        heartbeat.tick().await;
        let mut timeout_check = tokio::time::interval(Duration::from_secs(10));
        let mut last_received = Instant::now();
        let mut fragments: HashMap<String, FragmentEntry> = HashMap::new();

        send_ping(&mut write, service_id, &mut sequence).await?;
        loop {
            tokio::select! {
                biased;
                _ = context.cancel.cancelled() => return Ok(()),
                _ = listener_cancel.cancelled() => return Ok(()),
                _ = heartbeat.tick() => {
                    send_ping(&mut write, service_id, &mut sequence).await?;
                    let cutoff = Instant::now().checked_sub(Duration::from_secs(300)).unwrap_or_else(Instant::now);
                    fragments.retain(|_, (_, timestamp)| *timestamp > cutoff);
                }
                _ = timeout_check.tick() => {
                    if last_received.elapsed() > WS_HEARTBEAT_TIMEOUT {
                        return Err(execution_error("ws_heartbeat_timeout", "Feishu WebSocket heartbeat timed out", None, true));
                    }
                }
                message = read.next() => {
                    let raw = match message {
                        Some(Ok(WsMessage::Binary(bytes))) => { last_received = Instant::now(); bytes }
                        Some(Ok(WsMessage::Ping(bytes))) => { write.send(WsMessage::Pong(bytes)).await.map_err(|error| execution_error("ws_pong", format!("Feishu WebSocket pong failed: {error}"), None, true))?; continue; }
                        Some(Ok(WsMessage::Close(_))) | None => return Ok(()),
                        Some(Ok(_)) => continue,
                        Some(Err(error)) => return Err(execution_error("ws_read", format!("Feishu WebSocket read failed: {error}"), None, true)),
                    };
                    let frame = PbFrame::decode(raw.as_slice()).map_err(|error| execution_error("ws_frame_decode", format!("Feishu protobuf frame decode failed: {error}"), None, false))?;
                    if frame.method == 0 {
                        continue;
                    }
                    let message_type = frame.header_value("type");
                    let message_id = frame.header_value("message_id").to_string();
                    let fragment_count = frame.header_value("sum").parse::<usize>().unwrap_or(1).max(1);
                    let fragment_index = frame.header_value("seq").parse::<usize>().unwrap_or(0);
                    let payload = if fragment_count == 1 || message_id.is_empty() || fragment_index >= fragment_count {
                        frame.payload.clone().unwrap_or_default()
                    } else {
                        let entry = fragments.entry(message_id.clone()).or_insert_with(|| (vec![None; fragment_count], Instant::now()));
                        if entry.0.len() != fragment_count {
                            *entry = (vec![None; fragment_count], Instant::now());
                        }
                        entry.0[fragment_index] = frame.payload.clone();
                        if entry.0.iter().all(Option::is_some) {
                            let full = entry.0.iter().flat_map(|part| part.as_deref().unwrap_or_default()).copied().collect::<Vec<_>>();
                            fragments.remove(&message_id);
                            full
                        } else {
                            continue;
                        }
                    };
                    if message_type == "event" {
                        tokio::time::timeout(INGRESS_ADMISSION_DEADLINE, self.handle_event_payload(&payload, context)).await.map_err(|_| execution_error("fabric_admission_timeout", "Feishu event exceeded ACK deadline", Some(Duration::from_millis(100)), true))??;
                    }
                    let ack = ack_frame(&frame);
                    write.send(WsMessage::Binary(ack.encode_to_vec())).await.map_err(|error| execution_error("ws_ack", format!("Feishu ACK failed: {error}"), None, true))?;
                }
            }
        }
    }

    async fn handle_event_payload(
        &self,
        payload: &[u8],
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        let event: LarkEvent = serde_json::from_slice(payload).map_err(|error| {
            execution_error(
                "event_malformed",
                format!("Feishu event payload was malformed: {error}"),
                None,
                false,
            )
        })?;
        if event.header.event_type != "im.message.receive_v1" {
            return Ok(());
        }
        let received: MsgReceivePayload = serde_json::from_value(event.event).map_err(|error| {
            execution_error(
                "event_malformed",
                format!("Feishu receive event was malformed: {error}"),
                None,
                false,
            )
        })?;
        if matches!(received.sender.sender_type.as_str(), "app" | "bot") {
            return Ok(());
        }
        let sender_id = received.sender.sender_id.open_id.as_deref().unwrap_or("");
        if !is_sender_allowed(&self.config.allow_from, sender_id) {
            return Ok(());
        }
        let dedup_key = dedup_key(
            Some(&event.header.event_id),
            Some(&received.message.message_id),
        );
        let reserved = if let Some(key) = dedup_key.as_deref() {
            self.reserve_dedup(key).await
        } else {
            true
        };
        if !reserved {
            return Ok(());
        }
        let permit = match tokio::time::timeout(
            INGRESS_ADMISSION_DEADLINE,
            self.admission.clone().acquire_owned(),
        )
        .await
        {
            Ok(Ok(permit)) => permit,
            Ok(Err(_)) | Err(_) => {
                if let Some(key) = dedup_key.as_deref() {
                    self.release_dedup(key).await;
                }
                return Err(execution_error(
                    "fabric_admission_busy",
                    "Feishu adapter admission permit was busy",
                    Some(Duration::from_millis(100)),
                    true,
                ));
            }
        };
        let result = match self.build_inbound_parts(&received.message, sender_id).await {
            Ok(parts) => {
                let address = ChannelAddress {
                    channel: FEISHU_CHANNEL.to_string(),
                    account_id: Some(self.config.app_id.clone()),
                    sender_id: Some(sender_id.to_string()),
                    chat_id: received.message.chat_id.clone(),
                    thread_id: received
                        .message
                        .thread_id
                        .clone()
                        .or_else(|| received.message.root_id.clone()),
                };
                let mut correlation =
                    Correlation::new(format!("{FEISHU_CHANNEL}:{}", received.message.chat_id));
                correlation.message_id = Some(received.message.message_id.clone());
                correlation.reply_to = received.message.parent_id.clone();
                let mut envelope =
                    external_message_envelope(address, correlation, parts, None, None);
                envelope.extensions.insert(
                    "agent-diva.feishu_chat_type".to_string(),
                    Value::String(received.message.chat_type.clone()),
                );
                context
                    .fabric
                    .admit_ingress(envelope, INGRESS_ADMISSION_DEADLINE, &context.cancel)
                    .await
                    .map_err(|error| {
                        execution_error(
                            "fabric_admission",
                            error.to_string(),
                            Some(Duration::from_millis(100)),
                            true,
                        )
                    })
            }
            Err(error) => Err(error),
        };
        drop(permit);
        match result {
            Ok(()) => {
                if let Some(key) = dedup_key.as_deref() {
                    self.commit_dedup(key).await;
                }
                // DIVA's reaction is a best-effort seen marker, not a declared
                // generic InteractionReaction capability and never inbound failure.
                let _ = tokio::time::timeout(
                    Duration::from_millis(100),
                    self.add_seen_reaction(&received.message.message_id),
                )
                .await;
                Ok(())
            }
            Err(error) => {
                if let Some(key) = dedup_key.as_deref() {
                    self.release_dedup(key).await;
                }
                Err(error)
            }
        }
    }

    async fn build_inbound_parts(
        &self,
        message: &LarkMessage,
        sender_id: &str,
    ) -> Result<Vec<ContentPart>, AdapterError> {
        let message_type = message.message_type.as_str();
        match message_type {
            "text" => Ok(vec![ContentPart::Text {
                text: extract_text(&message.content),
            }]),
            "post" => Ok(vec![ContentPart::Markdown {
                markdown: rich_post_markdown(&message.content),
            }]),
            "image" => Ok(vec![ContentPart::Image {
                attachment: self
                    .fetch_and_store_media(message, sender_id, "image")
                    .await?,
            }]),
            "file" | "sticker" => Ok(vec![ContentPart::File {
                attachment: self
                    .fetch_and_store_media(message, sender_id, "file")
                    .await?,
            }]),
            "audio" => Ok(vec![ContentPart::Audio {
                attachment: self
                    .fetch_and_store_media(message, sender_id, "audio")
                    .await?,
                transcript: None,
            }]),
            "media" | "video" => Ok(vec![ContentPart::Video {
                attachment: self
                    .fetch_and_store_media(message, sender_id, "video")
                    .await?,
            }]),
            _ => Ok(vec![ContentPart::Text {
                text: format!("[{message_type}]"),
            }]),
        }
    }

    async fn fetch_and_store_media(
        &self,
        message: &LarkMessage,
        sender_id: &str,
        media_kind: &str,
    ) -> Result<agent_diva_core::channel::AttachmentRef, AdapterError> {
        let key = extract_media_key(&message.content).ok_or_else(|| {
            execution_error(
                "media_malformed",
                format!("Feishu {media_kind} message omitted a resource key"),
                None,
                false,
            )
        })?;
        if !valid_path_segment(&message.message_id) || !valid_path_segment(&key) {
            return Err(execution_error(
                "media_malformed",
                "Feishu media identifiers contain invalid path characters",
                None,
                false,
            ));
        }
        let token = self.get_access_token().await?;
        let url = format!(
            "{}/im/v1/messages/{}/resources/{}",
            self.endpoint.api_base, message.message_id, key
        );
        let response = tokio::time::timeout(
            MEDIA_REQUEST_TIMEOUT,
            self.http_client
                .get(url)
                .query(&[("type", media_kind)])
                .bearer_auth(token)
                .send(),
        )
        .await
        .map_err(|_| {
            execution_error(
                "media_timeout",
                "Feishu media download exceeded the bounded timeout",
                None,
                true,
            )
        })?
        .map_err(|error| {
            execution_error(
                "media_transport",
                format!("Feishu media download failed: {error}"),
                None,
                true,
            )
        })?;
        if !response.status().is_success() {
            return Err(map_http_error(response, "Feishu media download").await);
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_ATTACHMENT_BYTES)
        {
            return Err(execution_error(
                "media_too_large",
                "Feishu media exceeds the bounded attachment limit",
                None,
                false,
            ));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(';').next())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("application/octet-stream")
            .to_string();
        let expected_mime = match media_kind {
            "image" => content_type.starts_with("image/"),
            "audio" => content_type.starts_with("audio/"),
            "video" => content_type.starts_with("video/"),
            // Feishu's file resource may legitimately be any registered MIME
            // (including a vendor type), so only typed media gets a strict
            // family check here.
            _ => true,
        };
        if !expected_mime {
            return Err(execution_error(
                "media_mime_mismatch",
                format!("Feishu {media_kind} resource returned an incompatible MIME type"),
                None,
                false,
            ));
        }
        let bytes = response.bytes().await.map_err(|error| {
            execution_error(
                "media_read",
                format!("Feishu media body could not be read: {error}"),
                None,
                true,
            )
        })?;
        if bytes.is_empty() {
            return Err(execution_error(
                "media_empty",
                "Feishu media response was empty",
                None,
                false,
            ));
        }
        if bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
            return Err(execution_error(
                "media_too_large",
                "Feishu media exceeds the bounded attachment limit",
                None,
                false,
            ));
        }
        self.services
            .attachments
            .put(IngressAttachment {
                source_channel: FEISHU_CHANNEL.to_string(),
                platform_message_id: Some(message.message_id.clone()),
                sender_id: Some(sender_id.to_string()),
                file_name: extract_file_name(&message.content),
                declared_mime: Some(content_type),
                bytes: bytes.to_vec(),
            })
            .await
            .map_err(|error| {
                execution_error(
                    "attachment_store",
                    format!("Feishu attachment storage failed: {error}"),
                    None,
                    false,
                )
            })
    }

    async fn add_seen_reaction(&self, message_id: &str) -> Result<(), AdapterError> {
        if !valid_path_segment(message_id) {
            return Ok(());
        }
        let token = self.get_access_token().await?;
        let response = self
            .http_client
            .post(format!(
                "{}/im/v1/messages/{message_id}/reactions",
                self.endpoint.api_base
            ))
            .bearer_auth(token)
            .json(&json!({
                "reaction_type": { "emoji_type": "THUMBSUP" }
            }))
            .send()
            .await
            .map_err(|error| {
                execution_error(
                    "seen_reaction_transport",
                    format!("Feishu seen reaction failed: {error}"),
                    None,
                    true,
                )
            })?;
        if !response.status().is_success() {
            let _ = response.bytes().await;
        }
        Ok(())
    }

    async fn reserve_dedup(&self, key: &str) -> bool {
        let now = Instant::now();
        let mut state = self.dedup.lock().await;
        if state
            .last_cleanup
            .map(|last| now.duration_since(last) >= DEDUP_CLEANUP_INTERVAL)
            .unwrap_or(true)
        {
            state
                .seen
                .retain(|_, timestamp| now.duration_since(*timestamp) < DEDUP_TTL);
            state.last_cleanup = Some(now);
        }
        if state.seen.contains_key(key) || state.pending.contains(key) {
            return false;
        }
        state.pending.insert(key.to_string());
        true
    }

    async fn commit_dedup(&self, key: &str) {
        let mut state = self.dedup.lock().await;
        state.pending.remove(key);
        state.seen.insert(key.to_string(), Instant::now());
    }

    async fn release_dedup(&self, key: &str) {
        self.dedup.lock().await.pending.remove(key);
    }

    fn build_card(&self, content: &str) -> Value {
        let mut elements = Vec::new();
        let mut last_end = 0;
        for matched in table_regex().find_iter(content) {
            let before = content[last_end..matched.start()].trim();
            if !before.is_empty() {
                elements.push(json!({ "tag": "markdown", "content": before }));
            }
            elements.push(
                parse_markdown_table(matched.as_str())
                    .unwrap_or_else(|| json!({ "tag": "markdown", "content": matched.as_str() })),
            );
            last_end = matched.end();
        }
        let remaining = content[last_end..].trim();
        if !remaining.is_empty() {
            elements.push(json!({ "tag": "markdown", "content": remaining }));
        }
        if elements.is_empty() {
            elements.push(json!({ "tag": "markdown", "content": content }));
        }
        json!({
            "config": { "wide_screen_mode": true },
            "elements": elements,
        })
    }
}

#[async_trait]
impl ChannelAdapter for FeishuAdapter {
    fn name(&self) -> ChannelId {
        match ChannelId::new(FEISHU_CHANNEL) {
            Ok(id) => id,
            Err(_) => unreachable!("constant Feishu channel ID is non-empty"),
        }
    }

    fn capabilities(&self) -> ChannelCapabilities {
        Self::static_capabilities()
    }

    async fn start(&self, context: AdapterContext) -> Result<(), AdapterError> {
        if self.running.swap(true, Ordering::AcqRel) {
            return Err(execution_error(
                "already_running",
                "Feishu adapter listener is already running",
                None,
                false,
            ));
        }
        let listener_cancel = context.cancel.child_token();
        self.listener_cancel
            .lock()
            .await
            .replace(listener_cancel.clone());
        let result = self.run_listener(context, listener_cancel).await;
        self.listener_cancel.lock().await.take();
        self.running.store(false, Ordering::Release);
        if result.is_err() {
            self.set_health(
                ChannelHealthStatus::Degraded,
                result.as_ref().err().map(ToString::to_string),
            );
        }
        result
    }

    async fn execute(&self, command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError> {
        self.ensure_supported(&command)?;
        match command {
            ChannelCommand::Send {
                envelope,
                idempotency_key,
            } => self.execute_send(envelope, idempotency_key.is_some()).await,
            ChannelCommand::Typing { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionTyping,
            }),
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
            ChannelCommand::React { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionReaction,
            }),
            ChannelCommand::FinalizeStream {
                address,
                correlation,
                parts,
                idempotency_key,
            } => {
                self.ensure_parts_supported(&parts)?;
                if let Some(target_message_id) = correlation.message_id.clone() {
                    self.execute_edit(address, target_message_id, parts).await
                } else {
                    self.send_parts(&address, &correlation, &parts, idempotency_key.is_some())
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
        self.running.store(false, Ordering::Release);
        if let Some(cancel) = self.listener_cancel.lock().await.take() {
            cancel.cancel();
        }
        self.set_health(
            ChannelHealthStatus::Down,
            Some("Feishu adapter stopped".to_string()),
        );
        Ok(())
    }
}

#[derive(Debug)]
struct OutboundPayload {
    msg_type: &'static str,
    content: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    code: i32,
    #[serde(default)]
    tenant_access_token: Option<String>,
    #[serde(default)]
    expire: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct WsEndpointResponse {
    code: i32,
    #[serde(default)]
    data: Option<WsEndpointData>,
}

#[derive(Debug, Deserialize)]
struct WsEndpointData {
    #[serde(rename = "URL")]
    url: String,
    #[serde(rename = "ClientConfig")]
    client_config: Option<WsClientConfig>,
}

#[derive(Debug, Deserialize, Default, Clone)]
struct WsClientConfig {
    #[serde(rename = "PingInterval")]
    ping_interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    code: i32,
    #[serde(default)]
    data: Option<MessageData>,
}

#[derive(Debug, Deserialize)]
struct MessageData {
    #[serde(default)]
    message_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadImageResponse {
    code: i32,
    #[serde(default)]
    data: Option<UploadImageData>,
}

#[derive(Debug, Deserialize)]
struct UploadImageData {
    #[serde(default)]
    image_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UploadFileResponse {
    code: i32,
    #[serde(default)]
    data: Option<UploadFileData>,
}

#[derive(Debug, Deserialize)]
struct UploadFileData {
    #[serde(default)]
    file_key: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LarkEvent {
    header: LarkEventHeader,
    event: Value,
}

#[derive(Debug, Deserialize)]
struct LarkEventHeader {
    event_type: String,
    #[serde(default)]
    event_id: String,
}

#[derive(Debug, Deserialize)]
struct MsgReceivePayload {
    sender: LarkSender,
    message: LarkMessage,
}

#[derive(Debug, Deserialize)]
struct LarkSender {
    sender_id: LarkSenderId,
    #[serde(default)]
    sender_type: String,
}

#[derive(Debug, Deserialize, Default)]
struct LarkSenderId {
    #[serde(default)]
    open_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LarkMessage {
    message_id: String,
    chat_id: String,
    #[serde(default)]
    chat_type: String,
    message_type: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    thread_id: Option<String>,
    #[serde(default)]
    root_id: Option<String>,
    #[serde(default)]
    parent_id: Option<String>,
}

#[derive(Clone, PartialEq, ProstDeriveMessage)]
struct PbHeader {
    #[prost(string, tag = "1")]
    key: String,
    #[prost(string, tag = "2")]
    value: String,
}

#[derive(Clone, PartialEq, ProstDeriveMessage)]
struct PbFrame {
    #[prost(uint64, tag = "1")]
    seq_id: u64,
    #[prost(uint64, tag = "2")]
    log_id: u64,
    #[prost(int32, tag = "3")]
    service: i32,
    #[prost(int32, tag = "4")]
    method: i32,
    #[prost(message, repeated, tag = "5")]
    headers: Vec<PbHeader>,
    #[prost(bytes = "vec", optional, tag = "8")]
    payload: Option<Vec<u8>>,
}

type FragmentEntry = (Vec<Option<Vec<u8>>>, Instant);

impl PbFrame {
    fn header_value(&self, key: &str) -> &str {
        self.headers
            .iter()
            .find(|header| header.key == key)
            .map(|header| header.value.as_str())
            .unwrap_or("")
    }
}

async fn send_ping<S>(
    write: &mut S,
    service_id: i32,
    sequence: &mut u64,
) -> Result<(), AdapterError>
where
    S: SinkExt<WsMessage> + Unpin,
    S::Error: std::fmt::Display,
{
    *sequence = sequence.wrapping_add(1);
    let ping = PbFrame {
        seq_id: *sequence,
        log_id: 0,
        service: service_id,
        method: 0,
        headers: vec![PbHeader {
            key: "type".to_string(),
            value: "ping".to_string(),
        }],
        payload: None,
    };
    write
        .send(WsMessage::Binary(ping.encode_to_vec()))
        .await
        .map_err(|error| {
            execution_error(
                "ws_ping",
                format!("Feishu heartbeat failed: {error}"),
                None,
                true,
            )
        })
}

fn ack_frame(frame: &PbFrame) -> PbFrame {
    let mut ack = frame.clone();
    ack.payload = Some(br#"{"code":200,"headers":{},"data":[]}"#.to_vec());
    ack.headers.push(PbHeader {
        key: "biz_rt".to_string(),
        value: "0".to_string(),
    });
    ack
}

fn valid_path_segment(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn dedup_key(event_id: Option<&str>, message_id: Option<&str>) -> Option<String> {
    event_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| format!("event:{value}"))
        .or_else(|| {
            message_id
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(|value| format!("message:{value}"))
        })
}

fn extract_text(content: &str) -> String {
    serde_json::from_str::<Value>(content)
        .ok()
        .and_then(|value| {
            value
                .get("text")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .unwrap_or_else(|| content.to_string())
}

fn extract_media_key(content: &str) -> Option<String> {
    if let Ok(value) = serde_json::from_str::<Value>(content) {
        if let Some(key) = ["image_key", "file_key", "media_key", "fileKey", "imageKey"]
            .iter()
            .find_map(|key| value.get(*key).and_then(Value::as_str).map(str::to_string))
        {
            return Some(key);
        }
    }
    let raw = content.trim();
    (!raw.is_empty()).then(|| raw.to_string())
}

fn extract_file_name(content: &str) -> Option<String> {
    serde_json::from_str::<Value>(content)
        .ok()
        .and_then(|value| {
            value
                .get("file_name")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .filter(|name| !name.contains('/') && !name.contains('\\') && !name.trim().is_empty())
}

fn rich_post_markdown(content: &str) -> String {
    let Ok(value) = serde_json::from_str::<Value>(content) else {
        return content.to_string();
    };
    let mut output = String::new();
    collect_post_text(&value, &mut output);
    if output.trim().is_empty() {
        content.to_string()
    } else {
        output.trim().to_string()
    }
}

fn collect_post_text(value: &Value, output: &mut String) {
    match value {
        Value::String(text) => {
            if !text.trim().is_empty() {
                if !output.is_empty() {
                    output.push('\n');
                }
                output.push_str(text);
            }
        }
        Value::Array(items) => items
            .iter()
            .for_each(|item| collect_post_text(item, output)),
        Value::Object(object) => {
            if let Some(tag) = object.get("tag").and_then(Value::as_str) {
                if matches!(tag, "text" | "a" | "at" | "md" | "markdown") {
                    if let Some(text) = object
                        .get("text")
                        .or_else(|| object.get("content"))
                        .and_then(Value::as_str)
                    {
                        collect_post_text(&Value::String(text.to_string()), output);
                    }
                }
            }
            for (key, child) in object {
                if key != "tag" && key != "text" && key != "content" {
                    collect_post_text(child, output);
                }
            }
        }
        _ => {}
    }
}

fn table_regex() -> regex::Regex {
    regex::Regex::new(
        r"((?:^[ \t]*\|.+\|[ \t]*\n)(?:^[ \t]*\|[-:\s|]+\|[ \t]*\n)(?:^[ \t]*\|.+\|[ \t]*\n?)+)",
    )
    .unwrap_or_else(|_| regex::Regex::new(r"\A\z").unwrap_or_else(|_| unreachable!()))
}

fn parse_markdown_table(table: &str) -> Option<Value> {
    let lines = table
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();
    if lines.len() < 3 {
        return None;
    }
    let split = |line: &str| {
        line.trim_matches('|')
            .split('|')
            .map(|cell| cell.trim().to_string())
            .collect::<Vec<_>>()
    };
    let headers = split(lines[0]);
    let rows = lines[2..]
        .iter()
        .map(|line| split(line))
        .collect::<Vec<_>>();
    let columns = headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            json!({
                "tag": "column",
                "name": format!("c{index}"),
                "display_name": header,
                "width": "auto",
            })
        })
        .collect::<Vec<_>>();
    let row_objects = rows
        .iter()
        .map(|row| {
            let mut object = serde_json::Map::new();
            for (index, _) in headers.iter().enumerate() {
                object.insert(
                    format!("c{index}"),
                    json!(row.get(index).map(String::as_str).unwrap_or("")),
                );
            }
            Value::Object(object)
        })
        .collect::<Vec<_>>();
    Some(json!({
        "tag": "table",
        "page_size": rows.len() + 1,
        "columns": columns,
        "rows": row_objects,
    }))
}

async fn map_http_error(response: reqwest::Response, operation: &str) -> AdapterError {
    let status = response.status();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map(Duration::from_secs);
    let body = response.text().await.unwrap_or_default();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return AdapterError::RateLimited {
            retry_after: retry_after.unwrap_or(Duration::from_secs(1)),
        };
    }
    execution_error(
        format!("http_{}", status.as_u16()),
        format!("{operation} returned HTTP {}", status.as_u16()),
        retry_after,
        status.is_server_error()
            || status == reqwest::StatusCode::REQUEST_TIMEOUT
            || !body.is_empty() && status == reqwest::StatusCode::UNAUTHORIZED,
    )
}

/// Verify an ordinary Feishu/Lark webhook signature.
///
/// The signature is SHA-256 over `timestamp + nonce + encrypt_key + body`.
/// Missing fields and mismatches are fail-closed.  Comparison is constant-time
/// with respect to the provided byte lengths and never includes secret values
/// in the returned diagnosis.
pub fn verify_webhook_signature(
    timestamp: Option<&str>,
    nonce: Option<&str>,
    signature: Option<&str>,
    body: &[u8],
    encrypt_key: &str,
) -> Result<(), AdapterError> {
    let timestamp = timestamp
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            execution_error(
                "webhook_signature_missing",
                "Feishu webhook timestamp is missing",
                None,
                false,
            )
        })?;
    let nonce = nonce
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            execution_error(
                "webhook_signature_missing",
                "Feishu webhook nonce is missing",
                None,
                false,
            )
        })?;
    let signature = signature
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            execution_error(
                "webhook_signature_missing",
                "Feishu webhook signature is missing",
                None,
                false,
            )
        })?;
    if encrypt_key.trim().is_empty() {
        return Err(execution_error(
            "webhook_crypto_unconfigured",
            "Feishu webhook encrypt_key is not configured",
            None,
            false,
        ));
    }
    let mut hasher = Sha256::new();
    hasher.update(timestamp.as_bytes());
    hasher.update(nonce.as_bytes());
    hasher.update(encrypt_key.as_bytes());
    hasher.update(body);
    let expected = format!("{:x}", hasher.finalize());
    if !constant_time_eq(expected.as_bytes(), signature.as_bytes()) {
        return Err(execution_error(
            "webhook_signature_invalid",
            "Feishu webhook signature did not verify",
            None,
            false,
        ));
    }
    Ok(())
}

/// Decrypt a Feishu encrypted webhook payload using AES-256-CBC + PKCS#7.
pub fn decrypt_webhook_payload(encoded: &str, encrypt_key: &str) -> Result<Vec<u8>, AdapterError> {
    if encoded.trim().is_empty() || encrypt_key.trim().is_empty() {
        return Err(execution_error(
            "webhook_crypto_invalid",
            "Feishu encrypted webhook payload or key is empty",
            None,
            false,
        ));
    }
    let ciphertext = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|_| {
            execution_error(
                "webhook_crypto_invalid",
                "Feishu encrypted webhook was not valid base64",
                None,
                false,
            )
        })?;
    let digest = Sha256::digest(encrypt_key.as_bytes());
    let mut key = [0_u8; 32];
    key.copy_from_slice(&digest);
    let mut iv = [0_u8; 16];
    iv.copy_from_slice(&digest[..16]);
    aes256_cbc_decrypt(&ciphertext, &key, &iv)
}

/// Parse a Feishu webhook request with plaintext URL verification and signed
/// or encrypted ordinary events.  Ordinary events always require signature
/// headers; only the URL verification challenge may omit them.
pub fn parse_webhook_event(
    body: &[u8],
    timestamp: Option<&str>,
    nonce: Option<&str>,
    signature: Option<&str>,
    config: &FeishuConfig,
) -> Result<Value, AdapterError> {
    let value: Value = serde_json::from_slice(body).map_err(|error| {
        execution_error(
            "webhook_malformed",
            format!("Feishu webhook JSON was malformed: {error}"),
            None,
            false,
        )
    })?;
    if value.get("type").and_then(Value::as_str) == Some("url_verification") {
        if let Some(expected) = value.get("token").and_then(Value::as_str) {
            if !config.verification_token.is_empty() && expected != config.verification_token {
                return Err(execution_error(
                    "webhook_token_invalid",
                    "Feishu URL verification token did not match",
                    None,
                    false,
                ));
            }
        }
        return Ok(value);
    }
    verify_webhook_signature(timestamp, nonce, signature, body, &config.encrypt_key)?;
    if let Some(encrypted) = value.get("encrypt").and_then(Value::as_str) {
        let plaintext = decrypt_webhook_payload(encrypted, &config.encrypt_key)?;
        return serde_json::from_slice(&plaintext).map_err(|error| {
            execution_error(
                "webhook_crypto_invalid",
                format!("Feishu decrypted webhook JSON was malformed: {error}"),
                None,
                false,
            )
        });
    }
    Ok(value)
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = (left.len() ^ right.len()) as u8;
    for index in 0..left.len().max(right.len()) {
        difference |=
            left.get(index).copied().unwrap_or(0) ^ right.get(index).copied().unwrap_or(0);
    }
    difference == 0
}

// Minimal dependency-free AES-256-CBC decryptor for Feishu's webhook crypto
// contract.  The channel crate intentionally has no AES dependency; this
// implementation is limited to decrypting one bounded webhook body and is
// covered by the standard AES-256 test vector below.
fn aes256_cbc_decrypt(
    ciphertext: &[u8],
    key: &[u8; 32],
    iv: &[u8; 16],
) -> Result<Vec<u8>, AdapterError> {
    if ciphertext.is_empty() || ciphertext.len() % 16 != 0 {
        return Err(execution_error(
            "webhook_crypto_invalid",
            "Feishu encrypted webhook has an invalid block length",
            None,
            false,
        ));
    }
    let round_keys = aes256_expand_key(key);
    let mut previous = *iv;
    let mut plaintext = Vec::with_capacity(ciphertext.len());
    for block in ciphertext.chunks_exact(16) {
        let mut state = [0_u8; 16];
        state.copy_from_slice(block);
        aes_inv_cipher(&mut state, &round_keys);
        for (index, value) in state.iter_mut().enumerate() {
            *value ^= previous[index];
        }
        plaintext.extend_from_slice(&state);
        previous.copy_from_slice(block);
    }
    let padding = plaintext.last().copied().unwrap_or(0) as usize;
    if !(1..=16).contains(&padding) || padding > plaintext.len() {
        return Err(execution_error(
            "webhook_crypto_invalid",
            "Feishu encrypted webhook padding was invalid",
            None,
            false,
        ));
    }
    if plaintext[plaintext.len() - padding..]
        .iter()
        .any(|byte| usize::from(*byte) != padding)
    {
        return Err(execution_error(
            "webhook_crypto_invalid",
            "Feishu encrypted webhook padding was invalid",
            None,
            false,
        ));
    }
    plaintext.truncate(plaintext.len() - padding);
    Ok(plaintext)
}

fn aes256_expand_key(key: &[u8; 32]) -> [[u8; 4]; 60] {
    let mut words = [[0_u8; 4]; 60];
    for (index, chunk) in key.chunks_exact(4).enumerate() {
        words[index].copy_from_slice(chunk);
    }
    for index in 8..60 {
        let mut temp = words[index - 1];
        if index % 8 == 0 {
            temp = [temp[1], temp[2], temp[3], temp[0]];
            temp.iter_mut().for_each(|byte| *byte = aes_sbox(*byte));
            temp[0] ^= aes_rcon(index / 8);
        } else if index % 8 == 4 {
            temp.iter_mut().for_each(|byte| *byte = aes_sbox(*byte));
        }
        for (offset, value) in temp.iter().enumerate() {
            words[index][offset] = words[index - 8][offset] ^ value;
        }
    }
    words
}

fn aes_inv_cipher(state: &mut [u8; 16], words: &[[u8; 4]; 60]) {
    aes_add_round_key(state, words, 14);
    for round in (1..14).rev() {
        aes_inv_shift_rows(state);
        state
            .iter_mut()
            .for_each(|byte| *byte = aes_inv_sbox(*byte));
        aes_add_round_key(state, words, round);
        aes_inv_mix_columns(state);
    }
    aes_inv_shift_rows(state);
    state
        .iter_mut()
        .for_each(|byte| *byte = aes_inv_sbox(*byte));
    aes_add_round_key(state, words, 0);
}

fn aes_add_round_key(state: &mut [u8; 16], words: &[[u8; 4]; 60], round: usize) {
    for column in 0..4 {
        for row in 0..4 {
            state[column * 4 + row] ^= words[round * 4 + column][row];
        }
    }
}

fn aes_inv_shift_rows(state: &mut [u8; 16]) {
    let original = *state;
    for row in 0..4 {
        for column in 0..4 {
            state[column * 4 + row] = original[((column + 4 - row) % 4) * 4 + row];
        }
    }
}

fn aes_inv_mix_columns(state: &mut [u8; 16]) {
    for column in state.chunks_exact_mut(4) {
        let [a, b, c, d] = [column[0], column[1], column[2], column[3]];
        column[0] = aes_gf_mul(a, 14) ^ aes_gf_mul(b, 11) ^ aes_gf_mul(c, 13) ^ aes_gf_mul(d, 9);
        column[1] = aes_gf_mul(a, 9) ^ aes_gf_mul(b, 14) ^ aes_gf_mul(c, 11) ^ aes_gf_mul(d, 13);
        column[2] = aes_gf_mul(a, 13) ^ aes_gf_mul(b, 9) ^ aes_gf_mul(c, 14) ^ aes_gf_mul(d, 11);
        column[3] = aes_gf_mul(a, 11) ^ aes_gf_mul(b, 13) ^ aes_gf_mul(c, 9) ^ aes_gf_mul(d, 14);
    }
}

fn aes_gf_mul(mut left: u8, mut right: u8) -> u8 {
    let mut result = 0;
    while right != 0 {
        if right & 1 != 0 {
            result ^= left;
        }
        let high = left & 0x80;
        left <<= 1;
        if high != 0 {
            left ^= 0x1b;
        }
        right >>= 1;
    }
    result
}

fn aes_sbox(value: u8) -> u8 {
    let inverse = if value == 0 {
        0
    } else {
        aes_gf_pow(value, 254)
    };
    inverse
        ^ inverse.rotate_left(1)
        ^ inverse.rotate_left(2)
        ^ inverse.rotate_left(3)
        ^ inverse.rotate_left(4)
        ^ 0x63
}

fn aes_inv_sbox(value: u8) -> u8 {
    (0_u16..=255)
        .find(|candidate| aes_sbox(*candidate as u8) == value)
        .map(|candidate| candidate as u8)
        .unwrap_or(0)
}

fn aes_gf_pow(mut value: u8, mut power: u16) -> u8 {
    let mut result = 1;
    while power > 0 {
        if power & 1 != 0 {
            result = aes_gf_mul(result, value);
        }
        value = aes_gf_mul(value, value);
        power >>= 1;
    }
    result
}

fn aes_rcon(round: usize) -> u8 {
    let mut value = 1;
    for _ in 1..round {
        value = aes_gf_mul(value, 2);
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ChannelAttachmentStore, StoredAttachment};
    use agent_diva_core::channel::{AttachmentRef, FabricKernel, ReactionOperation, TypingState};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    #[derive(Default)]
    struct MemoryStore {
        puts: AtomicUsize,
    }

    #[async_trait]
    impl ChannelAttachmentStore for MemoryStore {
        async fn put(
            &self,
            input: IngressAttachment,
        ) -> Result<AttachmentRef, crate::AttachmentStoreError> {
            self.puts.fetch_add(1, Ordering::SeqCst);
            let digest = format!("{:x}", Sha256::digest(&input.bytes));
            Ok(AttachmentRef {
                uri: format!("sha256:{digest}"),
                media_type: input
                    .declared_mime
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                size_bytes: input.bytes.len() as u64,
                sha256: digest,
                file_name: input.file_name,
            })
        }

        async fn get(
            &self,
            reference: &AttachmentRef,
        ) -> Result<StoredAttachment, crate::AttachmentStoreError> {
            Err(crate::AttachmentStoreError::NotFound {
                uri: reference.uri.clone(),
            })
        }
    }

    fn config() -> FeishuConfig {
        FeishuConfig {
            enabled: true,
            app_id: "app-test".to_string(),
            app_secret: "secret-test".to_string(),
            ..FeishuConfig::default()
        }
    }

    fn services() -> AdapterServices {
        AdapterServices::new(Arc::new(MemoryStore::default()))
    }

    fn event(event_id: &str, message_id: &str, message_type: &str, content: &str) -> Vec<u8> {
        json!({
            "header": {"event_type": "im.message.receive_v1", "event_id": event_id},
            "event": {
                "sender": {"sender_id": {"open_id": "ou-test"}, "sender_type": "user"},
                "message": {
                    "message_id": message_id,
                    "chat_id": "oc-chat",
                    "chat_type": "group",
                    "message_type": message_type,
                    "content": content,
                    "root_id": "om-root"
                }
            }
        })
        .to_string()
        .into_bytes()
    }

    #[test]
    fn capabilities_match_frozen_feishu_matrix() {
        let adapter = FeishuAdapter::new(config(), services());
        assert!(adapter
            .capabilities()
            .supports(ChannelCapability::IngressMarkdown));
        assert!(adapter
            .capabilities()
            .supports(ChannelCapability::EgressCard));
        assert!(adapter
            .capabilities()
            .supports(ChannelCapability::InteractionEdit));
        assert!(adapter
            .capabilities()
            .supports(ChannelCapability::ReliabilityTokenRefresh));
        assert!(!adapter
            .capabilities()
            .supports(ChannelCapability::EgressAudio));
        assert!(!adapter
            .capabilities()
            .supports(ChannelCapability::InteractionReaction));
        assert!(!adapter
            .capabilities()
            .supports(ChannelCapability::ReliabilityResume));
    }

    #[tokio::test]
    async fn unsupported_typing_and_reaction_have_zero_transport_side_effects() {
        let adapter = FeishuAdapter::new(config(), services());
        let address = ChannelAddress::new(FEISHU_CHANNEL, "oc-chat");
        let correlation = Correlation::new("feishu:oc-chat");
        let typing = ChannelCommand::Typing {
            address: address.clone(),
            correlation: correlation.clone(),
            state: TypingState::Started,
            idempotency_key: None,
        };
        assert!(matches!(
            adapter.execute(typing).await,
            Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionTyping
            })
        ));
        let reaction = ChannelCommand::React {
            address,
            correlation,
            target_message_id: "om-1".to_string(),
            operation: ReactionOperation::Add,
            emoji: "THUMBSUP".to_string(),
            idempotency_key: None,
        };
        assert!(matches!(
            adapter.execute(reaction).await,
            Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionReaction
            })
        ));
    }

    #[tokio::test]
    async fn duplicate_events_are_not_admitted_twice() {
        let adapter = FeishuAdapter::new(config(), services());
        let kernel = FabricKernel::new();
        let (handle, mut consumer) = kernel.into_parts();
        let context = AdapterContext {
            fabric: handle,
            cancel: CancellationToken::new(),
        };
        let payload = event("evt-1", "om-1", "text", r#"{"text":"hello"}"#);
        adapter
            .handle_event_payload(&payload, &context)
            .await
            .unwrap();
        let first = tokio::time::timeout(Duration::from_secs(1), consumer.recv_ingress())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(first.envelope().address.chat_id, "oc-chat");
        assert_eq!(
            first.envelope().correlation.message_id.as_deref(),
            Some("om-1")
        );
        adapter
            .handle_event_payload(&payload, &context)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(50), consumer.recv_ingress())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn media_fixture_is_stored_as_typed_attachment_after_authentication() {
        let (base, server) = spawn_http_fixture(vec![
            (
                "/auth/v3/tenant_access_token/internal",
                r#"{"code":0,"tenant_access_token":"token-fixture","expire":7200}"#,
            ),
            (
                "/im/v1/messages/om-fixture-message/resources/file-key-fixture?type=image",
                "fixture-image-bytes",
            ),
            (
                "/im/v1/messages/om-fixture-message/reactions",
                r#"{"code":0}"#,
            ),
        ])
        .await;
        let store = Arc::new(MemoryStore::default());
        let services = AdapterServices::new(store.clone());
        let adapter =
            FeishuAdapter::with_test_endpoint(config(), services, &base, FeishuRegion::China);
        let kernel = FabricKernel::new();
        let (handle, mut consumer) = kernel.into_parts();
        let context = AdapterContext {
            fabric: handle,
            cancel: CancellationToken::new(),
        };
        let payload = event(
            "evt-media-fixture",
            "om-fixture-message",
            "image",
            r#"{"image_key":"file-key-fixture"}"#,
        );
        adapter
            .handle_event_payload(&payload, &context)
            .await
            .unwrap();
        let envelope = tokio::time::timeout(Duration::from_secs(1), consumer.recv_ingress())
            .await
            .unwrap()
            .unwrap()
            .envelope()
            .clone();
        assert!(matches!(
            envelope.payload,
            ChannelPayloadV1::Message { ref parts, .. }
                if matches!(parts.first(), Some(ContentPart::Image { .. }))
        ));
        assert_eq!(store.puts.load(Ordering::SeqCst), 1);
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[0].contains("/auth/v3/tenant_access_token/internal"));
        assert!(requests[1]
            .contains("/im/v1/messages/om-fixture-message/resources/file-key-fixture?type=image"));
        assert!(requests[2].contains("/im/v1/messages/om-fixture-message/reactions"));
    }

    #[tokio::test]
    async fn send_fixture_returns_real_message_id_and_reply_uses_bound_endpoint() {
        let (base, server) = spawn_http_fixture(vec![
            (
                "/auth/v3/tenant_access_token/internal",
                r#"{"code":0,"tenant_access_token":"token-fixture","expire":7200}"#,
            ),
            (
                "/im/v1/messages?receive_id_type=chat_id",
                r#"{"code":0,"data":{"message_id":"om-fixture-sent"}}"#,
            ),
            (
                "/im/v1/messages/om-fixture-parent/reply",
                r#"{"code":0,"data":{"message_id":"om-fixture-reply"}}"#,
            ),
        ])
        .await;
        let adapter =
            FeishuAdapter::with_test_endpoint(config(), services(), &base, FeishuRegion::China);
        let mut address = ChannelAddress::new(FEISHU_CHANNEL, "oc_fixture_group");
        address.thread_id = Some("om-fixture-root".to_string());
        let mut correlation = Correlation::new("feishu:oc_fixture_group");
        let envelope = external_message_envelope(
            address.clone(),
            correlation.clone(),
            vec![ContentPart::Text {
                text: "fixture send".to_string(),
            }],
            None,
            None,
        );
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope,
                idempotency_key: Some("send-fixture".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            receipt.platform_message_id.as_deref(),
            Some("om-fixture-sent")
        );
        correlation.reply_to = Some("om-fixture-parent".to_string());
        let reply = adapter
            .execute(ChannelCommand::Send {
                envelope: external_message_envelope(
                    address,
                    correlation,
                    vec![ContentPart::Markdown {
                        markdown: "**reply**".to_string(),
                    }],
                    None,
                    None,
                ),
                idempotency_key: Some("reply-fixture".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            reply.platform_message_id.as_deref(),
            Some("om-fixture-reply")
        );
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 3);
        assert!(requests[1].starts_with("POST /im/v1/messages?receive_id_type=chat_id"));
        assert!(requests[2].starts_with("POST /im/v1/messages/om-fixture-parent/reply"));
    }

    #[tokio::test]
    async fn send_retries_once_after_401_and_reuses_cached_token() {
        let (base, server) = spawn_http_fixture_with_status(vec![
            (
                "/auth/v3/tenant_access_token/internal",
                200,
                r#"{"code":0,"tenant_access_token":"token-first","expire":7200}"#,
            ),
            (
                "/im/v1/messages?receive_id_type=open_id",
                401,
                r#"{"code":99991663,"msg":"token invalid"}"#,
            ),
            (
                "/auth/v3/tenant_access_token/internal",
                200,
                r#"{"code":0,"tenant_access_token":"token-second","expire":7200}"#,
            ),
            (
                "/im/v1/messages?receive_id_type=open_id",
                200,
                r#"{"code":0,"data":{"message_id":"om-retried"}}"#,
            ),
        ])
        .await;
        let adapter =
            FeishuAdapter::with_test_endpoint(config(), services(), &base, FeishuRegion::China);
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope: external_message_envelope(
                    ChannelAddress::new(FEISHU_CHANNEL, "ou_fixture_user"),
                    Correlation::new("feishu:ou_fixture_user"),
                    vec![ContentPart::Text {
                        text: "retry fixture".to_string(),
                    }],
                    None,
                    None,
                ),
                idempotency_key: Some("retry-fixture".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(receipt.platform_message_id.as_deref(), Some("om-retried"));
        let requests = server.await.unwrap();
        assert_eq!(requests.len(), 4);
        assert!(requests[1]
            .to_ascii_lowercase()
            .contains("authorization: bearer token-first"));
        assert!(requests[3]
            .to_ascii_lowercase()
            .contains("authorization: bearer token-second"));
    }

    #[test]
    fn protocol_and_webhook_fixtures_are_parseable() {
        let event_fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/feishu/protobuf-event.json"
        ))
        .unwrap();
        let payload = event_fixture.to_string();
        let parsed: LarkEvent = serde_json::from_str(&payload).unwrap();
        assert_eq!(parsed.header.event_type, "im.message.receive_v1");
        let frame_fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/feishu/protobuf-frame.json"
        ))
        .unwrap();
        let frame = PbFrame {
            seq_id: 1,
            log_id: 2,
            service: frame_fixture["service"].as_i64().unwrap_or_default() as i32,
            method: frame_fixture["method"].as_i64().unwrap_or_default() as i32,
            headers: vec![
                PbHeader {
                    key: "type".to_string(),
                    value: "event".to_string(),
                },
                PbHeader {
                    key: "message_id".to_string(),
                    value: "frame-fixture-001".to_string(),
                },
            ],
            payload: Some(payload.into_bytes()),
        };
        let decoded = PbFrame::decode(frame.encode_to_vec().as_slice()).unwrap();
        let ack = ack_frame(&decoded);
        assert_eq!(ack.header_value("biz_rt"), "0");
        assert_eq!(
            ack.payload.as_deref(),
            Some(br#"{"code":200,"headers":{},"data":[]}"#.as_slice())
        );
        let mut webhook_config = config();
        webhook_config.verification_token = "fixture-verification-token".to_string();
        let url_body =
            include_bytes!("../../tests/fixtures/c5/feishu/webhook-url-verification.json");
        assert!(parse_webhook_event(url_body, None, None, None, &webhook_config).is_ok());
        let encrypted_fixture: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/feishu/webhook-encrypted-event.json"
        ))
        .unwrap();
        assert!(encrypted_fixture.get("encrypt").is_some());
    }

    async fn spawn_http_fixture(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        spawn_http_fixture_with_status(
            responses
                .into_iter()
                .map(|(path, body)| (path, 200, body))
                .collect(),
        )
        .await
    }

    async fn spawn_http_fixture_with_status(
        responses: Vec<(&'static str, u16, &'static str)>,
    ) -> (String, tokio::task::JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let handle = tokio::spawn(async move {
            let mut requests = Vec::new();
            for (expected_path, status, body) in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buffer = vec![0_u8; 16 * 1024];
                let size = socket.read(&mut buffer).await.unwrap();
                let request = String::from_utf8_lossy(&buffer[..size]).to_string();
                assert!(request
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .contains(expected_path));
                requests.push(request);
                let content_type = if expected_path.contains("type=image") {
                    "image/png"
                } else if expected_path.contains("type=audio") {
                    "audio/mpeg"
                } else if expected_path.contains("type=video") {
                    "video/mp4"
                } else {
                    "application/json"
                };
                let reason = match status {
                    200 => "OK",
                    401 => "Unauthorized",
                    429 => "Too Many Requests",
                    _ => "Fixture",
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
            requests
        });
        (format!("http://{address}"), handle)
    }

    #[test]
    fn webhook_signature_is_fail_closed() {
        let body = br#"{"event":"ok"}"#;
        let timestamp = "1700000000";
        let nonce = "nonce";
        let mut hasher = Sha256::new();
        hasher.update(timestamp.as_bytes());
        hasher.update(nonce.as_bytes());
        hasher.update(b"encrypt-key");
        hasher.update(body);
        let signature = format!("{:x}", hasher.finalize());
        assert!(verify_webhook_signature(
            Some(timestamp),
            Some(nonce),
            Some(&signature),
            body,
            "encrypt-key"
        )
        .is_ok());
        assert!(verify_webhook_signature(
            Some(timestamp),
            Some(nonce),
            Some("bad"),
            body,
            "encrypt-key"
        )
        .is_err());
        assert!(
            verify_webhook_signature(None, Some(nonce), Some(&signature), body, "encrypt-key")
                .is_err()
        );
    }

    #[test]
    fn aes256_decrypt_round_trip_and_bad_padding_fail_closed() {
        assert_eq!(aes_sbox(0x53), 0xed);
        assert_eq!(aes_sbox(0x00), 0x63);
        assert_eq!(aes_sbox(0x09), 0x01);
        assert_eq!(aes_sbox(0x7b), 0x21);
        assert_eq!(aes_sbox(0x7c), 0x10);
        assert_eq!(
            (0_u8..=0x0f).map(aes_sbox).collect::<Vec<_>>(),
            vec![
                0x63, 0x7c, 0x77, 0x7b, 0xf2, 0x6b, 0x6f, 0xc5, 0x30, 0x01, 0x67, 0x2b, 0xfe, 0xd7,
                0xab, 0x76
            ]
        );
        assert_eq!(aes_gf_mul(0x57, 0x13), 0xfe);
        assert_eq!(aes_inv_sbox(0xed), 0x53);
        let nist_key = [
            0x60, 0x3d, 0xeb, 0x10, 0x15, 0xca, 0x71, 0xbe, 0x2b, 0x73, 0xae, 0xf0, 0x85, 0x7d,
            0x77, 0x81, 0x1f, 0x35, 0x2c, 0x07, 0x3b, 0x61, 0x08, 0xd7, 0x2d, 0x98, 0x10, 0xa3,
            0x09, 0x14, 0xdf, 0xf4,
        ];
        let nist_plain = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a,
        ];
        let nist_iv = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f,
        ];
        let nist_words = aes256_expand_key(&nist_key);
        assert_eq!(nist_words[8], [0x9b, 0xa3, 0x54, 0x11]);
        assert_eq!(nist_words[9], [0x8e, 0x69, 0x25, 0xaf]);
        let nist_cipher = aes256_cbc_encrypt_for_test(&nist_plain, &nist_key, &nist_iv);
        assert_eq!(
            nist_cipher[..16],
            [
                0xf5, 0x8c, 0x4c, 0x04, 0xd6, 0xe5, 0xf1, 0xba, 0x77, 0x9e, 0xab, 0xfb, 0x5f, 0x7b,
                0xfb, 0xd6,
            ]
        );
        let block_key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d,
            0x0e, 0x0f, 0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b,
            0x1c, 0x1d, 0x1e, 0x1f,
        ];
        let block_plain = [
            0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd,
            0xee, 0xff,
        ];
        let block_cipher = aes256_cbc_encrypt_for_test(&block_plain, &block_key, &[0_u8; 16]);
        assert_eq!(
            block_cipher[..16],
            [
                0x8e, 0xa2, 0xb7, 0xca, 0x51, 0x67, 0x45, 0xbf, 0xea, 0xfc, 0x49, 0x90, 0x4b, 0x49,
                0x60, 0x89,
            ]
        );
        let key = [0x00_u8; 32];
        let iv = [0x00_u8; 16];
        let plaintext = b"feishu webhook";
        let ciphertext = aes256_cbc_encrypt_for_test(plaintext, &key, &iv);
        assert_eq!(
            aes256_cbc_decrypt(&ciphertext, &key, &iv).unwrap(),
            plaintext
        );
        let mut bad = ciphertext.clone();
        if let Some(last) = bad.last_mut() {
            *last ^= 0xff;
        }
        assert!(aes256_cbc_decrypt(&bad, &key, &iv).is_err());
    }

    #[test]
    fn region_defaults_are_explicit_and_distinct() {
        assert_eq!(
            FeishuRegion::China.api_base(),
            "https://open.feishu.cn/open-apis"
        );
        assert_eq!(
            FeishuRegion::Global.api_base(),
            "https://open.larksuite.com/open-apis"
        );
        assert_eq!(FeishuRegion::Lark.ws_base(), "https://open.larksuite.com");
        let adapter = FeishuAdapter::with_test_endpoint(
            config(),
            services(),
            "http://127.0.0.1:12345/",
            FeishuRegion::Global,
        );
        assert_eq!(adapter.endpoint.api_base, "http://127.0.0.1:12345");
    }

    fn aes256_cbc_encrypt_for_test(plaintext: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> Vec<u8> {
        let padding = 16 - plaintext.len() % 16;
        let mut padded = plaintext.to_vec();
        padded.extend(std::iter::repeat(padding as u8).take(padding));
        let round_keys = aes256_expand_key(key);
        let mut previous = *iv;
        let mut output = Vec::with_capacity(padded.len());
        for block in padded.chunks_exact(16) {
            let mut state = [0_u8; 16];
            for index in 0..16 {
                state[index] = block[index] ^ previous[index];
            }
            aes_cipher_for_test(&mut state, &round_keys);
            output.extend_from_slice(&state);
            previous = state;
        }
        output
    }

    fn aes_cipher_for_test(state: &mut [u8; 16], words: &[[u8; 4]; 60]) {
        aes_add_round_key(state, words, 0);
        for round in 1..14 {
            aes_sub_shift_mix_for_test(state);
            aes_add_round_key(state, words, round);
        }
        aes_sub_shift_for_test(state);
        aes_add_round_key(state, words, 14);
    }

    fn aes_sub_shift_mix_for_test(state: &mut [u8; 16]) {
        state.iter_mut().for_each(|byte| *byte = aes_sbox(*byte));
        let original = *state;
        for row in 0..4 {
            for column in 0..4 {
                state[column * 4 + row] = original[((column + row) % 4) * 4 + row];
            }
        }
        for column in state.chunks_exact_mut(4) {
            let [a, b, c, d] = [column[0], column[1], column[2], column[3]];
            column[0] = aes_gf_mul(a, 2) ^ aes_gf_mul(b, 3) ^ c ^ d;
            column[1] = a ^ aes_gf_mul(b, 2) ^ aes_gf_mul(c, 3) ^ d;
            column[2] = a ^ b ^ aes_gf_mul(c, 2) ^ aes_gf_mul(d, 3);
            column[3] = aes_gf_mul(a, 3) ^ b ^ c ^ aes_gf_mul(d, 2);
        }
    }

    fn aes_sub_shift_for_test(state: &mut [u8; 16]) {
        state.iter_mut().for_each(|byte| *byte = aes_sbox(*byte));
        let original = *state;
        for row in 0..4 {
            for column in 0..4 {
                state[column * 4 + row] = original[((column + row) % 4) * 4 + row];
            }
        }
    }
}
