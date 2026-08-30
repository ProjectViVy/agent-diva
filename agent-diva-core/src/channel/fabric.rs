//! Bounded in-memory transport lanes for the Super Channel Fabric.
//!
//! The Fabric owns transport admission and ordering only. It deliberately does
//! not claim that an admitted envelope has started an Agent generation, and it
//! does not provide durable persistence; the Projection Journal is introduced
//! by CHANNEL-EPIC C3.

use super::{capacity, ChannelContractError, ChannelEnvelopeV1};
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::{mpsc, oneshot, Mutex, Notify, Semaphore};
use tokio::task::JoinSet;
use tokio_util::sync::CancellationToken;

/// Retry hint returned when the bounded ingress lane cannot admit before its deadline.
pub const INGRESS_RETRY_HINT: Duration = Duration::from_millis(100);

/// A logical bounded lane in the Fabric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FabricLane {
    Control,
    Ingress,
    DurableEvent,
    TransientEvent,
}

impl fmt::Display for FabricLane {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Control => "control",
            Self::Ingress => "ingress",
            Self::DurableEvent => "durable_event",
            Self::TransientEvent => "transient_event",
        })
    }
}

/// Errors returned before an envelope is accepted by a Fabric lane.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum FabricAdmissionError {
    #[error("invalid Fabric envelope: {0}")]
    InvalidEnvelope(#[from] ChannelContractError),
    #[error("Fabric {lane} lane is busy; retry after {retry_after:?}")]
    Busy {
        lane: FabricLane,
        retry_after: Duration,
    },
    #[error("Fabric {lane} admission was cancelled")]
    Cancelled { lane: FabricLane },
    #[error("Fabric {lane} lane is closed")]
    Closed { lane: FabricLane },
}

/// Item consumed from the priority ingress side of the Fabric.
#[derive(Debug, Clone, PartialEq)]
pub enum FabricIngressItem {
    Control(ChannelEnvelopeV1),
    Envelope(ChannelEnvelopeV1),
}

impl FabricIngressItem {
    /// Access the enclosed transport envelope.
    pub fn envelope(&self) -> &ChannelEnvelopeV1 {
        match self {
            Self::Control(envelope) | Self::Envelope(envelope) => envelope,
        }
    }
}

/// Stable coalescing identity for high-frequency transient events.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransientKey {
    pub session_key: String,
    pub request_id: Option<String>,
    pub event_class: String,
}

impl TransientKey {
    /// Build a transient key from envelope correlation and a caller-owned event class.
    pub fn from_envelope(envelope: &ChannelEnvelopeV1, event_class: impl Into<String>) -> Self {
        Self {
            session_key: envelope.correlation.session_key.clone(),
            request_id: envelope.correlation.request_id.clone(),
            event_class: event_class.into(),
        }
    }
}

/// Result of publishing a transient event into the coalescing lane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransientPublishOutcome {
    Queued,
    Coalesced,
    GapScheduled,
    Fenced,
}

/// Gap emitted when a new transient key displaces an older key at capacity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FabricTransientGap {
    pub session_key: String,
    pub request_id: Option<String>,
    pub reason: &'static str,
}

/// Item consumed from the transient lane.
#[derive(Debug, Clone, PartialEq)]
pub enum FabricTransientItem {
    Envelope(Box<ChannelEnvelopeV1>),
    Gap(FabricTransientGap),
}

#[derive(Debug)]
struct TransientSlot {
    key: TransientKey,
    envelope: ChannelEnvelopeV1,
    gap_before: Option<FabricTransientGap>,
}

#[derive(Debug, Default)]
struct TransientState {
    queue: VecDeque<TransientSlot>,
    fences: HashMap<(String, String), u64>,
    fence_order: VecDeque<(String, String)>,
    closed: bool,
}

#[derive(Debug, Default)]
struct TransientLane {
    state: Mutex<TransientState>,
    available: Notify,
}

