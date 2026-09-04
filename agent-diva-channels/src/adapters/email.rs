//! Native Email adapter for the Super Channel Fabric.
//!
//! This module deliberately keeps the existing DIVA IMAP/SMTP behavior instead
//! of routing through `EmailHandler`.  IMAP and SMTP are blocking libraries in
//! this workspace, so every blocking operation is isolated behind
//! `spawn_blocking`; the Fabric admission and cancellation boundary remains
//! asynchronous and bounded.

use crate::adapter::{
    accepted_receipt, external_message_envelope, is_sender_allowed, AdapterContext, AdapterError,
    AdapterServices, AttachmentStoreError, ChannelAdapter, IngressAttachment,
};
use agent_diva_core::channel::{
    ChannelAddress, ChannelCapabilities, ChannelCapability, ChannelCommand, ChannelHealth,
    ChannelHealthStatus, ChannelId, ChannelPayloadV1, ContentPart, Correlation, DeliveryReceipt,
    FabricAdmissionError,
};
use agent_diva_core::config::schema::EmailConfig;
use async_trait::async_trait;
use chrono::Utc;
use mail_parser::{MessageParser, MimeHeaders, PartType};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::{Duration, Instant};
use thiserror::Error;
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

const DEFAULT_MAILBOX: &str = "INBOX";
const DEFAULT_POLL_INTERVAL_SECS: u64 = 30;
const MAX_EMAIL_TOPIC_CHARS: usize = 120;
const MAX_ATTACHMENT_BYTES: u64 = 25 * 1024 * 1024;
const FABRIC_ADMISSION_DEADLINE: Duration = Duration::from_secs(1);
const IMAP_IO_TIMEOUT: Duration = Duration::from_secs(15);
const SMTP_IO_TIMEOUT: Duration = Duration::from_secs(15);
const BLOCKING_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
// A cancelled waiter cannot abort a `spawn_blocking` operation.  Keep a hard
// per-adapter cap so repeated sends/probes cannot accumulate an unbounded set
// of native blocking workers while an external DNS/IMAP call is wedged.
const MAX_BLOCKING_TASKS: usize = 8;
// Health probing is deliberately much shorter than ordinary mail polling or
// delivery. This gives the owning BlockingTaskRegistry a small, explicit
// drain bound after the public probe budget expires.
const EMAIL_PROBE_OPERATION_TIMEOUT: Duration = Duration::from_secs(3);
const EMAIL_PROBE_IO_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, Clone, PartialEq, Eq, Error)]
enum EmailTransportError {
    #[error("email transport failed: {diagnosis}")]
    Failed { diagnosis: String },
    #[error("invalid RFC822 message: {diagnosis}")]
    InvalidMessage { diagnosis: String },
    #[error("invalid MIME type: {media_type}")]
    InvalidMime { media_type: String },
    #[error("email attachment is too large ({size_bytes} > {max_bytes} bytes)")]
    TooLarge { size_bytes: u64, max_bytes: u64 },
    #[error("invalid {field} address: {diagnosis}")]
    InvalidAddress {
        field: &'static str,
        diagnosis: String,
    },
}

fn transport_failure(diagnosis: impl Into<String>) -> EmailTransportError {
    EmailTransportError::Failed {
        diagnosis: diagnosis.into(),
    }
}

#[derive(Debug, Error)]
enum BlockingTaskError {
    #[error("blocking task registry is closed")]
    Closed,
    #[error("blocking task registry is at capacity")]
    AtCapacity,
    #[error("blocking task did not return a result")]
    MissingResult,
    #[error("blocking task panicked: {diagnosis}")]
    Join { diagnosis: String },
}

/// Owns every `spawn_blocking` handle created by one adapter.
///
/// A cancelled waiter may stop observing a blocking operation, but the handle
/// remains owned here until `close_and_drain` joins it. This prevents an IMAP
/// or SMTP operation from becoming detached work after adapter shutdown.
struct BlockingTaskRegistry {
    closed: AtomicBool,
    next_id: AtomicU64,
    tasks: Mutex<HashMap<u64, JoinHandle<()>>>,
}

impl Default for BlockingTaskRegistry {
    fn default() -> Self {
        Self {
            closed: AtomicBool::new(false),
            next_id: AtomicU64::new(1),
            tasks: Mutex::new(HashMap::new()),
        }
    }
}

impl BlockingTaskRegistry {
    async fn run<R, F>(&self, operation: F) -> Result<R, BlockingTaskError>
    where
        R: Send + 'static,
        F: FnOnce() -> R + Send + 'static,
    {
        let mut tasks = self.tasks.lock().await;
        // A caller may time out while the native operation continues.  Once
        // that operation has completed, its orphaned JoinHandle no longer
        // owns blocking work and must not consume a capacity slot forever.
        tasks.retain(|_, task| !task.is_finished());
        if self.closed.load(Ordering::Acquire) {
            return Err(BlockingTaskError::Closed);
        }
        if tasks.len() >= MAX_BLOCKING_TASKS {
            return Err(BlockingTaskError::AtCapacity);
        }
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (sender, receiver) = oneshot::channel();
        let handle = tokio::task::spawn_blocking(move || {
            let result = operation();
            let _ = sender.send(result);
        });
        tasks.insert(id, handle);
        drop(tasks);

        let result = receiver.await.map_err(|_| BlockingTaskError::MissingResult);
        let handle = self.tasks.lock().await.remove(&id);
        if let Some(handle) = handle {
            handle.await.map_err(|error| BlockingTaskError::Join {
                diagnosis: error.to_string(),
            })?;
        }
        result
    }

    async fn close_and_drain(&self) -> Result<(), String> {
        self.closed.store(true, Ordering::Release);
        let handles = self
            .tasks
            .lock()
            .await
            .drain()
            .map(|(_, task)| task)
            .collect::<Vec<_>>();
        let mut first_error = None;
        for handle in handles {
            if let Err(error) = handle.await {
                first_error.get_or_insert_with(|| error.to_string());
            }
        }
        first_error.map_or(Ok(()), Err)
    }
}

/// A parsed RFC822 message kept independent of the blocking IMAP session.
#[derive(Debug, Clone)]
struct ParsedEmail {
    sender: String,
    subject: String,
    message_id: Option<String>,
    in_reply_to: Option<String>,
    references: Option<String>,
    thread_topic: String,
    text_body: String,
    uid: String,
    date: String,
    attachments: Vec<ParsedAttachment>,
}

#[derive(Debug, Clone)]
struct ParsedAttachment {
    file_name: Option<String>,
    media_type: String,
    bytes: Vec<u8>,
}

impl ParsedEmail {
    fn dedup_key(&self) -> String {
        self.message_id
            .clone()
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| format!("uid:{}", self.uid))
    }

    fn platform_message_id(&self) -> String {
        self.message_id
            .clone()
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| format!("uid:{}", self.uid))
    }
}

/// Native Email adapter.  The constructor is crate-visible so only the C6
/// factory can assemble production adapters; tests use the private fixture
/// helpers in this module.
pub struct EmailAdapter {
    config: EmailConfig,
    services: AdapterServices,
    transport: Arc<dyn EmailTransport>,
    processed: Arc<Mutex<HashSet<String>>>,
    in_flight: Arc<Mutex<HashSet<String>>>,
    pending_seen: Arc<Mutex<HashMap<String, String>>>,
    blocking: Arc<BlockingTaskRegistry>,
    shutdown: CancellationToken,
    health: Arc<StdRwLock<ChannelHealth>>,
    next_message_id: AtomicU64,
}

/// Blocking IMAP/SMTP operations are isolated behind this private seam. The
/// production implementation keeps the existing libraries, while tests can
/// run a deterministic fake IMAP/SMTP transcript without touching a network.
trait EmailTransport: Send + Sync + 'static {
    fn fetch_messages(
        &self,
        config: &EmailConfig,
        mailbox: &str,
        processed: &HashSet<String>,
    ) -> Result<Vec<ParsedEmail>, EmailTransportError>;

    fn mark_seen(
        &self,
        config: &EmailConfig,
        mailbox: &str,
        uid: &str,
    ) -> Result<(), EmailTransportError>;

    fn send(
        &self,
        config: &EmailConfig,
        message: SmtpMessage,
    ) -> Result<String, EmailTransportError>;

    fn probe(&self, config: &EmailConfig, deadline: Instant) -> Result<(), EmailTransportError>;

    fn record_event(&self, _event: &str) {}
}

struct SystemEmailTransport;

impl EmailTransport for SystemEmailTransport {
    fn fetch_messages(
        &self,
        config: &EmailConfig,
        mailbox: &str,
        processed: &HashSet<String>,
    ) -> Result<Vec<ParsedEmail>, EmailTransportError> {
        fetch_messages_blocking(config, mailbox, processed)
    }

    fn mark_seen(
        &self,
        config: &EmailConfig,
        mailbox: &str,
        uid: &str,
    ) -> Result<(), EmailTransportError> {
        mark_seen_blocking(config, mailbox, uid)
    }

    fn send(
        &self,
        config: &EmailConfig,
        message: SmtpMessage,
    ) -> Result<String, EmailTransportError> {
        smtp_send_blocking(config, message)
    }

    fn probe(&self, config: &EmailConfig, deadline: Instant) -> Result<(), EmailTransportError> {
        probe_email_blocking_until(config, deadline)
    }
}

impl EmailAdapter {
    /// Construct a native adapter without registering or starting it.
    pub(crate) fn new(config: EmailConfig, services: AdapterServices) -> Self {
        Self::with_transport(config, services, Arc::new(SystemEmailTransport))
    }

    fn with_transport(
        config: EmailConfig,
        services: AdapterServices,
        transport: Arc<dyn EmailTransport>,
    ) -> Self {
        Self {
            config,
            services,
            transport,
            processed: Arc::new(Mutex::new(HashSet::new())),
            in_flight: Arc::new(Mutex::new(HashSet::new())),
            pending_seen: Arc::new(Mutex::new(HashMap::new())),
            blocking: Arc::new(BlockingTaskRegistry::default()),
            shutdown: CancellationToken::new(),
            health: Arc::new(StdRwLock::new(ChannelHealth::new(
                ChannelHealthStatus::Unknown,
            ))),
            next_message_id: AtomicU64::new(1),
        }
    }

    fn validate_config(&self) -> Result<(), AdapterError> {
        let mut missing = Vec::new();
        if self.config.imap_host.trim().is_empty() {
            missing.push("imap_host");
        }
        if self.config.imap_username.trim().is_empty() {
            missing.push("imap_username");
        }
        if self.config.imap_password.is_empty() {
            missing.push("imap_password");
        }
        if self.config.smtp_host.trim().is_empty() {
            missing.push("smtp_host");
        }
        if self.config.smtp_username.trim().is_empty() {
            missing.push("smtp_username");
        }
        if self.config.smtp_password.is_empty() {
            missing.push("smtp_password");
        }
        if self.config.from_address.trim().is_empty()
            && self.config.smtp_username.trim().is_empty()
            && self.config.imap_username.trim().is_empty()
        {
            missing.push("from_address");
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(AdapterError::Execution {
                code: "invalid_config".to_string(),
                diagnosis: format!("missing required email fields: {}", missing.join(", ")),
                retry_after: None,
                retryable: false,
            })
        }
    }

    fn mark_healthy(&self) {
        if let Ok(mut health) = self.health.write() {
            health.status = ChannelHealthStatus::Healthy;
            health.diagnosis = None;
            health.consecutive_failures = 0;
            health.checked_at = Utc::now();
        }
    }

    fn mark_degraded(&self, diagnosis: impl Into<String>) {
        if let Ok(mut health) = self.health.write() {
            health.status = ChannelHealthStatus::Degraded;
            health.diagnosis = Some(diagnosis.into());
            health.consecutive_failures = health.consecutive_failures.saturating_add(1);
            health.checked_at = Utc::now();
        }
    }

    fn mark_down(&self) {
        if let Ok(mut health) = self.health.write() {
            health.status = ChannelHealthStatus::Down;
            health.checked_at = Utc::now();
        }
    }

    fn mailbox(&self) -> &str {
        if self.config.imap_mailbox.trim().is_empty() {
            DEFAULT_MAILBOX
        } else {
            self.config.imap_mailbox.trim()
        }
    }

    fn poll_interval(&self) -> Duration {
        Duration::from_secs(if self.config.poll_interval_seconds == 0 {
            DEFAULT_POLL_INTERVAL_SECS
        } else {
            self.config.poll_interval_seconds
        })
    }

