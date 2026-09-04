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
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::{
    fmt,
    sync::Arc,
    sync::OnceLock,
    time::{Duration, Instant},
};
use thiserror::Error;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::timeout;
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
            .is_some_and(|value| !is_valid_media_type(value))
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
    if !is_valid_media_type(&reference.media_type) {
        return Err(AttachmentStoreError::InvalidReference {
            diagnosis: "media_type is not a valid MIME essence".to_string(),
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

fn is_valid_media_type(value: &str) -> bool {
    let essence = value.split(';').next().map(str::trim).unwrap_or_default();
    let mut components = essence.split('/');
    let Some(kind) = components.next() else {
        return false;
    };
    let Some(subtype) = components.next() else {
        return false;
    };
    !kind.is_empty()
        && !subtype.is_empty()
        && components.next().is_none()
        && !kind.chars().any(char::is_whitespace)
        && !subtype.chars().any(char::is_whitespace)
        && !kind.chars().any(char::is_control)
        && !subtype.chars().any(char::is_control)
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

/// Error returned by an explicit desktop/API channel probe.
///
/// This deliberately stores only stable error codes and retry metadata.  Native
/// adapter diagnostics may contain transport response bodies or other
/// configuration-derived values, so they must not cross the Manager API
/// boundary.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ChannelProbeError {
    #[error("unknown channel: {channel}")]
    UnknownChannel { channel: String },
    #[error("invalid channel probe configuration")]
    InvalidConfig,
    #[error("failed to construct a native channel probe adapter")]
    Build,
    #[error("channel probe timed out")]
    Timeout,
    #[error("channel probe failed ({code})")]
    Adapter {
        code: String,
        retry_after_ms: Option<u64>,
        retryable: bool,
    },
    #[error("channel probe cleanup failed ({code})")]
    Cleanup { code: String },
}

impl ChannelProbeError {
    /// Stable machine-readable code suitable for an HTTP/Tauri error body.
    pub fn code(&self) -> &str {
        match self {
            Self::UnknownChannel { .. } => "unknown_channel",
            Self::InvalidConfig => "invalid_config",
            Self::Build => "probe_build_failed",
            Self::Timeout => "probe_timeout",
            Self::Adapter { code, .. } => code,
            Self::Cleanup { .. } => "probe_cleanup_failed",
        }
    }

    /// Whether retrying the same candidate may succeed without changing it.
    pub fn retryable(&self) -> bool {
        match self {
            Self::Adapter { retryable, .. } => *retryable,
            Self::Timeout => true,
            Self::UnknownChannel { .. }
            | Self::InvalidConfig
            | Self::Build
            | Self::Cleanup { .. } => false,
        }
    }

    /// Retry hint from the native adapter, if one was provided.
    pub fn retry_after_ms(&self) -> Option<u64> {
        match self {
            Self::Adapter { retry_after_ms, .. } => *retry_after_ms,
            _ => None,
        }
    }

    /// Return a bounded code safe to expose outside the adapter process.
    ///
    /// Native adapters currently use fixed snake-case codes, but keeping this
    /// boundary defensive prevents a future transport/parser code from
    /// carrying response text or configuration-derived data.
    pub fn public_code(&self) -> String {
        match self {
            Self::Adapter { code, .. } if is_safe_probe_code(code) => code.clone(),
            Self::Adapter { .. } => "adapter_error".to_string(),
            _ => self.code().to_string(),
        }
    }
}

fn is_safe_probe_code(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 64
        && code.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        })
}

/// Owns cleanup tasks that outlive the public probe response deadline.
///
/// `spawn_blocking` cannot be force-aborted once a native library has entered
/// a blocking call. When a candidate probe times out, this registry keeps the
/// adapter and its cleanup JoinHandle alive until `stop` has actually drained
/// the adapter-owned work. The handle is never dropped at the public timeout
/// boundary.
struct ProbeCleanupRegistry {
    next_id: AtomicU64,
    tasks: Mutex<BTreeMap<u64, JoinHandle<Result<(), AdapterError>>>>,
    reaper_started: AtomicBool,
}

/// A probe occupies both one process-wide slot and one channel slot until the
/// temporary adapter has actually stopped.  The latter is important for
/// blocking adapters: a caller timing out must not be able to start a second
/// DNS/IMAP operation for the same channel while the first one is still held
/// by its cleanup owner.
const MAX_CONCURRENT_PROBES: usize = 4;

struct ProbeAdmission {
    global: Arc<Semaphore>,
    channels: [Arc<Semaphore>; 6],
}

struct ProbeAdmissionLease {
    _global: OwnedSemaphorePermit,
    _channel: OwnedSemaphorePermit,
}

impl ProbeAdmission {
    fn new() -> Self {
        Self {
            global: Arc::new(Semaphore::new(MAX_CONCURRENT_PROBES)),
            channels: std::array::from_fn(|_| Arc::new(Semaphore::new(1))),
        }
    }

    fn acquire(&self, channel: &str) -> Result<ProbeAdmissionLease, ChannelProbeError> {
        let Some(index) = probe_channel_index(channel) else {
            return Err(ChannelProbeError::UnknownChannel {
                channel: channel.to_string(),
            });
        };
        let channel_permit = self.channels[index]
            .clone()
            .try_acquire_owned()
            .map_err(|_| probe_busy_error("probe_busy"))?;
        let global_permit = self
            .global
            .clone()
            .try_acquire_owned()
            .map_err(|_| probe_busy_error("probe_busy"))?;
        Ok(ProbeAdmissionLease {
            _global: global_permit,
            _channel: channel_permit,
        })
    }
}

fn probe_admission() -> &'static ProbeAdmission {
    static ADMISSION: OnceLock<ProbeAdmission> = OnceLock::new();
    ADMISSION.get_or_init(ProbeAdmission::new)
}

