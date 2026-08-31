//! Native Super Channel adapter contract.
//!
//! This API does not extend or wrap the retired handler trait. The old interface
//! remains untouched until the atomic CHANNEL-EPIC C6 cutover.

use agent_diva_core::channel::{
    AttachmentRef, ChannelAddress, ChannelCapabilities, ChannelCapability, ChannelCommand,
    ChannelDirection, ChannelEnvelopeV1, ChannelHealth, ChannelId, ChannelOrigin, ChannelPayloadV1,
    ContentPart, Correlation, DeliveryReceipt, DeliveryStatus, FabricHandle,
};
use agent_diva_core::config::Config;
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::{fmt, sync::Arc, time::Duration};
use thiserror::Error;
use tokio_util::sync::CancellationToken;

/// Services shared by native adapters without changing the listener context.
///
/// The Manager supplies the production implementation during the C6 cutover;
/// C5 adapters and tests receive an explicit service value at construction.
#[derive(Clone)]
pub struct AdapterServices {
    /// Content-addressed attachment authority used for typed media ingress.
    pub attachments: Arc<dyn ChannelAttachmentStore>,
}

impl AdapterServices {
    /// Construct adapter services from the single attachment authority.
    pub fn new(attachments: Arc<dyn ChannelAttachmentStore>) -> Self {
        Self { attachments }
    }
}

impl fmt::Debug for AdapterServices {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("AdapterServices")
            .field("attachments", &"<channel attachment store>")
            .finish()
    }
}

/// Raw, bounded media received from an external platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngressAttachment {
    /// Stable DIVA channel identifier, never an arbitrary remote URL.
    pub source_channel: String,
    /// Platform message identifier that carried the media, when available.
    pub platform_message_id: Option<String>,
    /// External sender identifier, when available.
    pub sender_id: Option<String>,
    /// Original file name supplied by the platform, when available.
    pub file_name: Option<String>,
    /// MIME type declared by the platform, when available.
    pub declared_mime: Option<String>,
    /// Bytes fetched only after policy and admission checks.
    pub bytes: Vec<u8>,
}

impl IngressAttachment {
    /// Validate metadata before delegating to a storage authority.
    pub fn validate(&self) -> Result<(), AttachmentStoreError> {
        if self.source_channel.trim().is_empty() {
            return Err(AttachmentStoreError::InvalidInput {
                field: "source_channel",
            });
        }
        if self
            .platform_message_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AttachmentStoreError::InvalidInput {
                field: "platform_message_id",
            });
        }
        if self
            .sender_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AttachmentStoreError::InvalidInput { field: "sender_id" });
        }
        if let Some(file_name) = &self.file_name {
            if file_name.trim().is_empty()
                || file_name.contains('/')
                || file_name.contains('\\')
                || std::path::Path::new(file_name).is_absolute()
            {
                return Err(AttachmentStoreError::InvalidInput { field: "file_name" });
            }
        }
        if self
            .declared_mime
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err(AttachmentStoreError::InvalidInput {
                field: "declared_mime",
            });
        }
        Ok(())
    }
}

/// Stored attachment returned by the content-addressed authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredAttachment {
    /// Validated metadata and content identity.
    pub reference: AttachmentRef,
    /// Bytes addressed by `reference`.
    pub bytes: Vec<u8>,
}

impl StoredAttachment {
    /// Verify that the content and content-addressed metadata agree.
    pub fn validate(&self) -> Result<(), AttachmentStoreError> {
        validate_attachment_reference(&self.reference, &self.bytes)
    }
}

/// Typed failures for the attachment seam.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AttachmentStoreError {
    #[error("invalid attachment input field: {field}")]
    InvalidInput { field: &'static str },
    #[error("attachment exceeds the configured bound ({size_bytes} > {max_bytes} bytes)")]
    TooLarge { size_bytes: u64, max_bytes: u64 },
    #[error("invalid attachment reference: {diagnosis}")]
    InvalidReference { diagnosis: String },
    #[error("attachment not found: {uri}")]
    NotFound { uri: String },
    #[error("attachment digest mismatch: {uri}")]
    DigestMismatch { uri: String },
    #[error("attachment backend failed: {diagnosis}")]
    Backend { diagnosis: String },
}