    fn capabilities_snapshot(&self) -> ChannelCapabilities {
        let mut capabilities = ChannelCapabilities::new([
            ChannelCapability::IngressText,
            ChannelCapability::IngressThread,
            ChannelCapability::IngressDirect,
            ChannelCapability::IngressTypedAttachments,
            ChannelCapability::IngressDedupId,
            ChannelCapability::EgressText,
            ChannelCapability::EgressReply,
            ChannelCapability::EgressImage,
            ChannelCapability::EgressAudio,
            ChannelCapability::EgressVideo,
            ChannelCapability::EgressFile,
            ChannelCapability::ReliabilityHealth,
            ChannelCapability::ReliabilityPacing,
            ChannelCapability::ReliabilitySupervisedRestart,
        ]);
        capabilities.limits.max_attachment_bytes = Some(MAX_ATTACHMENT_BYTES);
        capabilities
    }

    async fn process_email(
        &self,
        context: &AdapterContext,
        email: ParsedEmail,
    ) -> Result<bool, AdapterError> {
        let key = email.dedup_key();
        {
            let mut in_flight = self.in_flight.lock().await;
            if self.processed.lock().await.contains(&key) || in_flight.contains(&key) {
                return Ok(false);
            }
            in_flight.insert(key.clone());
        }

        let result = self
            .process_email_reserved(context, email, key.clone())
            .await;
        self.in_flight.lock().await.remove(&key);
        result
    }

    async fn process_email_reserved(
        &self,
        context: &AdapterContext,
        email: ParsedEmail,
        key: String,
    ) -> Result<bool, AdapterError> {
        // Access policy and self-reply filtering happen before attachment
        // extraction/storage or Fabric admission.  Rejected mail remains
        // unseen, allowing a later policy change to reconsider it.
        if !email_sender_allowed(&self.config.allow_from, &email.sender) {
            debug!("email sender rejected by allowlist");
            return Ok(false);
        }
        if should_skip_self_reply(&email.sender, &email.subject, &self.config) {
            debug!("self-reply rejected to prevent an email loop");
            return Ok(false);
        }

        let mut parts = Vec::new();
        if !email.text_body.is_empty() {
            parts.push(ContentPart::Text {
                text: email.text_body.clone(),
            });
        }

        // Validate every MIME part before the first attachment-store write.
        // This keeps malformed platform data from producing partial side
        // effects or a misleading Fabric admission.
        for attachment in &email.attachments {
            validate_attachment_metadata(
                attachment.file_name.as_deref(),
                &attachment.media_type,
                attachment.bytes.len() as u64,
            )?;
        }

        for attachment in &email.attachments {
            let reference = self
                .services
                .attachments
                .put(IngressAttachment {
                    source_channel: "email".to_string(),
                    platform_message_id: Some(email.platform_message_id()),
                    sender_id: Some(email.sender.clone()),
                    file_name: attachment.file_name.clone(),
                    declared_mime: Some(attachment.media_type.clone()),
                    bytes: attachment.bytes.clone(),
                })
                .await
                .map_err(|error| AdapterError::Execution {
                    code: "attachment_store_failed".to_string(),
                    diagnosis: error.to_string(),
                    retry_after: None,
                    retryable: true,
                })?;

            let part = if attachment.media_type.starts_with("image/") {
                ContentPart::Image {
                    attachment: reference,
                }
            } else if attachment.media_type.starts_with("audio/") {
                ContentPart::Audio {
                    attachment: reference,
                    transcript: None,
                }
            } else if attachment.media_type.starts_with("video/") {
                ContentPart::Video {
                    attachment: reference,
                }
            } else {
                ContentPart::File {
                    attachment: reference,
                }
            };
            parts.push(part);
        }

        if parts.is_empty() {
            return Ok(false);
        }

        let mut address = ChannelAddress::new("email", email.sender.clone());
        address.account_id = Some(self.config.imap_username.clone());
        address.thread_id = Some(email.thread_topic.clone());

        let mut correlation = Correlation::new(format!("email:{}", email.sender));
        correlation.message_id = Some(email.platform_message_id());
        correlation.reply_to = email.in_reply_to.clone();

        let subject = (!email.subject.trim().is_empty()).then(|| email.subject.clone());
        let mut envelope = external_message_envelope(address, correlation, parts, subject, None);
        envelope
            .extensions
            .insert("email.uid".to_string(), json!(email.uid));
        envelope
            .extensions
            .insert("email.thread_topic".to_string(), json!(email.thread_topic));
        envelope
            .extensions
            .insert("email.message_id".to_string(), json!(email.message_id));
        envelope
            .extensions
            .insert("email.in_reply_to".to_string(), json!(email.in_reply_to));
        envelope
            .extensions
            .insert("email.references".to_string(), json!(email.references));
        envelope
            .extensions
            .insert("email.date".to_string(), json!(email.date));

        let admission = tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => return Err(AdapterError::Stopped),
            result = context.fabric.admit_ingress(
                envelope,
                FABRIC_ADMISSION_DEADLINE,
                &context.cancel,
            ) => result,
        };
        admission.map_err(map_fabric_error)?;
        self.transport.record_event("fabric.admission.accepted");

        // The marker is committed only after Fabric acceptance.  Mark-seen is
        // a separate blocking operation so a busy Fabric leaves the message
        // eligible for retry.
        self.processed.lock().await.insert(key);
        if self.config.mark_seen {
            self.pending_seen
                .lock()
                .await
                .insert(email.uid.clone(), email.dedup_key());
            if let Err(error) = self
                .mark_seen_once(email.uid.clone(), &context.cancel)
                .await
            {
                if !matches!(error, AdapterError::Stopped) {
                    self.mark_degraded(format!("IMAP mark-seen failed: {error}"));
                }
                warn!("IMAP mark-seen failed: {error}");
            } else {
                self.pending_seen.lock().await.remove(&email.uid);
            }
        }
        Ok(true)
    }

    async fn run_blocking<R, F>(
        &self,
        timeout_code: &'static str,
        operation: F,
    ) -> Result<R, AdapterError>
    where
        R: Send + 'static,
        F: FnOnce() -> R + Send + 'static,
    {
        self.run_blocking_with_timeout(BLOCKING_OPERATION_TIMEOUT, timeout_code, operation)
            .await
    }

    async fn run_blocking_with_timeout<R, F>(
        &self,
        timeout: Duration,
        timeout_code: &'static str,
        operation: F,
    ) -> Result<R, AdapterError>
    where
        R: Send + 'static,
        F: FnOnce() -> R + Send + 'static,
    {
        tokio::time::timeout(timeout, self.blocking.run(operation))
            .await
            .map_err(|_| AdapterError::Execution {
                code: timeout_code.to_string(),
                diagnosis: "blocking email operation exceeded its deadline".to_string(),
                retry_after: None,
                retryable: true,
            })?
            .map_err(|error| AdapterError::Execution {
                code: "blocking_task_failed".to_string(),
                diagnosis: error.to_string(),
                retry_after: None,
                retryable: true,
            })
    }

    async fn mark_seen_once(
        &self,
        uid: String,
        context_cancel: &CancellationToken,
    ) -> Result<(), AdapterError> {
        let config = self.config.clone();
        let mailbox = self.mailbox().to_string();
        let transport = self.transport.clone();
        let operation = self.run_blocking("imap_mark_seen_timeout", move || {
            transport.mark_seen(&config, &mailbox, &uid)
        });
        let result = tokio::select! {
            result = operation => result
                .and_then(|result| result.map_err(|error| {
                    map_email_transport_error("imap_mark_seen_failed", error)
                })),
            _ = self.shutdown.cancelled() => Err(AdapterError::Stopped),
            _ = context_cancel.cancelled() => Err(AdapterError::Stopped),
        };
        result
    }

    async fn retry_pending_seen(&self, context_cancel: &CancellationToken) {
        let pending = self.pending_seen.lock().await.clone();
        for (uid, _) in pending {
            if self.shutdown.is_cancelled() || context_cancel.is_cancelled() {
                return;
            }
            match self.mark_seen_once(uid.clone(), context_cancel).await {
                Ok(()) => {
                    self.pending_seen.lock().await.remove(&uid);
                    debug!(uid, "email pending mark-seen retry succeeded");
                }
                Err(error) => {
                    if !matches!(error, AdapterError::Stopped) {
                        self.mark_degraded(format!("IMAP mark-seen retry failed: {error}"));
                    }
                    warn!(uid, "IMAP mark-seen retry failed: {error}");
                }
            }
        }
    }

    async fn poll_once(&self, context: &AdapterContext) -> Result<usize, AdapterError> {
        self.retry_pending_seen(&context.cancel).await;
        if self.shutdown.is_cancelled() || context.cancel.is_cancelled() {
            return Ok(0);
        }

        let processed = self.processed.lock().await.clone();
        let config = self.config.clone();
        let mailbox = self.mailbox().to_string();
        let transport = self.transport.clone();
        let fetch = self.run_blocking("imap_poll_timeout", move || {
            transport.fetch_messages(&config, &mailbox, &processed)
        });

        let fetch_result = tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => return Ok(0),
            _ = context.cancel.cancelled() => return Ok(0),
            result = fetch => result,
        };
        let messages = match fetch_result.and_then(|result| {
            result.map_err(|error| map_email_transport_error("imap_poll_failed", error))
        }) {
            Ok(messages) => messages,
            Err(error) => {
                self.mark_degraded(error.to_string());
                return Err(error);
            }
        };

        let mut accepted = 0;
        for email in messages {
            if self.shutdown.is_cancelled() || context.cancel.is_cancelled() {
                break;
            }
            match self.process_email(context, email).await {
                Ok(true) => accepted += 1,
                Ok(false) => {}
                Err(error) => {
                    self.mark_degraded(error.to_string());
                    return Err(error);
                }
            }
        }
        if self.pending_seen.lock().await.is_empty() {
            self.mark_healthy();
        }
        Ok(accepted)
    }

    async fn run_poll_loop(&self, context: AdapterContext) -> Result<(), AdapterError> {
        let interval = self.poll_interval();
        loop {
            if self.shutdown.is_cancelled() || context.cancel.is_cancelled() {
                self.mark_down();
                let _ = self.close_blocking_tasks().await;
                return Ok(());
            }

            match self.poll_once(&context).await {
                Ok(count) => {
                    if count > 0 {
                        info!(count, "Email adapter admitted messages");
                    }
                }
                Err(error) => {
                    self.mark_degraded(error.to_string());
                    warn!("Email adapter poll failed: {error}");
                }
            }

            tokio::select! {
                biased;
                _ = self.shutdown.cancelled() => {
                    self.mark_down();
                    let _ = self.close_blocking_tasks().await;
                    return Ok(());
                }
                _ = context.cancel.cancelled() => {
                    self.mark_down();
                    let _ = self.close_blocking_tasks().await;
                    return Ok(());
                }
                _ = tokio::time::sleep(interval) => {}
            }
        }
    }

    async fn close_blocking_tasks(&self) -> Result<(), AdapterError> {
        self.blocking
            .close_and_drain()
            .await
            .map_err(|diagnosis| AdapterError::Execution {
                code: "blocking_shutdown_failed".to_string(),
                diagnosis,
                retry_after: None,
                retryable: true,
            })
    }

    async fn execute_send(
        &self,
        envelope: agent_diva_core::channel::ChannelEnvelopeV1,
    ) -> Result<DeliveryReceipt, AdapterError> {
        if self.shutdown.is_cancelled() {
            return Err(AdapterError::Stopped);
        }

        let (parts, payload_subject) = match &envelope.payload {
            ChannelPayloadV1::Message { parts, subject, .. } => (
                parts.clone(),
                subject
                    .clone()
                    .or_else(|| subject_from_extensions(&envelope)),
            ),
            ChannelPayloadV1::Stream { parts, .. } => {
                (parts.clone(), subject_from_extensions(&envelope))
            }
            _ => {
                return Err(AdapterError::Execution {
                    code: "invalid_send_payload".to_string(),
                    diagnosis: "email adapter accepts Message or final Stream payloads".to_string(),
                    retry_after: None,
                    retryable: false,
                });
            }
        };

        // Validate content capabilities before consent or attachment reads so
        // an unsupported command has no external side effect at all.
        let body = text_body_from_parts(&parts)?;
        if !self.config.consent_granted {
            return Err(AdapterError::Execution {
                code: "consent_required".to_string(),
                diagnosis: "email sending requires explicit consent_granted".to_string(),
                retry_after: None,
                retryable: false,
            });
        }

        let recipient = envelope.address.chat_id.trim().to_string();
        validate_recipient_address(&recipient)?;

        let subject = payload_subject
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_string())
            .unwrap_or_else(|| self.reply_subject(""));

        let in_reply_to = envelope.correlation.reply_to.clone();
        if in_reply_to.is_some() && !self.config.auto_reply_enabled {
            return Err(AdapterError::Execution {
                code: "auto_reply_disabled".to_string(),
                diagnosis: "automatic email replies are disabled by configuration".to_string(),
                retry_after: None,
                retryable: false,
            });
        }

        let references = envelope
            .extensions
            .get("email.references")
            .and_then(|value| value.as_str())
            .map(ToOwned::to_owned)
            .or_else(|| in_reply_to.clone());
        let from = if !self.config.from_address.trim().is_empty() {
            self.config.from_address.trim().to_string()
        } else if !self.config.smtp_username.trim().is_empty() {
            self.config.smtp_username.trim().to_string()
        } else {
            self.config.imap_username.trim().to_string()
        };
        validate_from_address(&from)?;
        let attachments = self.outbound_attachments(&parts).await?;

        if body.is_empty() && attachments.is_empty() {
            return Ok(accepted_receipt(
                "email",
                recipient,
                None,
                envelope.address.thread_id,
            ));
        }

        let message_id = self.next_message_id.fetch_add(1, Ordering::Relaxed);
        let generated_id = format!(
            "<agent-diva-{}-{}@{}>",
            message_id,
            uuid::Uuid::new_v4(),
            message_id_domain(&from)
        );
        let config = self.config.clone();
        let recipient_for_task = recipient.clone();
        let subject_for_task = subject.clone();
        let body_for_task = body.clone();
        let references_for_task = references.clone();
        let in_reply_to_for_task = in_reply_to.clone();
        let from_for_task = from.clone();
        let generated_for_task = generated_id.clone();
        let smtp_message = SmtpMessage {
            from: from_for_task,
            to: recipient_for_task,
            subject: subject_for_task,
            body: body_for_task,
            in_reply_to: in_reply_to_for_task,
            references: references_for_task,
            attachments,
            message_id: generated_for_task,
        };
        let transport = self.transport.clone();
        let send = self.run_blocking("smtp_send_timeout", move || {
            transport.send(&config, smtp_message)
        });

        let result = tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => return Err(AdapterError::Stopped),
            result = send => result?
                .map_err(|error| map_email_transport_error("smtp_send_failed", error))?,
        };

        self.mark_healthy();
        Ok(accepted_receipt(
            "email",
            recipient,
            Some(result),
            envelope.address.thread_id,
        ))
    }

    async fn outbound_attachments(
        &self,
        parts: &[ContentPart],
    ) -> Result<Vec<OutboundAttachment>, AdapterError> {
        let mut attachments = Vec::new();
        for part in parts {
            let reference = match part {
                ContentPart::Image { attachment }
                | ContentPart::Audio { attachment, .. }
                | ContentPart::Video { attachment }
                | ContentPart::File { attachment } => attachment,
                ContentPart::Text { .. } => continue,
                ContentPart::Markdown { .. } => {
                    return Err(AdapterError::UnsupportedCapability {
                        capability: ChannelCapability::EgressMarkdown,
                    });
                }
                ContentPart::Card { .. } => {
                    return Err(AdapterError::UnsupportedCapability {
                        capability: ChannelCapability::EgressCard,
                    });
                }
                ContentPart::Location { .. } | ContentPart::Reference { .. } => continue,
            };
            let stored = self
                .services
                .attachments
                .get(reference)
                .await
                .map_err(|error| AdapterError::Execution {
                    code: "attachment_read_failed".to_string(),
                    diagnosis: error.to_string(),
                    retry_after: None,
                    retryable: false,
                })?;
            stored.validate().map_err(|error| {
                let code = match &error {
                    AttachmentStoreError::InvalidReference { diagnosis }
                        if diagnosis.contains("media_type") =>
                    {
                        "invalid_mime"
                    }
                    _ => "attachment_invalid",
                };
                AdapterError::Execution {
                    code: code.to_string(),
                    diagnosis: error.to_string(),
                    retry_after: None,
                    retryable: false,
                }
            })?;
            let media_type = validate_attachment_metadata(
                reference.file_name.as_deref(),
                &reference.media_type,
                stored.bytes.len() as u64,
            )?;
            attachments.push(OutboundAttachment {
                file_name: reference
                    .file_name
                    .clone()
                    .unwrap_or_else(|| "attachment".to_string()),
                media_type,
                bytes: stored.bytes,
            });
        }
        Ok(attachments)
    }

    fn reply_subject(&self, base_subject: &str) -> String {
        let base = if base_subject.trim().is_empty() {
            "agent-diva reply"
        } else {
            base_subject.trim()
        };
        if base.to_ascii_lowercase().starts_with("re:") {
            base.to_string()
        } else {
            let prefix = if self.config.subject_prefix.trim().is_empty() {
                "Re: "
            } else {
                &self.config.subject_prefix
            };
            format!("{}{}", prefix, base)
        }
    }

    async fn execute_probe_health(&self) -> Result<DeliveryReceipt, AdapterError> {
        if self.shutdown.is_cancelled() {
            return Err(AdapterError::Stopped);
        }
        if !self.config.consent_granted {
            return Err(AdapterError::Execution {
                code: "consent_required".to_string(),
                diagnosis: "email health probing requires explicit consent_granted".to_string(),
                retry_after: None,
                retryable: false,
            });
        }
        self.validate_config()?;

        let config = self.config.clone();
        let transport = self.transport.clone();
        let deadline = Instant::now() + EMAIL_PROBE_OPERATION_TIMEOUT;
        let probe = self.run_blocking_with_timeout(
            EMAIL_PROBE_OPERATION_TIMEOUT,
            "health_probe_timeout",
            move || transport.probe(&config, deadline),
        );
        let probe_result = tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => return Err(AdapterError::Stopped),
            result = probe => match result {
                Ok(result) => result
                    .map_err(|error| map_email_transport_error("health_probe_failed", error)),
                Err(error) => Err(error),
            },
        };
        match probe_result {
            Ok(()) => {
                self.mark_healthy();
                Ok(accepted_receipt("email", "", None, None))
            }
            Err(error) => {
                self.mark_degraded(error.to_string());
                Err(error)
            }
        }
    }
}

