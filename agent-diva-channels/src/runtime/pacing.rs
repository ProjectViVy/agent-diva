use crate::adapter::{AdapterError, ChannelAdapter};
use agent_diva_core::channel::{
    capacity, ChannelCapabilities, ChannelCapability, ChannelCommand, ChannelPayloadV1,
    ContentPart, DeliveryReceipt, DeliveryStatus,
};
use serde_json::json;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

const PACING_RETRY_HINT: Duration = Duration::from_millis(100);
const LOCAL_RETRY_BASE: Duration = Duration::from_millis(250);
const MAX_ATTEMPTS: usize = 3;

#[derive(Debug, Error)]
pub enum PacingError {
    #[error("adapter pacing lane is busy; retry after {retry_after:?}")]
    Busy { retry_after: Duration },
    #[error("adapter pacing lane is closed")]
    Closed,
    #[error("adapter pacing request was cancelled")]
    Cancelled,
    #[error("command targets {actual}, but pacing lane belongs to {expected}")]
    WrongAdapter { expected: String, actual: String },
    #[error("invalid adapter command: {0}")]
    InvalidCommand(String),
    #[error(transparent)]
    Adapter(#[from] AdapterError),
}

struct PacingRequest {
    command: ChannelCommand,
    cancel: CancellationToken,
    response: oneshot::Sender<Result<DeliveryReceipt, AdapterError>>,
}

/// Cloneable bounded sender for one adapter pacing worker.
#[derive(Clone)]
pub struct AdapterPacingHandle {
    channel: String,
    sender: mpsc::Sender<PacingRequest>,
    shutdown: CancellationToken,
}

impl std::fmt::Debug for AdapterPacingHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdapterPacingHandle")
            .field("channel", &self.channel)
            .finish_non_exhaustive()
    }
}

impl AdapterPacingHandle {
    pub fn remaining_capacity(&self) -> usize {
        self.sender.capacity()
    }

    /// Await bounded admission, then await the adapter receipt.
    pub async fn submit(
        &self,
        command: ChannelCommand,
        admission_deadline: Duration,
        cancel: &CancellationToken,
    ) -> Result<DeliveryReceipt, PacingError> {
        let actual = command
            .target_channel()
            .map_err(|error| PacingError::InvalidCommand(error.to_string()))?
            .to_string();
        if actual != self.channel {
            return Err(PacingError::WrongAdapter {
                expected: self.channel.clone(),
                actual,
            });
        }
        let (response_tx, response_rx) = oneshot::channel();
        let request = PacingRequest {
            command,
            cancel: cancel.clone(),
            response: response_tx,
        };
        tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => return Err(PacingError::Closed),
            _ = cancel.cancelled() => return Err(PacingError::Cancelled),
            result = tokio::time::timeout(admission_deadline, self.sender.send(request)) => match result {
                Ok(Ok(())) => {}
                Ok(Err(_)) => return Err(PacingError::Closed),
                Err(_) => return Err(PacingError::Busy { retry_after: PACING_RETRY_HINT }),
            }
        }
        tokio::select! {
            biased;
            _ = cancel.cancelled() => Err(PacingError::Cancelled),
            result = response_rx => result.map_err(|_| PacingError::Closed)?.map_err(Into::into),
        }
    }
}

/// Owned worker for one adapter's bounded egress and pacing lifecycle.
pub struct AdapterPacingLane {
    handle: AdapterPacingHandle,
    shutdown: CancellationToken,
    worker: JoinHandle<()>,
}

impl std::fmt::Debug for AdapterPacingLane {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("AdapterPacingLane")
            .field("handle", &self.handle)
            .finish_non_exhaustive()
    }
}

impl AdapterPacingLane {
    pub fn spawn(adapter: Arc<dyn ChannelAdapter>) -> Self {
        let (sender, receiver) = mpsc::channel(capacity::ADAPTER_EGRESS);
        let shutdown = CancellationToken::new();
        let channel = adapter.name().to_string();
        let worker_shutdown = shutdown.clone();
        let worker = tokio::spawn(run_worker(adapter, receiver, worker_shutdown));
        Self {
            handle: AdapterPacingHandle {
                channel,
                sender,
                shutdown: shutdown.clone(),
            },
            shutdown,
            worker,
        }
    }