fn probe_channel_index(channel: &str) -> Option<usize> {
    match channel {
        "telegram" => Some(0),
        "discord" => Some(1),
        "feishu" => Some(2),
        "dingtalk" => Some(3),
        "email" => Some(4),
        "qq" => Some(5),
        _ => None,
    }
}

fn probe_busy_error(code: &'static str) -> ChannelProbeError {
    ChannelProbeError::Adapter {
        code: code.to_string(),
        retry_after_ms: Some(250),
        retryable: true,
    }
}

impl ProbeCleanupRegistry {
    fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            tasks: Mutex::new(BTreeMap::new()),
            reaper_started: AtomicBool::new(false),
        }
    }

    async fn adopt(self: &Arc<Self>, task: JoinHandle<Result<(), AdapterError>>) {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let mut tasks = self.tasks.lock().await;
        let finished = tasks
            .iter()
            .filter_map(|(id, task)| task.is_finished().then_some(*id))
            .collect::<Vec<_>>();
        for id in finished {
            tasks.remove(&id);
        }
        // Every adopted task owns one ProbeAdmissionLease.  The process-wide
        // semaphore therefore makes this map bounded by
        // MAX_CONCURRENT_PROBES, even when an adapter's blocking stop never
        // returns.  Keep this invariant next to the ownership transfer so a
        // future caller cannot accidentally bypass the admission budget.
        debug_assert!(tasks.len() < MAX_CONCURRENT_PROBES);
        tasks.insert(id, task);
        drop(tasks);
        self.start_reaper();
    }

    fn start_reaper(self: &Arc<Self>) {
        if self
            .reaper_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let registry = Arc::clone(self);
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(20)).await;
                let finished = {
                    let tasks = registry.tasks.lock().await;
                    tasks
                        .iter()
                        .filter_map(|(id, task)| task.is_finished().then_some(*id))
                        .collect::<Vec<_>>()
                };
                for id in finished {
                    let task = registry.tasks.lock().await.remove(&id);
                    if let Some(task) = task {
                        let _ = task.await;
                    }
                }
                if registry.tasks.lock().await.is_empty() {
                    registry.reaper_started.store(false, Ordering::Release);
                    // An adoption may race with the empty check. Re-check
                    // after releasing the flag and keep this reaper alive if
                    // another task arrived before it could restart one.
                    if registry.tasks.lock().await.is_empty() {
                        break;
                    }
                    registry.reaper_started.store(true, Ordering::Release);
                }
            }
        });
    }

    #[cfg(test)]
    async fn pending(&self) -> usize {
        self.tasks.lock().await.len()
    }
}