#[async_trait]
impl ChannelAdapter for EmailAdapter {
    fn name(&self) -> ChannelId {
        ChannelId::new("email").expect("static channel id is valid")
    }

    fn capabilities(&self) -> ChannelCapabilities {
        self.capabilities_snapshot()
    }

    async fn start(&self, context: AdapterContext) -> Result<(), AdapterError> {
        if !self.config.enabled {
            return Err(AdapterError::Execution {
                code: "not_configured".to_string(),
                diagnosis: "email channel is not enabled".to_string(),
                retry_after: None,
                retryable: false,
            });
        }
        if !self.config.consent_granted {
            return Err(AdapterError::Execution {
                code: "consent_required".to_string(),
                diagnosis: "email channel requires explicit consent_granted".to_string(),
                retry_after: None,
                retryable: false,
            });
        }
        self.validate_config()?;
        self.run_poll_loop(context).await
    }

    async fn execute(&self, command: ChannelCommand) -> Result<DeliveryReceipt, AdapterError> {
        let target = command
            .target_channel()
            .map_err(|error| AdapterError::Execution {
                code: "invalid_target".to_string(),
                diagnosis: error.to_string(),
                retry_after: None,
                retryable: false,
            })?;
        if target.as_str() != "email" {
            return Err(AdapterError::Execution {
                code: "wrong_channel".to_string(),
                diagnosis: format!("EmailAdapter cannot execute {} command", target),
                retry_after: None,
                retryable: false,
            });
        }

        match command {
            ChannelCommand::Send { envelope, .. } => self.execute_send(envelope).await,
            ChannelCommand::ProbeHealth { .. } => self.execute_probe_health().await,
            ChannelCommand::Typing { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionTyping,
            }),
            ChannelCommand::Edit { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionEdit,
            }),
            ChannelCommand::Delete { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionDelete,
            }),
            ChannelCommand::React { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionReaction,
            }),
            ChannelCommand::FinalizeStream { .. } => Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionStreamFinalize,
            }),
        }
    }

    fn health(&self) -> ChannelHealth {
        self.health
            .read()
            .map(|health| health.clone())
            .unwrap_or_else(|_| ChannelHealth::new(ChannelHealthStatus::Down))
    }

    async fn stop(&self) -> Result<(), AdapterError> {
        self.shutdown.cancel();
        self.mark_down();
        self.close_blocking_tasks().await
    }
}

#[derive(Debug, Clone)]
struct OutboundAttachment {
    file_name: String,
    media_type: String,
    bytes: Vec<u8>,
}

#[derive(Debug, Clone)]
struct SmtpMessage {
    from: String,
    to: String,
    subject: String,
    body: String,
    in_reply_to: Option<String>,
    references: Option<String>,
    attachments: Vec<OutboundAttachment>,
    message_id: String,
}

fn map_fabric_error(error: FabricAdmissionError) -> AdapterError {
    match error {
        FabricAdmissionError::Busy { retry_after, .. } => AdapterError::Execution {
            code: "fabric_busy".to_string(),
            diagnosis: "email ingress lane is full".to_string(),
            retry_after: Some(retry_after),
            retryable: true,
        },
        FabricAdmissionError::Cancelled { .. } => AdapterError::Stopped,
        FabricAdmissionError::Closed { .. } => AdapterError::Execution {
            code: "fabric_closed".to_string(),
            diagnosis: "email Fabric ingress is closed".to_string(),
            retry_after: None,
            retryable: false,
        },
        FabricAdmissionError::InvalidEnvelope(error) => AdapterError::Execution {
            code: "invalid_envelope".to_string(),
            diagnosis: error.to_string(),
            retry_after: None,
            retryable: false,
        },
    }
}

fn map_email_transport_error(operation_code: &str, error: EmailTransportError) -> AdapterError {
    match error {
        EmailTransportError::Failed { diagnosis } => AdapterError::Execution {
            code: operation_code.to_string(),
            diagnosis,
            retry_after: None,
            retryable: true,
        },
        EmailTransportError::InvalidMessage { diagnosis } => AdapterError::Execution {
            code: "invalid_email_message".to_string(),
            diagnosis,
            retry_after: None,
            retryable: false,
        },
        EmailTransportError::InvalidMime { media_type } => AdapterError::Execution {
            code: "invalid_mime".to_string(),
            diagnosis: format!("invalid MIME type: {media_type}"),
            retry_after: None,
            retryable: false,
        },
        EmailTransportError::TooLarge {
            size_bytes,
            max_bytes,
        } => AdapterError::Execution {
            code: "attachment_too_large".to_string(),
            diagnosis: format!(
                "email attachment exceeds configured bound ({size_bytes} > {max_bytes})"
            ),
            retry_after: None,
            retryable: false,
        },
        EmailTransportError::InvalidAddress { field, diagnosis } => AdapterError::Execution {
            code: if field == "recipient" {
                "invalid_email_address"
            } else {
                "invalid_from_address"
            }
            .to_string(),
            diagnosis,
            retry_after: None,
            retryable: false,
        },
    }
}

fn validate_email_address(value: &str) -> Result<(), AdapterError> {
    if value.parse::<lettre::Address>().is_err() {
        return Err(AdapterError::Execution {
            code: "invalid_email_address".to_string(),
            diagnosis: "email address failed RFC validation".to_string(),
            retry_after: None,
            retryable: false,
        });
    }
    Ok(())
}

fn validate_recipient_address(value: &str) -> Result<(), AdapterError> {
    if value.trim().is_empty() {
        return Err(AdapterError::Execution {
            code: "missing_recipient".to_string(),
            diagnosis: "email recipient must not be empty".to_string(),
            retry_after: None,
            retryable: false,
        });
    }
    validate_email_address(value)
}

fn validate_from_address(value: &str) -> Result<(), AdapterError> {
    if value.parse::<lettre::Address>().is_err() {
        return Err(AdapterError::Execution {
            code: "invalid_from_address".to_string(),
            diagnosis: "email from address failed RFC validation".to_string(),
            retry_after: None,
            retryable: false,
        });
    }
    Ok(())
}

fn validate_attachment_metadata(
    file_name: Option<&str>,
    media_type: &str,
    size_bytes: u64,
) -> Result<String, AdapterError> {
    if size_bytes > MAX_ATTACHMENT_BYTES {
        return Err(AdapterError::Execution {
            code: "attachment_too_large".to_string(),
            diagnosis: format!(
                "email attachment exceeds configured bound ({size_bytes} > {MAX_ATTACHMENT_BYTES})"
            ),
            retry_after: None,
            retryable: false,
        });
    }
    if let Some(file_name) = file_name {
        if file_name.trim().is_empty()
            || file_name.contains('/')
            || file_name.contains('\\')
            || std::path::Path::new(file_name).is_absolute()
        {
            return Err(AdapterError::Execution {
                code: "invalid_attachment_name".to_string(),
                diagnosis: "email attachment name must be a relative leaf name".to_string(),
                retry_after: None,
                retryable: false,
            });
        }
    }
    normalize_mime_type(media_type).map_err(|media_type| AdapterError::Execution {
        code: "invalid_mime".to_string(),
        diagnosis: format!("invalid MIME type: {media_type}"),
        retry_after: None,
        retryable: false,
    })
}

