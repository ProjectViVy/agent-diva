//! Native Email adapter for the Super Channel Fabric.
//!
//! This module deliberately keeps the existing DIVA IMAP/SMTP behavior instead
//! of routing through `EmailHandler`.  IMAP and SMTP are blocking libraries in
//! this workspace, so every blocking operation is isolated behind
//! `spawn_blocking`; the Fabric admission and cancellation boundary remains
//! asynchronous and bounded.

use crate::adapter::{
    accepted_receipt, external_message_envelope, is_sender_allowed, AdapterContext, AdapterError,
    AdapterServices, ChannelAdapter, IngressAttachment,
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
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock as StdRwLock};
use std::time::Duration;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{debug, info, warn};

const DEFAULT_MAILBOX: &str = "INBOX";
const DEFAULT_POLL_INTERVAL_SECS: u64 = 30;
const MAX_EMAIL_TOPIC_CHARS: usize = 120;
const MAX_ATTACHMENT_BYTES: u64 = 25 * 1024 * 1024;
const FABRIC_ADMISSION_DEADLINE: Duration = Duration::from_secs(1);

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
    processed: Arc<Mutex<HashSet<String>>>,
    shutdown: CancellationToken,
    health: Arc<StdRwLock<ChannelHealth>>,
    next_message_id: AtomicU64,
}

impl EmailAdapter {
    /// Construct a native adapter without registering or starting it.
    pub(crate) fn new(config: EmailConfig, services: AdapterServices) -> Self {
        Self {
            config,
            services,
            processed: Arc::new(Mutex::new(HashSet::new())),
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
        if self.processed.lock().await.contains(&key) {
            return Ok(false);
        }

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

        for attachment in &email.attachments {
            if attachment.bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
                return Err(AdapterError::Execution {
                    code: "attachment_too_large".to_string(),
                    diagnosis: "email attachment exceeds configured bound".to_string(),
                    retry_after: None,
                    retryable: false,
                });
            }
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

        // The marker is committed only after Fabric acceptance.  Mark-seen is
        // a separate blocking operation so a busy Fabric leaves the message
        // eligible for retry.
        self.processed.lock().await.insert(key);
        if self.config.mark_seen {
            self.mark_seen_after_admission(email.uid).await;
        }
        Ok(true)
    }

    async fn mark_seen_after_admission(&self, uid: String) {
        let config = self.config.clone();
        let mailbox = self.mailbox().to_string();
        let handle =
            tokio::task::spawn_blocking(move || mark_seen_blocking(&config, &mailbox, &uid));

        tokio::select! {
            result = handle => match result {
                Ok(Ok(())) => debug!("email marked seen after Fabric admission"),
                Ok(Err(error)) => {
                    self.mark_degraded("IMAP mark-seen failed");
                    warn!("IMAP mark-seen failed: {error}");
                }
                Err(error) => {
                    self.mark_degraded("IMAP mark-seen task failed");
                    warn!("IMAP mark-seen task failed: {error}");
                }
            },
            _ = self.shutdown.cancelled() => {
                // Never abort a blocking IMAP call.  Fence its late result by
                // detaching the join handle after shutdown wins the select.
                debug!("detaching in-flight IMAP mark-seen after cancellation");
            }
        }
    }

    async fn poll_once(&self, context: &AdapterContext) -> Result<usize, AdapterError> {
        let processed = self.processed.lock().await.clone();
        let config = self.config.clone();
        let mailbox = self.mailbox().to_string();
        let fetch_task: JoinHandle<Result<Vec<ParsedEmail>, String>> =
            tokio::task::spawn_blocking(move || {
                fetch_messages_blocking(&config, &mailbox, &processed)
            });

        let messages = tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => {
                // The blocking task owns its resources and is intentionally
                // detached; aborting it could leave imap state inconsistent.
                return Ok(0);
            }
            _ = context.cancel.cancelled() => return Ok(0),
            result = fetch_task => result
                .map_err(|error| AdapterError::Execution {
                    code: "imap_task_failed".to_string(),
                    diagnosis: error.to_string(),
                    retry_after: None,
                    retryable: true,
                })?
                .map_err(|error| AdapterError::Execution {
                    code: "imap_poll_failed".to_string(),
                    diagnosis: error,
                    retry_after: None,
                    retryable: true,
                })?,
        };

        let mut accepted = 0;
        for email in messages {
            if self.shutdown.is_cancelled() || context.cancel.is_cancelled() {
                break;
            }
            if self.process_email(context, email).await? {
                accepted += 1;
            }
        }
        self.mark_healthy();
        Ok(accepted)
    }