fn probe_cleanup_registry() -> &'static Arc<ProbeCleanupRegistry> {
    static REGISTRY: OnceLock<Arc<ProbeCleanupRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Arc::new(ProbeCleanupRegistry::new()))
}

#[cfg(test)]
pub(crate) async fn pending_probe_cleanup_tasks() -> usize {
    probe_cleanup_registry().pending().await
}

/// Construct one native adapter for a candidate channel configuration.
///
/// Unlike [`build_active_adapters`], this function is intentionally not tied
/// to the persisted `enabled` flag: a desktop user may validate credentials
/// before saving an enabled configuration.  The candidate is still never
/// registered with [`crate::runtime::AdapterRegistry`] and no listener is
/// started by this function.
pub fn build_probe_adapter(
    config: &Config,
    channel: &str,
    services: AdapterServices,
) -> Result<Arc<dyn ChannelAdapter>, ChannelProbeError> {
    match channel {
        "telegram" => {
            let mut candidate = config.channels.telegram.clone();
            candidate.enabled = true;
            Ok(Arc::new(crate::adapters::telegram::TelegramAdapter::new(
                candidate, services,
            )))
        }
        "discord" => {
            let mut candidate = config.channels.discord.clone();
            candidate.enabled = true;
            Ok(Arc::new(crate::adapters::discord::DiscordAdapter::new(
                candidate, services,
            )))
        }
        "feishu" => {
            let mut candidate = config.channels.feishu.clone();
            candidate.enabled = true;
            Ok(Arc::new(crate::adapters::feishu::FeishuAdapter::new(
                candidate, services,
            )))
        }
        "dingtalk" => {
            let mut candidate = config.channels.dingtalk.clone();
            candidate.enabled = true;
            Ok(Arc::new(crate::adapters::dingtalk::DingTalkAdapter::new(
                &candidate, services,
            )))
        }
        "email" => {
            let mut candidate = config.channels.email.clone();
            candidate.enabled = true;
            Ok(Arc::new(crate::adapters::email::EmailAdapter::new(
                candidate, services,
            )))
        }
        "qq" => {
            let mut candidate = config.channels.qq.clone();
            candidate.enabled = true;
            Ok(Arc::new(crate::adapters::qq::QqAdapter::new(
                candidate, services,
            )))
        }
        other => Err(ChannelProbeError::UnknownChannel {
            channel: other.to_string(),
        }),
    }
}

fn map_probe_adapter_error(error: AdapterError) -> ChannelProbeError {
    ChannelProbeError::Adapter {
        code: if is_safe_probe_code(error.code()) {
            error.code().to_string()
        } else {
            "adapter_error".to_string()
        },
        retry_after_ms: error
            .retry_after()
            .and_then(|duration| u64::try_from(duration.as_millis()).ok()),
        retryable: error.is_retryable(),
    }
}

enum ProbeExecutionResult {
    Receipt(DeliveryReceipt),
    Adapter(AdapterError),
    Timeout,
}

fn finalize_probe_result(
    probe_result: ProbeExecutionResult,
    cleanup_error: Option<ChannelProbeError>,
) -> Result<DeliveryReceipt, ChannelProbeError> {
    // The operation result is more useful and deterministic than a cleanup
    // diagnostic.  Cleanup failures are surfaced only when the probe itself
    // succeeded; a timed-out/failed probe keeps its primary error class.
    match (probe_result, cleanup_error) {
        (ProbeExecutionResult::Receipt(_receipt), Some(error)) => Err(error),
        (ProbeExecutionResult::Receipt(receipt), None) => Ok(receipt),
        (ProbeExecutionResult::Adapter(error), _) => Err(map_probe_adapter_error(error)),
        (ProbeExecutionResult::Timeout, _) => Err(ChannelProbeError::Timeout),
    }
}