fn normalize_mime_type(value: &str) -> Result<String, String> {
    let value = value.trim();
    let (base, parameters) = value
        .split_once(';')
        .map_or((value, None), |(base, rest)| (base.trim(), Some(rest)));
    let Some((kind, subtype)) = base.split_once('/') else {
        return Err(value.to_string());
    };
    if !valid_mime_token(kind) || !valid_mime_token(subtype) {
        return Err(value.to_string());
    }
    if let Some(parameters) = parameters {
        if parameters.bytes().any(|byte| byte.is_ascii_control()) {
            return Err(value.to_string());
        }
    }
    let mut normalized = format!(
        "{}/{}",
        kind.to_ascii_lowercase(),
        subtype.to_ascii_lowercase()
    );
    if let Some(parameters) = parameters {
        let parameters = parameters.trim();
        if !parameters.is_empty() {
            normalized.push(';');
            normalized.push_str(parameters);
        }
    }
    Ok(normalized)
}

fn valid_mime_token(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'!' | b'#' | b'$' | b'&' | b'^' | b'_' | b'.' | b'+' | b'-'
                )
        })
}

fn subject_from_extensions(
    envelope: &agent_diva_core::channel::ChannelEnvelopeV1,
) -> Option<String> {
    envelope
        .extensions
        .get("email.subject")
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
}

fn text_body_from_parts(parts: &[ContentPart]) -> Result<String, AdapterError> {
    let mut body = String::new();
    for part in parts {
        match part {
            ContentPart::Text { text } => body.push_str(text),
            ContentPart::Reference { uri, title, .. } => {
                if !body.is_empty() {
                    body.push('\n');
                }
                if let Some(title) = title {
                    body.push_str(title);
                    body.push_str(": ");
                }
                body.push_str(uri);
            }
            ContentPart::Location {
                latitude,
                longitude,
                label,
            } => {
                if !body.is_empty() {
                    body.push('\n');
                }
                body.push_str(&format!("Location: {latitude}, {longitude}"));
                if let Some(label) = label {
                    body.push_str(" ( ");
                    body.push_str(label);
                    body.push_str(" )");
                }
            }
            ContentPart::Markdown { .. } => {
                return Err(AdapterError::UnsupportedCapability {
                    capability: ChannelCapability::EgressMarkdown,
                });
            }
            ContentPart::Card { .. } => {
                return Err(AdapterError::UnsupportedCapability {
                    capability: ChannelCapability::EgressCard,
                });
            }
            ContentPart::Image { .. }
            | ContentPart::Audio { .. }
            | ContentPart::Video { .. }
            | ContentPart::File { .. } => {}
        }
    }
    Ok(body)
}

fn message_id_domain(from: &str) -> String {
    from.rsplit_once('@')
        .map(|(_, domain)| {
            domain.trim_matches(|ch: char| ch == '>' || ch == '<' || ch.is_whitespace())
        })
        .filter(|domain| !domain.is_empty())
        .unwrap_or("localhost")
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
        .collect::<String>()
        .if_empty_then("localhost")
}

trait StringFallback {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringFallback for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn parse_email_bytes(
    body: &[u8],
    uid: String,
    max_body_chars: usize,
) -> Result<ParsedEmail, EmailTransportError> {
    let parsed = MessageParser::default().parse(body).ok_or_else(|| {
        EmailTransportError::InvalidMessage {
            diagnosis: "RFC822 parser rejected the message".to_string(),
        }
    })?;
    let sender = parsed
        .from()
        .and_then(|addresses| addresses.first())
        .and_then(|address| address.address())
        .map(canonical_email_address)
        .filter(|sender| !sender.is_empty())
        .ok_or_else(|| EmailTransportError::InvalidMessage {
            diagnosis: "RFC822 message has no usable From address".to_string(),
        })?;
    let subject = parsed.subject().unwrap_or_default().trim().to_string();
    let message_id = parsed
        .message_id()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let in_reply_to = header_text_list(parsed.in_reply_to());
    let references = header_text_list(parsed.references());
    let thread_topic = email_thread_topic(
        message_id.as_deref(),
        in_reply_to.as_deref(),
        references.as_deref(),
        &subject,
    );
    let raw_body = parsed
        .body_text(0)
        .map(|body| body.into_owned())
        .or_else(|| {
            parsed
                .body_html(0)
                .map(|body| html_body_to_text(body.as_ref()))
        })
        .unwrap_or_default();
    let text_body = truncate_utf8(raw_body.trim(), max_body_chars);
    let date = parsed
        .date()
        .map(|date| date.to_rfc3339())
        .unwrap_or_default();

    let mut attachments = Vec::new();
    for part in parsed.attachments() {
        let bytes = match &part.body {
            PartType::Binary(bytes) | PartType::InlineBinary(bytes) => bytes.to_vec(),
            PartType::Text(text) | PartType::Html(text) => text.as_bytes().to_vec(),
            PartType::Message(message) => message.raw_message.to_vec(),
            PartType::Multipart(_) => {
                return Err(EmailTransportError::InvalidMessage {
                    diagnosis: "multipart attachment has no concrete body".to_string(),
                });
            }
        };
        let content_type = part
            .content_type()
            .ok_or_else(|| EmailTransportError::InvalidMime {
                media_type: "<missing>".to_string(),
            })?;
        let subtype =
            content_type
                .c_subtype
                .as_deref()
                .ok_or_else(|| EmailTransportError::InvalidMime {
                    media_type: content_type.c_type.to_string(),
                })?;
        let media_type = format!("{}/{}", content_type.c_type, subtype);
        let media_type = normalize_mime_type(&media_type)
            .map_err(|media_type| EmailTransportError::InvalidMime { media_type })?;
        let file_name = part.attachment_name().map(ToOwned::to_owned);
        validate_parsed_attachment(file_name.as_deref(), &media_type, bytes.len() as u64)?;
        attachments.push(ParsedAttachment {
            file_name,
            media_type,
            bytes,
        });
    }

    Ok(ParsedEmail {
        sender,
        subject,
        message_id,
        in_reply_to,
        references,
        thread_topic,
        text_body,
        uid,
        date,
        attachments,
    })
}

fn validate_parsed_attachment(
    file_name: Option<&str>,
    media_type: &str,
    size_bytes: u64,
) -> Result<(), EmailTransportError> {
    if size_bytes > MAX_ATTACHMENT_BYTES {
        return Err(EmailTransportError::TooLarge {
            size_bytes,
            max_bytes: MAX_ATTACHMENT_BYTES,
        });
    }
    if file_name.is_some_and(|name| {
        name.trim().is_empty()
            || name.contains('/')
            || name.contains('\\')
            || std::path::Path::new(name).is_absolute()
    }) {
        return Err(EmailTransportError::InvalidMessage {
            diagnosis: "attachment filename must be a relative leaf name".to_string(),
        });
    }
    if normalize_mime_type(media_type).is_err() {
        return Err(EmailTransportError::InvalidMime {
            media_type: media_type.to_string(),
        });
    }
    Ok(())
}

fn html_body_to_text(value: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    let mut tag = String::new();
    for ch in value.chars() {
        match ch {
            '<' => {
                in_tag = true;
                tag.clear();
            }
            '>' if in_tag => {
                in_tag = false;
                let tag_name = tag.trim().to_ascii_lowercase();
                if ["br", "/p", "p", "/div", "div", "/li", "li"]
                    .iter()
                    .any(|name| tag_name.starts_with(name))
                    && !text.ends_with('\n')
                {
                    text.push('\n');
                }
            }
            _ if in_tag => tag.push(ch),
            _ => text.push(ch),
        }
    }
    html_escape::decode_html_entities(&text).trim().to_string()
}

fn header_text_list(value: &mail_parser::HeaderValue<'_>) -> Option<String> {
    value
        .as_text_list()
        .map(|values| values.join(" "))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn truncate_utf8(value: &str, max_chars: usize) -> String {
    value.chars().take(max_chars).collect()
}

fn fetch_messages_blocking(
    config: &EmailConfig,
    mailbox: &str,
    processed: &HashSet<String>,
) -> Result<Vec<ParsedEmail>, EmailTransportError> {
    // `imap_use_ssl` is intentionally a live branch: true uses implicit TLS,
    // false uses the configured plain IMAP socket (normally port 143).
    if config.imap_use_ssl {
        let client = connect_imap_tls(config)?;
        let session = login_imap(client, config)?;
        fetch_messages_session(session, config, mailbox, processed)
    } else {
        let client = connect_imap_plain(config)?;
        let session = login_imap(client, config)?;
        fetch_messages_session(session, config, mailbox, processed)
    }
}

fn mark_seen_blocking(
    config: &EmailConfig,
    mailbox: &str,
    uid: &str,
) -> Result<(), EmailTransportError> {
    if config.imap_use_ssl {
        let client = connect_imap_tls(config)?;
        let session = login_imap(client, config)?;
        mark_seen_session(session, mailbox, uid)
    } else {
        let client = connect_imap_plain(config)?;
        let session = login_imap(client, config)?;
        mark_seen_session(session, mailbox, uid)
    }
}

fn resolve_imap_address(config: &EmailConfig) -> Result<std::net::SocketAddr, EmailTransportError> {
    (config.imap_host.as_str(), config.imap_port)
        .to_socket_addrs()
        .map_err(|error| transport_failure(format!("IMAP address resolution failed: {error}")))?
        .next()
        .ok_or_else(|| transport_failure("IMAP address resolution returned no addresses"))
}

fn connect_imap_tcp_with_timeout(
    config: &EmailConfig,
    io_timeout: Duration,
) -> Result<TcpStream, EmailTransportError> {
    let address = resolve_imap_address(config)?;
    let stream = TcpStream::connect_timeout(&address, io_timeout)
        .map_err(|error| transport_failure(format!("IMAP connect failed: {error}")))?;
    stream
        .set_read_timeout(Some(io_timeout))
        .map_err(|error| transport_failure(format!("IMAP read timeout setup failed: {error}")))?;
    stream
        .set_write_timeout(Some(io_timeout))
        .map_err(|error| transport_failure(format!("IMAP write timeout setup failed: {error}")))?;
    Ok(stream)
}

fn connect_imap_tls(
    config: &EmailConfig,
) -> Result<imap::Client<native_tls::TlsStream<TcpStream>>, EmailTransportError> {
    connect_imap_tls_with_timeout(config, IMAP_IO_TIMEOUT)
}

fn connect_imap_tls_with_timeout(
    config: &EmailConfig,
    io_timeout: Duration,
) -> Result<imap::Client<native_tls::TlsStream<TcpStream>>, EmailTransportError> {
    let connector = native_tls::TlsConnector::new()
        .map_err(|error| transport_failure(format!("IMAP TLS setup failed: {error}")))?;
    let stream = connect_imap_tcp_with_timeout(config, io_timeout)?;
    let stream = connector
        .connect(&config.imap_host, stream)
        .map_err(|error| transport_failure(format!("IMAP TLS handshake failed: {error}")))?;
    let mut client = imap::Client::new(stream);
    client
        .read_greeting()
        .map_err(|error| transport_failure(format!("IMAP greeting failed: {error}")))?;
    Ok(client)
}

fn connect_imap_plain(
    config: &EmailConfig,
) -> Result<imap::Client<TcpStream>, EmailTransportError> {
    connect_imap_plain_with_timeout(config, IMAP_IO_TIMEOUT)
}

fn connect_imap_plain_with_timeout(
    config: &EmailConfig,
    io_timeout: Duration,
) -> Result<imap::Client<TcpStream>, EmailTransportError> {
    let stream = connect_imap_tcp_with_timeout(config, io_timeout)?;
    let mut client = imap::Client::new(stream);
    client
        .read_greeting()
        .map_err(|error| transport_failure(format!("IMAP greeting failed: {error}")))?;
    Ok(client)
}

fn login_imap<T: Read + Write>(
    client: imap::Client<T>,
    config: &EmailConfig,
) -> Result<imap::Session<T>, EmailTransportError> {
    client
        .login(&config.imap_username, &config.imap_password)
        .map_err(|(error, _)| transport_failure(format!("IMAP login failed: {error:?}")))
}

fn fetch_messages_session<T: Read + Write>(
    mut session: imap::Session<T>,
    config: &EmailConfig,
    mailbox: &str,
    processed: &HashSet<String>,
) -> Result<Vec<ParsedEmail>, EmailTransportError> {
    let result = (|| {
        session
            .select(mailbox)
            .map_err(|error| transport_failure(format!("IMAP SELECT failed: {error}")))?;
        let uids = session
            .uid_search("UNSEEN")
            .map_err(|error| transport_failure(format!("IMAP SEARCH failed: {error}")))?;
        let mut messages = Vec::new();
        for uid in uids {
            let uid_string = uid.to_string();
            let fetched = session
                .uid_fetch(&uid_string, "(BODY.PEEK[] UID)")
                .map_err(|error| transport_failure(format!("IMAP FETCH failed: {error}")))?;
            for fetch in fetched.iter() {
                let Some(body) = fetch.body() else {
                    return Err(EmailTransportError::InvalidMessage {
                        diagnosis: format!(
                            "IMAP FETCH returned no RFC822 body for UID {uid_string}"
                        ),
                    });
                };
                let email = parse_email_bytes(body, uid_string.clone(), config.max_body_chars)?;
                if !processed.contains(&email.dedup_key()) {
                    messages.push(email);
                }
            }
        }
        Ok(messages)
    })();
    let _ = session.logout();
    result
}

fn mark_seen_session<T: Read + Write>(
    mut session: imap::Session<T>,
    mailbox: &str,
    uid: &str,
) -> Result<(), EmailTransportError> {
    let result = (|| {
        session
            .select(mailbox)
            .map_err(|error| transport_failure(format!("IMAP SELECT failed: {error}")))?;
        session
            .uid_store(uid, "+FLAGS (\\Seen)")
            .map_err(|error| transport_failure(format!("IMAP STORE failed: {error}")))?;
        Ok(())
    })();
    let _ = session.logout();
    result
}

fn probe_imap_blocking_until(
    config: &EmailConfig,
    deadline: Instant,
) -> Result<(), EmailTransportError> {
    let mailbox = if config.imap_mailbox.trim().is_empty() {
        DEFAULT_MAILBOX
    } else {
        config.imap_mailbox.trim()
    };
    let io_timeout = remaining_probe_timeout(deadline);
    if config.imap_use_ssl {
        let client = connect_imap_tls_with_timeout(config, io_timeout)?;
        probe_imap_session(login_imap(client, config)?, mailbox)
    } else {
        let client = connect_imap_plain_with_timeout(config, io_timeout)?;
        probe_imap_session(login_imap(client, config)?, mailbox)
    }
}

fn probe_imap_session<T: Read + Write>(
    mut session: imap::Session<T>,
    mailbox: &str,
) -> Result<(), EmailTransportError> {
    let result = (|| {
        session
            .select(mailbox)
            .map_err(|error| transport_failure(format!("IMAP SELECT failed: {error}")))?;
        session
            .noop()
            .map_err(|error| transport_failure(format!("IMAP NOOP failed: {error}")))
    })();
    let _ = session.logout();
    result
}

fn build_smtp_transport_with_timeout(
    config: &EmailConfig,
    io_timeout: Duration,
) -> Result<lettre::SmtpTransport, EmailTransportError> {
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::SmtpTransport;

    let credentials =
        || Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());
    let transport = if config.smtp_use_ssl {
        SmtpTransport::relay(&config.smtp_host)
            .map_err(|error| transport_failure(format!("SMTP SSL setup failed: {error}")))?
            .credentials(credentials())
            .port(config.smtp_port)
            .timeout(Some(io_timeout))
            .build()
    } else if config.smtp_use_tls {
        SmtpTransport::starttls_relay(&config.smtp_host)
            .map_err(|error| transport_failure(format!("SMTP STARTTLS setup failed: {error}")))?
            .credentials(credentials())
            .port(config.smtp_port)
            .timeout(Some(io_timeout))
            .build()
    } else {
        SmtpTransport::builder_dangerous(&config.smtp_host)
            .credentials(credentials())
            .port(config.smtp_port)
            .timeout(Some(io_timeout))
            .build()
    };
    Ok(transport)
}

fn build_smtp_transport(
    config: &EmailConfig,
) -> Result<lettre::SmtpTransport, EmailTransportError> {
    build_smtp_transport_with_timeout(config, SMTP_IO_TIMEOUT)
}

fn probe_smtp_blocking_until(
    config: &EmailConfig,
    deadline: Instant,
) -> Result<(), EmailTransportError> {
    let transport = build_smtp_transport_with_timeout(config, remaining_probe_timeout(deadline))?;
    match transport
        .test_connection()
        .map_err(|error| transport_failure(format!("SMTP health probe failed: {error}")))?
    {
        true => Ok(()),
        false => Err(transport_failure(
            "SMTP health probe returned a negative response",
        )),
    }
}

fn probe_email_blocking_until(
    config: &EmailConfig,
    deadline: Instant,
) -> Result<(), EmailTransportError> {
    probe_imap_blocking_until(config, deadline)?;
    probe_smtp_blocking_until(config, deadline)
}

fn remaining_probe_timeout(deadline: Instant) -> Duration {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        Duration::from_millis(1)
    } else {
        remaining.min(EMAIL_PROBE_IO_TIMEOUT)
    }
}