    pub fn handle(&self) -> AdapterPacingHandle {
        self.handle.clone()
    }

    /// Close admission, interrupt pacing waits and resolve every accepted request.
    pub async fn shutdown(self) {
        self.shutdown.cancel();
        let _ = self.worker.await;
    }
}

async fn run_worker(
    adapter: Arc<dyn ChannelAdapter>,
    mut receiver: mpsc::Receiver<PacingRequest>,
    shutdown: CancellationToken,
) {
    let mut draining = false;
    loop {
        if draining {
            while let Some(request) = receiver.recv().await {
                let _ = request.response.send(Err(AdapterError::Stopped));
            }
            break;
        }
        tokio::select! {
            biased;
            _ = shutdown.cancelled() => {
                receiver.close();
                draining = true;
            }
            request = receiver.recv() => match request {
                Some(request) => {
                    let result = execute_request(
                        adapter.as_ref(),
                        request.command,
                        &request.cancel,
                        &shutdown,
                    ).await;
                    let _ = request.response.send(result);
                }
                None => break,
            }
        }
    }
}

async fn execute_request(
    adapter: &dyn ChannelAdapter,
    command: ChannelCommand,
    cancel: &CancellationToken,
    shutdown: &CancellationToken,
) -> Result<DeliveryReceipt, AdapterError> {
    let commands = plan_commands(command, &adapter.capabilities())?;
    let total = commands.len();
    let mut receipts = Vec::with_capacity(total);
    for (index, command) in commands.into_iter().enumerate() {
        match execute_with_retry(adapter, command.clone(), cancel, shutdown).await {
            Ok(receipt) if receipt_is_success(&receipt) => receipts.push(receipt),
            Ok(receipt) if total == 1 => return Ok(receipt),
            Ok(receipt) => {
                return Ok(partial_receipt(
                    &command,
                    index,
                    total,
                    receipt.retry_after_ms,
                    receipt
                        .diagnosis
                        .as_deref()
                        .unwrap_or("adapter returned a failed receipt"),
                ));
            }
            Err(error) if total == 1 => return Err(error),
            Err(error) => {
                return Ok(partial_receipt(
                    &command,
                    index,
                    total,
                    error.retry_after().map(duration_millis),
                    &error.to_string(),
                ));
            }
        }
    }

    let all_delivered = receipts
        .iter()
        .all(|receipt| receipt.status == DeliveryStatus::Delivered);
    let Some(mut receipt) = receipts.pop() else {
        return Err(AdapterError::Execution {
            code: "empty_command_plan".to_string(),
            diagnosis: "adapter command planning produced no executable command".to_string(),
            retry_after: None,
            retryable: false,
        });
    };
    if total > 1 {
        receipt.status = if all_delivered {
            DeliveryStatus::Delivered
        } else {
            DeliveryStatus::Accepted
        };
        receipt.diagnosis = Some(format!("all {total} chunks accepted"));
    }
    Ok(receipt)
}

async fn execute_with_retry(
    adapter: &dyn ChannelAdapter,
    command: ChannelCommand,
    cancel: &CancellationToken,
    shutdown: &CancellationToken,
) -> Result<DeliveryReceipt, AdapterError> {
    let mut attempt = 1usize;
    loop {
        if cancel.is_cancelled() || shutdown.is_cancelled() {
            return Err(AdapterError::Stopped);
        }
        let execution = tokio::select! {
            biased;
            _ = shutdown.cancelled() => return Err(AdapterError::Stopped),
            _ = cancel.cancelled() => return Err(AdapterError::Stopped),
            result = adapter.execute(command.clone()) => result,
        };
        match execution {
            Ok(receipt) => {
                let retry_after = receipt.retry_after_ms.map(Duration::from_millis);
                if receipt_is_success(&receipt)
                    || retry_after.is_none()
                    || !command.is_retry_safe()
                    || attempt == MAX_ATTEMPTS
                {
                    return Ok(receipt);
                }
                if let Some(retry_after) = retry_after {
                    wait_retry(retry_after, cancel, shutdown).await?;
                }
            }
            Err(error) => {
                if !error.is_retryable() || !command.is_retry_safe() || attempt == MAX_ATTEMPTS {
                    return Err(error);
                }
                let delay = error
                    .retry_after()
                    .unwrap_or_else(|| local_backoff(attempt));
                wait_retry(delay, cancel, shutdown).await?;
            }
        }
        attempt += 1;
    }
}