/// Content-addressed attachment authority injected into native adapters.
#[async_trait]
pub trait ChannelAttachmentStore: Send + Sync + 'static {
    /// Store bounded bytes and return a validated `sha256:<hex>` reference.
    async fn put(&self, input: IngressAttachment) -> Result<AttachmentRef, AttachmentStoreError>;

    /// Read and validate an existing content-addressed reference.
    async fn get(
        &self,
        reference: &AttachmentRef,
    ) -> Result<StoredAttachment, AttachmentStoreError>;
}

/// Verify the non-path content identity required by `AttachmentRef`.
pub fn validate_attachment_reference(
    reference: &AttachmentRef,
    bytes: &[u8],
) -> Result<(), AttachmentStoreError> {
    if reference.media_type.trim().is_empty() {
        return Err(AttachmentStoreError::InvalidReference {
            diagnosis: "media_type is empty".to_string(),
        });
    }
    if reference.size_bytes != bytes.len() as u64 {
        return Err(AttachmentStoreError::InvalidReference {
            diagnosis: format!(
                "size mismatch (reference={}, actual={})",
                reference.size_bytes,
                bytes.len()
            ),
        });
    }
    let digest = format!("{:x}", Sha256::digest(bytes));
    if reference.sha256 != digest || reference.uri != format!("sha256:{digest}") {
        return Err(AttachmentStoreError::DigestMismatch {
            uri: reference.uri.clone(),
        });
    }
    if reference.file_name.as_deref().is_some_and(|file_name| {
        file_name.trim().is_empty()
            || file_name.contains('/')
            || file_name.contains('\\')
            || std::path::Path::new(file_name).is_absolute()
    }) {
        return Err(AttachmentStoreError::InvalidReference {
            diagnosis: "file_name must be a relative leaf name".to_string(),
        });
    }
    Ok(())
}

/// Apply the executable DIVA allowlist contract to a sender identifier.
///
/// An empty list is allow-all. Non-empty lists retain the legacy wildcard and
/// compound-identifier matching behavior without trusting external owner data.
pub fn is_sender_allowed(allow_from: &[String], sender_id: &str) -> bool {
    if allow_from.is_empty() {
        return true;
    }

    let matches_pattern = |pattern: &str, target: &str| {
        if !pattern.contains('*') {
            return pattern == target;
        }
        let mut regex_pattern = String::from("^");
        for (index, part) in pattern.split('*').enumerate() {
            if index > 0 {
                regex_pattern.push_str(".*");
            }
            regex_pattern.push_str(&regex::escape(part));
        }
        regex_pattern.push('$');
        regex::Regex::new(&regex_pattern)
            .map(|regex| regex.is_match(target))
            .unwrap_or(false)
    };

    if allow_from
        .iter()
        .any(|pattern| matches_pattern(pattern, sender_id))
    {
        return true;
    }
    sender_id.contains('|')
        && sender_id
            .split('|')
            .filter(|part| !part.is_empty())
            .any(|part| {
                allow_from
                    .iter()
                    .any(|pattern| matches_pattern(pattern, part))
            })
}

/// Typed construction failures while native platform adapters are assembled.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AdapterBuildError {
    #[error("native adapters are not available yet for enabled channel(s): {channels:?}")]
    Unavailable { channels: Vec<String> },
}

/// Construct configured native adapters without registering or starting them.
///
/// Gate 1 intentionally reports enabled channels as unavailable until their
/// owner commits land. Returning a typed error prevents a silent empty adapter
/// or a default-success no-op; the Lead wires real constructors after Gate 2.
pub fn build_active_adapters(
    config: &Config,
    _services: AdapterServices,
) -> Result<Vec<Arc<dyn ChannelAdapter>>, AdapterBuildError> {
    let enabled = [
        ("telegram", config.channels.telegram.enabled),
        ("discord", config.channels.discord.enabled),
        ("feishu", config.channels.feishu.enabled),
        ("dingtalk", config.channels.dingtalk.enabled),
        ("email", config.channels.email.enabled),
        ("qq", config.channels.qq.enabled),
    ]
    .into_iter()
    .filter_map(|(channel, enabled)| enabled.then_some(channel.to_string()))
    .collect::<Vec<_>>();

    if enabled.is_empty() {
        Ok(Vec::new())
    } else {
        Err(AdapterBuildError::Unavailable { channels: enabled })
    }
}