fn smtp_send_blocking(
    config: &EmailConfig,
    message: SmtpMessage,
) -> Result<String, EmailTransportError> {
    use lettre::message::header::ContentType;
    use lettre::message::{Attachment, MultiPart, SinglePart};
    use lettre::{Message, Transport};

    let mut builder = Message::builder()
        .from(
            message
                .from
                .parse::<lettre::Address>()
                .map_err(|error| EmailTransportError::InvalidAddress {
                    field: "from",
                    diagnosis: error.to_string(),
                })?
                .into(),
        )
        .to(message
            .to
            .parse::<lettre::Address>()
            .map_err(|error| EmailTransportError::InvalidAddress {
                field: "recipient",
                diagnosis: error.to_string(),
            })?
            .into())
        .subject(message.subject)
        .message_id(Some(message.message_id.clone()));
    if let Some(reply_to) = message
        .in_reply_to
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        builder = builder.in_reply_to(reply_to.to_string());
    }
    if let Some(references) = message
        .references
        .as_deref()
        .filter(|value| !value.trim().is_empty())
    {
        builder = builder.references(references.to_string());
    }

    let email = if message.attachments.is_empty() {
        builder
            .header(ContentType::TEXT_PLAIN)
            .body(message.body)
            .map_err(|error| transport_failure(format!("build email failed: {error}")))?
    } else {
        let mut multipart = MultiPart::mixed().singlepart(SinglePart::plain(message.body));
        for attachment in message.attachments {
            let media_type = normalize_mime_type(&attachment.media_type)
                .map_err(|media_type| EmailTransportError::InvalidMime { media_type })?;
            let content_type = ContentType::parse(&media_type)
                .map_err(|_| EmailTransportError::InvalidMime { media_type })?;
            multipart = multipart.singlepart(
                Attachment::new(attachment.file_name).body(attachment.bytes, content_type),
            );
        }
        builder
            .multipart(multipart)
            .map_err(|error| transport_failure(format!("build multipart email failed: {error}")))?
    };

    let transport = build_smtp_transport(config)?;
    transport
        .send(&email)
        .map_err(|error| transport_failure(format!("SMTP send failed: {error}")))?;
    Ok(message.message_id)
}

fn email_thread_topic(
    message_id: Option<&str>,
    in_reply_to: Option<&str>,
    references: Option<&str>,
    subject: &str,
) -> String {
    let topic_source = references
        .and_then(first_message_id)
        .or_else(|| in_reply_to.and_then(first_message_id))
        .or_else(|| message_id.and_then(first_message_id))
        .or_else(|| normalized_subject(subject))
        .unwrap_or_else(|| "untitled".to_string());
    format!("email-{}", sanitize_topic_component(&topic_source))
}

fn first_message_id(value: &str) -> Option<String> {
    value
        .split_whitespace()
        .map(|part| part.trim().trim_matches('<').trim_matches('>'))
        .find(|part| !part.is_empty())
        .map(ToOwned::to_owned)
}

fn normalized_subject(subject: &str) -> Option<String> {
    let mut value = subject.trim();
    loop {
        let lower = value.to_ascii_lowercase();
        let Some(prefix) = ["re:", "fw:", "fwd:"]
            .into_iter()
            .find(|prefix| lower.starts_with(prefix))
        else {
            break;
        };
        value = value[prefix.len()..].trim();
    }
    (!value.is_empty()).then(|| value.to_string())
}

fn sanitize_topic_component(value: &str) -> String {
    let mut output = String::new();
    let mut separator = false;
    for ch in value.trim().chars() {
        if output.chars().count() >= MAX_EMAIL_TOPIC_CHARS {
            break;
        }
        if ch.is_alphanumeric() || ch == '-' {
            output.push(ch);
            separator = false;
        } else if !separator && !output.is_empty() {
            output.push('_');
            separator = true;
        }
    }
    while output.ends_with('_') {
        output.pop();
    }
    if output.is_empty() {
        "untitled".to_string()
    } else {
        output
    }
}

fn extract_email_address(value: &str) -> String {
    if let Some(start) = value.rfind('<') {
        if let Some(relative_end) = value[start..].find('>') {
            return value[start + 1..start + relative_end].trim().to_string();
        }
    }
    value.trim().to_string()
}

fn canonical_email_address(value: &str) -> String {
    extract_email_address(value).to_ascii_lowercase()
}

fn email_sender_allowed(allow_from: &[String], sender: &str) -> bool {
    let patterns = allow_from
        .iter()
        .map(|pattern| {
            if pattern.contains('*') {
                pattern
                    .split('*')
                    .map(canonical_email_address)
                    .collect::<Vec<_>>()
                    .join("*")
            } else {
                canonical_email_address(pattern)
            }
        })
        .collect::<Vec<_>>();
    is_sender_allowed(&patterns, &canonical_email_address(sender))
}

fn subject_is_reply(subject: &str) -> bool {
    subject.trim_start().to_ascii_lowercase().starts_with("re:")
}

fn should_skip_self_reply(sender: &str, subject: &str, config: &EmailConfig) -> bool {
    if !subject_is_reply(subject) {
        return false;
    }
    let sender = canonical_email_address(sender);
    if sender.is_empty() {
        return false;
    }
    [
        config.from_address.as_str(),
        config.smtp_username.as_str(),
        config.imap_username.as_str(),
    ]
    .into_iter()
    .filter(|value| !value.trim().is_empty())
    .any(|value| canonical_email_address(value) == sender)
}