async fn wait_retry(
    delay: Duration,
    cancel: &CancellationToken,
    shutdown: &CancellationToken,
) -> Result<(), AdapterError> {
    tokio::select! {
        biased;
        _ = shutdown.cancelled() => Err(AdapterError::Stopped),
        _ = cancel.cancelled() => Err(AdapterError::Stopped),
        _ = tokio::time::sleep(delay) => Ok(()),
    }
}

fn local_backoff(completed_attempt: usize) -> Duration {
    LOCAL_RETRY_BASE.saturating_mul(1u32 << completed_attempt.saturating_sub(1).min(3))
}

fn plan_commands(
    command: ChannelCommand,
    capabilities: &ChannelCapabilities,
) -> Result<Vec<ChannelCommand>, AdapterError> {
    let command = degrade_send(command, capabilities)?;
    if let Some(capability) = command
        .required_capabilities()
        .into_iter()
        .find(|capability| !capabilities.supports(*capability))
    {
        return Err(AdapterError::UnsupportedCapability { capability });
    }
    split_send(command, capabilities)
}

fn degrade_send(
    mut command: ChannelCommand,
    capabilities: &ChannelCapabilities,
) -> Result<ChannelCommand, AdapterError> {
    let ChannelCommand::Send { envelope, .. } = &mut command else {
        return Ok(command);
    };
    let ChannelPayloadV1::Message { parts, .. } = &mut envelope.payload else {
        return Ok(command);
    };
    for part in parts {
        match part {
            ContentPart::Markdown { markdown }
                if !capabilities.supports(ChannelCapability::EgressMarkdown)
                    && capabilities.supports(ChannelCapability::EgressText) =>
            {
                *part = ContentPart::Text {
                    text: markdown.clone(),
                };
            }
            ContentPart::Card { schema, body }
                if !capabilities.supports(ChannelCapability::EgressCard)
                    && capabilities.supports(ChannelCapability::EgressText) =>
            {
                *part = ContentPart::Text {
                    text: format!("{schema}: {body}"),
                };
            }
            _ => validate_attachment(part, capabilities)?,
        }
    }
    if envelope.correlation.reply_to.is_some()
        && !capabilities.supports(ChannelCapability::EgressReply)
    {
        return Err(AdapterError::UnsupportedCapability {
            capability: ChannelCapability::EgressReply,
        });
    }
    Ok(command)
}

fn validate_attachment(
    part: &ContentPart,
    capabilities: &ChannelCapabilities,
) -> Result<(), AdapterError> {
    let attachment = match part {
        ContentPart::Image { attachment }
        | ContentPart::Video { attachment }
        | ContentPart::File { attachment }
        | ContentPart::Audio { attachment, .. } => attachment,
        _ => return Ok(()),
    };
    if capabilities
        .limits
        .max_attachment_bytes
        .is_some_and(|maximum| attachment.size_bytes > maximum)
    {
        return Err(AdapterError::Execution {
            code: "attachment_too_large".to_string(),
            diagnosis: format!(
                "attachment is {} bytes, exceeding adapter limit",
                attachment.size_bytes
            ),
            retry_after: None,
            retryable: false,
        });
    }
    if !capabilities.limits.supported_mime_types.is_empty()
        && !capabilities
            .limits
            .supported_mime_types
            .contains(&attachment.media_type)
    {
        return Err(AdapterError::Execution {
            code: "unsupported_attachment".to_string(),
            diagnosis: format!("unsupported MIME type: {}", attachment.media_type),
            retry_after: None,
            retryable: false,
        });
    }
    Ok(())
}