/// Execute one native `ProbeHealth` command and always stop the temporary
/// adapter afterward.  The adapter is never registered, supervised, or
/// started as a listener.
pub async fn probe_candidate(
    config: &Config,
    channel: &str,
    services: AdapterServices,
    probe_timeout: Duration,
) -> Result<DeliveryReceipt, ChannelProbeError> {
    let adapter = build_probe_adapter(config, channel, services)?;
    probe_adapter(adapter, channel, probe_timeout).await
}

/// Run and clean up a pre-built candidate adapter. This is kept crate-visible
/// for adapter lifecycle tests; production callers should use
/// [`probe_candidate`] so construction remains restricted to native factories.
pub(crate) async fn probe_adapter(
    adapter: Arc<dyn ChannelAdapter>,
    channel: &str,
    probe_timeout: Duration,
) -> Result<DeliveryReceipt, ChannelProbeError> {
    let deadline = Instant::now() + probe_timeout;
    let channel_id =
        ChannelId::new(channel.to_string()).map_err(|_| ChannelProbeError::UnknownChannel {
            channel: channel.to_string(),
        })?;
    let admission = probe_admission().acquire(channel)?;

    let probe_result = match timeout(
        deadline.saturating_duration_since(Instant::now()),
        adapter.execute(ChannelCommand::ProbeHealth {
            channel: channel_id,
        }),
    )
    .await
    {
        Ok(Ok(receipt)) => ProbeExecutionResult::Receipt(receipt),
        Ok(Err(error)) => ProbeExecutionResult::Adapter(error),
        Err(_) => ProbeExecutionResult::Timeout,
    };

    // Cleanup is always owned by a JoinHandle. If an adapter needs longer
    // than the response grace period to drain, transfer that handle to the
    // process-level registry instead of dropping it and detaching work.
    let cleanup_adapter = adapter.clone();
    let mut cleanup = tokio::spawn(async move {
        let result = cleanup_adapter.stop().await;
        // Keep the lease inside the cleanup owner.  If the public probe
        // deadline expires, dropping the response future cannot release the
        // per-channel/global slot before stop has completed.
        drop(admission);
        result
    });
    let cleanup_error = match timeout(
        deadline.saturating_duration_since(Instant::now()),
        &mut cleanup,
    )
    .await
    {
        Ok(Ok(Ok(()))) => None,
        Ok(Ok(Err(error))) => Some(ChannelProbeError::Cleanup {
            code: if is_safe_probe_code(error.code()) {
                error.code().to_string()
            } else {
                "cleanup_error".to_string()
            },
        }),
        Ok(Err(_)) => Some(ChannelProbeError::Cleanup {
            code: "cleanup_task_failed".to_string(),
        }),
        Err(_) => {
            // The JoinHandle remains the sole owner of the adapter and lease
            // after this transfer.  Its registry is bounded by the admission
            // semaphore above, so a permanently blocked DNS task cannot turn
            // retries into an unbounded cleanup queue.
            probe_cleanup_registry().adopt(cleanup).await;
            None
        }
    };

    finalize_probe_result(probe_result, cleanup_error)
}

