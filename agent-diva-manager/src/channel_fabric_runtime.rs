use crate::neuro_link::{NeuroLinkRuntime, NeuroLinkRuntimeError};
use agent_diva_agent::runtime_control::RuntimeControlCommand;
use agent_diva_core::channel::{
    ChannelEnvelopeV1, FabricAdmissionError, FabricConsumer, FabricHandle, FabricIngressItem,
    TurnCancelParams, TurnCancelResultV1, TurnStartResultV1,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio::time::timeout;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

type AdmissionResult = Result<agent_diva_core::bus::SessionAdmissionObservation, String>;
type PendingAdmissions = Arc<Mutex<HashMap<Uuid, oneshot::Sender<AdmissionResult>>>>;

#[derive(Clone)]
pub struct FabricNeuroLinkRuntime {
    fabric: FabricHandle,
    pending: PendingAdmissions,
    control_tx: mpsc::Sender<RuntimeControlCommand>,
}

impl FabricNeuroLinkRuntime {
    pub fn new(
        fabric: FabricHandle,
        pending: PendingAdmissions,
        control_tx: mpsc::Sender<RuntimeControlCommand>,
    ) -> Self {
        Self {
            fabric,
            pending,
            control_tx,
        }
    }
}

#[async_trait]
impl NeuroLinkRuntime for FabricNeuroLinkRuntime {
    async fn start_turn(
        &self,
        envelope: ChannelEnvelopeV1,
    ) -> Result<TurnStartResultV1, NeuroLinkRuntimeError> {
        let envelope_id = envelope.envelope_id;
        let session_key = envelope.correlation.session_key.clone();
        let request_id = envelope.correlation.request_id.clone().unwrap_or_default();
        let trace_id = envelope.correlation.trace_id.clone().unwrap_or_default();
        let (reply_tx, reply_rx) = oneshot::channel();
        self.pending.lock().await.insert(envelope_id, reply_tx);
        let cancel = CancellationToken::new();
        if let Err(error) = self
            .fabric
            .admit_ingress(envelope, Duration::from_secs(2), &cancel)
            .await
        {
            self.pending.lock().await.remove(&envelope_id);
            return Err(map_fabric_error(error));
        }
        let admission = match timeout(Duration::from_secs(5), reply_rx).await {
            Ok(Ok(result)) => result.map_err(NeuroLinkRuntimeError::Rejected)?,
            Ok(Err(_)) => {
                self.pending.lock().await.remove(&envelope_id);
                return Err(NeuroLinkRuntimeError::Unavailable(
                    "Fabric admission reply dropped".into(),
                ));
            }
            Err(_) => {
                self.pending.lock().await.remove(&envelope_id);
                return Err(NeuroLinkRuntimeError::Unavailable(
                    "Fabric admission timed out".into(),
                ));
            }
        };
        if admission.session_key != session_key
            || admission.request_id != request_id
            || admission.trace_id != trace_id
        {
            return Err(NeuroLinkRuntimeError::Internal(
                "Fabric returned mismatched admission correlation".into(),
            ));
        }
        Ok(TurnStartResultV1 {
            session_key,
            request_id,
            trace_id,
            admission,
        })
    }

    async fn cancel_turn(
        &self,
        params: TurnCancelParams,
    ) -> Result<TurnCancelResultV1, NeuroLinkRuntimeError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.control_tx
            .send(RuntimeControlCommand::StopSession {
                session_key: params.session_key,
                request_id: Some(params.request_id),
                reply_tx,
            })
            .await
            .map_err(|_| {
                NeuroLinkRuntimeError::Unavailable("AgentLoop control lane is closed".into())
            })?;
        let outcome = timeout(Duration::from_secs(5), reply_rx)
            .await
            .map_err(|_| {
                NeuroLinkRuntimeError::Unavailable("AgentLoop cancellation timed out".into())
            })?
            .map_err(|_| {
                NeuroLinkRuntimeError::Unavailable("AgentLoop dropped cancellation reply".into())
            })?;
        Ok(TurnCancelResultV1 { outcome })
    }
}

pub fn pending_admissions() -> PendingAdmissions {
    Arc::new(Mutex::new(HashMap::new()))
}

