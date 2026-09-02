//! Native DingTalk Stream/OpenAPI adapter for CHANNEL-EPIC C5.
//!
//! The native adapter deliberately keeps the DIVA Stream connection as the
//! primary ingress path.  The Octos webhook implementation is used only for
//! its independently useful HMAC and short-lived session-webhook semantics;
//! it is never used as a text-only replacement for the Stream/media path.

use crate::adapter::{
    accepted_receipt, execution_error, external_message_envelope, is_sender_allowed,
    AdapterContext, AdapterError, AdapterServices, ChannelAdapter, IngressAttachment,
};
use agent_diva_core::channel::{
    ChannelAddress, ChannelCapabilities, ChannelCapability, ChannelCommand, ChannelEnvelopeV1,
    ChannelHealth, ChannelHealthStatus, ChannelId, ChannelPayloadV1, ContentPart, Correlation,
    DeliveryReceipt, FabricAdmissionError,
};
use agent_diva_core::config::schema::DingTalkConfig;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use tokio_util::sync::CancellationToken;

#[allow(dead_code)]
const DINGTALK_API_BASE: &str = "https://api.dingtalk.com";
#[allow(dead_code)]
const DINGTALK_MEDIA_BASE: &str = "https://oapi.dingtalk.com";
const DINGTALK_STREAM_REGISTER_PATH: &str = "/v1.0/gateway/connections/open";
const DINGTALK_TOKEN_PATH: &str = "/v1.0/oauth2/accessToken";
const DINGTALK_GROUP_SEND_PATH: &str = "/v1.0/robot/groupMessages/send";
const DINGTALK_PRIVATE_SEND_PATH: &str = "/v1.0/robot/oToMessages/batchSend";
const DINGTALK_MEDIA_UPLOAD_PATH: &str = "/media/upload";
const MAX_TEXT_CHARS: usize = 3_600;
const MAX_ATTACHMENT_BYTES: u64 = 20 * 1024 * 1024;
const MAX_DEDUP_ENTRIES: usize = 1_000;
const TOKEN_SAFETY_WINDOW: Duration = Duration::from_secs(60);
const STREAM_ADMISSION_DEADLINE: Duration = Duration::from_secs(2);
const INITIAL_RECONNECT_BACKOFF: Duration = Duration::from_secs(5);
const MAX_RECONNECT_BACKOFF: Duration = Duration::from_secs(60);
const MAX_SESSION_WEBHOOKS: usize = 1_000;

/// Minimal response returned by the private transport seam.
#[derive(Debug, Clone)]
struct HttpResponse {
    status: u16,
    body: Value,
    retry_after: Option<Duration>,
}

#[derive(Debug, Clone)]
struct BytesResponse {
    status: u16,
    bytes: Vec<u8>,
    retry_after: Option<Duration>,
}

#[allow(dead_code)]
#[derive(Debug, Error, Clone)]
enum TransportError {
    #[error("DingTalk transport request failed: {0}")]
    Request(String),
    #[error("DingTalk transport response was invalid: {0}")]
    Decode(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct UploadPart {
    filename: String,
    media_type: Option<String>,
    bytes: Vec<u8>,
}

/// Platform HTTP calls are isolated behind this private seam so tests can
/// assert exact paths/payloads without process-wide endpoint overrides.
#[async_trait]
trait DingTalkHttp: Send + Sync {
    async fn post_json(
        &self,
        path: &str,
        access_token: Option<&str>,
        body: Value,
    ) -> Result<HttpResponse, TransportError>;

    async fn post_multipart(
        &self,
        path: &str,
        access_token: &str,
        media_kind: &str,
        part: UploadPart,
    ) -> Result<HttpResponse, TransportError>;

    async fn get_bytes(
        &self,
        url: &str,
        access_token: Option<&str>,
    ) -> Result<BytesResponse, TransportError>;
}

#[allow(dead_code)]
#[derive(Clone)]
struct ReqwestDingTalkHttp {
    client: reqwest::Client,
    api_base: String,
    media_base: String,
}

#[allow(dead_code)]
impl ReqwestDingTalkHttp {
    fn new(client: reqwest::Client) -> Self {
        Self {
            client,
            api_base: DINGTALK_API_BASE.to_string(),
            media_base: DINGTALK_MEDIA_BASE.to_string(),
        }
    }

    fn url(&self, base: &str, path: &str) -> String {
        format!("{}{}", base.trim_end_matches('/'), path)
    }

    async fn decode_response(response: reqwest::Response) -> Result<HttpResponse, TransportError> {
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs);
        let body = response
            .json::<Value>()
            .await
            .map_err(|error| TransportError::Decode(error.to_string()))?;
        Ok(HttpResponse {
            status,
            body,
            retry_after,
        })
    }
}

#[async_trait]
impl DingTalkHttp for ReqwestDingTalkHttp {
    async fn post_json(
        &self,
        path: &str,
        access_token: Option<&str>,
        body: Value,
    ) -> Result<HttpResponse, TransportError> {
        let mut request = self.client.post(self.url(&self.api_base, path)).json(&body);
        if let Some(token) = access_token {
            request = request.header("x-acs-dingtalk-access-token", token);
        }
        let response = request
            .send()
            .await
            .map_err(|error| TransportError::Request(error.to_string()))?;
        Self::decode_response(response).await
    }

    async fn post_multipart(
        &self,
        path: &str,
        access_token: &str,
        media_kind: &str,
        part: UploadPart,
    ) -> Result<HttpResponse, TransportError> {
        let url = format!(
            "{}{}?access_token={}&type={}",
            self.media_base.trim_end_matches('/'),
            path,
            percent_encode(access_token),
            percent_encode(media_kind)
        );
        let mut file = reqwest::multipart::Part::bytes(part.bytes).file_name(part.filename);
        if let Some(media_type) = part.media_type {
            file = file.mime_str(&media_type).map_err(|error| {
                TransportError::Request(format!("invalid upload MIME: {error}"))
            })?;
        }
        let form = reqwest::multipart::Form::new().part("media", file);
        let response = self
            .client
            .post(url)
            .multipart(form)
            .send()
            .await
            .map_err(|error| TransportError::Request(error.to_string()))?;
        Self::decode_response(response).await
    }

    async fn get_bytes(
        &self,
        url: &str,
        access_token: Option<&str>,
    ) -> Result<BytesResponse, TransportError> {
        let mut request = self.client.get(url);
        if let Some(token) = access_token {
            request = request.header("x-acs-dingtalk-access-token", token);
        }
        let response = request
            .send()
            .await
            .map_err(|error| TransportError::Request(error.to_string()))?;
        let status = response.status().as_u16();
        let retry_after = response
            .headers()
            .get("retry-after")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse::<u64>().ok())
            .map(Duration::from_secs);
        let bytes = response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|error| TransportError::Request(error.to_string()))?;
        Ok(BytesResponse {
            status,
            bytes,
            retry_after,
        })
    }
}

#[derive(Debug, Clone)]
struct CachedToken {
    value: String,
    expires_at: Instant,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct CachedSessionWebhook {
    url: String,
    expires_at: Instant,
}

#[derive(Debug, Default)]
struct DedupState {
    seen: HashSet<String>,
    order: VecDeque<String>,
}

impl DedupState {
    fn contains(&self, id: &str) -> bool {
        self.seen.contains(id)
    }

    fn insert(&mut self, id: String) {
        if self.seen.contains(&id) {
            return;
        }
        if self.order.len() >= MAX_DEDUP_ENTRIES {
            if let Some(old) = self.order.pop_front() {
                self.seen.remove(&old);
            }
        }
        self.seen.insert(id.clone());
        self.order.push_back(id);
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct StreamMessage {
    #[serde(rename = "specVersion", default)]
    spec_version: String,
    #[serde(rename = "type")]
    msg_type: String,
    headers: StreamHeaders,
    data: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct StreamHeaders {
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(default)]
    topic: String,
    #[serde(rename = "contentType", default)]
    content_type: String,
    #[serde(default)]
    time: String,
    #[serde(rename = "appId", default)]
    app_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct StreamResponse {
    code: i32,
    message: String,
    headers: ResponseHeaders,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
}

#[derive(Debug, Serialize)]
struct ResponseHeaders {
    #[serde(rename = "messageId")]
    message_id: String,
    #[serde(rename = "contentType")]
    content_type: String,
}

#[derive(Debug, Clone)]
struct ParsedAttachment {
    kind: AttachmentKind,
    file_name: Option<String>,
    declared_mime: Option<String>,
    url: String,
}

#[derive(Debug, Clone, Copy)]
enum AttachmentKind {
    Image,
    Audio,
    Video,
    File,
}

#[derive(Debug, Clone)]
struct ParsedEvent {
    message_id: String,
    conversation_id: String,
    conversation_type: String,
    sender_id: String,
    sender_name: Option<String>,
    text: String,
    attachments: Vec<ParsedAttachment>,
    session_webhook: Option<String>,
    raw: Value,
}

/// Native DingTalk adapter. The concrete type is public for Registry use;
/// production construction remains crate-private until C6 factory wiring.
pub struct DingTalkAdapter {
    config: DingTalkConfig,
    services: AdapterServices,
    http: Arc<dyn DingTalkHttp>,
    token: Arc<RwLock<Option<CachedToken>>>,
    token_refresh: Arc<tokio::sync::Mutex<()>>,
    processed: Arc<Mutex<DedupState>>,
    session_webhooks: Arc<Mutex<HashMap<String, CachedSessionWebhook>>>,
    health: Arc<Mutex<ChannelHealth>>,
    stopped: CancellationToken,
    reconnect_base: Duration,
}

impl DingTalkAdapter {
    /// Build the production adapter. The factory is intentionally crate-local;
    /// C6 owns registration and Manager service assembly.
    #[allow(dead_code)]
    pub(crate) fn new(config: &DingTalkConfig, services: AdapterServices) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());
        Self::with_http(config, services, Arc::new(ReqwestDingTalkHttp::new(client)))
    }

    #[allow(dead_code)]
    fn with_http(
        config: &DingTalkConfig,
        services: AdapterServices,
        http: Arc<dyn DingTalkHttp>,
    ) -> Self {
        Self {
            config: config.clone(),
            services,
            http,
            token: Arc::new(RwLock::new(None)),
            token_refresh: Arc::new(tokio::sync::Mutex::new(())),
            processed: Arc::new(Mutex::new(DedupState::default())),
            session_webhooks: Arc::new(Mutex::new(HashMap::new())),
            health: Arc::new(Mutex::new(ChannelHealth::new(ChannelHealthStatus::Unknown))),
            stopped: CancellationToken::new(),
            reconnect_base: INITIAL_RECONNECT_BACKOFF,
        }
    }

    fn channel_id() -> ChannelId {
        ChannelId::new("dingtalk").unwrap_or_else(|_| unreachable!("literal channel id is valid"))
    }

    fn is_group_chat(conversation_id: &str, conversation_type: &str) -> bool {
        conversation_type == "2"
            || conversation_type.eq_ignore_ascii_case("group")
            || conversation_id.starts_with("cid")
    }

    fn policy_allows(&self, sender_id: &str, conversation_id: &str, is_group: bool) -> bool {
        let policy = if is_group {
            &self.config.group_policy
        } else {
            &self.config.dm_policy
        };
        match policy.trim().to_ascii_lowercase().as_str() {
            "deny" | "closed" | "disabled" => false,
            "allowlist" => {
                is_sender_allowed(&self.config.allow_from, sender_id)
                    || is_sender_allowed(&self.config.allow_from, conversation_id)
            }
            _ => is_sender_allowed(&self.config.allow_from, sender_id),
        }
    }

    fn set_health(&self, status: ChannelHealthStatus, diagnosis: Option<String>) {
        let mut health = self
            .health
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        health.status = status;
        health.diagnosis = diagnosis;
        health.checked_at = chrono::Utc::now();
        if matches!(status, ChannelHealthStatus::Healthy) {
            health.consecutive_failures = 0;
        } else {
            health.consecutive_failures = health.consecutive_failures.saturating_add(1);
        }
    }

    fn invalidate_token(&self) {
        let mut token = self
            .token
            .write()
            .unwrap_or_else(|error| error.into_inner());
        *token = None;
    }

    async fn access_token(&self) -> Result<String, AdapterError> {
        if self.config.client_id.trim().is_empty() {
            return Err(execution_error(
                "invalid_config",
                "DingTalk client_id is empty",
                None,
                false,
            ));
        }
        if self.config.client_secret.trim().is_empty() {
            return Err(execution_error(
                "invalid_config",
                "DingTalk client_secret is empty",
                None,
                false,
            ));
        }

        {
            let token = self.token.read().unwrap_or_else(|error| error.into_inner());
            if token
                .as_ref()
                .is_some_and(|cached| Instant::now() < cached.expires_at)
            {
                return Ok(token
                    .as_ref()
                    .map(|cached| cached.value.clone())
                    .unwrap_or_default());
            }
        }

        let _guard = self.token_refresh.lock().await;
        {
            let token = self.token.read().unwrap_or_else(|error| error.into_inner());
            if token
                .as_ref()
                .is_some_and(|cached| Instant::now() < cached.expires_at)
            {
                return Ok(token
                    .as_ref()
                    .map(|cached| cached.value.clone())
                    .unwrap_or_default());
            }
        }

        let response = self
            .http
            .post_json(
                DINGTALK_TOKEN_PATH,
                None,
                json!({
                    "appKey": self.config.client_id,
                    "appSecret": self.config.client_secret,
                }),
            )
            .await
            .map_err(|error| execution_error("token_transport", error.to_string(), None, true))?;
        if response.status < 200 || response.status >= 300 {
            return Err(map_http_failure("token", response));
        }
        let value = response
            .body
            .get("accessToken")
            .or_else(|| response.body.get("access_token"))
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                execution_error(
                    "token_invalid",
                    "DingTalk token response has no accessToken",
                    None,
                    false,
                )
            })?
            .to_string();
        let expires_in = response
            .body
            .get("expireIn")
            .or_else(|| response.body.get("expires_in"))
            .and_then(Value::as_u64)
            .unwrap_or(3_600);
        let lifetime = Duration::from_secs(expires_in).saturating_sub(TOKEN_SAFETY_WINDOW);
        let expires_at = Instant::now() + lifetime.max(Duration::from_secs(1));
        let mut token = self
            .token
            .write()
            .unwrap_or_else(|error| error.into_inner());
        *token = Some(CachedToken {
            value: value.clone(),
            expires_at,
        });
        Ok(value)
    }