/// Build an inbound external-user message without owner context.
pub fn external_message_envelope(
    address: ChannelAddress,
    correlation: Correlation,
    parts: Vec<ContentPart>,
    subject: Option<String>,
    locale: Option<String>,
) -> ChannelEnvelopeV1 {
    ChannelEnvelopeV1::new(
        ChannelDirection::Ingress,
        address,
        correlation,
        ChannelOrigin::ExternalUser,
        ChannelPayloadV1::Message {
            parts,
            subject,
            locale,
            context: None,
        },
    )
}

/// Build a truthful receipt for a platform operation.
pub fn delivery_receipt(
    status: DeliveryStatus,
    channel: impl Into<String>,
    chat_id: impl Into<String>,
    platform_message_id: Option<String>,
    thread_id: Option<String>,
) -> DeliveryReceipt {
    DeliveryReceipt {
        status,
        channel: channel.into(),
        chat_id: chat_id.into(),
        platform_message_id,
        thread_id,
        retry_after_ms: None,
        error_code: None,
        diagnosis: None,
    }
}

/// Build the default receipt for an accepted platform request.
pub fn accepted_receipt(
    channel: impl Into<String>,
    chat_id: impl Into<String>,
    platform_message_id: Option<String>,
    thread_id: Option<String>,
) -> DeliveryReceipt {
    delivery_receipt(
        DeliveryStatus::Accepted,
        channel,
        chat_id,
        platform_message_id,
        thread_id,
    )
}

/// Build a receipt only when the platform explicitly confirms delivery.
pub fn delivered_receipt(
    channel: impl Into<String>,
    chat_id: impl Into<String>,
    platform_message_id: Option<String>,
    thread_id: Option<String>,
) -> DeliveryReceipt {
    delivery_receipt(
        DeliveryStatus::Delivered,
        channel,
        chat_id,
        platform_message_id,
        thread_id,
    )
}

/// Map a platform failure to the stable typed adapter error contract.
pub fn execution_error(
    code: impl Into<String>,
    diagnosis: impl Into<String>,
    retry_after: Option<Duration>,
    retryable: bool,
) -> AdapterError {
    AdapterError::Execution {
        code: code.into(),
        diagnosis: diagnosis.into(),
        retry_after,
        retryable,
    }
}

/// Context supplied to one long-running adapter listener.
#[derive(Debug, Clone)]
pub struct AdapterContext {
    pub fabric: FabricHandle,
    pub cancel: CancellationToken,
}

impl AdapterContext {
    pub fn with_cancel(&self, cancel: CancellationToken) -> Self {
        Self {
            fabric: self.fabric.clone(),
            cancel,
        }
    }
}

/// Typed adapter failure; unsupported operations can never become silent success.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AdapterError {
    #[error("channel capability is unsupported: {capability:?}")]
    UnsupportedCapability { capability: ChannelCapability },
    #[error("adapter rate limited; retry after {retry_after:?}")]
    RateLimited { retry_after: Duration },
    #[error("adapter execution failed ({code}): {diagnosis}")]
    Execution {
        code: String,
        diagnosis: String,
        retry_after: Option<Duration>,
        retryable: bool,
    },
    #[error("adapter stopped")]
    Stopped,
}

impl AdapterError {
    pub fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after } => Some(*retry_after),
            Self::Execution { retry_after, .. } => *retry_after,
            Self::UnsupportedCapability { .. } | Self::Stopped => None,
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self {
            Self::RateLimited { .. } => true,
            Self::Execution { retryable, .. } => *retryable,
            Self::UnsupportedCapability { .. } | Self::Stopped => false,
        }
    }

    pub fn code(&self) -> &str {
        match self {
            Self::UnsupportedCapability { .. } => "unsupported_capability",
            Self::RateLimited { .. } => "rate_limited",
            Self::Execution { code, .. } => code,
            Self::Stopped => "adapter_stopped",
        }
    }
}

/// Clean-break adapter API used by the new Registry and supervisor.
#[async_trait]
pub trait ChannelAdapter: Send + Sync + 'static {
    fn name(&self) -> ChannelId;

    /// Return the current effective snapshot (static declaration merged with live probe).
    fn capabilities(&self) -> ChannelCapabilities;

    /// Run the listener until cancellation, stop, or a transport failure.
    async fn start(&self, context: AdapterContext) -> Result<(), AdapterError>;

    /// Execute one already capability-checked platform command.
    async fn execute(&self, command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError>;

    /// Return the latest local health snapshot without performing network I/O.
    fn health(&self) -> ChannelHealth;

    /// Interrupt platform reads and release adapter-owned resources.
    async fn stop(&self) -> Result<(), AdapterError>;
}