impl TransientLane {
    async fn publish(
        &self,
        key: TransientKey,
        envelope: ChannelEnvelopeV1,
    ) -> Result<TransientPublishOutcome, FabricAdmissionError> {
        envelope.validate()?;
        let mut state = self.state.lock().await;
        if state.closed {
            return Err(FabricAdmissionError::Closed {
                lane: FabricLane::TransientEvent,
            });
        }

        if is_fenced(&state, &key, &envelope) {
            return Ok(TransientPublishOutcome::Fenced);
        }

        if let Some(existing) = state.queue.iter_mut().find(|slot| slot.key == key) {
            existing.envelope = envelope;
            return Ok(TransientPublishOutcome::Coalesced);
        }

        let gap_before = if state.queue.len() == capacity::TRANSIENT_EVENT {
            state.queue.pop_front().map(|displaced| FabricTransientGap {
                session_key: displaced.key.session_key,
                request_id: displaced.key.request_id,
                reason: "transient_capacity_eviction",
            })
        } else {
            None
        };
        let outcome = if gap_before.is_some() {
            TransientPublishOutcome::GapScheduled
        } else {
            TransientPublishOutcome::Queued
        };
        state.queue.push_back(TransientSlot {
            key,
            envelope,
            gap_before,
        });
        drop(state);
        self.available.notify_one();
        Ok(outcome)
    }

    async fn recv(&self) -> Option<FabricTransientItem> {
        loop {
            let notified = self.available.notified();
            let mut state = self.state.lock().await;
            if let Some(mut slot) = state.queue.pop_front() {
                if let Some(gap) = slot.gap_before.take() {
                    state.queue.push_front(slot);
                    return Some(FabricTransientItem::Gap(gap));
                }
                return Some(FabricTransientItem::Envelope(Box::new(slot.envelope)));
            }
            if state.closed {
                return None;
            }
            drop(state);
            notified.await;
        }
    }

    async fn fence(&self, session_key: &str, request_id: &str, sequence: u64) {
        let mut state = self.state.lock().await;
        let key = (session_key.to_string(), request_id.to_string());
        if !state.fences.contains_key(&key) {
            if state.fence_order.len() == capacity::DURABLE_EVENT {
                if let Some(expired) = state.fence_order.pop_front() {
                    state.fences.remove(&expired);
                }
            }
            state.fence_order.push_back(key.clone());
        }
        state.fences.insert(key, sequence);
        state.queue.retain(|slot| {
            !(slot.key.session_key == session_key
                && slot.key.request_id.as_deref() == Some(request_id)
                && slot.envelope.correlation.sequence.unwrap_or_default() > sequence)
        });
    }

    async fn close(&self) {
        self.state.lock().await.closed = true;
        self.available.notify_waiters();
    }
}

fn is_fenced(state: &TransientState, key: &TransientKey, envelope: &ChannelEnvelopeV1) -> bool {
    let Some(request_id) = key.request_id.as_ref() else {
        return false;
    };
    let Some(fence) = state
        .fences
        .get(&(key.session_key.clone(), request_id.clone()))
    else {
        return false;
    };
    envelope.correlation.sequence.unwrap_or_default() > *fence
}

/// Cloneable producer side of all bounded Fabric lanes.
#[derive(Debug, Clone)]
pub struct FabricHandle {
    control: mpsc::Sender<ChannelEnvelopeV1>,
    ingress: mpsc::Sender<ChannelEnvelopeV1>,
    durable: mpsc::Sender<ChannelEnvelopeV1>,
    transient: Arc<TransientLane>,
    shutdown: CancellationToken,
}

impl FabricHandle {
    /// Send a control envelope through reserved capacity.
    pub async fn send_control(
        &self,
        envelope: ChannelEnvelopeV1,
        cancel: &CancellationToken,
    ) -> Result<(), FabricAdmissionError> {
        send_cancellable(
            &self.control,
            envelope,
            FabricLane::Control,
            cancel,
            &self.shutdown,
        )
        .await
    }