    async fn authenticated_json(
        &self,
        path: &str,
        body: Value,
    ) -> Result<HttpResponse, AdapterError> {
        let token = self.access_token().await?;
        let mut response = self
            .http
            .post_json(path, Some(&token), body.clone())
            .await
            .map_err(|error| {
                execution_error("dingtalk_transport", error.to_string(), None, true)
            })?;
        if response.status == 401 || response.status == 403 {
            self.invalidate_token();
            let refreshed = self.access_token().await?;
            response = self
                .http
                .post_json(path, Some(&refreshed), body)
                .await
                .map_err(|error| {
                    execution_error("dingtalk_transport", error.to_string(), None, true)
                })?;
        }
        if response.status == 429 {
            return Err(AdapterError::RateLimited {
                retry_after: response.retry_after.unwrap_or(Duration::from_secs(1)),
            });
        }
        if response.status < 200 || response.status >= 300 {
            return Err(map_http_failure(path, response));
        }
        Ok(response)
    }

    async fn authenticated_multipart(
        &self,
        media_kind: &str,
        part: UploadPart,
    ) -> Result<HttpResponse, AdapterError> {
        let token = self.access_token().await?;
        let mut response = self
            .http
            .post_multipart(DINGTALK_MEDIA_UPLOAD_PATH, &token, media_kind, part.clone())
            .await
            .map_err(|error| execution_error("media_transport", error.to_string(), None, true))?;
        if response.status == 401 || response.status == 403 {
            self.invalidate_token();
            let refreshed = self.access_token().await?;
            response = self
                .http
                .post_multipart(DINGTALK_MEDIA_UPLOAD_PATH, &refreshed, media_kind, part)
                .await
                .map_err(|error| {
                    execution_error("media_transport", error.to_string(), None, true)
                })?;
        }
        if response.status == 429 {
            return Err(AdapterError::RateLimited {
                retry_after: response.retry_after.unwrap_or(Duration::from_secs(1)),
            });
        }
        if response.status < 200 || response.status >= 300 {
            return Err(map_http_failure("media_upload", response));
        }
        Ok(response)
    }

    async fn authenticated_bytes(&self, url: &str) -> Result<Vec<u8>, AdapterError> {
        let token = self.access_token().await?;
        let mut response = self
            .http
            .get_bytes(url, Some(&token))
            .await
            .map_err(|error| execution_error("media_transport", error.to_string(), None, true))?;
        if response.status == 401 || response.status == 403 {
            self.invalidate_token();
            let refreshed = self.access_token().await?;
            response = self
                .http
                .get_bytes(url, Some(&refreshed))
                .await
                .map_err(|error| {
                    execution_error("media_transport", error.to_string(), None, true)
                })?;
        }
        if response.status == 429 {
            return Err(AdapterError::RateLimited {
                retry_after: response.retry_after.unwrap_or(Duration::from_secs(1)),
            });
        }
        if response.status < 200 || response.status >= 300 {
            return Err(execution_error(
                format!("media_download_http_{}", response.status),
                format!("DingTalk media download returned HTTP {}", response.status),
                response.retry_after,
                response.status >= 500,
            ));
        }
        Ok(response.bytes)
    }