/// Construct configured native adapters without registering or starting them.
///
/// C5 assembles native channel objects only. Registration, supervision, and
/// listener startup remain the responsibility of the C6 runtime cutover.
pub fn build_active_adapters(
    config: &Config,
    services: AdapterServices,
) -> Result<Vec<Arc<dyn ChannelAdapter>>, AdapterBuildError> {
    let mut adapters: Vec<Arc<dyn ChannelAdapter>> = Vec::new();

    if config.channels.discord.enabled && !config.channels.removed.contains("discord") {
        adapters.push(Arc::new(crate::adapters::discord::DiscordAdapter::new(
            config.channels.discord.clone(),
            services.clone(),
        )));
    }
    if config.channels.dingtalk.enabled && !config.channels.removed.contains("dingtalk") {
        adapters.push(Arc::new(crate::adapters::dingtalk::DingTalkAdapter::new(
            &config.channels.dingtalk,
            services.clone(),
        )));
    }
    if config.channels.email.enabled && !config.channels.removed.contains("email") {
        adapters.push(Arc::new(crate::adapters::email::EmailAdapter::new(
            config.channels.email.clone(),
            services.clone(),
        )));
    }
    if config.channels.feishu.enabled && !config.channels.removed.contains("feishu") {
        adapters.push(Arc::new(crate::adapters::feishu::FeishuAdapter::new(
            config.channels.feishu.clone(),
            services.clone(),
        )));
    }
    if config.channels.telegram.enabled && !config.channels.removed.contains("telegram") {
        adapters.push(Arc::new(crate::adapters::telegram::TelegramAdapter::new(
            config.channels.telegram.clone(),
            services.clone(),
        )));
    }
    if config.channels.qq.enabled && !config.channels.removed.contains("qq") {
        adapters.push(Arc::new(crate::adapters::qq::QqAdapter::new(
            config.channels.qq.clone(),
            services.clone(),
        )));
    }

    Ok(adapters)
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

#[cfg(test)]
mod probe_error_tests {
    use super::{
        finalize_probe_result, AdapterError, ChannelProbeError, ProbeAdmission,
        ProbeExecutionResult,
    };

    #[tokio::test]
    async fn probe_admission_rejects_duplicate_channel_until_cleanup_releases() {
        let admission = ProbeAdmission::new();
        let lease = admission.acquire("email").expect("email slot");
        let error = match admission.acquire("email") {
            Ok(_) => panic!("duplicate email probe must be busy"),
            Err(error) => error,
        };
        assert_eq!(error.public_code(), "probe_busy");
        drop(lease);
        assert!(admission.acquire("email").is_ok());
    }

    #[test]
    fn probe_admission_rejects_work_beyond_global_capacity() {
        let admission = ProbeAdmission::new();
        let leases = ["telegram", "discord", "feishu", "dingtalk"]
            .into_iter()
            .map(|channel| admission.acquire(channel).expect("global probe slot"))
            .collect::<Vec<_>>();

        let error = match admission.acquire("email") {
            Ok(_) => panic!("global probe capacity must be bounded"),
            Err(error) => error,
        };
        assert_eq!(error.public_code(), "probe_busy");
        drop(leases);
        assert!(admission.acquire("email").is_ok());
    }

    #[test]
    fn public_probe_codes_drop_untrusted_diagnostics() {
        let error = ChannelProbeError::Adapter {
            code: "remote response: bearer-secret".to_string(),
            retry_after_ms: None,
            retryable: false,
        };
        assert_eq!(error.public_code(), "adapter_error");
    }

    #[test]
    fn public_probe_codes_preserve_stable_native_codes() {
        let error = ChannelProbeError::Adapter {
            code: "telegram_api".to_string(),
            retry_after_ms: Some(1000),
            retryable: true,
        };
        assert_eq!(error.public_code(), "telegram_api");
    }

    #[test]
    fn probe_error_takes_precedence_over_cleanup_error() {
        let cleanup = Some(ChannelProbeError::Cleanup {
            code: "cleanup_failed".to_string(),
        });
        let result = finalize_probe_result(
            ProbeExecutionResult::Adapter(AdapterError::Stopped),
            cleanup,
        );

        assert_eq!(
            result.expect_err("probe failure").public_code(),
            "adapter_stopped"
        );
    }

    #[test]
    fn probe_timeout_takes_precedence_over_cleanup_error() {
        let cleanup = Some(ChannelProbeError::Cleanup {
            code: "cleanup_failed".to_string(),
        });
        let result = finalize_probe_result(ProbeExecutionResult::Timeout, cleanup);

        assert_eq!(
            result.expect_err("probe timeout"),
            ChannelProbeError::Timeout
        );
    }

    #[test]
    fn successful_probe_surfaces_cleanup_error() {
        let cleanup = Some(ChannelProbeError::Cleanup {
            code: "cleanup_failed".to_string(),
        });
        let result = finalize_probe_result(
            ProbeExecutionResult::Receipt(super::accepted_receipt("email", "probe", None, None)),
            cleanup.clone(),
        );

        assert_eq!(result, Err(cleanup.expect("cleanup error")));
    }
}