    async fn run_poll_loop(&self, context: AdapterContext) -> Result<(), AdapterError> {
        let interval = self.poll_interval();
        loop {
            if self.shutdown.is_cancelled() || context.cancel.is_cancelled() {
                self.mark_down();
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
                    return Ok(());
                }
                _ = context.cancel.cancelled() => {
                    self.mark_down();
                    return Ok(());
                }
                _ = tokio::time::sleep(interval) => {}
            }
        }
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
        validate_email_address(&recipient)?;

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
        let attachments = self.outbound_attachments(&parts).await?;
        let from = if !self.config.from_address.trim().is_empty() {
            self.config.from_address.trim().to_string()
        } else if !self.config.smtp_username.trim().is_empty() {
            self.config.smtp_username.trim().to_string()
        } else {
            self.config.imap_username.trim().to_string()
        };
        validate_email_address(&from)?;

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
        let send_task =
            tokio::task::spawn_blocking(move || smtp_send_blocking(&config, smtp_message));

        let result = tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => {
                // Do not abort an active SMTP transaction.  It is detached and
                // cannot feed a late result back into this adapter.
                return Err(AdapterError::Stopped);
            }
            result = send_task => result
                .map_err(|error| AdapterError::Execution {
                    code: "smtp_task_failed".to_string(),
                    diagnosis: error.to_string(),
                    retry_after: None,
                    retryable: true,
                })?
                .map_err(|error| AdapterError::Execution {
                    code: "smtp_send_failed".to_string(),
                    diagnosis: error,
                    retry_after: None,
                    retryable: true,
                }),
        }?;

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
            stored.validate().map_err(|error| AdapterError::Execution {
                code: "attachment_invalid".to_string(),
                diagnosis: error.to_string(),
                retry_after: None,
                retryable: false,
            })?;
            if stored.bytes.len() as u64 > MAX_ATTACHMENT_BYTES {
                return Err(AdapterError::Execution {
                    code: "attachment_too_large".to_string(),
                    diagnosis: "email attachment exceeds configured bound".to_string(),
                    retry_after: None,
                    retryable: false,
                });
            }
            attachments.push(OutboundAttachment {
                file_name: reference
                    .file_name
                    .clone()
                    .unwrap_or_else(|| "attachment".to_string()),
                media_type: reference.media_type.clone(),
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
            ChannelCommand::ProbeHealth { .. } => Ok(accepted_receipt("email", "", None, None)),
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
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct OutboundAttachment {
    file_name: String,
    media_type: String,
    bytes: Vec<u8>,
}

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

fn parse_email_bytes(body: &[u8], uid: String, max_body_chars: usize) -> Option<ParsedEmail> {
    let parsed = MessageParser::default().parse(body)?;
    let sender = parsed
        .from()
        .and_then(|addresses| addresses.first())
        .and_then(|address| address.address())
        .map(canonical_email_address)
        .filter(|sender| !sender.is_empty())?;
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
        .or_else(|| parsed.body_html(0).map(|body| body.into_owned()))
        .unwrap_or_default();
    let text_body = truncate_utf8(raw_body.trim(), max_body_chars);
    let date = parsed
        .date()
        .map(|date| date.to_rfc3339())
        .unwrap_or_default();

    let attachments = parsed
        .attachments()
        .filter_map(|part| {
            let bytes = match &part.body {
                PartType::Binary(bytes) | PartType::InlineBinary(bytes) => bytes.to_vec(),
                PartType::Text(text) | PartType::Html(text) => text.as_bytes().to_vec(),
                PartType::Message(message) => message.raw_message.to_vec(),
                PartType::Multipart(_) => return None,
            };
            let media_type = part
                .content_type()
                .map(|content_type| {
                    content_type
                        .c_subtype
                        .as_deref()
                        .map(|subtype| format!("{}/{}", content_type.c_type, subtype))
                        .unwrap_or_else(|| content_type.c_type.to_string())
                })
                .unwrap_or_else(|| "application/octet-stream".to_string());
            Some(ParsedAttachment {
                file_name: part.attachment_name().map(ToOwned::to_owned),
                media_type,
                bytes,
            })
        })
        .collect();

    Some(ParsedEmail {
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
) -> Result<Vec<ParsedEmail>, String> {
    use native_tls::TlsConnector;

    let tls = TlsConnector::new().map_err(|error| format!("IMAP TLS setup failed: {error}"))?;
    let client = imap::connect(
        (config.imap_host.as_str(), config.imap_port),
        &config.imap_host,
        &tls,
    )
    .map_err(|error| format!("IMAP connect failed: {error}"))?;
    let mut session = client
        .login(&config.imap_username, &config.imap_password)
        .map_err(|(error, _)| format!("IMAP login failed: {error:?}"))?;
    session
        .select(mailbox)
        .map_err(|error| format!("IMAP SELECT failed: {error}"))?;
    let uids = session
        .uid_search("UNSEEN")
        .map_err(|error| format!("IMAP SEARCH failed: {error}"))?;
    let mut messages = Vec::new();
    for uid in uids {
        let uid_string = uid.to_string();
        let fetched = session
            .uid_fetch(&uid_string, "(BODY.PEEK[] UID)")
            .map_err(|error| format!("IMAP FETCH failed: {error}"))?;
        for fetch in fetched.iter() {
            if let Some(body) = fetch.body() {
                if let Some(email) =
                    parse_email_bytes(body, uid_string.clone(), config.max_body_chars)
                {
                    if !processed.contains(&email.dedup_key()) {
                        messages.push(email);
                    }
                }
            }
        }
    }
    let _ = session.logout();
    Ok(messages)
}

fn mark_seen_blocking(config: &EmailConfig, mailbox: &str, uid: &str) -> Result<(), String> {
    use native_tls::TlsConnector;

    let tls = TlsConnector::new().map_err(|error| format!("IMAP TLS setup failed: {error}"))?;
    let client = imap::connect(
        (config.imap_host.as_str(), config.imap_port),
        &config.imap_host,
        &tls,
    )
    .map_err(|error| format!("IMAP connect failed: {error}"))?;
    let mut session = client
        .login(&config.imap_username, &config.imap_password)
        .map_err(|(error, _)| format!("IMAP login failed: {error:?}"))?;
    session
        .select(mailbox)
        .map_err(|error| format!("IMAP SELECT failed: {error}"))?;
    session
        .uid_store(uid, "+FLAGS (\\Seen)")
        .map_err(|error| format!("IMAP STORE failed: {error}"))?;
    let _ = session.logout();
    Ok(())
}

fn smtp_send_blocking(config: &EmailConfig, message: SmtpMessage) -> Result<String, String> {
    use lettre::message::header::ContentType;
    use lettre::message::{Attachment, MultiPart, SinglePart};
    use lettre::transport::smtp::authentication::Credentials;
    use lettre::{Message, SmtpTransport, Transport};

    let mut builder = Message::builder()
        .from(
            message
                .from
                .parse()
                .map_err(|error| format!("invalid from address: {error}"))?,
        )
        .to(message
            .to
            .parse()
            .map_err(|error| format!("invalid recipient address: {error}"))?)
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
            .map_err(|error| format!("build email failed: {error}"))?
    } else {
        let multipart = message.attachments.into_iter().fold(
            MultiPart::mixed().singlepart(SinglePart::plain(message.body)),
            |multipart, attachment| {
                let content_type = attachment.media_type.parse().unwrap_or_else(|_| {
                    ContentType::parse("application/octet-stream")
                        .unwrap_or(ContentType::TEXT_PLAIN)
                });
                multipart.singlepart(
                    Attachment::new(attachment.file_name).body(attachment.bytes, content_type),
                )
            },
        );
        builder
            .multipart(multipart)
            .map_err(|error| format!("build multipart email failed: {error}"))?
    };

    let credentials = Credentials::new(config.smtp_username.clone(), config.smtp_password.clone());
    let transport = if config.smtp_use_ssl {
        SmtpTransport::relay(&config.smtp_host)
            .map_err(|error| format!("SMTP relay setup failed: {error}"))?
            .credentials(credentials)
            .port(config.smtp_port)
            .build()
    } else if config.smtp_use_tls {
        SmtpTransport::starttls_relay(&config.smtp_host)
            .map_err(|error| format!("SMTP STARTTLS setup failed: {error}"))?
            .credentials(credentials)
            .port(config.smtp_port)
            .build()
    } else {
        SmtpTransport::builder_dangerous(&config.smtp_host)
            .credentials(credentials)
            .port(config.smtp_port)
            .build()
    };
    transport
        .send(&email)
        .map_err(|error| format!("SMTP send failed: {error}"))?;
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
    use agent_diva_core::channel::{ChannelDirection, ChannelOrigin, FabricKernel};
    use agent_diva_core::config::schema::EmailConfig;
    use async_trait::async_trait;
    use sha2::Digest;
    use std::collections::BTreeMap;

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
            _ => Vec::new(),
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
    async fn unsupported_commands_return_without_network_or_smtp_side_effect() {
        let adapter = adapter();
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
    }

    #[test]
    fn generated_fixture_message_id_is_stable_shape() {
        assert_eq!(message_id_for_fixture(Some("<id@test>"), "1"), "<id@test>");
        assert_eq!(message_id_for_fixture(None, "1"), "uid:1");
    }
}