    async fn register_stream(&self) -> Result<(String, String), AdapterError> {
        let response = self
            .http
            .post_json(
                DINGTALK_STREAM_REGISTER_PATH,
                None,
                json!({
                    "clientId": self.config.client_id,
                    "clientSecret": self.config.client_secret,
                    "localIp": "127.0.0.1",
                    "subscriptions": [
                        {"type": "EVENT", "topic": "*"},
                        {"type": "CALLBACK", "topic": "/v1.0/im/bot/messages/get"}
                    ],
                    "ua": "agent-diva/0.9.9"
                }),
            )
            .await
            .map_err(|error| {
                execution_error("stream_register_transport", error.to_string(), None, true)
            })?;
        if response.status == 429 {
            return Err(AdapterError::RateLimited {
                retry_after: response.retry_after.unwrap_or(Duration::from_secs(1)),
            });
        }
        if response.status < 200 || response.status >= 300 {
            return Err(map_http_failure("stream_register", response));
        }
        let endpoint = response
            .body
            .get("endpoint")
            .and_then(Value::as_str)
            .filter(|value| value.starts_with("ws"))
            .ok_or_else(|| {
                execution_error(
                    "stream_register_invalid",
                    "DingTalk endpoint missing",
                    None,
                    true,
                )
            })?
            .to_string();
        let ticket = response
            .body
            .get("ticket")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                execution_error(
                    "stream_register_invalid",
                    "DingTalk ticket missing",
                    None,
                    true,
                )
            })?
            .to_string();
        Ok((endpoint, ticket))
    }

    fn parse_event(&self, message: &StreamMessage) -> Result<Option<ParsedEvent>, AdapterError> {
        let raw: Value = serde_json::from_str(&message.data).map_err(|error| {
            execution_error("malformed_stream_payload", error.to_string(), None, false)
        })?;
        let message_id = raw
            .get("msgId")
            .or_else(|| raw.get("msgid"))
            .or_else(|| raw.get("messageId"))
            .and_then(string_value)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| message.headers.message_id.clone());
        if message_id.is_empty() {
            return Ok(None);
        }
        let sender_id = raw
            .get("senderStaffId")
            .or_else(|| raw.get("senderId"))
            .or_else(|| raw.get("senderUserId"))
            .or_else(|| raw.get("senderUnionId"))
            .and_then(string_value)
            .unwrap_or_default();
        let conversation_id = raw
            .get("conversationId")
            .or_else(|| raw.get("openConversationId"))
            .and_then(string_value)
            .unwrap_or_default();
        if sender_id.is_empty() || conversation_id.is_empty() {
            return Ok(None);
        }
        let conversation_type = raw
            .get("conversationType")
            .or_else(|| raw.get("conversation_type"))
            .and_then(string_value)
            .unwrap_or_default();
        let is_group = Self::is_group_chat(&conversation_id, &conversation_type);
        if !self.policy_allows(&sender_id, &conversation_id, is_group) {
            return Ok(None);
        }

        let raw_content = raw.get("content").cloned().unwrap_or(Value::Null);
        let text = extract_dingtalk_text(&raw, &raw_content);
        let attachments = parse_attachments(&raw, &raw_content)?;
        let session_webhook = raw
            .get("sessionWebhook")
            .and_then(Value::as_str)
            .filter(|value| is_trusted_dingtalk_url(value))
            .map(ToOwned::to_owned);
        Ok(Some(ParsedEvent {
            message_id,
            conversation_id,
            conversation_type,
            sender_id,
            sender_name: raw
                .get("senderNick")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            text,
            attachments,
            session_webhook,
            raw,
        }))
    }

    async fn admit_event(
        &self,
        event: ParsedEvent,
        context: &AdapterContext,
        update_message_id: &str,
    ) -> Result<(), AdapterError> {
        let is_group = Self::is_group_chat(&event.conversation_id, &event.conversation_type);
        let chat_id = event.conversation_id.clone();
        let mut address = ChannelAddress::new("dingtalk", chat_id.clone());
        address.sender_id = Some(event.sender_id.clone());
        address.account_id = Some(self.config.robot_code.clone()).filter(|value| !value.is_empty());
        let mut correlation = Correlation::new(format!("dingtalk:{chat_id}"));
        correlation.message_id = Some(event.message_id.clone());
        correlation.sequence = None;
        let mut parts = Vec::new();
        if !event.text.trim().is_empty() {
            parts.push(ContentPart::Text {
                text: event.text.clone(),
            });
        }

        let token = if event.attachments.is_empty() {
            None
        } else {
            Some(self.access_token().await?)
        };
        for attachment in event.attachments {
            if token.is_none() {
                return Err(execution_error(
                    "media_auth_missing",
                    "DingTalk media requires an access token",
                    None,
                    true,
                ));
            }
            let bytes = self.authenticated_bytes(&attachment.url).await?;
            if bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
                return Err(execution_error(
                    "attachment_too_large",
                    format!("DingTalk attachment exceeds {MAX_ATTACHMENT_BYTES} bytes"),
                    None,
                    false,
                ));
            }
            let reference = self
                .services
                .attachments
                .put(IngressAttachment {
                    source_channel: "dingtalk".to_string(),
                    platform_message_id: Some(event.message_id.clone()),
                    sender_id: Some(event.sender_id.clone()),
                    file_name: attachment.file_name,
                    declared_mime: attachment.declared_mime,
                    bytes,
                })
                .await
                .map_err(|error| {
                    execution_error("attachment_store", error.to_string(), None, false)
                })?;
            let part = match attachment.kind {
                AttachmentKind::Image => ContentPart::Image {
                    attachment: reference,
                },
                AttachmentKind::Audio => ContentPart::Audio {
                    attachment: reference,
                    transcript: None,
                },
                AttachmentKind::Video => ContentPart::Video {
                    attachment: reference,
                },
                AttachmentKind::File => ContentPart::File {
                    attachment: reference,
                },
            };
            parts.push(part);
        }
        if parts.is_empty() {
            return Ok(());
        }
        let mut envelope = external_message_envelope(address, correlation, parts, None, None);
        envelope.extensions.insert(
            "dingtalk.conversation_type".to_string(),
            json!(event.conversation_type),
        );
        envelope
            .extensions
            .insert("dingtalk.group".to_string(), json!(is_group));
        envelope.extensions.insert(
            "dingtalk.message_type".to_string(),
            json!(event
                .raw
                .get("msgtype")
                .or_else(|| event.raw.get("msgType"))),
        );
        if let Some(name) = event.sender_name {
            envelope
                .extensions
                .insert("dingtalk.sender_name".to_string(), json!(name));
        }
        let session_webhook = event.session_webhook.clone();
        if let Some(url) = session_webhook.as_deref() {
            envelope
                .extensions
                .insert("dingtalk.session_webhook".to_string(), json!(url));
        }
        context
            .fabric
            .admit_ingress(envelope, STREAM_ADMISSION_DEADLINE, &context.cancel)
            .await
            .map_err(|error| map_admission_failure(error, update_message_id))?;
        if let Some(url) = session_webhook {
            self.cache_session_webhook(&chat_id, &url);
        }
        self.mark_processed(update_message_id);
        Ok(())
    }

    fn is_processed(&self, message_id: &str) -> bool {
        self.processed
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .contains(message_id)
    }

    fn mark_processed(&self, message_id: &str) {
        self.processed
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .insert(message_id.to_string());
    }

    fn cache_session_webhook(&self, conversation_id: &str, url: &str) {
        self.cache_session_webhook_with_ttl(conversation_id, url, Duration::from_secs(3_600));
    }

    fn cache_session_webhook_with_ttl(&self, conversation_id: &str, url: &str, ttl: Duration) {
        if conversation_id.is_empty() || !is_trusted_dingtalk_url(url) {
            return;
        }
        let mut cache = self
            .session_webhooks
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if cache.len() >= MAX_SESSION_WEBHOOKS && !cache.contains_key(conversation_id) {
            if let Some(oldest) = cache.keys().next().cloned() {
                cache.remove(&oldest);
            }
        }
        cache.insert(
            conversation_id.to_string(),
            CachedSessionWebhook {
                url: url.to_string(),
                expires_at: Instant::now() + ttl,
            },
        );
    }

    fn cached_session_webhook(&self, conversation_id: &str) -> Option<String> {
        let mut cache = self
            .session_webhooks
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        if cache
            .get(conversation_id)
            .is_some_and(|entry| Instant::now() >= entry.expires_at)
        {
            cache.remove(conversation_id);
        }
        cache.get(conversation_id).map(|entry| entry.url.clone())
    }

    async fn send_text(
        &self,
        address: &ChannelAddress,
        correlation: &Correlation,
        text: &str,
    ) -> Result<DeliveryReceipt, AdapterError> {
        if text.chars().count() > MAX_TEXT_CHARS {
            return Err(execution_error(
                "text_too_long",
                format!("DingTalk text exceeds {MAX_TEXT_CHARS} characters"),
                None,
                false,
            ));
        }
        let _cached_session_webhook = self.cached_session_webhook(&address.chat_id);
        let is_group = address.chat_id.starts_with("cid")
            || address
                .thread_id
                .as_deref()
                .is_some_and(|thread| thread.starts_with("cid"));
        let robot_code = if self.config.robot_code.trim().is_empty() {
            self.config.client_id.clone()
        } else {
            self.config.robot_code.clone()
        };
        let msg_param = json!({"text": text, "title": "agent-diva reply"}).to_string();
        let body = if is_group {
            json!({
                "robotCode": robot_code,
                "openConversationId": address.chat_id,
                "msgKey": "sampleMarkdown",
                "msgParam": msg_param,
            })
        } else {
            json!({
                "robotCode": robot_code,
                "userIds": [address.chat_id],
                "msgKey": "sampleMarkdown",
                "msgParam": msg_param,
            })
        };
        let path = if is_group {
            DINGTALK_GROUP_SEND_PATH
        } else {
            DINGTALK_PRIVATE_SEND_PATH
        };
        let response = self.authenticated_json(path, body).await?;
        let platform_id = response_message_id(&response.body);
        Ok(accepted_receipt(
            "dingtalk",
            address.chat_id.clone(),
            platform_id,
            address
                .thread_id
                .clone()
                .or_else(|| correlation.reply_to.clone()),
        ))
    }

    async fn send_attachment(
        &self,
        address: &ChannelAddress,
        attachment: &agent_diva_core::channel::AttachmentRef,
        kind: AttachmentKind,
    ) -> Result<DeliveryReceipt, AdapterError> {
        if attachment.size_bytes > MAX_ATTACHMENT_BYTES {
            return Err(execution_error(
                "attachment_too_large",
                format!("attachment exceeds {MAX_ATTACHMENT_BYTES} bytes"),
                None,
                false,
            ));
        }
        let stored = self
            .services
            .attachments
            .get(attachment)
            .await
            .map_err(|error| execution_error("attachment_store", error.to_string(), None, false))?;
        stored.validate().map_err(|error| {
            execution_error("attachment_invalid", error.to_string(), None, false)
        })?;
        if stored.bytes.len() as u64 != attachment.size_bytes {
            return Err(execution_error(
                "attachment_invalid",
                "stored attachment size does not match reference",
                None,
                false,
            ));
        }
        let media_kind = match kind {
            AttachmentKind::Image => "image",
            AttachmentKind::Audio => "voice",
            AttachmentKind::Video | AttachmentKind::File => "file",
        };
        let filename = stored
            .reference
            .file_name
            .clone()
            .unwrap_or_else(|| "attachment.bin".to_string());
        let upload_response = self
            .authenticated_multipart(
                media_kind,
                UploadPart {
                    filename: filename.clone(),
                    media_type: Some(stored.reference.media_type.clone()),
                    bytes: stored.bytes,
                },
            )
            .await?;
        let media_id = upload_response
            .body
            .get("media_id")
            .or_else(|| upload_response.body.get("mediaId"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                execution_error(
                    "media_upload_invalid",
                    "DingTalk response has no media_id",
                    None,
                    false,
                )
            })?;
        let is_group = address.chat_id.starts_with("cid");
        let robot_code = if self.config.robot_code.trim().is_empty() {
            self.config.client_id.clone()
        } else {
            self.config.robot_code.clone()
        };
        let (msg_key, msg_param) = match kind {
            AttachmentKind::Image => ("sampleImageMsg", json!({"photoURL": media_id}).to_string()),
            AttachmentKind::Audio => (
                "sampleFileMsg",
                json!({"mediaId": media_id, "fileName": filename, "fileType": "voice"}).to_string(),
            ),
            AttachmentKind::Video => (
                "sampleFileMsg",
                json!({"mediaId": media_id, "fileName": filename, "fileType": "video"}).to_string(),
            ),
            AttachmentKind::File => (
                "sampleFileMsg",
                json!({"mediaId": media_id, "fileName": filename, "fileType": "file"}).to_string(),
            ),
        };
        let body = if is_group {
            json!({"robotCode": robot_code, "openConversationId": address.chat_id, "msgKey": msg_key, "msgParam": msg_param})
        } else {
            json!({"robotCode": robot_code, "userIds": [address.chat_id], "msgKey": msg_key, "msgParam": msg_param})
        };
        let path = if is_group {
            DINGTALK_GROUP_SEND_PATH
        } else {
            DINGTALK_PRIVATE_SEND_PATH
        };
        let response = self.authenticated_json(path, body).await?;
        Ok(accepted_receipt(
            "dingtalk",
            address.chat_id.clone(),
            response_message_id(&response.body),
            address.thread_id.clone(),
        ))
    }

    async fn execute_send(
        &self,
        envelope: ChannelEnvelopeV1,
    ) -> Result<DeliveryReceipt, AdapterError> {
        let (ChannelPayloadV1::Message { parts, .. } | ChannelPayloadV1::Stream { parts, .. }) =
            envelope.payload
        else {
            return Err(execution_error(
                "invalid_command",
                "DingTalk only sends message parts",
                None,
                false,
            ));
        };
        let address = envelope.address;
        let correlation = envelope.correlation;
        let mut receipt = None;
        let mut text_parts = Vec::new();
        for part in parts {
            match part {
                ContentPart::Text { text } => text_parts.push(text),
                ContentPart::Markdown { markdown } => text_parts.push(markdown),
                ContentPart::Location {
                    latitude,
                    longitude,
                    label,
                } => text_parts.push(label.map_or_else(
                    || format!("{latitude},{longitude}"),
                    |label| format!("{label} ({latitude},{longitude})"),
                )),
                ContentPart::Reference { uri, title, .. } => {
                    text_parts.push(title.map_or(uri.clone(), |title| format!("{title}: {uri}")))
                }
                ContentPart::Image { attachment } => {
                    if !text_parts.is_empty() {
                        let text = text_parts.join("\n");
                        let sent = self.send_text(&address, &correlation, &text).await;
                        let sent = sent.map_err(|error| map_partial_send_error(&receipt, error))?;
                        receipt = Some(sent);
                        text_parts.clear();
                    }
                    let sent = self
                        .send_attachment(&address, &attachment, AttachmentKind::Image)
                        .await
                        .map_err(|error| map_partial_send_error(&receipt, error))?;
                    receipt = Some(sent);
                }
                ContentPart::Audio { attachment, .. } => {
                    if !text_parts.is_empty() {
                        let text = text_parts.join("\n");
                        let sent = self.send_text(&address, &correlation, &text).await;
                        let sent = sent.map_err(|error| map_partial_send_error(&receipt, error))?;
                        receipt = Some(sent);
                        text_parts.clear();
                    }
                    let sent = self
                        .send_attachment(&address, &attachment, AttachmentKind::Audio)
                        .await
                        .map_err(|error| map_partial_send_error(&receipt, error))?;
                    receipt = Some(sent);
                }
                ContentPart::Video { attachment } => {
                    if !text_parts.is_empty() {
                        let text = text_parts.join("\n");
                        let sent = self.send_text(&address, &correlation, &text).await;
                        let sent = sent.map_err(|error| map_partial_send_error(&receipt, error))?;
                        receipt = Some(sent);
                        text_parts.clear();
                    }
                    let sent = self
                        .send_attachment(&address, &attachment, AttachmentKind::Video)
                        .await
                        .map_err(|error| map_partial_send_error(&receipt, error))?;
                    receipt = Some(sent);
                }
                ContentPart::File { attachment } => {
                    if !text_parts.is_empty() {
                        let text = text_parts.join("\n");
                        let sent = self.send_text(&address, &correlation, &text).await;
                        let sent = sent.map_err(|error| map_partial_send_error(&receipt, error))?;
                        receipt = Some(sent);
                        text_parts.clear();
                    }
                    let sent = self
                        .send_attachment(&address, &attachment, AttachmentKind::File)
                        .await
                        .map_err(|error| map_partial_send_error(&receipt, error))?;
                    receipt = Some(sent);
                }
                ContentPart::Card { .. } => {
                    return Err(AdapterError::UnsupportedCapability {
                        capability: ChannelCapability::EgressCard,
                    });
                }
            }
        }
        if !text_parts.is_empty() {
            let sent = self
                .send_text(&address, &correlation, &text_parts.join("\n"))
                .await
                .map_err(|error| map_partial_send_error(&receipt, error))?;
            receipt = Some(sent);
        }
        receipt.ok_or_else(|| {
            execution_error(
                "empty_message",
                "DingTalk message has no sendable parts",
                None,
                false,
            )
        })
    }

    async fn handle_stream_message(
        &self,
        message: StreamMessage,
        context: &AdapterContext,
    ) -> Result<StreamOutcome, AdapterError> {
        if message.msg_type.eq_ignore_ascii_case("SYSTEM") {
            return Ok(StreamOutcome::Ack);
        }
        if !message.msg_type.eq_ignore_ascii_case("CALLBACK")
            || (!message.headers.topic.is_empty()
                && message.headers.topic != "/v1.0/im/bot/messages/get")
        {
            return Ok(StreamOutcome::Ack);
        }
        if self.is_processed(&message.headers.message_id) {
            return Ok(StreamOutcome::Ack);
        }
        let Some(event) = self.parse_event(&message)? else {
            self.mark_processed(&message.headers.message_id);
            return Ok(StreamOutcome::Ack);
        };
        if self.is_processed(&event.message_id) {
            return Ok(StreamOutcome::Ack);
        }
        let event_id = event.message_id.clone();
        self.admit_event(event, context, &message.headers.message_id)
            .await?;
        self.mark_processed(&event_id);
        Ok(StreamOutcome::Ack)
    }

    async fn run_stream_once(
        &self,
        endpoint: String,
        ticket: String,
        context: &AdapterContext,
    ) -> Result<(), AdapterError> {
        let separator = if endpoint.contains('?') { '&' } else { '?' };
        let ws_url = format!("{endpoint}{separator}ticket={}", percent_encode(&ticket));
        let connected = tokio::select! {
            biased;
            _ = context.cancel.cancelled() => return Ok(()),
            _ = self.stopped.cancelled() => return Ok(()),
            result = connect_async(&ws_url) => result,
        };
        let (socket, _) = connected
            .map_err(|error| execution_error("stream_connect", error.to_string(), None, true))?;
        self.set_health(ChannelHealthStatus::Healthy, None);
        let (mut writer, mut reader) = socket.split();
        loop {
            let incoming = tokio::select! {
                biased;
                _ = context.cancel.cancelled() => return Ok(()),
                _ = self.stopped.cancelled() => return Ok(()),
                message = reader.next() => message,
            };
            let Some(incoming) = incoming else {
                return Err(execution_error(
                    "stream_closed",
                    "DingTalk Stream closed",
                    None,
                    true,
                ));
            };
            match incoming
                .map_err(|error| execution_error("stream_read", error.to_string(), None, true))?
            {
                WsMessage::Text(text) => {
                    let parsed: StreamMessage = serde_json::from_str(&text).map_err(|error| {
                        execution_error("malformed_stream_frame", error.to_string(), None, false)
                    })?;
                    match self.handle_stream_message(parsed, context).await {
                        Ok(StreamOutcome::Ack) => {
                            let response = StreamResponse {
                                code: 200,
                                message: "OK".to_string(),
                                headers: ResponseHeaders {
                                    message_id: extract_message_id_from_frame(&text),
                                    content_type: "application/json".to_string(),
                                },
                                data: Some("{}".to_string()),
                            };
                            let encoded = serde_json::to_string(&response).map_err(|error| {
                                execution_error("stream_ack_encode", error.to_string(), None, false)
                            })?;
                            writer
                                .send(WsMessage::Text(encoded))
                                .await
                                .map_err(|error| {
                                    execution_error(
                                        "stream_ack_send",
                                        error.to_string(),
                                        None,
                                        true,
                                    )
                                })?;
                        }
                        Err(error) => return Err(error),
                    }
                }
                WsMessage::Ping(payload) => {
                    writer
                        .send(WsMessage::Pong(payload))
                        .await
                        .map_err(|error| {
                            execution_error("stream_pong", error.to_string(), None, true)
                        })?;
                }
                WsMessage::Close(_) => {
                    return Err(execution_error(
                        "stream_closed",
                        "DingTalk requested close",
                        None,
                        true,
                    ));
                }
                WsMessage::Binary(_) | WsMessage::Pong(_) | WsMessage::Frame(_) => {}
            }
        }
    }

    async fn wait_backoff(&self, duration: Duration, context: &AdapterContext) -> bool {
        tokio::select! {
            _ = context.cancel.cancelled() => false,
            _ = self.stopped.cancelled() => false,
            _ = sleep(duration) => true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum StreamOutcome {
    Ack,
}

#[async_trait]
impl ChannelAdapter for DingTalkAdapter {
    fn name(&self) -> ChannelId {
        Self::channel_id()
    }

    fn capabilities(&self) -> ChannelCapabilities {
        let mut capabilities = ChannelCapabilities::new([
            ChannelCapability::IngressText,
            ChannelCapability::IngressMarkdown,
            ChannelCapability::IngressGroup,
            ChannelCapability::IngressDirect,
            ChannelCapability::IngressTypedAttachments,
            ChannelCapability::IngressDedupId,
            ChannelCapability::EgressText,
            ChannelCapability::EgressMarkdown,
            ChannelCapability::EgressImage,
            ChannelCapability::EgressAudio,
            ChannelCapability::EgressVideo,
            ChannelCapability::EgressFile,
            ChannelCapability::ReliabilityHealth,
            ChannelCapability::ReliabilityHeartbeat,
            ChannelCapability::ReliabilityTokenRefresh,
            ChannelCapability::ReliabilityPacing,
            ChannelCapability::ReliabilitySupervisedRestart,
        ]);
        capabilities.limits.max_text_chars = Some(MAX_TEXT_CHARS);
        capabilities.limits.max_attachment_bytes = Some(MAX_ATTACHMENT_BYTES);
        capabilities.limits.supported_mime_types = [
            "image/jpeg",
            "image/png",
            "image/gif",
            "audio/mpeg",
            "audio/wav",
            "audio/amr",
            "video/mp4",
            "application/octet-stream",
        ]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect();
        capabilities
    }

    async fn start(&self, context: AdapterContext) -> Result<(), AdapterError> {
        let mut backoff = self.reconnect_base;
        loop {
            if context.cancel.is_cancelled() || self.stopped.is_cancelled() {
                return Ok(());
            }
            let registration = tokio::select! {
                biased;
                _ = context.cancel.cancelled() => return Ok(()),
                _ = self.stopped.cancelled() => return Ok(()),
                result = self.register_stream() => result,
            };
            let (endpoint, ticket) = match registration {
                Ok(value) => {
                    backoff = self.reconnect_base;
                    value
                }
                Err(error) => {
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                    let wait = error
                        .retry_after()
                        .unwrap_or(backoff)
                        .min(MAX_RECONNECT_BACKOFF);
                    if !self.wait_backoff(wait, &context).await {
                        return Ok(());
                    }
                    backoff = backoff.saturating_mul(2).min(MAX_RECONNECT_BACKOFF);
                    continue;
                }
            };
            match self.run_stream_once(endpoint, ticket, &context).await {
                Ok(()) => {
                    if !self.stopped.is_cancelled() {
                        self.set_health(ChannelHealthStatus::Healthy, None);
                    }
                    return Ok(());
                }
                Err(error) => {
                    self.set_health(ChannelHealthStatus::Degraded, Some(error.to_string()));
                    let wait = error
                        .retry_after()
                        .unwrap_or(backoff)
                        .min(MAX_RECONNECT_BACKOFF);
                    if !self.wait_backoff(wait, &context).await {
                        return Ok(());
                    }
                    backoff = backoff.saturating_mul(2).min(MAX_RECONNECT_BACKOFF);
                }
            }
        }
    }

    async fn execute(&self, command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError> {
        let target = command
            .target_channel()
            .map_err(|error| execution_error("invalid_channel", error.to_string(), None, false))?;
        if target != Self::channel_id() {
            return Err(execution_error(
                "wrong_channel",
                "DingTalk adapter received another channel",
                None,
                false,
            ));
        }
        let required = command.required_capabilities();
        let capabilities = self.capabilities();
        if let Some(capability) = required
            .into_iter()
            .find(|capability| !capabilities.supports(*capability))
        {
            return Err(AdapterError::UnsupportedCapability { capability });
        }
        if let ChannelCommand::Send { envelope, .. } = &command {
            if envelope.correlation.reply_to.is_some() {
                return Err(AdapterError::UnsupportedCapability {
                    capability: ChannelCapability::EgressReply,
                });
            }
        }
        match command {
            ChannelCommand::Send {
                envelope,
                idempotency_key,
            } => {
                // DingTalk exposes no documented idempotency field or header for
                // these endpoints. Keep the runtime key local and do not claim
                // that an outbound retry is safe; only an auth refresh retry is
                // performed after a 401/403 response.
                let _ = idempotency_key;
                self.execute_send(envelope).await
            }
            ChannelCommand::ProbeHealth { channel } => {
                if channel != Self::channel_id() {
                    return Err(execution_error(
                        "wrong_channel",
                        "DingTalk adapter received another channel",
                        None,
                        false,
                    ));
                }
                let response = self.authenticated_json("/v1.0/robot/info", json!({})).await;
                match response {
                    Ok(_) => {
                        self.set_health(ChannelHealthStatus::Healthy, None);
                        Ok(accepted_receipt("dingtalk", "", None, None))
                    }
                    Err(error) => {
                        self.set_health(ChannelHealthStatus::Down, Some(error.to_string()));
                        Err(error)
                    }
                }
            }
            ChannelCommand::Typing { .. }
            | ChannelCommand::Edit { .. }
            | ChannelCommand::Delete { .. }
            | ChannelCommand::React { .. }
            | ChannelCommand::FinalizeStream { .. } => Err(AdapterError::UnsupportedCapability {
                capability: command
                    .required_capabilities()
                    .into_iter()
                    .next()
                    .unwrap_or(ChannelCapability::InteractionTyping),
            }),
        }
    }

    fn health(&self) -> ChannelHealth {
        self.health
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    async fn stop(&self) -> Result<(), AdapterError> {
        self.stopped.cancel();
        self.set_health(
            ChannelHealthStatus::Down,
            Some("DingTalk adapter stopped".to_string()),
        );
        Ok(())
    }
}

fn map_partial_send_error(previous: &Option<DeliveryReceipt>, error: AdapterError) -> AdapterError {
    if previous.is_none() {
        return error;
    }
    execution_error(
        "partial_delivery",
        format!("DingTalk send stopped after an earlier part was accepted: {error}"),
        error.retry_after(),
        error.is_retryable(),
    )
}

fn map_http_failure(operation: &str, response: HttpResponse) -> AdapterError {
    let diagnosis = response
        .body
        .get("message")
        .or_else(|| response.body.get("code"))
        .and_then(Value::as_str)
        .unwrap_or("DingTalk API request failed")
        .to_string();
    execution_error(
        format!("{operation}_http_{}", response.status),
        diagnosis,
        response.retry_after,
        response.status >= 500,
    )
}

fn map_admission_failure(error: FabricAdmissionError, message_id: &str) -> AdapterError {
    match error {
        FabricAdmissionError::Busy { retry_after, .. } => AdapterError::Execution {
            code: "fabric_busy".to_string(),
            diagnosis: format!("DingTalk event {message_id} was not ACKed: Fabric busy"),
            retry_after: Some(retry_after),
            retryable: true,
        },
        FabricAdmissionError::Cancelled { .. } => AdapterError::Stopped,
        FabricAdmissionError::Closed { .. } => AdapterError::Execution {
            code: "fabric_closed".to_string(),
            diagnosis: "DingTalk event was not ACKed: Fabric closed".to_string(),
            retry_after: None,
            retryable: true,
        },
        FabricAdmissionError::InvalidEnvelope(error) => execution_error(
            "invalid_envelope",
            format!("DingTalk event {message_id}: {error}"),
            None,
            false,
        ),
    }
}

fn string_value(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(ToOwned::to_owned)
        .or_else(|| value.as_i64().map(|value| value.to_string()))
}

fn is_trusted_dingtalk_url(value: &str) -> bool {
    let Ok(url) = reqwest::Url::parse(value) else {
        return false;
    };
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| host == "dingtalk.com" || host.ends_with(".dingtalk.com"))
}

fn response_message_id(body: &Value) -> Option<String> {
    let direct = [
        "messageId",
        "message_id",
        "msgId",
        "processQueryKey",
        "taskId",
    ]
    .into_iter()
    .find_map(|key| body.get(key).and_then(string_value));
    direct.or_else(|| body.get("result").and_then(response_message_id))
}

fn extract_dingtalk_text(raw: &Value, content: &Value) -> String {
    if let Some(text) = raw
        .get("text")
        .and_then(|value| value.get("content"))
        .and_then(Value::as_str)
    {
        return text.trim().to_string();
    }
    if let Some(text) = content.get("text").and_then(Value::as_str) {
        return text.trim().to_string();
    }
    if let Some(items) = content.get("richText").and_then(Value::as_array) {
        let joined = items
            .iter()
            .filter_map(|item| item.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("");
        if !joined.trim().is_empty() {
            return joined.trim().to_string();
        }
    }
    content
        .get("recognition")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn parse_attachments(raw: &Value, content: &Value) -> Result<Vec<ParsedAttachment>, AdapterError> {
    let mut attachments = Vec::new();
    let candidates = [
        ("image", AttachmentKind::Image),
        ("picture", AttachmentKind::Image),
        ("audio", AttachmentKind::Audio),
        ("voice", AttachmentKind::Audio),
        ("video", AttachmentKind::Video),
        ("file", AttachmentKind::File),
    ];
    for (key, kind) in candidates {
        let Some(value) = content.get(key).or_else(|| raw.get(key)) else {
            continue;
        };
        let object = if value.is_object() {
            value
        } else {
            continue;
        };
        let url = object
            .get("downloadUrl")
            .or_else(|| object.get("download_url"))
            .or_else(|| object.get("url"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if url.is_empty() {
            return Err(execution_error(
                "media_path_missing",
                format!("DingTalk {key} event has no authenticated download URL"),
                None,
                false,
            ));
        }
        if !is_trusted_dingtalk_url(url) {
            return Err(execution_error(
                "media_url_rejected",
                "DingTalk media URL must use HTTPS on a DingTalk host",
                None,
                false,
            ));
        }
        attachments.push(ParsedAttachment {
            kind,
            file_name: object
                .get("fileName")
                .or_else(|| object.get("file_name"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            declared_mime: object
                .get("mimeType")
                .or_else(|| object.get("mime_type"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            url: url.to_string(),
        });
    }
    Ok(attachments)
}

fn extract_message_id_from_frame(frame: &str) -> String {
    serde_json::from_str::<Value>(frame)
        .ok()
        .and_then(|value| value.get("headers").cloned())
        .and_then(|headers| {
            headers
                .get("messageId")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned)
        })
        .unwrap_or_default()
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

/// Octos-compatible HMAC-SHA256 signature for the optional session webhook.
pub fn dingtalk_signature(timestamp: &str, secret: &str) -> String {
    let mut key = secret.as_bytes().to_vec();
    if key.len() > 64 {
        key = Sha256::digest(&key).to_vec();
    }
    key.resize(64, 0);
    let mut inner = vec![0x36_u8; 64];
    let mut outer = vec![0x5c_u8; 64];
    for (index, byte) in key.iter().enumerate() {
        inner[index] ^= byte;
        outer[index] ^= byte;
    }
    let mut inner_input = inner;
    inner_input.extend_from_slice(format!("{timestamp}\n{secret}").as_bytes());
    let inner_hash = Sha256::digest(&inner_input);
    let mut outer_input = outer;
    outer_input.extend_from_slice(&inner_hash);
    BASE64.encode(Sha256::digest(&outer_input))
}

/// Verify a signature without exposing secret material in diagnostics.
pub fn verify_dingtalk_signature(secret: &str, timestamp: &str, signature: &str) -> bool {
    if timestamp.is_empty() || signature.is_empty() {
        return false;
    }
    let expected = dingtalk_signature(timestamp, secret);
    constant_time_equal(expected.as_bytes(), signature.as_bytes())
}

fn constant_time_equal(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        difference |= usize::from(left.get(index).copied().unwrap_or_default())
            ^ usize::from(right.get(index).copied().unwrap_or_default());
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::channel::{
        AttachmentRef, ChannelDirection, ChannelEnvelopeV1, ChannelOrigin, ChannelPayloadV1,
        FabricConsumer,
    };
    use async_trait::async_trait;
    use futures::{SinkExt, StreamExt};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::net::TcpListener;
    use tokio::sync::oneshot;
    use tokio_tungstenite::accept_async;

    #[derive(Default)]
    struct TestStore {
        puts: AtomicUsize,
    }

    #[async_trait]
    impl crate::adapter::ChannelAttachmentStore for TestStore {
        async fn put(
            &self,
            input: IngressAttachment,
        ) -> Result<AttachmentRef, crate::adapter::AttachmentStoreError> {
            self.puts.fetch_add(1, Ordering::AcqRel);
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
        ) -> Result<crate::adapter::StoredAttachment, crate::adapter::AttachmentStoreError>
        {
            Ok(crate::adapter::StoredAttachment {
                reference: reference.clone(),
                bytes: vec![1, 2, 3],
            })
        }
    }

    #[derive(Default)]
    struct FakeHttp {
        json_calls: Mutex<Vec<(String, Option<String>, Value)>>,
        multipart_calls: Mutex<Vec<(String, String, String, UploadPart)>>,
        json_responses: Mutex<VecDeque<HttpResponse>>,
        downloads: Mutex<VecDeque<BytesResponse>>,
        download_calls: Mutex<Vec<(String, Option<String>)>>,
        json_call_times: Mutex<Vec<(String, Instant)>>,
    }

    #[async_trait]
    impl DingTalkHttp for FakeHttp {
        async fn post_json(
            &self,
            path: &str,
            access_token: Option<&str>,
            body: Value,
        ) -> Result<HttpResponse, TransportError> {
            self.json_call_times
                .lock()
                .unwrap()
                .push((path.to_string(), Instant::now()));
            self.json_calls.lock().unwrap().push((
                path.to_string(),
                access_token.map(ToOwned::to_owned),
                body,
            ));
            self.json_responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| TransportError::Request("no scripted response".to_string()))
        }

        async fn post_multipart(
            &self,
            path: &str,
            access_token: &str,
            media_kind: &str,
            part: UploadPart,
        ) -> Result<HttpResponse, TransportError> {
            self.multipart_calls.lock().unwrap().push((
                path.to_string(),
                access_token.to_string(),
                media_kind.to_string(),
                part,
            ));
            self.json_responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| TransportError::Request("no scripted response".to_string()))
        }

        async fn get_bytes(
            &self,
            url: &str,
            access_token: Option<&str>,
        ) -> Result<BytesResponse, TransportError> {
            self.download_calls
                .lock()
                .unwrap()
                .push((url.to_string(), access_token.map(ToOwned::to_owned)));
            self.downloads
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| TransportError::Request("no scripted download".to_string()))
        }
    }

    fn config() -> DingTalkConfig {
        DingTalkConfig {
            enabled: true,
            client_id: "client".to_string(),
            client_secret: "secret".to_string(),
            robot_code: "robot".to_string(),
            dm_policy: "open".to_string(),
            group_policy: "open".to_string(),
            allow_from: Vec::new(),
        }
    }

    fn adapter(fake: Arc<FakeHttp>) -> DingTalkAdapter {
        DingTalkAdapter::with_http(
            &config(),
            AdapterServices::new(Arc::new(TestStore::default())),
            fake,
        )
    }

    fn text_envelope(chat_id: &str, text: &str) -> ChannelEnvelopeV1 {
        let mut address = ChannelAddress::new("dingtalk", chat_id);
        address.sender_id = Some("sender".to_string());
        ChannelEnvelopeV1::new(
            ChannelDirection::Egress,
            address,
            Correlation::new(format!("dingtalk:{chat_id}")),
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Message {
                parts: vec![ContentPart::Text {
                    text: text.to_string(),
                }],
                subject: None,
                locale: None,
                context: None,
            },
        )
    }

    fn fixture_attachment() -> AttachmentRef {
        let digest = format!("{:x}", Sha256::digest([1_u8, 2, 3]));
        AttachmentRef {
            uri: format!("sha256:{digest}"),
            media_type: "image/png".to_string(),
            size_bytes: 3,
            sha256: digest,
            file_name: Some("fixture.png".to_string()),
        }
    }

    fn fixture_attachment_with(media_type: &str, file_name: &str) -> AttachmentRef {
        let mut attachment = fixture_attachment();
        attachment.media_type = media_type.to_string();
        attachment.file_name = Some(file_name.to_string());
        attachment
    }

    fn fixture_value(path: &str) -> Value {
        match path {
            "token-response.json" => serde_json::from_str(include_str!(
                "../../tests/fixtures/c5/dingtalk/token-response.json"
            ))
            .unwrap(),
            "group-send-response.json" => serde_json::from_str(include_str!(
                "../../tests/fixtures/c5/dingtalk/group-send-response.json"
            ))
            .unwrap(),
            "media-upload-response.json" => serde_json::from_str(include_str!(
                "../../tests/fixtures/c5/dingtalk/media-upload-response.json"
            ))
            .unwrap(),
            "media-send-responses.json" => serde_json::from_str(include_str!(
                "../../tests/fixtures/c5/dingtalk/media-send-responses.json"
            ))
            .unwrap(),
            "inbound-media.json" => serde_json::from_str(include_str!(
                "../../tests/fixtures/c5/dingtalk/inbound-media.json"
            ))
            .unwrap(),
            "stream-reconnect.json" => serde_json::from_str(include_str!(
                "../../tests/fixtures/c5/dingtalk/stream-reconnect.json"
            ))
            .unwrap(),
            "stream-transport.json" => serde_json::from_str(include_str!(
                "../../tests/fixtures/c5/dingtalk/stream-transport.json"
            ))
            .unwrap(),
            "hmac-session-webhook.json" => serde_json::from_str(include_str!(
                "../../tests/fixtures/c5/dingtalk/hmac-session-webhook.json"
            ))
            .unwrap(),
            _ => panic!("unknown DingTalk test fixture: {path}"),
        }
    }

    #[test]
    fn signature_matches_octos_shape_and_fails_closed() {
        let signature = dingtalk_signature("1700000000000", "secret");
        assert!(verify_dingtalk_signature(
            "secret",
            "1700000000000",
            &signature
        ));
        assert!(!verify_dingtalk_signature("secret", "1700000000000", "bad"));
        assert!(!verify_dingtalk_signature("secret", "", &signature));
    }

    #[tokio::test]
    async fn hmac_fixture_and_session_webhook_cache_are_bounded_and_fail_closed() {
        let fixture = fixture_value("hmac-session-webhook.json");
        let timestamp = fixture["timestamp"].as_str().unwrap();
        let secret = fixture["secret"].as_str().unwrap();
        let signature = fixture["sign"].as_str().unwrap();
        assert_eq!(dingtalk_signature(timestamp, secret), signature);
        assert!(verify_dingtalk_signature(secret, timestamp, signature));
        assert!(!verify_dingtalk_signature(secret, timestamp, "short"));
        assert!(!verify_dingtalk_signature(secret, "", signature));

        let adapter = adapter(Arc::new(FakeHttp::default()));
        let trusted = fixture["sessionWebhook"].as_str().unwrap();
        let untrusted = fixture["untrustedSessionWebhook"].as_str().unwrap();
        adapter.cache_session_webhook("cid-cache", trusted);
        adapter.cache_session_webhook("cid-cache", untrusted);
        assert_eq!(
            adapter.cached_session_webhook("cid-cache").as_deref(),
            Some(trusted)
        );

        adapter.cache_session_webhook_with_ttl("cid-expiring", trusted, Duration::from_millis(1));
        assert!(adapter.cached_session_webhook("cid-expiring").is_some());
        tokio::time::sleep(Duration::from_millis(5)).await;
        assert!(adapter.cached_session_webhook("cid-expiring").is_none());
        assert!(is_trusted_dingtalk_url(trusted));
        assert!(!is_trusted_dingtalk_url(untrusted));
    }

    #[test]
    fn capability_snapshot_keeps_stream_media_and_rejects_interactions() {
        let fake = Arc::new(FakeHttp::default());
        let adapter = adapter(fake);
        let capabilities = adapter.capabilities();
        assert!(capabilities.supports(ChannelCapability::IngressTypedAttachments));
        assert!(capabilities.supports(ChannelCapability::ReliabilityHeartbeat));
        assert!(capabilities.supports(ChannelCapability::EgressVideo));
        assert!(!capabilities.supports(ChannelCapability::EgressChunking));
        assert!(!capabilities.supports(ChannelCapability::InteractionEdit));
    }

    #[tokio::test]
    async fn token_is_cached_and_group_payload_uses_open_conversation_id() {
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: fixture_value("token-response.json"),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: fixture_value("group-send-response.json"),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"messageId":"message-2"}),
                retry_after: None,
            },
        ]);
        let sender_adapter = adapter(fake.clone());
        let receipt = sender_adapter
            .execute(ChannelCommand::Send {
                envelope: text_envelope("cid-group", "hello"),
                idempotency_key: Some("idempotent".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            receipt.platform_message_id.as_deref(),
            Some("dingtalk-message-redacted")
        );
        let second_receipt = sender_adapter
            .execute(ChannelCommand::Send {
                envelope: text_envelope("cid-group", "cached token"),
                idempotency_key: Some("idempotent-2".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            second_receipt.platform_message_id.as_deref(),
            Some("message-2")
        );
        let calls = fake.json_calls.lock().unwrap();
        assert_eq!(calls.len(), 3);
        assert_eq!(calls[0].0, DINGTALK_TOKEN_PATH);
        assert_eq!(calls[1].0, DINGTALK_GROUP_SEND_PATH);
        assert_eq!(calls[1].1.as_deref(), Some("redacted-token"));
        assert_eq!(calls[1].2["openConversationId"], "cid-group");
        assert_eq!(calls[2].0, DINGTALK_GROUP_SEND_PATH);
        assert_eq!(calls[2].1.as_deref(), Some("redacted-token"));
    }

    #[tokio::test]
    async fn authenticated_send_refreshes_once_after_401() {
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: json!({"accessToken":"token-first", "expireIn":3600}),
                retry_after: None,
            },
            HttpResponse {
                status: 401,
                body: json!({"message":"expired"}),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"accessToken":"token-second", "expireIn":3600}),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"messageId":"message-after-retry"}),
                retry_after: None,
            },
        ]);
        let adapter = adapter(fake.clone());
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope: text_envelope("user-1", "retry me"),
                idempotency_key: Some("retry-key".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            receipt.platform_message_id.as_deref(),
            Some("message-after-retry")
        );
        let calls = fake.json_calls.lock().unwrap();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[1].1.as_deref(), Some("token-first"));
        assert_eq!(calls[3].1.as_deref(), Some("token-second"));
        assert!(calls
            .iter()
            .all(|(_, _, body)| !body.to_string().contains("retry-key")));
    }

    #[tokio::test]
    async fn oauth_cache_expiry_and_single_flight_are_observable() {
        let fake = Arc::new(FakeHttp::default());
        let mut short_lived = fixture_value("token-response.json");
        short_lived["accessToken"] = json!("token-first");
        short_lived["expireIn"] = json!(1);
        fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: short_lived,
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"accessToken":"token-after-expiry", "expireIn":3600}),
                retry_after: None,
            },
        ]);
        let adapter = Arc::new(adapter(fake.clone()));
        let tasks = (0..8)
            .map(|_| {
                let adapter = adapter.clone();
                tokio::spawn(async move { adapter.access_token().await })
            })
            .collect::<Vec<_>>();
        for task in tasks {
            assert_eq!(task.await.unwrap().unwrap(), "token-first");
        }
        assert_eq!(
            fake.json_calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| call.0 == DINGTALK_TOKEN_PATH)
                .count(),
            1
        );

        tokio::time::sleep(Duration::from_millis(1_100)).await;
        assert_eq!(adapter.access_token().await.unwrap(), "token-after-expiry");
        assert_eq!(
            fake.json_calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| call.0 == DINGTALK_TOKEN_PATH)
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn media_upload_and_send_preserve_typed_part_and_partial_failure() {
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: json!({"accessToken":"token", "expireIn":3600}),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"media_id":"media-1"}),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"messageId":"media-message-1"}),
                retry_after: None,
            },
        ]);
        let sender_adapter = adapter(fake.clone());
        let attachment = fixture_attachment();
        let receipt = sender_adapter
            .execute(ChannelCommand::Send {
                envelope: ChannelEnvelopeV1::new(
                    ChannelDirection::Egress,
                    ChannelAddress::new("dingtalk", "user-1"),
                    Correlation::new("dingtalk:user-1"),
                    ChannelOrigin::Runtime,
                    ChannelPayloadV1::Message {
                        parts: vec![ContentPart::Image { attachment }],
                        subject: None,
                        locale: None,
                        context: None,
                    },
                ),
                idempotency_key: Some("media-key".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            receipt.platform_message_id.as_deref(),
            Some("media-message-1")
        );
        {
            let uploads = fake.multipart_calls.lock().unwrap();
            assert_eq!(uploads.len(), 1);
            assert_eq!(uploads[0].0, DINGTALK_MEDIA_UPLOAD_PATH);
            assert_eq!(uploads[0].2, "image");
            assert_eq!(uploads[0].3.filename, "fixture.png");
            assert_eq!(uploads[0].3.bytes, vec![1, 2, 3]);
        }
        {
            let calls = fake.json_calls.lock().unwrap();
            assert_eq!(calls[1].0, DINGTALK_PRIVATE_SEND_PATH);
            assert_eq!(calls[1].2["msgKey"], "sampleImageMsg");
            assert!(calls[1].2["msgParam"].as_str().unwrap().contains("media-1"));
        }

        let failed = Arc::new(FakeHttp::default());
        failed.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: json!({"accessToken":"token", "expireIn":3600}),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"media_id":"media-2"}),
                retry_after: None,
            },
            HttpResponse {
                status: 429,
                body: json!({"message":"rate limited"}),
                retry_after: Some(Duration::from_secs(7)),
            },
        ]);
        let failed_adapter = adapter(failed.clone());
        let result = failed_adapter
            .execute(ChannelCommand::Send {
                envelope: ChannelEnvelopeV1::new(
                    ChannelDirection::Egress,
                    ChannelAddress::new("dingtalk", "user-1"),
                    Correlation::new("dingtalk:user-1"),
                    ChannelOrigin::Runtime,
                    ChannelPayloadV1::Message {
                        parts: vec![ContentPart::Image {
                            attachment: fixture_attachment(),
                        }],
                        subject: None,
                        locale: None,
                        context: None,
                    },
                ),
                idempotency_key: Some("media-failure".to_string()),
            })
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::RateLimited { retry_after })
                if retry_after == Duration::from_secs(7)
        ));
        assert_eq!(failed.json_calls.lock().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn media_upload_refreshes_oauth_once_after_401() {
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: json!({"accessToken":"token-upload-first", "expireIn":3600}),
                retry_after: None,
            },
            HttpResponse {
                status: 401,
                body: json!({"message":"expired"}),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"accessToken":"token-upload-second", "expireIn":3600}),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: fixture_value("media-upload-response.json"),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: fixture_value("group-send-response.json"),
                retry_after: None,
            },
        ]);
        let adapter = adapter(fake.clone());
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope: ChannelEnvelopeV1::new(
                    ChannelDirection::Egress,
                    ChannelAddress::new("dingtalk", "cid-group"),
                    Correlation::new("dingtalk:cid-group"),
                    ChannelOrigin::Runtime,
                    ChannelPayloadV1::Message {
                        parts: vec![ContentPart::Image {
                            attachment: fixture_attachment(),
                        }],
                        subject: None,
                        locale: None,
                        context: None,
                    },
                ),
                idempotency_key: Some("upload-retry-key".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            receipt.platform_message_id.as_deref(),
            Some("dingtalk-message-redacted")
        );
        let uploads = fake.multipart_calls.lock().unwrap();
        assert_eq!(uploads.len(), 2);
        assert_eq!(uploads[0].1, "token-upload-first");
        assert_eq!(uploads[1].1, "token-upload-second");
        assert_eq!(uploads[0].2, "image");
        assert_eq!(uploads[1].3.bytes, vec![1, 2, 3]);
        let calls = fake.json_calls.lock().unwrap();
        assert_eq!(
            calls
                .iter()
                .filter(|call| call.0 == DINGTALK_TOKEN_PATH)
                .count(),
            2
        );
        assert!(calls
            .iter()
            .all(|(_, _, body)| !body.to_string().contains("upload-retry-key")));
    }

    #[tokio::test]
    async fn media_image_audio_video_file_use_typed_group_wire_shapes() {
        let fixture = fixture_value("media-send-responses.json");
        let uploads = fixture["uploads"].as_array().unwrap();
        let sends = fixture["sends"].as_array().unwrap();
        assert_eq!(uploads.len(), 4);
        assert_eq!(sends.len(), 4);

        let fake = Arc::new(FakeHttp::default());
        let mut responses = fake.json_responses.lock().unwrap();
        responses.push_back(HttpResponse {
            status: 200,
            body: fixture_value("token-response.json"),
            retry_after: None,
        });
        for (upload, send) in uploads.iter().zip(sends.iter()) {
            responses.push_back(HttpResponse {
                status: 200,
                body: upload.clone(),
                retry_after: None,
            });
            responses.push_back(HttpResponse {
                status: 200,
                body: send.clone(),
                retry_after: None,
            });
        }
        drop(responses);

        let envelope = ChannelEnvelopeV1::new(
            ChannelDirection::Egress,
            ChannelAddress::new("dingtalk", "cid-group"),
            Correlation::new("dingtalk:cid-group"),
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Message {
                parts: vec![
                    ContentPart::Image {
                        attachment: fixture_attachment_with("image/png", "fixture.png"),
                    },
                    ContentPart::Audio {
                        attachment: fixture_attachment_with("audio/mpeg", "fixture.mp3"),
                        transcript: None,
                    },
                    ContentPart::Video {
                        attachment: fixture_attachment_with("video/mp4", "fixture.mp4"),
                    },
                    ContentPart::File {
                        attachment: fixture_attachment_with(
                            "application/octet-stream",
                            "fixture.bin",
                        ),
                    },
                ],
                subject: None,
                locale: None,
                context: None,
            },
        );
        let adapter = adapter(fake.clone());
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope,
                idempotency_key: Some("platform-key-not-forwarded".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            receipt.platform_message_id.as_deref(),
            Some("message-file-redacted")
        );

        let uploads = fake.multipart_calls.lock().unwrap();
        assert_eq!(uploads.len(), 4);
        assert_eq!(
            uploads
                .iter()
                .map(|call| call.2.as_str())
                .collect::<Vec<_>>(),
            ["image", "voice", "file", "file"]
        );
        assert_eq!(uploads[0].3.media_type.as_deref(), Some("image/png"));
        assert_eq!(uploads[1].3.media_type.as_deref(), Some("audio/mpeg"));
        assert_eq!(uploads[2].3.media_type.as_deref(), Some("video/mp4"));
        assert_eq!(
            uploads[3].3.media_type.as_deref(),
            Some("application/octet-stream")
        );
        assert_eq!(
            uploads
                .iter()
                .map(|call| call.3.filename.as_str())
                .collect::<Vec<_>>(),
            ["fixture.png", "fixture.mp3", "fixture.mp4", "fixture.bin"]
        );
        drop(uploads);

        let calls = fake.json_calls.lock().unwrap();
        let sends = calls
            .iter()
            .filter(|call| call.0 == DINGTALK_GROUP_SEND_PATH)
            .collect::<Vec<_>>();
        assert_eq!(sends.len(), 4);
        let expected = [
            ("sampleImageMsg", "photoURL", "media-image-redacted"),
            ("sampleFileMsg", "fileType", "voice"),
            ("sampleFileMsg", "fileType", "video"),
            ("sampleFileMsg", "fileType", "file"),
        ];
        for (call, (msg_key, parameter, value)) in sends.iter().zip(expected) {
            assert_eq!(call.2["openConversationId"], "cid-group");
            assert_eq!(call.2["msgKey"], msg_key);
            let params: Value = serde_json::from_str(call.2["msgParam"].as_str().unwrap()).unwrap();
            assert_eq!(params[parameter], value);
        }
        assert!(calls
            .iter()
            .all(|(_, _, body)| !body.to_string().contains("platform-key-not-forwarded")));
    }

    #[tokio::test]
    async fn media_multi_part_failure_is_typed_and_never_claims_retry_safe() {
        let fixture = fixture_value("media-send-responses.json");
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: fixture_value("token-response.json"),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: fixture["uploads"][0].clone(),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: fixture["sends"][0].clone(),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: fixture["uploads"][1].clone(),
                retry_after: None,
            },
            HttpResponse {
                status: fixture["partial_failure"]["status"].as_u64().unwrap() as u16,
                body: fixture["partial_failure"]["body"].clone(),
                retry_after: Some(Duration::from_secs(
                    fixture["partial_failure"]["retry_after_seconds"]
                        .as_u64()
                        .unwrap(),
                )),
            },
        ]);
        let envelope = ChannelEnvelopeV1::new(
            ChannelDirection::Egress,
            ChannelAddress::new("dingtalk", "cid-group"),
            Correlation::new("dingtalk:cid-group"),
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Message {
                parts: vec![
                    ContentPart::Image {
                        attachment: fixture_attachment_with("image/png", "first.png"),
                    },
                    ContentPart::File {
                        attachment: fixture_attachment_with(
                            "application/octet-stream",
                            "second.bin",
                        ),
                    },
                ],
                subject: None,
                locale: None,
                context: None,
            },
        );
        let error = adapter(fake.clone())
            .execute(ChannelCommand::Send {
                envelope,
                idempotency_key: Some("unsafe-retry-key".to_string()),
            })
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            AdapterError::Execution {
                ref code,
                retry_after: Some(after),
                retryable: true,
                ..
            } if code == "partial_delivery" && after == Duration::from_secs(7)
        ));
        assert_eq!(fake.multipart_calls.lock().unwrap().len(), 2);
        assert_eq!(fake.json_calls.lock().unwrap().len(), 3);
        assert!(fake
            .json_calls
            .lock()
            .unwrap()
            .iter()
            .all(|(_, _, body)| !body.to_string().contains("unsafe-retry-key")));
    }

    #[tokio::test]
    async fn stream_register_callback_ack_cancel_and_dedup_are_wire_bound() {
        let (endpoint, server) = spawn_stream_fixture().await;
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().push_back(HttpResponse {
            status: 200,
            body: json!({"endpoint": endpoint, "ticket": "ticket-fixture"}),
            retry_after: None,
        });
        let adapter = Arc::new(adapter(fake.clone()));
        let (registered_endpoint, ticket) = adapter.register_stream().await.unwrap();
        assert_eq!(ticket, "ticket-fixture");
        {
            let register_calls = fake.json_calls.lock().unwrap();
            assert_eq!(register_calls.len(), 1);
            assert_eq!(register_calls[0].0, DINGTALK_STREAM_REGISTER_PATH);
            assert_eq!(register_calls[0].2["clientId"], "client");
        }

        let (fabric, mut consumer) = FabricConsumer::new();
        let cancel = CancellationToken::new();
        let task = tokio::spawn({
            let adapter = adapter.clone();
            let cancel = cancel.clone();
            async move {
                adapter
                    .run_stream_once(
                        registered_endpoint,
                        ticket,
                        &AdapterContext { fabric, cancel },
                    )
                    .await
            }
        });
        let envelope = tokio::time::timeout(Duration::from_secs(2), consumer.recv_ingress())
            .await
            .unwrap()
            .unwrap()
            .envelope()
            .clone();
        assert_eq!(envelope.address.chat_id, "cid-redacted");
        assert_eq!(
            envelope.address.sender_id.as_deref(),
            Some("sender-redacted")
        );
        assert_eq!(
            envelope.correlation.message_id.as_deref(),
            Some("message-redacted")
        );
        assert!(
            tokio::time::timeout(Duration::from_millis(50), consumer.recv_ingress())
                .await
                .is_err()
        );
        tokio::time::sleep(Duration::from_millis(50)).await;
        cancel.cancel();
        assert!(task.await.unwrap().is_ok());
        let responses = server.await.unwrap();
        assert!(responses.iter().any(|response| {
            response["code"] == 200 && response["headers"]["messageId"] == "stream-message-redacted"
        }));
        assert_eq!(responses.len(), 2);
    }

    #[tokio::test]
    async fn stream_server_ping_gets_transport_pong_without_active_heartbeat() {
        let fixture = fixture_value("stream-transport.json");
        let payload = fixture["server_to_client"]["ping"]["payload"]
            .as_str()
            .unwrap()
            .as_bytes()
            .to_vec();
        let (endpoint, server) = spawn_stream_ping_fixture(payload.clone()).await;
        let adapter = adapter(Arc::new(FakeHttp::default()));
        let context = AdapterContext {
            fabric: FabricConsumer::new().0,
            cancel: CancellationToken::new(),
        };
        let result = adapter
            .run_stream_once(endpoint, "ticket-fixture".to_string(), &context)
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::Execution { ref code, .. }) if code == "stream_closed"
        ));
        assert_eq!(server.await.unwrap(), payload);
        assert_eq!(adapter.health().status, ChannelHealthStatus::Healthy);
    }

    #[tokio::test]
    async fn stream_admission_failure_is_typed_and_does_not_commit_ack_state() {
        let adapter = adapter(Arc::new(FakeHttp::default()));
        let (fabric, consumer) = FabricConsumer::new();
        drop(consumer);
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let message: StreamMessage = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/dingtalk/stream-callback.json"
        ))
        .unwrap();
        let error = adapter
            .handle_stream_message(message, &context)
            .await
            .unwrap_err();
        assert!(matches!(
            error,
            AdapterError::Execution { ref code, retryable: true, .. }
                if code == "fabric_closed"
        ));
        assert!(!adapter.is_processed("stream-message-redacted"));
        assert!(!adapter.is_processed("message-redacted"));
        assert!(adapter.cached_session_webhook("cid-redacted").is_none());
    }

    #[tokio::test]
    async fn start_reconnects_with_backoff_and_transitions_health() {
        let fixture = fixture_value("stream-reconnect.json");
        let first_ticket = fixture["register"]["success"][0]["ticket"]
            .as_str()
            .unwrap()
            .to_string();
        let second_ticket = fixture["register"]["success"][1]["ticket"]
            .as_str()
            .unwrap()
            .to_string();
        let (endpoint, first_closed, second_ready, server) = spawn_stream_reconnect_fixture().await;
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: json!({"endpoint":endpoint,"ticket":first_ticket}),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"endpoint":endpoint,"ticket":second_ticket}),
                retry_after: None,
            },
        ]);
        let mut native = adapter(fake.clone());
        native.reconnect_base =
            Duration::from_millis(fixture["backoff"]["initial_ms"].as_u64().unwrap());
        let adapter = Arc::new(native);
        let (fabric, _consumer) = FabricConsumer::new();
        let cancel = CancellationToken::new();
        let task = tokio::spawn({
            let adapter = adapter.clone();
            let cancel = cancel.clone();
            async move { adapter.start(AdapterContext { fabric, cancel }).await }
        });

        tokio::time::timeout(Duration::from_secs(2), first_closed)
            .await
            .unwrap()
            .unwrap();
        for _ in 0..100 {
            if adapter.health().status == ChannelHealthStatus::Degraded {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        assert_eq!(adapter.health().status, ChannelHealthStatus::Degraded);

        tokio::time::timeout(Duration::from_secs(2), second_ready)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(adapter.health().status, ChannelHealthStatus::Healthy);
        let register_times = fake
            .json_call_times
            .lock()
            .unwrap()
            .iter()
            .filter(|(path, _)| path == DINGTALK_STREAM_REGISTER_PATH)
            .map(|(_, at)| *at)
            .collect::<Vec<_>>();
        assert_eq!(register_times.len(), 2);
        assert!(register_times[1].duration_since(register_times[0]) >= Duration::from_millis(15));

        cancel.cancel();
        assert!(task.await.unwrap().is_ok());
        server.await.unwrap();
    }

    #[tokio::test]
    async fn stop_interrupts_reconnect_backoff_and_marks_health_down() {
        let fixture = fixture_value("stream-reconnect.json");
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().push_back(HttpResponse {
            status: fixture["register"]["failure"]["status"].as_u64().unwrap() as u16,
            body: fixture["register"]["failure"]["body"].clone(),
            retry_after: None,
        });
        let mut native = adapter(fake.clone());
        native.reconnect_base = Duration::from_secs(60);
        let adapter = Arc::new(native);
        let (fabric, _consumer) = FabricConsumer::new();
        let task = tokio::spawn({
            let adapter = adapter.clone();
            async move {
                adapter
                    .start(AdapterContext {
                        fabric,
                        cancel: CancellationToken::new(),
                    })
                    .await
            }
        });
        for _ in 0..100 {
            if fake.json_calls.lock().unwrap().len() == 1 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
        assert_eq!(fake.json_calls.lock().unwrap().len(), 1);
        adapter.stop().await.unwrap();
        tokio::time::timeout(Duration::from_secs(1), task)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(adapter.health().status, ChannelHealthStatus::Down);
        assert_eq!(
            adapter.health().diagnosis.as_deref(),
            Some("DingTalk adapter stopped")
        );
    }

    #[tokio::test]
    async fn health_probe_returns_receipt_and_typed_down_failure() {
        let healthy_fake = Arc::new(FakeHttp::default());
        healthy_fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: fixture_value("token-response.json"),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"status":"ok"}),
                retry_after: None,
            },
        ]);
        let healthy = adapter(healthy_fake.clone());
        let receipt = healthy
            .execute(ChannelCommand::ProbeHealth {
                channel: DingTalkAdapter::channel_id(),
            })
            .await
            .unwrap();
        assert_eq!(
            receipt.status,
            agent_diva_core::channel::DeliveryStatus::Accepted
        );
        assert_eq!(healthy.health().status, ChannelHealthStatus::Healthy);
        assert_eq!(
            healthy_fake.json_calls.lock().unwrap()[1].0,
            "/v1.0/robot/info"
        );

        let failed_fake = Arc::new(FakeHttp::default());
        failed_fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: fixture_value("token-response.json"),
                retry_after: None,
            },
            HttpResponse {
                status: 503,
                body: json!({"message":"robot unavailable"}),
                retry_after: None,
            },
        ]);
        let failed = adapter(failed_fake);
        let error = failed
            .execute(ChannelCommand::ProbeHealth {
                channel: DingTalkAdapter::channel_id(),
            })
            .await
            .unwrap_err();
        assert_eq!(failed.health().status, ChannelHealthStatus::Down);
        assert!(error.is_retryable());
        assert!(error.code().contains("robot/info_http_503"));
    }

    #[tokio::test]
    async fn media_ingress_downloads_after_policy_and_admits_typed_attachment() {
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().push_back(HttpResponse {
            status: 200,
            body: json!({"accessToken":"token", "expireIn":3600}),
            retry_after: None,
        });
        fake.downloads.lock().unwrap().push_back(BytesResponse {
            status: 200,
            bytes: vec![9, 8, 7],
            retry_after: None,
        });
        let store = Arc::new(TestStore::default());
        let adapter = DingTalkAdapter::with_http(
            &config(),
            AdapterServices::new(store.clone()),
            fake.clone(),
        );
        let (fabric, mut consumer) = FabricConsumer::new();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let message = StreamMessage {
            spec_version: "1.0".to_string(),
            msg_type: "CALLBACK".to_string(),
            headers: StreamHeaders {
                message_id: "stream-media-1".to_string(),
                topic: "/v1.0/im/bot/messages/get".to_string(),
                content_type: "application/json".to_string(),
                time: "fixture".to_string(),
                app_id: None,
            },
            data: json!({
                "msgId":"media-message-1",
                "senderStaffId":"sender",
                "conversationId":"cid-group",
                "conversationType":"2",
                "content":{"image":{
                    "downloadUrl":"https://oapi.dingtalk.com/media/fixture",
                    "fileName":"fixture.png",
                    "mimeType":"image/png"
                }}
            })
            .to_string(),
        };
        assert!(matches!(
            adapter.handle_stream_message(message, &context).await,
            Ok(StreamOutcome::Ack)
        ));
        let envelope = consumer.recv_ingress().await.unwrap().envelope().clone();
        assert!(matches!(
            envelope.payload,
            ChannelPayloadV1::Message { ref parts, .. }
                if matches!(parts.first(), Some(ContentPart::Image { .. }))
        ));
        assert_eq!(store.puts.load(Ordering::Acquire), 1);
        assert_eq!(fake.json_calls.lock().unwrap().len(), 1);
        assert!(fake.downloads.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn fixture_media_events_download_and_admit_all_four_typed_parts() {
        let fixture = fixture_value("inbound-media.json");
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().push_back(HttpResponse {
            status: 200,
            body: fixture_value("token-response.json"),
            retry_after: None,
        });
        fake.downloads
            .lock()
            .unwrap()
            .extend((0..4).map(|_| BytesResponse {
                status: 200,
                bytes: vec![9, 8, 7],
                retry_after: None,
            }));
        let store = Arc::new(TestStore::default());
        let adapter = DingTalkAdapter::with_http(
            &config(),
            AdapterServices::new(store.clone()),
            fake.clone(),
        );
        let (fabric, mut consumer) = FabricConsumer::new();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        for (index, event) in fixture["events"].as_array().unwrap().iter().enumerate() {
            let message = StreamMessage {
                spec_version: "1.0".to_string(),
                msg_type: "CALLBACK".to_string(),
                headers: StreamHeaders {
                    message_id: format!("media-update-{index}"),
                    topic: "/v1.0/im/bot/messages/get".to_string(),
                    content_type: "application/json".to_string(),
                    time: "fixture".to_string(),
                    app_id: None,
                },
                data: event.to_string(),
            };
            assert!(matches!(
                adapter.handle_stream_message(message, &context).await,
                Ok(StreamOutcome::Ack)
            ));
            let envelope = consumer.recv_ingress().await.unwrap().envelope().clone();
            let ChannelPayloadV1::Message { parts, .. } = envelope.payload else {
                panic!("expected message payload");
            };
            assert_eq!(parts.len(), 1);
            match (index, &parts[0]) {
                (0, ContentPart::Image { attachment }) => {
                    assert_eq!(attachment.media_type, "image/png")
                }
                (1, ContentPart::Audio { attachment, .. }) => {
                    assert_eq!(attachment.media_type, "audio/mpeg")
                }
                (2, ContentPart::Video { attachment }) => {
                    assert_eq!(attachment.media_type, "video/mp4")
                }
                (3, ContentPart::File { attachment }) => {
                    assert_eq!(attachment.media_type, "application/octet-stream")
                }
                _ => panic!("unexpected DingTalk typed media part at index {index}"),
            }
        }
        assert_eq!(store.puts.load(Ordering::Acquire), 4);
        assert_eq!(fake.download_calls.lock().unwrap().len(), 4);
        assert!(fake.downloads.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn media_download_refreshes_oauth_once_after_401() {
        let fake = Arc::new(FakeHttp::default());
        fake.json_responses.lock().unwrap().extend([
            HttpResponse {
                status: 200,
                body: json!({"accessToken":"token-download-first", "expireIn":3600}),
                retry_after: None,
            },
            HttpResponse {
                status: 200,
                body: json!({"accessToken":"token-download-second", "expireIn":3600}),
                retry_after: None,
            },
        ]);
        fake.downloads.lock().unwrap().extend([
            BytesResponse {
                status: 401,
                bytes: Vec::new(),
                retry_after: None,
            },
            BytesResponse {
                status: 200,
                bytes: vec![9, 8, 7],
                retry_after: None,
            },
        ]);
        let store = Arc::new(TestStore::default());
        let adapter = DingTalkAdapter::with_http(
            &config(),
            AdapterServices::new(store.clone()),
            fake.clone(),
        );
        let (fabric, mut consumer) = FabricConsumer::new();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let message = StreamMessage {
            spec_version: "1.0".to_string(),
            msg_type: "CALLBACK".to_string(),
            headers: StreamHeaders {
                message_id: "download-update".to_string(),
                topic: "/v1.0/im/bot/messages/get".to_string(),
                content_type: "application/json".to_string(),
                time: "fixture".to_string(),
                app_id: None,
            },
            data: json!({
                "msgId":"download-message",
                "senderStaffId":"sender",
                "conversationId":"cid-group",
                "conversationType":"2",
                "content":{"image":{
                    "downloadUrl":"https://oapi.dingtalk.com/media/fixture",
                    "fileName":"fixture.png",
                    "mimeType":"image/png"
                }}
            })
            .to_string(),
        };
        assert!(matches!(
            adapter.handle_stream_message(message, &context).await,
            Ok(StreamOutcome::Ack)
        ));
        assert!(consumer.recv_ingress().await.is_some());
        let calls = fake.download_calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].1.as_deref(), Some("token-download-first"));
        assert_eq!(calls[1].1.as_deref(), Some("token-download-second"));
        assert_eq!(store.puts.load(Ordering::Acquire), 1);
    }

    #[test]
    fn shipped_stream_and_media_fixtures_are_parseable() {
        let stream: StreamMessage = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/dingtalk/stream-callback.json"
        ))
        .unwrap();
        assert_eq!(stream.msg_type, "CALLBACK");
        assert_eq!(stream.headers.topic, "/v1.0/im/bot/messages/get");
        let token: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/dingtalk/token-response.json"
        ))
        .unwrap();
        assert!(token.get("accessToken").is_some());
        let media: Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/c5/dingtalk/media-upload-response.json"
        ))
        .unwrap();
        assert!(media.get("media_id").is_some());
        let transport = fixture_value("stream-transport.json");
        assert_eq!(transport["server_to_client"]["ping"]["kind"], "ping");
        assert_eq!(
            transport["expected_client_to_server"]["pong"]["kind"],
            "pong"
        );
        let reconnect = fixture_value("stream-reconnect.json");
        assert_eq!(
            reconnect["register"]["success"].as_array().unwrap().len(),
            2
        );
        assert_eq!(reconnect["register"]["failure"]["status"], 503);
        let media_responses = fixture_value("media-send-responses.json");
        assert_eq!(media_responses["uploads"].as_array().unwrap().len(), 4);
        assert_eq!(media_responses["sends"].as_array().unwrap().len(), 4);
        let inbound_media = fixture_value("inbound-media.json");
        assert_eq!(inbound_media["events"].as_array().unwrap().len(), 4);
        let hmac = fixture_value("hmac-session-webhook.json");
        assert_eq!(hmac["header_names"].as_array().unwrap().len(), 2);
    }

    #[tokio::test]
    async fn unsupported_edit_has_zero_http_side_effect() {
        let fake = Arc::new(FakeHttp::default());
        let adapter = adapter(fake.clone());
        let command = ChannelCommand::Edit {
            address: ChannelAddress::new("dingtalk", "cid-group"),
            correlation: Correlation::new("dingtalk:cid-group"),
            target_message_id: "message-1".to_string(),
            parts: vec![ContentPart::Text {
                text: "edited".to_string(),
            }],
            idempotency_key: None,
        };
        assert!(matches!(
            adapter.execute(command).await,
            Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionEdit
            })
        ));
        assert!(fake.json_calls.lock().unwrap().is_empty());
    }

    #[test]
    fn parser_applies_allowlist_before_media_url_resolution() {
        let mut restricted = config();
        restricted.allow_from = vec!["allowed".to_string()];
        let fake = Arc::new(FakeHttp::default());
        let store = Arc::new(TestStore::default());
        let adapter = DingTalkAdapter::with_http(
            &restricted,
            AdapterServices::new(store.clone()),
            fake.clone(),
        );
        let message = StreamMessage {
            spec_version: "1.0".to_string(),
            msg_type: "CALLBACK".to_string(),
            headers: StreamHeaders {
                message_id: "stream-1".to_string(),
                topic: "/v1.0/im/bot/messages/get".to_string(),
                content_type: "application/json".to_string(),
                time: "now".to_string(),
                app_id: None,
            },
            data: json!({
                "msgId":"message-1",
                "senderStaffId":"denied",
                "conversationId":"cid-group",
                "conversationType":"2",
                "content":{"image":{"downloadUrl":"https://example.invalid/media"}}
            })
            .to_string(),
        };
        assert!(adapter.parse_event(&message).unwrap().is_none());
        assert!(fake.downloads.lock().unwrap().is_empty());
        assert!(fake.json_calls.lock().unwrap().is_empty());
        assert_eq!(store.puts.load(Ordering::Acquire), 0);
    }

    #[test]
    fn deny_group_policy_blocks_media_before_url_validation() {
        let mut restricted = config();
        restricted.group_policy = "deny".to_string();
        let fake = Arc::new(FakeHttp::default());
        let adapter = DingTalkAdapter::with_http(
            &restricted,
            AdapterServices::new(Arc::new(TestStore::default())),
            fake.clone(),
        );
        let message = StreamMessage {
            spec_version: "1.0".to_string(),
            msg_type: "CALLBACK".to_string(),
            headers: StreamHeaders {
                message_id: "stream-denied".to_string(),
                topic: "/v1.0/im/bot/messages/get".to_string(),
                content_type: "application/json".to_string(),
                time: "now".to_string(),
                app_id: None,
            },
            data: json!({
                "msgId":"message-denied",
                "senderStaffId":"sender",
                "conversationId":"cid-group",
                "conversationType":"2",
                "content":{"image":{"downloadUrl":"not-a-trusted-url"}}
            })
            .to_string(),
        };
        assert!(adapter.parse_event(&message).unwrap().is_none());
        assert!(fake.downloads.lock().unwrap().is_empty());
        assert!(fake.json_calls.lock().unwrap().is_empty());
    }

    async fn spawn_stream_fixture() -> (String, tokio::task::JoinHandle<Vec<Value>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let endpoint = format!("ws://{address}/stream");
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            let callback =
                include_str!("../../tests/fixtures/c5/dingtalk/stream-callback.json").to_string();
            socket
                .send(WsMessage::Text(callback.clone()))
                .await
                .unwrap();
            socket.send(WsMessage::Text(callback)).await.unwrap();
            let mut responses = Vec::new();
            while let Ok(Some(Ok(message))) =
                tokio::time::timeout(Duration::from_secs(3), socket.next()).await
            {
                let WsMessage::Text(text) = message else {
                    continue;
                };
                responses.push(serde_json::from_str::<Value>(&text).unwrap());
            }
            responses
        });
        (endpoint, handle)
    }

    async fn spawn_stream_ping_fixture(
        payload: Vec<u8>,
    ) -> (String, tokio::task::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let endpoint = format!("ws://{address}/stream");
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            socket.send(WsMessage::Ping(payload.clone())).await.unwrap();
            let response = tokio::time::timeout(Duration::from_secs(2), socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            let pong = match response {
                WsMessage::Pong(pong) => pong,
                other => panic!("expected Pong after Ping, got {other:?}"),
            };
            socket.close(None).await.unwrap();
            pong
        });
        (endpoint, handle)
    }

    async fn spawn_stream_reconnect_fixture() -> (
        String,
        oneshot::Receiver<()>,
        oneshot::Receiver<()>,
        tokio::task::JoinHandle<()>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let endpoint = format!("ws://{address}/stream");
        let (first_closed_tx, first_closed_rx) = oneshot::channel();
        let (second_ready_tx, second_ready_rx) = oneshot::channel();
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            socket.close(None).await.unwrap();
            let _ = first_closed_tx.send(());

            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();
            socket
                .send(WsMessage::Ping(b"reconnect-ping".to_vec()))
                .await
                .unwrap();
            let response = tokio::time::timeout(Duration::from_secs(2), socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            assert!(matches!(response, WsMessage::Pong(payload) if payload == b"reconnect-ping"));
            let _ = second_ready_tx.send(());
            let _ = tokio::time::timeout(Duration::from_secs(2), socket.next()).await;
        });
        (endpoint, first_closed_rx, second_ready_rx, handle)
    }
}