    /// Admit ingress before the caller-owned transport deadline.
    pub async fn admit_ingress(
        &self,
        envelope: ChannelEnvelopeV1,
        deadline: Duration,
        cancel: &CancellationToken,
    ) -> Result<(), FabricAdmissionError> {
        envelope.validate()?;
        let send = self.ingress.send(envelope);
        tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => Err(FabricAdmissionError::Closed { lane: FabricLane::Ingress }),
            _ = cancel.cancelled() => Err(FabricAdmissionError::Cancelled { lane: FabricLane::Ingress }),
            result = tokio::time::timeout(deadline, send) => match result {
                Ok(Ok(())) => Ok(()),
                Ok(Err(_)) => Err(FabricAdmissionError::Closed { lane: FabricLane::Ingress }),
                Err(_) => Err(FabricAdmissionError::Busy {
                    lane: FabricLane::Ingress,
                    retry_after: INGRESS_RETRY_HINT,
                }),
            }
        }
    }

    /// Publish a durable in-memory event, waiting for bounded capacity.
    pub async fn publish_durable(
        &self,
        envelope: ChannelEnvelopeV1,
        cancel: &CancellationToken,
    ) -> Result<(), FabricAdmissionError> {
        send_cancellable(
            &self.durable,
            envelope,
            FabricLane::DurableEvent,
            cancel,
            &self.shutdown,
        )
        .await
    }

    /// Publish or coalesce a transient event.
    pub async fn publish_transient(
        &self,
        key: TransientKey,
        envelope: ChannelEnvelopeV1,
    ) -> Result<TransientPublishOutcome, FabricAdmissionError> {
        if self.shutdown.is_cancelled() {
            return Err(FabricAdmissionError::Closed {
                lane: FabricLane::TransientEvent,
            });
        }
        self.transient.publish(key, envelope).await
    }

    /// Fence late transient events after a request has reached cancellation.
    pub async fn fence_request(&self, session_key: &str, request_id: &str, sequence: u64) {
        self.transient
            .fence(session_key, request_id, sequence)
            .await;
    }

    /// Whether shutdown admission has begun.
    pub fn is_closed(&self) -> bool {
        self.shutdown.is_cancelled()
    }
}

async fn send_cancellable(
    sender: &mpsc::Sender<ChannelEnvelopeV1>,
    envelope: ChannelEnvelopeV1,
    lane: FabricLane,
    cancel: &CancellationToken,
    shutdown: &CancellationToken,
) -> Result<(), FabricAdmissionError> {
    envelope.validate()?;
    tokio::select! {
        biased;
        _ = shutdown.cancelled() => Err(FabricAdmissionError::Closed { lane }),
        _ = cancel.cancelled() => Err(FabricAdmissionError::Cancelled { lane }),
        result = sender.send(envelope) => result.map_err(|_| FabricAdmissionError::Closed { lane }),
    }
}

/// Single-owner consumer side of the bounded Fabric lanes.
#[derive(Debug)]
pub struct FabricConsumer {
    control: mpsc::Receiver<ChannelEnvelopeV1>,
    ingress: mpsc::Receiver<ChannelEnvelopeV1>,
    durable: mpsc::Receiver<ChannelEnvelopeV1>,
    transient: Arc<TransientLane>,
    shutdown: CancellationToken,
    control_open: bool,
    ingress_open: bool,
}

/// Constructor for a bounded Super Channel Fabric runtime.
#[derive(Debug)]
pub struct FabricKernel {
    handle: FabricHandle,
    consumer: FabricConsumer,
}

impl FabricKernel {
    /// Create a bounded Fabric kernel.
    pub fn new() -> Self {
        let (handle, consumer) = FabricConsumer::new();
        Self { handle, consumer }
    }

    /// Split the kernel into cloneable producer and single-owner consumer endpoints.
    pub fn into_parts(self) -> (FabricHandle, FabricConsumer) {
        (self.handle, self.consumer)
    }
}

impl Default for FabricKernel {
    fn default() -> Self {
        Self::new()
    }
}