#[cfg(test)]
fn message_id_for_fixture(message_id: Option<&str>, uid: &str) -> String {
    message_id
        .filter(|value| !value.trim().is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("uid:{uid}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter::{ChannelAttachmentStore, StoredAttachment};
    use agent_diva_core::channel::{
        ChannelAddress, ChannelDirection, ChannelEnvelopeV1, ChannelOrigin, ChannelPayloadV1,
        ContentPart, Correlation, FabricKernel,
    };
    use agent_diva_core::config::schema::EmailConfig;
    use async_trait::async_trait;
    use base64::Engine;
    use sha2::Digest;
    use std::collections::{BTreeMap, VecDeque};
    use std::sync::Mutex as StdMutex;

    #[derive(Default)]
    struct MemoryAttachments {
        values: tokio::sync::Mutex<BTreeMap<String, StoredAttachment>>,
    }

    #[async_trait]
    impl ChannelAttachmentStore for MemoryAttachments {
        async fn put(
            &self,
            input: IngressAttachment,
        ) -> Result<agent_diva_core::channel::AttachmentRef, crate::adapter::AttachmentStoreError>
        {
            let digest = format!("{:x}", sha2::Sha256::digest(&input.bytes));
            let reference = agent_diva_core::channel::AttachmentRef {
                uri: format!("sha256:{digest}"),
                media_type: input
                    .declared_mime
                    .unwrap_or_else(|| "application/octet-stream".to_string()),
                size_bytes: input.bytes.len() as u64,
                sha256: digest,
                file_name: input.file_name,
            };
            let stored = StoredAttachment {
                reference: reference.clone(),
                bytes: input.bytes,
            };
            self.values
                .lock()
                .await
                .insert(reference.uri.clone(), stored);
            Ok(reference)
        }

        async fn get(
            &self,
            reference: &agent_diva_core::channel::AttachmentRef,
        ) -> Result<StoredAttachment, crate::adapter::AttachmentStoreError> {
            self.values
                .lock()
                .await
                .get(&reference.uri)
                .cloned()
                .ok_or_else(|| crate::adapter::AttachmentStoreError::NotFound {
                    uri: reference.uri.clone(),
                })
        }
    }

    #[derive(Default)]
    struct FakeEmailTransport {
        fetches: StdMutex<VecDeque<Result<Vec<ParsedEmail>, EmailTransportError>>>,
        seen: StdMutex<Vec<String>>,
        sent: StdMutex<Vec<SmtpMessage>>,
        mark_seen_results: StdMutex<VecDeque<Result<(), EmailTransportError>>>,
        send_results: StdMutex<VecDeque<Result<String, EmailTransportError>>>,
        probe_results: StdMutex<VecDeque<Result<(), EmailTransportError>>>,
        events: StdMutex<Vec<String>>,
        smtp_wire: StdMutex<Vec<String>>,
        send_gate: StdMutex<Option<Arc<FakeBlockingGate>>>,
        probe_gate: StdMutex<Option<Arc<FakeBlockingGate>>>,
    }

    struct FakeBlockingGate {
        started: StdMutex<std::sync::mpsc::Sender<()>>,
        release: StdMutex<std::sync::mpsc::Receiver<()>>,
    }

    #[tokio::test]
    async fn blocking_task_registry_rejects_work_beyond_hard_capacity() {
        let registry = Arc::new(BlockingTaskRegistry::default());
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let release_rx = Arc::new(StdMutex::new(release_rx));
        let mut workers = Vec::new();
        for _ in 0..MAX_BLOCKING_TASKS {
            let registry = registry.clone();
            let release_rx = release_rx.clone();
            workers.push(tokio::spawn(async move {
                registry
                    .run(move || {
                        let _ = release_rx.lock().expect("release lock").recv();
                    })
                    .await
            }));
        }

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if registry.tasks.lock().await.len() == MAX_BLOCKING_TASKS {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("all blocking workers admitted");

        assert!(matches!(
            registry.run(|| ()).await,
            Err(BlockingTaskError::AtCapacity)
        ));
        for _ in 0..MAX_BLOCKING_TASKS {
            release_tx.send(()).expect("release blocking worker");
        }
        for worker in workers {
            worker
                .await
                .expect("blocking worker join")
                .expect("worker result");
        }
    }

    #[tokio::test]
    async fn completed_orphan_blocking_handle_does_not_consume_capacity() {
        let registry = Arc::new(BlockingTaskRegistry::default());
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let waiter_registry = registry.clone();
        let waiter = tokio::spawn(async move {
            waiter_registry
                .run(move || {
                    let _ = release_rx.recv();
                })
                .await
        });

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if registry.tasks.lock().await.len() == 1 {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("blocking worker admitted");
        waiter.abort();
        let _ = waiter.await;
        release_tx.send(()).expect("release orphaned worker");

        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if registry
                    .tasks
                    .lock()
                    .await
                    .values()
                    .all(|task| task.is_finished())
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("orphaned worker completed");

        registry
            .run(|| ())
            .await
            .expect("completed orphan must be purged before admission");
    }

    impl EmailTransport for FakeEmailTransport {
        fn fetch_messages(
            &self,
            config: &EmailConfig,
            mailbox: &str,
            _processed: &HashSet<String>,
        ) -> Result<Vec<ParsedEmail>, EmailTransportError> {
            self.events.lock().unwrap().push(format!(
                "imap.fetch ssl={} mailbox={} command=UID SEARCH UNSEEN; UID FETCH (BODY.PEEK[] UID)",
                config.imap_use_ssl, mailbox
            ));
            self.fetches
                .lock()
                .expect("fake fetch lock")
                .pop_front()
                .unwrap_or_else(|| Ok(Vec::new()))
        }

        fn mark_seen(
            &self,
            config: &EmailConfig,
            mailbox: &str,
            uid: &str,
        ) -> Result<(), EmailTransportError> {
            self.events.lock().unwrap().push(format!(
                "imap.store_seen ssl={} mailbox={} uid={} command=UID STORE +FLAGS (\\Seen)",
                config.imap_use_ssl, mailbox, uid
            ));
            self.seen
                .lock()
                .expect("fake seen lock")
                .push(uid.to_string());
            self.mark_seen_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(()))
        }

        fn send(
            &self,
            config: &EmailConfig,
            message: SmtpMessage,
        ) -> Result<String, EmailTransportError> {
            let mode = smtp_mode(config);
            self.events.lock().unwrap().push(format!(
                "smtp.send mode={mode} command=MAIL FROM RCPT TO DATA"
            ));
            let mut sent = self.sent.lock().expect("fake SMTP lock");
            let id = format!("smtp-fixture-{}", sent.len() + 1);
            let response = format!("250 2.0.0 queued as {id}");
            self.smtp_wire
                .lock()
                .unwrap()
                .push(fake_smtp_wire(&message, mode, &response));
            sent.push(message);
            drop(sent);
            if let Some(gate) = self.send_gate.lock().unwrap().clone() {
                let _ = gate.started.lock().unwrap().send(());
                let _ = gate.release.lock().unwrap().recv();
            }
            self.send_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    parse_smtp_acceptance(&response).map(Ok).unwrap_or_else(|| {
                        Err(transport_failure("fixture SMTP response was malformed"))
                    })
                })
        }

        fn probe(
            &self,
            config: &EmailConfig,
            _deadline: Instant,
        ) -> Result<(), EmailTransportError> {
            let mode = smtp_mode(config);
            let mut events = self.events.lock().unwrap();
            events.push(format!(
                "imap.probe ssl={} command=NOOP",
                config.imap_use_ssl
            ));
            events.push(format!("smtp.probe mode={mode} command=EHLO NOOP"));
            drop(events);
            if let Some(gate) = self.probe_gate.lock().unwrap().clone() {
                let _ = gate.started.lock().unwrap().send(());
                let _ = gate.release.lock().unwrap().recv();
            }
            self.probe_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(Ok(()))
        }

        fn record_event(&self, event: &str) {
            self.events.lock().unwrap().push(event.to_string());
        }
    }

    fn smtp_mode(config: &EmailConfig) -> &'static str {
        if config.smtp_use_ssl {
            "ssl"
        } else if config.smtp_use_tls {
            "starttls"
        } else {
            "plain"
        }
    }

    fn fake_smtp_wire(message: &SmtpMessage, mode: &str, response: &str) -> String {
        let mut wire = format!(
            "220 fixture SMTP\nEHLO agent-diva.test\nMODE {mode}\nMAIL FROM:<{}>\nRCPT TO:<{}>\nDATA\n",
            message.from, message.to
        );
        wire.push_str(&format!("Message-ID: {}\n", message.message_id));
        if let Some(in_reply_to) = &message.in_reply_to {
            wire.push_str(&format!("In-Reply-To: {in_reply_to}\n"));
        }
        if let Some(references) = &message.references {
            wire.push_str(&format!("References: {references}\n"));
        }
        if message.attachments.is_empty() {
            wire.push_str(&format!(
                "Subject: {}\nContent-Type: text/plain; charset=utf-8\n\n{}\n.\n{}",
                message.subject, message.body, response
            ));
        } else {
            wire.push_str(&format!(
                "Subject: {}\nContent-Type: multipart/mixed; boundary=\"fixture-boundary\"\n\n\
                 --fixture-boundary\nContent-Type: text/plain; charset=utf-8\n\n{}\n",
                message.subject, message.body
            ));
            for attachment in &message.attachments {
                wire.push_str(&format!(
                    "--fixture-boundary\nContent-Type: {}\nContent-Disposition: attachment; filename=\"{}\"\n\
                     Content-Transfer-Encoding: base64\n\n{}\n",
                    attachment.media_type,
                    attachment.file_name,
                    base64::engine::general_purpose::STANDARD.encode(&attachment.bytes)
                ));
            }
            wire.push_str(&format!("--fixture-boundary--\n.\n{response}"));
        }
        wire
    }

    fn parse_smtp_acceptance(response: &str) -> Option<String> {
        response.lines().find_map(|line| {
            line.strip_prefix("250 2.0.0 queued as ")
                .map(ToOwned::to_owned)
        })
    }

    fn config() -> EmailConfig {
        EmailConfig {
            enabled: true,
            consent_granted: true,
            imap_host: "imap.example.test".to_string(),
            imap_port: 993,
            imap_username: "bot@example.test".to_string(),
            imap_password: "secret".to_string(),
            imap_mailbox: "INBOX".to_string(),
            imap_use_ssl: true,
            smtp_host: "smtp.example.test".to_string(),
            smtp_port: 587,
            smtp_username: "bot@example.test".to_string(),
            smtp_password: "secret".to_string(),
            smtp_use_tls: true,
            smtp_use_ssl: false,
            from_address: "bot@example.test".to_string(),
            auto_reply_enabled: true,
            poll_interval_seconds: 1,
            mark_seen: false,
            max_body_chars: 12000,
            subject_prefix: "Re: ".to_string(),
            allow_from: Vec::new(),
        }
    }

    fn adapter() -> EmailAdapter {
        EmailAdapter::new(
            config(),
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
        )
    }

    fn fixture(name: &str) -> Vec<u8> {
        match name {
            "plain" => include_bytes!("../../tests/fixtures/c5/email/plain.eml").to_vec(),
            "reply" => include_bytes!("../../tests/fixtures/c5/email/reply.eml").to_vec(),
            "multipart" => include_bytes!("../../tests/fixtures/c5/email/multipart.eml").to_vec(),
            "html" => include_bytes!("../../tests/fixtures/c5/email/html.eml").to_vec(),
            "no-message-id" => {
                include_bytes!("../../tests/fixtures/c5/email/no-message-id.eml").to_vec()
            }
            "rfc822" => include_bytes!("../../tests/fixtures/c5/email/rfc822.eml").to_vec(),
            "multipart-rich" => {
                include_bytes!("../../tests/fixtures/c5/email/multipart-rich.eml").to_vec()
            }
            "invalid-mime" => {
                include_bytes!("../../tests/fixtures/c5/email/invalid-mime.eml").to_vec()
            }
            _ => Vec::new(),
        }
    }

    fn send_envelope(recipient: &str, body: &str) -> ChannelEnvelopeV1 {
        ChannelEnvelopeV1::new(
            ChannelDirection::Egress,
            ChannelAddress::new("email", recipient),
            Correlation::new(format!("email:{recipient}")),
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Message {
                parts: vec![ContentPart::Text {
                    text: body.to_string(),
                }],
                subject: Some("Fixture subject".to_string()),
                locale: None,
                context: None,
            },
        )
    }

    fn assert_error_code(result: Result<DeliveryReceipt, AdapterError>, expected: &str) {
        match result {
            Err(error) => assert_eq!(error.code(), expected, "unexpected adapter error: {error}"),
            Ok(receipt) => panic!("expected {expected}, got receipt {receipt:?}"),
        }
    }

    #[test]
    fn capabilities_do_not_advertise_false_email_operations() {
        let capabilities = adapter().capabilities();
        assert!(capabilities.supports(ChannelCapability::IngressThread));
        assert!(capabilities.supports(ChannelCapability::IngressTypedAttachments));
        assert!(capabilities.supports(ChannelCapability::EgressReply));
        assert!(!capabilities.supports(ChannelCapability::IngressGroup));
        assert!(!capabilities.supports(ChannelCapability::EgressMarkdown));
        assert!(!capabilities.supports(ChannelCapability::EgressChunking));
        assert!(!capabilities.supports(ChannelCapability::InteractionTyping));
        assert!(!capabilities.supports(ChannelCapability::ReliabilityHeartbeat));
    }

    #[test]
    fn octos_thread_precedence_and_subject_normalization_are_preserved() {
        assert_eq!(
            email_thread_topic(
                Some("<reply@example.test>"),
                Some("<parent@example.test>"),
                Some("<root@example.test> <parent@example.test>"),
                "Re: Quarterly plan",
            ),
            "email-root_example_test"
        );
        assert_eq!(
            email_thread_topic(None, None, None, "Re: Fwd: Quarterly plan"),
            "email-Quarterly_plan"
        );
    }

    #[test]
    fn parser_uses_message_id_and_uid_fallback_and_utf8_safe_limit() {
        let email =
            parse_email_bytes(&fixture("plain"), "uid-7".to_string(), 12000).expect("fixture");
        assert_eq!(email.sender, "user@example.test");
        assert_eq!(email.message_id.as_deref(), Some("plain@example.test"));
        assert_eq!(email.dedup_key(), "plain@example.test");
        assert!(email.text_body.contains("Hello"));

        let mut no_id = email.clone();
        no_id.message_id = None;
        assert_eq!(no_id.dedup_key(), "uid:uid-7");
        assert_eq!(truncate_utf8("你好世界", 3), "你好世");
    }

    #[test]
    fn multipart_fixture_extracts_typed_attachment() {
        let email =
            parse_email_bytes(&fixture("multipart"), "uid-8".to_string(), 12000).expect("fixture");
        assert_eq!(email.attachments.len(), 1);
        assert_eq!(email.attachments[0].media_type, "image/png");
        assert_eq!(email.attachments[0].file_name.as_deref(), Some("pixel.png"));
        assert_eq!(email.attachments[0].bytes, vec![0, 1, 2, 3]);
    }

    #[test]
    fn parser_handles_html_rfc822_and_uid_fallback_shapes() {
        let html = parse_email_bytes(&fixture("html"), "uid-html".to_string(), 12000)
            .expect("HTML fixture");
        assert!(html.text_body.contains("Hello & welcome"));
        assert!(html.text_body.contains("HTML fixture"));
        assert!(!html.text_body.contains('<'));

        let no_id = parse_email_bytes(
            &fixture("no-message-id"),
            "uid-no-message-id".to_string(),
            12000,
        )
        .expect("UID fallback fixture");
        assert_eq!(no_id.message_id, None);
        assert_eq!(no_id.dedup_key(), "uid:uid-no-message-id");
        assert_eq!(no_id.platform_message_id(), "uid:uid-no-message-id");

        let rfc822 = parse_email_bytes(&fixture("rfc822"), "uid-rfc822".to_string(), 12000)
            .expect("RFC822 attachment fixture");
        assert_eq!(rfc822.attachments.len(), 1);
        assert_eq!(rfc822.attachments[0].media_type, "message/rfc822");
        assert_eq!(
            rfc822.attachments[0].file_name.as_deref(),
            Some("nested.eml")
        );
        assert!(String::from_utf8_lossy(&rfc822.attachments[0].bytes)
            .contains("Message-ID: <nested@example.test>"));
    }

    #[test]
    fn malformed_mime_is_typed_and_size_is_rejected_before_transport() {
        let invalid = parse_email_bytes(
            &fixture("invalid-mime"),
            "uid-invalid-mime".to_string(),
            12000,
        )
        .expect_err("invalid MIME fixture must be rejected");
        assert!(matches!(invalid, EmailTransportError::InvalidMime { .. }));

        let too_large = validate_attachment_metadata(
            Some("large.bin"),
            "application/octet-stream",
            MAX_ATTACHMENT_BYTES + 1,
        )
        .expect_err("oversized MIME part must be rejected");
        assert_eq!(too_large.code(), "attachment_too_large");
    }

    #[test]
    fn fixture_transcripts_describe_wire_commands_and_responses() {
        let imap_transcript = include_str!("../../tests/fixtures/c5/email/imap-transcript.txt");
        assert!(imap_transcript.contains("UID SEARCH UNSEEN"));
        assert!(imap_transcript.contains("UID FETCH (BODY.PEEK[] UID)"));
        assert!(imap_transcript.contains("UID STORE <uid> +FLAGS"));
        let smtp_transcript = include_str!("../../tests/fixtures/c5/email/smtp-transcript.txt");
        assert!(smtp_transcript.contains("MAIL FROM:<bot@example.test>"));
        assert!(smtp_transcript.contains("In-Reply-To: <parent@example.test>"));
        assert!(smtp_transcript.contains("250 2.0.0 queued as smtp-fixture-1"));
        assert_eq!(
            parse_smtp_acceptance("250 2.0.0 queued as smtp-fixture-1"),
            Some("smtp-fixture-1".to_string())
        );
    }

    #[tokio::test]
    async fn rich_multipart_is_admitted_as_typed_parts_after_mime_validation() {
        let email = parse_email_bytes(&fixture("multipart-rich"), "uid-rich".to_string(), 12000)
            .expect("rich multipart fixture");
        assert_eq!(
            email
                .attachments
                .iter()
                .map(|attachment| attachment.media_type.as_str())
                .collect::<Vec<_>>(),
            ["audio/wav", "video/mp4", "application/pdf"]
        );

        let fake = Arc::new(FakeEmailTransport::default());
        let adapter = EmailAdapter::with_transport(
            config(),
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        assert!(adapter.process_email(&context, email).await.unwrap());
        let envelope = consumer
            .recv_ingress()
            .await
            .expect("rich multipart admission")
            .envelope()
            .clone();
        let ChannelPayloadV1::Message { parts, .. } = envelope.payload else {
            panic!("expected message payload");
        };
        assert!(parts
            .iter()
            .any(|part| matches!(part, ContentPart::Audio { .. })));
        assert!(parts
            .iter()
            .any(|part| matches!(part, ContentPart::Video { .. })));
        assert!(parts
            .iter()
            .any(|part| matches!(part, ContentPart::File { .. })));
        assert_eq!(
            fake.events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| event.as_str() == "fabric.admission.accepted")
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn send_policy_rejections_happen_before_fake_smtp() {
        let mut cfg = config();
        cfg.consent_granted = false;
        let fake = Arc::new(FakeEmailTransport::default());
        let adapter = EmailAdapter::with_transport(
            cfg,
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        assert_error_code(
            adapter
                .execute(ChannelCommand::Send {
                    envelope: send_envelope("recipient@example.test", "body"),
                    idempotency_key: None,
                })
                .await,
            "consent_required",
        );
        assert!(fake.events.lock().unwrap().is_empty());

        let mut cfg = config();
        cfg.auto_reply_enabled = false;
        let fake = Arc::new(FakeEmailTransport::default());
        let adapter = EmailAdapter::with_transport(
            cfg,
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        let mut reply = send_envelope("recipient@example.test", "reply body");
        reply.correlation.reply_to = Some("<parent@example.test>".to_string());
        assert_error_code(
            adapter
                .execute(ChannelCommand::Send {
                    envelope: reply,
                    idempotency_key: None,
                })
                .await,
            "auto_reply_disabled",
        );
        assert!(fake.events.lock().unwrap().is_empty());

        let fake = Arc::new(FakeEmailTransport::default());
        let adapter = EmailAdapter::with_transport(
            config(),
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        assert_error_code(
            adapter
                .execute(ChannelCommand::Send {
                    envelope: send_envelope("", "body"),
                    idempotency_key: None,
                })
                .await,
            "missing_recipient",
        );
        assert_error_code(
            adapter
                .execute(ChannelCommand::Send {
                    envelope: send_envelope("not-an-email", "body"),
                    idempotency_key: None,
                })
                .await,
            "invalid_email_address",
        );
        assert!(fake.events.lock().unwrap().is_empty());

        let mut cfg = config();
        cfg.from_address = "malformed-from".to_string();
        let fake = Arc::new(FakeEmailTransport::default());
        let adapter = EmailAdapter::with_transport(
            cfg,
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        assert_error_code(
            adapter
                .execute(ChannelCommand::Send {
                    envelope: send_envelope("recipient@example.test", "body"),
                    idempotency_key: None,
                })
                .await,
            "invalid_from_address",
        );
        assert!(fake.events.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn allowlist_rejects_inbound_before_attachment_storage_or_fabric() {
        let fake = Arc::new(FakeEmailTransport::default());
        let mut cfg = config();
        cfg.allow_from = vec!["trusted@example.test".to_string()];
        let adapter = EmailAdapter::with_transport(
            cfg,
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let email = parse_email_bytes(&fixture("multipart"), "uid-blocked".to_string(), 12000)
            .expect("multipart fixture");
        assert!(!adapter.process_email(&context, email).await.unwrap());
        assert!(adapter.processed.lock().await.is_empty());
        assert!(adapter.pending_seen.lock().await.is_empty());
        assert!(fake.events.lock().unwrap().is_empty());
        assert!(
            tokio::time::timeout(Duration::from_millis(20), consumer.recv_ingress())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn outbound_invalid_mime_is_typed_before_smtp() {
        let fake = Arc::new(FakeEmailTransport::default());
        let store = Arc::new(MemoryAttachments::default());
        let attachment = store
            .put(IngressAttachment {
                source_channel: "email".to_string(),
                platform_message_id: Some("fixture-invalid-mime".to_string()),
                sender_id: Some("user@example.test".to_string()),
                file_name: Some("broken.bin".to_string()),
                declared_mime: Some("image".to_string()),
                bytes: vec![1, 2, 3],
            })
            .await
            .unwrap();
        let adapter =
            EmailAdapter::with_transport(config(), AdapterServices::new(store), fake.clone());
        let mut envelope = send_envelope("recipient@example.test", "body");
        if let ChannelPayloadV1::Message { parts, .. } = &mut envelope.payload {
            parts.push(ContentPart::File { attachment });
        }
        assert_error_code(
            adapter
                .execute(ChannelCommand::Send {
                    envelope,
                    idempotency_key: None,
                })
                .await,
            "invalid_mime",
        );
        assert!(fake.sent.lock().unwrap().is_empty());
        assert!(fake.events.lock().unwrap().is_empty());
    }

    #[test]
    fn self_reply_and_allowlist_are_fail_closed() {
        let mut cfg = config();
        cfg.allow_from = vec!["user@example.test".to_string()];
        assert!(!email_sender_allowed(&cfg.allow_from, "other@example.test"));
        assert!(email_sender_allowed(
            &["USER@example.test".to_string()],
            "user@example.test"
        ));
        assert!(should_skip_self_reply(
            "bot@example.test",
            "Re: hello",
            &cfg
        ));
        assert!(!should_skip_self_reply("user@example.test", "hello", &cfg));
    }

    #[test]
    fn smtp_message_id_domain_is_redacted_to_safe_host_component() {
        assert_eq!(message_id_domain("Bot <bot@example.test>"), "example.test");
        assert_eq!(message_id_domain("invalid"), "localhost");
    }

    #[tokio::test]
    async fn inbound_admission_happens_before_processed_marker() {
        let adapter = adapter();
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        let email =
            parse_email_bytes(&fixture("reply"), "uid-reply".to_string(), 12000).expect("fixture");
        assert!(!adapter.processed.lock().await.contains(&email.dedup_key()));
        assert!(adapter.process_email(&context, email.clone()).await.is_ok());
        assert!(adapter.processed.lock().await.contains(&email.dedup_key()));
        let envelope = consumer
            .recv_ingress()
            .await
            .expect("admitted envelope")
            .envelope()
            .clone();
        assert_eq!(envelope.origin, ChannelOrigin::ExternalUser);
        assert_eq!(envelope.direction, ChannelDirection::Ingress);
        assert_eq!(
            envelope.address.thread_id.as_deref(),
            Some("email-root_example_test")
        );
        assert_eq!(
            envelope.correlation.reply_to.as_deref(),
            Some("parent@example.test")
        );
    }

    #[tokio::test]
    async fn fake_imap_poll_admits_before_store_seen_and_deduplicates_uid() {
        let fake = Arc::new(FakeEmailTransport::default());
        let email =
            parse_email_bytes(&fixture("reply"), "uid-reply".to_string(), 12000).expect("fixture");
        fake.fetches
            .lock()
            .unwrap()
            .extend([Ok(vec![email.clone()]), Ok(vec![email])]);
        fake.mark_seen_results.lock().unwrap().extend([
            Err(transport_failure("fixture STORE transient failure")),
            Ok(()),
        ]);
        let mut cfg = config();
        cfg.mark_seen = true;
        let adapter = EmailAdapter::with_transport(
            cfg,
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        assert_eq!(adapter.poll_once(&context).await.unwrap(), 1);
        assert!(adapter
            .processed
            .lock()
            .await
            .contains("reply@example.test"));
        assert_eq!(fake.seen.lock().unwrap().as_slice(), ["uid-reply"]);
        assert_eq!(adapter.health().status, ChannelHealthStatus::Degraded);
        assert!(consumer.recv_ingress().await.is_some());
        assert_eq!(adapter.poll_once(&context).await.unwrap(), 0);
        assert!(
            tokio::time::timeout(Duration::from_millis(20), consumer.recv_ingress())
                .await
                .is_err()
        );
        assert_eq!(
            fake.seen.lock().unwrap().as_slice(),
            ["uid-reply", "uid-reply"]
        );
        assert_eq!(adapter.health().status, ChannelHealthStatus::Healthy);
        let events = fake.events.lock().unwrap().clone();
        assert!(events[0].starts_with("imap.fetch ssl=true"));
        assert_eq!(
            events
                .iter()
                .filter(|event| event.as_str() == "fabric.admission.accepted")
                .count(),
            1
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.contains("imap.store_seen"))
                .count(),
            2
        );
        assert!(
            events
                .iter()
                .position(|event| event == "fabric.admission.accepted")
                .unwrap()
                < events
                    .iter()
                    .position(|event| event.contains("imap.store_seen"))
                    .unwrap()
        );
    }

    #[tokio::test]
    async fn fake_smtp_receives_reply_headers_multipart_and_real_receipt_id() {
        let fake = Arc::new(FakeEmailTransport::default());
        let store = Arc::new(MemoryAttachments::default());
        let attachment = store
            .put(IngressAttachment {
                source_channel: "email".to_string(),
                platform_message_id: Some("fixture".to_string()),
                sender_id: Some("user@example.test".to_string()),
                file_name: Some("pixel.png".to_string()),
                declared_mime: Some("image/png".to_string()),
                bytes: vec![0, 1, 2, 3],
            })
            .await
            .unwrap();
        let adapter =
            EmailAdapter::with_transport(config(), AdapterServices::new(store), fake.clone());
        let mut correlation = Correlation::new("email:recipient@example.test");
        correlation.reply_to = Some("<parent@example.test>".to_string());
        let mut envelope = ChannelEnvelopeV1::new(
            ChannelDirection::Egress,
            ChannelAddress::new("email", "recipient@example.test"),
            correlation,
            ChannelOrigin::Runtime,
            ChannelPayloadV1::Message {
                parts: vec![
                    ContentPart::Text {
                        text: "reply body".to_string(),
                    },
                    ContentPart::Image { attachment },
                ],
                subject: Some("Quarterly plan".to_string()),
                locale: None,
                context: None,
            },
        );
        envelope.extensions.insert(
            "email.references".to_string(),
            json!("<root@example.test> <parent@example.test>"),
        );
        let receipt = adapter
            .execute(ChannelCommand::Send {
                envelope,
                idempotency_key: Some("smtp-fixture".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(
            receipt.platform_message_id.as_deref(),
            Some("smtp-fixture-1")
        );
        let sent = fake.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].to, "recipient@example.test");
        assert_eq!(sent[0].subject, "Quarterly plan");
        assert_eq!(
            sent[0].in_reply_to.as_deref(),
            Some("<parent@example.test>")
        );
        assert_eq!(
            sent[0].references.as_deref(),
            Some("<root@example.test> <parent@example.test>")
        );
        assert_eq!(sent[0].attachments[0].file_name, "pixel.png");
        assert_eq!(sent[0].attachments[0].media_type, "image/png");
        assert_eq!(sent[0].attachments[0].bytes, vec![0, 1, 2, 3]);
        let wire = fake.smtp_wire.lock().unwrap().clone();
        assert!(wire[0].contains("Content-Type: multipart/mixed"));
        assert!(wire[0].contains("Content-Disposition: attachment; filename=\"pixel.png\""));
        assert!(wire[0].contains("Content-Transfer-Encoding: base64"));
        assert!(wire[0].contains("AAECAw=="));
        assert!(wire[0].contains("250 2.0.0 queued as smtp-fixture-1"));
    }

    #[tokio::test]
    async fn fake_transport_records_imap_ssl_and_smtp_security_modes() {
        let fake = Arc::new(FakeEmailTransport::default());
        let mut imap_plain_config = config();
        imap_plain_config.imap_use_ssl = false;
        imap_plain_config.mark_seen = false;
        let adapter = EmailAdapter::with_transport(
            imap_plain_config,
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        fake.fetches.lock().unwrap().push_back(Ok(Vec::new()));
        let (fabric, _consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        assert_eq!(adapter.poll_once(&context).await.unwrap(), 0);
        assert!(fake
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|event| event.starts_with("imap.fetch ssl=false")));

        for (smtp_use_ssl, smtp_use_tls, expected_mode) in [
            (true, true, "ssl"),
            (false, true, "starttls"),
            (false, false, "plain"),
        ] {
            let fake = Arc::new(FakeEmailTransport::default());
            let mut cfg = config();
            cfg.smtp_use_ssl = smtp_use_ssl;
            cfg.smtp_use_tls = smtp_use_tls;
            let adapter = EmailAdapter::with_transport(
                cfg,
                AdapterServices::new(Arc::new(MemoryAttachments::default())),
                fake.clone(),
            );
            adapter
                .execute(ChannelCommand::Send {
                    envelope: send_envelope("recipient@example.test", "mode body"),
                    idempotency_key: None,
                })
                .await
                .expect("fake SMTP mode send");
            let events = fake.events.lock().unwrap().clone();
            assert!(events.iter().any(|event| event
                == &format!("smtp.send mode={expected_mode} command=MAIL FROM RCPT TO DATA")));
            let wire = fake.smtp_wire.lock().unwrap().clone();
            assert!(wire[0].contains(&format!("MODE {expected_mode}")));
            assert!(wire[0].contains("250 2.0.0 queued as smtp-fixture-1"));
        }
    }

    #[tokio::test]
    async fn health_probe_and_reconnect_failures_are_typed_and_observable() {
        let fake = Arc::new(FakeEmailTransport::default());
        fake.probe_results.lock().unwrap().extend([
            Ok(()),
            Err(transport_failure("fixture reconnect failed")),
            Ok(()),
        ]);
        let adapter = EmailAdapter::with_transport(
            config(),
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        let command = ChannelCommand::ProbeHealth {
            channel: ChannelId::new("email").unwrap(),
        };
        adapter
            .execute(command.clone())
            .await
            .expect("healthy probe");
        assert_eq!(adapter.health().status, ChannelHealthStatus::Healthy);
        assert_error_code(
            adapter.execute(command.clone()).await,
            "health_probe_failed",
        );
        assert_eq!(adapter.health().status, ChannelHealthStatus::Degraded);
        adapter.execute(command).await.expect("reconnected probe");
        assert_eq!(adapter.health().status, ChannelHealthStatus::Healthy);
        let events = fake.events.lock().unwrap().clone();
        assert_eq!(
            events
                .iter()
                .filter(|event| event.starts_with("imap.probe ssl=true"))
                .count(),
            3
        );
        assert_eq!(
            events
                .iter()
                .filter(|event| event.starts_with("smtp.probe mode=starttls"))
                .count(),
            3
        );

        let fake = Arc::new(FakeEmailTransport::default());
        fake.fetches.lock().unwrap().extend([
            Err(transport_failure("fixture IMAP connection dropped")),
            Ok(Vec::new()),
        ]);
        let adapter = EmailAdapter::with_transport(
            config(),
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        let (fabric, _consumer) = FabricKernel::new().into_parts();
        let context = AdapterContext {
            fabric,
            cancel: CancellationToken::new(),
        };
        assert_error_code_for_poll(adapter.poll_once(&context).await, "imap_poll_failed");
        assert_eq!(adapter.health().status, ChannelHealthStatus::Degraded);
        assert_eq!(adapter.poll_once(&context).await.unwrap(), 0);
        assert_eq!(adapter.health().status, ChannelHealthStatus::Healthy);
        assert_eq!(
            fake.events
                .lock()
                .unwrap()
                .iter()
                .filter(|event| event.starts_with("imap.fetch ssl=true"))
                .count(),
            2
        );
    }

    #[tokio::test]
    async fn blocking_operation_timeout_is_typed_and_drained() {
        let adapter = adapter();
        let result = adapter
            .run_blocking_with_timeout(Duration::from_millis(1), "fixture_timeout", || {
                std::thread::sleep(Duration::from_millis(30));
                7_u8
            })
            .await;
        assert_error_code_for_any(result, "fixture_timeout");
        adapter.stop().await.expect("drain timed-out task");
    }

    #[tokio::test]
    async fn public_probe_timeout_keeps_email_cleanup_owned_until_drain() {
        let fake = Arc::new(FakeEmailTransport::default());
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        *fake.probe_gate.lock().unwrap() = Some(Arc::new(FakeBlockingGate {
            started: StdMutex::new(started_tx),
            release: StdMutex::new(release_rx),
        }));
        let adapter = Arc::new(EmailAdapter::with_transport(
            config(),
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake,
        ));
        let probe_adapter = adapter.clone();
        let probe_task = tokio::spawn(async move {
            crate::adapter::probe_adapter(probe_adapter, "email", Duration::from_millis(20)).await
        });
        tokio::task::spawn_blocking(move || started_rx.recv())
            .await
            .expect("probe started waiter")
            .expect("blocking email probe started");

        // The single public deadline must not wait for, or drop, cleanup while
        // the native transport is still blocked.
        let mut probe_task = probe_task;
        let result = tokio::time::timeout(Duration::from_millis(40), &mut probe_task)
            .await
            .expect("probe response deadline")
            .expect("probe join");
        assert!(matches!(
            result,
            Err(crate::adapter::ChannelProbeError::Timeout)
        ));
        assert!(crate::adapter::pending_probe_cleanup_tasks().await > 0);
        release_tx.send(()).expect("release email probe");

        for _ in 0..50 {
            if crate::adapter::pending_probe_cleanup_tasks().await == 0 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        panic!("email probe cleanup task was not drained");
    }

    #[tokio::test]
    async fn failed_fabric_admission_does_not_store_seen() {
        let fake = Arc::new(FakeEmailTransport::default());
        let mut cfg = config();
        cfg.mark_seen = true;
        let adapter = EmailAdapter::with_transport(
            cfg,
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        let (fabric, mut consumer) = FabricKernel::new().into_parts();
        let cancel = CancellationToken::new();
        cancel.cancel();
        let context = AdapterContext { fabric, cancel };
        let email = parse_email_bytes(&fixture("reply"), "uid-cancelled".to_string(), 12000)
            .expect("reply fixture");
        assert!(matches!(
            adapter.process_email(&context, email).await,
            Err(AdapterError::Stopped)
        ));
        assert!(adapter.processed.lock().await.is_empty());
        assert!(adapter.pending_seen.lock().await.is_empty());
        assert!(fake.seen.lock().unwrap().is_empty());
        assert!(
            tokio::time::timeout(Duration::from_millis(20), consumer.recv_ingress())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn stop_drains_blocking_smtp_and_cancels_waiter_without_late_send_result() {
        let fake = Arc::new(FakeEmailTransport::default());
        let (started_tx, started_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        *fake.send_gate.lock().unwrap() = Some(Arc::new(FakeBlockingGate {
            started: StdMutex::new(started_tx),
            release: StdMutex::new(release_rx),
        }));
        let adapter = Arc::new(EmailAdapter::with_transport(
            config(),
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        ));
        let send_adapter = adapter.clone();
        let send_task = tokio::spawn(async move {
            send_adapter
                .execute(ChannelCommand::Send {
                    envelope: send_envelope("recipient@example.test", "blocked body"),
                    idempotency_key: None,
                })
                .await
        });
        tokio::task::spawn_blocking(move || started_rx.recv())
            .await
            .expect("started waiter")
            .expect("blocking SMTP started");

        let stop_adapter = adapter.clone();
        let mut stop_task = tokio::spawn(async move { stop_adapter.stop().await });
        assert!(
            tokio::time::timeout(Duration::from_millis(20), &mut stop_task)
                .await
                .is_err()
        );
        release_tx.send(()).expect("release SMTP fake");
        stop_task.await.expect("stop join").expect("stop drain");
        assert!(matches!(
            send_task.await.expect("send join"),
            Err(AdapterError::Stopped)
        ));
        assert_eq!(adapter.health().status, ChannelHealthStatus::Down);
        let event_count = fake.events.lock().unwrap().len();
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert_eq!(fake.events.lock().unwrap().len(), event_count);
    }

    #[tokio::test]
    async fn unsupported_commands_return_without_network_or_smtp_side_effect() {
        let fake = Arc::new(FakeEmailTransport::default());
        let adapter = EmailAdapter::with_transport(
            config(),
            AdapterServices::new(Arc::new(MemoryAttachments::default())),
            fake.clone(),
        );
        let address = ChannelAddress::new("email", "user@example.test");
        let correlation = Correlation::new("email:user@example.test");
        let result = adapter
            .execute(ChannelCommand::Typing {
                address,
                correlation,
                state: agent_diva_core::channel::TypingState::Started,
                idempotency_key: None,
            })
            .await;
        assert!(matches!(
            result,
            Err(AdapterError::UnsupportedCapability {
                capability: ChannelCapability::InteractionTyping
            })
        ));
        assert!(fake.events.lock().unwrap().is_empty());
        assert!(fake.sent.lock().unwrap().is_empty());
    }

    fn assert_error_code_for_poll(result: Result<usize, AdapterError>, expected: &str) {
        match result {
            Err(error) => assert_eq!(error.code(), expected, "unexpected poll error: {error}"),
            Ok(count) => panic!("expected {expected}, got {count} admitted messages"),
        }
    }

    fn assert_error_code_for_any<T>(result: Result<T, AdapterError>, expected: &str) {
        match result {
            Err(error) => assert_eq!(error.code(), expected, "unexpected adapter error: {error}"),
            Ok(_) => panic!("expected {expected}, got success"),
        }
    }

    #[test]
    fn generated_fixture_message_id_is_stable_shape() {
        assert_eq!(message_id_for_fixture(Some("<id@test>"), "1"), "<id@test>");
        assert_eq!(message_id_for_fixture(None, "1"), "uid:1");
    }
}