pub fn spawn_fabric_ingress(
    mut consumer: FabricConsumer,
    pending: PendingAdmissions,
    control_tx: mpsc::Sender<RuntimeControlCommand>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        while let Some(item) = consumer.recv_ingress().await {
            let envelope = match item {
                FabricIngressItem::Envelope(envelope) | FabricIngressItem::Control(envelope) => {
                    envelope
                }
            };
            let envelope_id = envelope.envelope_id;
            let owner_reply = pending.lock().await.remove(&envelope_id);
            let (reply_tx, reply_rx) = oneshot::channel();
            if control_tx
                .send(RuntimeControlCommand::StartChannelTurn {
                    envelope: Box::new(envelope),
                    reply_tx,
                })
                .await
                .is_err()
            {
                if let Some(reply) = owner_reply {
                    let _ = reply.send(Err("AgentLoop control lane is closed".into()));
                }
                break;
            }
            tokio::spawn(async move {
                match reply_rx.await {
                    Ok(result) => {
                        if let Some(reply) = owner_reply {
                            let _ = reply.send(result);
                        } else if let Err(error) = result {
                            tracing::warn!(%error, "external channel ingress was rejected");
                        }
                    }
                    Err(_) => {
                        if let Some(reply) = owner_reply {
                            let _ = reply.send(Err("AgentLoop dropped admission reply".into()));
                        }
                    }
                }
            });
        }
        consumer.begin_shutdown().await;
    })
}

fn map_fabric_error(error: FabricAdmissionError) -> NeuroLinkRuntimeError {
    match error {
        FabricAdmissionError::Busy { .. } => NeuroLinkRuntimeError::Busy(error.to_string()),
        FabricAdmissionError::InvalidEnvelope(_) => {
            NeuroLinkRuntimeError::Rejected(error.to_string())
        }
        FabricAdmissionError::Closed { .. } | FabricAdmissionError::Cancelled { .. } => {
            NeuroLinkRuntimeError::Unavailable(error.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::bus::{SessionAdmissionObservation, SessionAdmissionPhase};
    use agent_diva_core::channel::{
        ChannelAddress, ChannelDirection, ChannelOrigin, ChannelPayloadV1, ContentPart,
        Correlation, FabricKernel, OwnerTurnContextV1, OwnerTurnIntent,
    };

    #[tokio::test]
    async fn neuro_link_reaches_agent_loop_only_after_fabric_admission() {
        let (fabric, consumer) = FabricKernel::new().into_parts();
        let pending = pending_admissions();
        let (control_tx, mut control_rx) =
            mpsc::channel(agent_diva_core::channel::capacity::CONTROL);
        let ingress = spawn_fabric_ingress(consumer, pending.clone(), control_tx.clone());
        let runtime = FabricNeuroLinkRuntime::new(fabric, pending, control_tx);
        let responder = tokio::spawn(async move {
            let RuntimeControlCommand::StartChannelTurn { envelope, reply_tx } =
                control_rx.recv().await.unwrap()
            else {
                panic!("expected typed turn");
            };
            assert_eq!(envelope.address.channel, "neuro-link");
            let _ = reply_tx.send(Ok(SessionAdmissionObservation {
                code: None,
                phase: SessionAdmissionPhase::Queued,
                session_key: envelope.correlation.session_key.clone(),
                request_id: envelope.correlation.request_id.clone().unwrap(),
                trace_id: envelope.correlation.trace_id.clone().unwrap(),
                queue_depth: 1,
                wait_latency_ms: 0,
            }));
        });
        let mut correlation = Correlation::new("gui:main");
        correlation.request_id = Some("request-1".into());
        correlation.trace_id = Some("trace-1".into());
        let result = runtime
            .start_turn(ChannelEnvelopeV1::new(
                ChannelDirection::Ingress,
                ChannelAddress::new("neuro-link", "main"),
                correlation,
                ChannelOrigin::OwnerFrontend,
                ChannelPayloadV1::Message {
                    parts: vec![ContentPart::Text {
                        text: "hello".into(),
                    }],
                    subject: None,
                    locale: None,
                    context: Some(OwnerTurnContextV1 {
                        intent: OwnerTurnIntent::Agent,
                        approval_policy: None,
                        execution: None,
                    }),
                },
            ))
            .await
            .unwrap();
        assert_eq!(result.request_id, "request-1");
        responder.await.unwrap();
        ingress.abort();
    }
}