impl FabricConsumer {
    /// Construct a new bounded Fabric producer/consumer pair.
    pub fn new() -> (FabricHandle, Self) {
        let (control_tx, control_rx) = mpsc::channel(capacity::CONTROL);
        let (ingress_tx, ingress_rx) = mpsc::channel(capacity::INGRESS);
        let (durable_tx, durable_rx) = mpsc::channel(capacity::DURABLE_EVENT);
        let transient = Arc::new(TransientLane::default());
        let shutdown = CancellationToken::new();
        (
            FabricHandle {
                control: control_tx,
                ingress: ingress_tx,
                durable: durable_tx,
                transient: transient.clone(),
                shutdown: shutdown.clone(),
            },
            Self {
                control: control_rx,
                ingress: ingress_rx,
                durable: durable_rx,
                transient,
                shutdown,
                control_open: true,
                ingress_open: true,
            },
        )
    }

    /// Receive control before ordinary ingress whenever both are ready.
    pub async fn recv_ingress(&mut self) -> Option<FabricIngressItem> {
        loop {
            match (self.control_open, self.ingress_open) {
                (true, true) => {
                    tokio::select! {
                        biased;
                        item = self.control.recv() => match item {
                            Some(envelope) => return Some(FabricIngressItem::Control(envelope)),
                            None => self.control_open = false,
                        },
                        item = self.ingress.recv() => match item {
                            Some(envelope) => return Some(FabricIngressItem::Envelope(envelope)),
                            None => self.ingress_open = false,
                        },
                    }
                }
                (true, false) => match self.control.recv().await {
                    Some(envelope) => return Some(FabricIngressItem::Control(envelope)),
                    None => self.control_open = false,
                },
                (false, true) => match self.ingress.recv().await {
                    Some(envelope) => return Some(FabricIngressItem::Envelope(envelope)),
                    None => self.ingress_open = false,
                },
                (false, false) => return None,
            }
        }
    }

    /// Receive the next accepted durable in-memory event.
    pub async fn recv_durable(&mut self) -> Option<ChannelEnvelopeV1> {
        self.durable.recv().await
    }

    /// Receive the latest transient event or a capacity gap marker.
    pub async fn recv_transient(&self) -> Option<FabricTransientItem> {
        self.transient.recv().await
    }

    /// Stop new admissions while allowing accepted items to drain.
    pub async fn begin_shutdown(&mut self) {
        self.shutdown.cancel();
        self.control.close();
        self.ingress.close();
        self.durable.close();
        self.transient.close().await;
    }
}

/// Error returned by a transport-neutral ingress sink.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("Fabric ingress dispatch failed ({code}): {diagnosis}")]
pub struct FabricDispatchError {
    pub code: String,
    pub diagnosis: String,
}

/// Downstream admission seam used by the per-session Fabric scheduler.
#[async_trait]
pub trait FabricIngressSink: Send + Sync + 'static {
    async fn submit(&self, envelope: ChannelEnvelopeV1) -> Result<(), FabricDispatchError>;
}

/// Bounded task scheduler that preserves order within a session while allowing
/// independent sessions to submit concurrently.
pub struct FabricIngressScheduler {
    sink: Arc<dyn FabricIngressSink>,
    session_tails: Arc<Mutex<HashMap<String, SessionTail>>>,
    permits: Arc<Semaphore>,
    tasks: JoinSet<Result<(), FabricDispatchError>>,
    shutdown: CancellationToken,
    next_sequence: u64,
}

struct SessionTail {
    sequence: u64,
    completion: oneshot::Receiver<()>,
}

impl fmt::Debug for FabricIngressScheduler {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("FabricIngressScheduler")
            .field("available_permits", &self.permits.available_permits())
            .field("task_count", &self.tasks.len())
            .finish_non_exhaustive()
    }
}