fn split_send(
    command: ChannelCommand,
    capabilities: &ChannelCapabilities,
) -> Result<Vec<ChannelCommand>, AdapterError> {
    let Some(max_chars) = capabilities.limits.max_text_chars else {
        return Ok(vec![command]);
    };
    if max_chars == 0 {
        return Err(AdapterError::Execution {
            code: "invalid_adapter_limit".to_string(),
            diagnosis: "max_text_chars must be greater than zero".to_string(),
            retry_after: None,
            retryable: false,
        });
    }
    let ChannelCommand::Send {
        envelope,
        idempotency_key,
    } = command
    else {
        return Ok(vec![command]);
    };
    let ChannelPayloadV1::Message {
        parts,
        subject,
        locale,
    } = &envelope.payload
    else {
        return Ok(vec![ChannelCommand::Send {
            envelope,
            idempotency_key,
        }]);
    };

    let needs_split = parts.iter().any(|part| match part {
        ContentPart::Text { text } => text.chars().count() > max_chars,
        ContentPart::Markdown { markdown } => markdown.chars().count() > max_chars,
        _ => false,
    });
    if !needs_split {
        return Ok(vec![ChannelCommand::Send {
            envelope,
            idempotency_key,
        }]);
    }
    if !capabilities.supports(ChannelCapability::EgressChunking) {
        return Err(AdapterError::UnsupportedCapability {
            capability: ChannelCapability::EgressChunking,
        });
    }

    let mut chunks = Vec::new();
    for part in parts {
        match part {
            ContentPart::Text { text } => chunks.extend(
                split_chars(text, max_chars)
                    .into_iter()
                    .map(|text| ContentPart::Text { text }),
            ),
            ContentPart::Markdown { markdown } => chunks.extend(
                split_chars(markdown, max_chars)
                    .into_iter()
                    .map(|markdown| ContentPart::Markdown { markdown }),
            ),
            other => chunks.push(other.clone()),
        }
    }
    let total = chunks.len();
    Ok(chunks
        .into_iter()
        .enumerate()
        .map(|(index, part)| {
            let mut chunk_envelope = envelope.clone();
            chunk_envelope.envelope_id = uuid::Uuid::new_v4();
            chunk_envelope.payload = ChannelPayloadV1::Message {
                parts: vec![part],
                subject: if index == 0 { subject.clone() } else { None },
                locale: locale.clone(),
            };
            chunk_envelope
                .extensions
                .insert("agent-diva.chunk_index".to_string(), json!(index));
            chunk_envelope
                .extensions
                .insert("agent-diva.chunk_count".to_string(), json!(total));
            ChannelCommand::Send {
                envelope: chunk_envelope,
                idempotency_key: idempotency_key
                    .as_ref()
                    .map(|key| format!("{key}:{index}:{total}")),
            }
        })
        .collect())
}

fn split_chars(value: &str, max_chars: usize) -> Vec<String> {
    let chars: Vec<char> = value.chars().collect();
    chars
        .chunks(max_chars)
        .map(|chunk| chunk.iter().collect())
        .collect()
}

fn receipt_is_success(receipt: &DeliveryReceipt) -> bool {
    matches!(
        receipt.status,
        DeliveryStatus::Accepted | DeliveryStatus::Delivered
    )
}

fn partial_receipt(
    command: &ChannelCommand,
    failed_index: usize,
    total: usize,
    retry_after_ms: Option<u64>,
    diagnosis: &str,
) -> DeliveryReceipt {
    let (channel, chat_id, thread_id) = command_target(command);
    DeliveryReceipt {
        status: DeliveryStatus::Failed,
        channel,
        chat_id,
        platform_message_id: None,
        thread_id,
        retry_after_ms,
        error_code: Some("partial_delivery".to_string()),
        diagnosis: Some(format!(
            "delivered {failed_index}/{total} chunks; chunk {} failed: {diagnosis}",
            failed_index + 1
        )),
    }
}

fn command_target(command: &ChannelCommand) -> (String, String, Option<String>) {
    match command {
        ChannelCommand::Send { envelope, .. } => (
            envelope.address.channel.clone(),
            envelope.address.chat_id.clone(),
            envelope.address.thread_id.clone(),
        ),
        ChannelCommand::Typing { address, .. }
        | ChannelCommand::Edit { address, .. }
        | ChannelCommand::Delete { address, .. }
        | ChannelCommand::React { address, .. }
        | ChannelCommand::FinalizeStream { address, .. } => (
            address.channel.clone(),
            address.chat_id.clone(),
            address.thread_id.clone(),
        ),
        ChannelCommand::ProbeHealth { channel } => (channel.to_string(), String::new(), None),
    }
}

fn duration_millis(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}