impl FabricIngressScheduler {
    pub fn new(sink: Arc<dyn FabricIngressSink>) -> Self {
        Self {
            sink,
            session_tails: Arc::new(Mutex::new(HashMap::new())),
            permits: Arc::new(Semaphore::new(capacity::INGRESS)),
            tasks: JoinSet::new(),
            shutdown: CancellationToken::new(),
            next_sequence: 1,
        }
    }

    /// Schedule one already-admitted envelope for its session.
    pub async fn dispatch(
        &mut self,
        envelope: ChannelEnvelopeV1,
    ) -> Result<(), FabricAdmissionError> {
        envelope.validate()?;
        let session_key = envelope.correlation.session_key.clone();
        let permit = tokio::select! {
            biased;
            _ = self.shutdown.cancelled() => return Err(FabricAdmissionError::Closed { lane: FabricLane::Ingress }),
            permit = self.permits.clone().acquire_owned() => permit.map_err(|_| FabricAdmissionError::Closed { lane: FabricLane::Ingress })?,
        };
        let sequence = self.next_sequence;
        self.next_sequence = self.next_sequence.saturating_add(1);
        let (completion_tx, completion_rx) = oneshot::channel();
        let previous = {
            let mut tails = self.session_tails.lock().await;
            tails
                .insert(
                    session_key.clone(),
                    SessionTail {
                        sequence,
                        completion: completion_rx,
                    },
                )
                .map(|tail| tail.completion)
        };
        let session_tails = self.session_tails.clone();
        let sink = self.sink.clone();
        self.tasks.spawn(async move {
            if let Some(previous) = previous {
                let _ = previous.await;
            }
            let result = sink.submit(envelope).await;
            let _ = completion_tx.send(());
            let mut tails = session_tails.lock().await;
            if tails
                .get(&session_key)
                .is_some_and(|tail| tail.sequence == sequence)
            {
                tails.remove(&session_key);
            }
            drop(permit);
            result
        });
        Ok(())
    }

    /// Await the next completed downstream submission.
    pub async fn join_next(&mut self) -> Option<Result<(), FabricDispatchError>> {
        match self.tasks.join_next().await {
            Some(Ok(result)) => Some(result),
            Some(Err(error)) => Some(Err(FabricDispatchError {
                code: "join_error".to_string(),
                diagnosis: error.to_string(),
            })),
            None => None,
        }
    }

    /// Reject new work and drain all accepted submissions.
    pub async fn shutdown(mut self) -> Vec<FabricDispatchError> {
        self.shutdown.cancel();
        self.permits.close();
        let mut errors = Vec::new();
        while let Some(result) = self.join_next().await {
            if let Err(error) = result {
                errors.push(error);
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::{
        ChannelAddress, ChannelDirection, ChannelOrigin, ChannelPayloadV1, Correlation, TypingState,
    };

    fn envelope(session: &str, request: &str, sequence: u64) -> ChannelEnvelopeV1 {
        let mut correlation = Correlation::new(session);
        correlation.request_id = Some(request.to_string());
        correlation.sequence = Some(sequence);
        ChannelEnvelopeV1::new(
            ChannelDirection::Ingress,
            ChannelAddress::new("test", session),
            correlation,
            ChannelOrigin::ExternalUser,
            ChannelPayloadV1::Typing {
                state: TypingState::Started,
            },
        )
    }

    #[tokio::test]
    async fn coalescing_keeps_the_latest_envelope() {
        let (handle, consumer) = FabricKernel::new().into_parts();
        let first = envelope("s", "r", 1);
        let latest = envelope("s", "r", 2);
        let key = TransientKey::from_envelope(&first, "delta");
        assert_eq!(
            handle.publish_transient(key.clone(), first).await.unwrap(),
            TransientPublishOutcome::Queued
        );
        assert_eq!(
            handle.publish_transient(key, latest.clone()).await.unwrap(),
            TransientPublishOutcome::Coalesced
        );
        assert_eq!(
            consumer.recv_transient().await,
            Some(FabricTransientItem::Envelope(Box::new(latest)))
        );
    }
}
