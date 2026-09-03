//! Async message queue implementation

use super::events::{AgentBusEvent, AgentEvent, InboundMessage, OutboundMessage, PokeEvent};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

/// Type alias for message channel senders
pub type OutboundSender = mpsc::Sender<OutboundMessage>;
pub type OutboundReceiver = mpsc::Receiver<OutboundMessage>;

const AGENT_MESSAGE_CAPACITY: usize = 256;

/// Async message bus that decouples chat channels from the agent core
///
/// Channels push messages to the inbound queue, and the agent processes
/// them and pushes responses to the outbound queue.
#[derive(Clone)]
pub struct MessageBus {
    /// Inbound messages from channels
    inbound_tx: mpsc::Sender<InboundMessage>,
    inbound_rx: Arc<RwLock<Option<mpsc::Receiver<InboundMessage>>>>,
    /// Outbound messages to channels
    outbound_tx: mpsc::Sender<OutboundMessage>,
    outbound_rx: Arc<RwLock<Option<mpsc::Receiver<OutboundMessage>>>>,
    /// Event broadcast channel
    event_tx: broadcast::Sender<AgentBusEvent>,
    /// Poke event broadcast channel for lifecycle/audit events
    poke_event_tx: broadcast::Sender<PokeEvent>,
    /// Running state
    running: Arc<RwLock<bool>>,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::channel(AGENT_MESSAGE_CAPACITY);
        let (outbound_tx, outbound_rx) = mpsc::channel(AGENT_MESSAGE_CAPACITY);
        let (event_tx, _) = broadcast::channel(1024);
        let (poke_event_tx, _) = broadcast::channel(1024);

        Self {
            inbound_tx,
            inbound_rx: Arc::new(RwLock::new(Some(inbound_rx))),
            outbound_tx,
            outbound_rx: Arc::new(RwLock::new(Some(outbound_rx))),
            event_tx,
            poke_event_tx,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Publish an event to the broadcast channel
    pub fn publish_event(
        &self,
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        event: AgentEvent,
    ) -> crate::Result<()> {
        let bus_event = AgentBusEvent {
            channel: channel.into(),
            chat_id: chat_id.into(),
            session_key: None,
            request_id: None,
            trace_id: None,
            event,
        };
        // We ignore the error if there are no receivers
        let _ = self.event_tx.send(bus_event);
        Ok(())
    }

    /// Publish a turn event with exact request/trace/session correlation.
    pub fn publish_correlated_event(
        &self,
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        session_key: impl Into<String>,
        request_id: impl Into<String>,
        trace_id: impl Into<String>,
        event: AgentEvent,
    ) -> crate::Result<()> {
        let bus_event = AgentBusEvent {
            channel: channel.into(),
            chat_id: chat_id.into(),
            session_key: Some(session_key.into()),
            request_id: Some(request_id.into()),
            trace_id: Some(trace_id.into()),
            event,
        };
        let _ = self.event_tx.send(bus_event);
        Ok(())
    }

    /// Subscribe to the event broadcast channel
    pub fn subscribe_events(&self) -> broadcast::Receiver<AgentBusEvent> {
        self.event_tx.subscribe()
    }

    /// Publish a poke event to the broadcast channel
    pub fn publish_poke_event(&self, event: PokeEvent) -> crate::Result<()> {
        // We ignore the error if there are no receivers
        let _ = self.poke_event_tx.send(event);
        Ok(())
    }

    /// Subscribe to the poke event broadcast channel
    pub fn subscribe_poke_events(&self) -> broadcast::Receiver<PokeEvent> {
        self.poke_event_tx.subscribe()
    }

    /// Take the inbound receiver (can only be called once)
    pub async fn take_inbound_receiver(&self) -> Option<mpsc::Receiver<InboundMessage>> {
        self.inbound_rx.write().await.take()
    }

    /// Take the outbound receiver (can only be called once)
    pub async fn take_outbound_receiver(&self) -> Option<mpsc::Receiver<OutboundMessage>> {
        self.outbound_rx.write().await.take()
    }

    /// Publish a message from a channel to the agent
    pub fn publish_inbound(&self, msg: InboundMessage) -> crate::Result<()> {
        self.inbound_tx
            .try_send(msg)
            .map_err(|error| crate::Error::Channel(format!("Inbound channel unavailable: {error}")))
    }

    /// Publish a response from the agent to channels
    pub fn publish_outbound(&self, msg: OutboundMessage) -> crate::Result<()> {
        self.outbound_tx.try_send(msg).map_err(|error| {
            crate::Error::Channel(format!("Outbound channel unavailable: {error}"))
        })
    }

    /// Stop the dispatcher loop
    pub async fn stop(&self) {
        *self.running.write().await = false;
    }

    /// Check if the bus is running
    pub async fn is_running(&self) -> bool {
        *self.running.read().await
    }
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_bus_creation() {
        let bus = MessageBus::new();
        assert!(!bus.is_running().await);
    }

    #[tokio::test]
    async fn test_publish_inbound() {
        let bus = MessageBus::new();
        let mut inbound_rx = bus.take_inbound_receiver().await.unwrap();

        let msg = InboundMessage::new("test", "user1", "chat1", "Hello");
        assert!(bus.publish_inbound(msg.clone()).is_ok());

        // Verify message was received
        let received = inbound_rx.try_recv();
        assert!(received.is_ok());
    }

    #[tokio::test]
    async fn inbound_queue_rejects_when_capacity_is_exhausted() {
        let bus = MessageBus::new();
        for index in 0..AGENT_MESSAGE_CAPACITY {
            bus.publish_inbound(InboundMessage::new(
                "test",
                "user1",
                format!("chat-{index}"),
                "queued",
            ))
            .expect("messages within the fixed capacity should be accepted");
        }

        let error = bus
            .publish_inbound(InboundMessage::new("test", "user1", "overflow", "rejected"))
            .expect_err("the bounded queue must reject overflow");
        assert!(error.to_string().contains("no available capacity"));
    }

    #[tokio::test]
    async fn test_poke_event_publish_subscribe() {
        let bus = MessageBus::new();
        let mut poke_rx = bus.subscribe_poke_events();

        bus.publish_poke_event(PokeEvent::ChatReceived {
            content: "hello".to_string(),
            sender_id: "user1".to_string(),
        })
        .unwrap();

        let received = poke_rx.try_recv().unwrap();
        match received {
            PokeEvent::ChatReceived { content, sender_id } => {
                assert_eq!(content, "hello");
                assert_eq!(sender_id, "user1");
            }
            _ => panic!("Expected ChatReceived, got {:?}", received),
        }
    }

    #[tokio::test]
    async fn test_both_event_channels_independent() {
        let bus = MessageBus::new();
        let mut event_rx = bus.subscribe_events();
        let mut poke_rx = bus.subscribe_poke_events();

        // Publish an AgentEvent
        bus.publish_event(
            "ch",
            "chat1",
            AgentEvent::FinalResponse {
                content: "response".to_string(),
            },
        )
        .unwrap();

        // Publish a PokeEvent
        bus.publish_poke_event(PokeEvent::PokeSend {
            message: "poke".to_string(),
        })
        .unwrap();

        // event_rx should only receive the AgentEvent
        let agent_received = event_rx.try_recv().unwrap();
        assert!(matches!(
            agent_received.event,
            AgentEvent::FinalResponse { .. }
        ));

        // poke_rx should only receive the PokeEvent
        let poke_received = poke_rx.try_recv().unwrap();
        assert!(matches!(poke_received, PokeEvent::PokeSend { .. }));

        // No cross-contamination: the other channel should have no further events
        assert!(event_rx.try_recv().is_err());
        assert!(poke_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn test_publish_event_wraps_correlation_context() {
        let bus = MessageBus::new();
        let mut event_rx = bus.subscribe_events();

        // The event stream wraps the agent event with channel correlation.
        bus.publish_event(
            "test_channel",
            "test_chat",
            AgentEvent::AssistantDelta {
                text: "delta content".to_string(),
            },
        )
        .unwrap();

        let received = event_rx.try_recv().unwrap();
        assert_eq!(received.channel, "test_channel");
        assert_eq!(received.chat_id, "test_chat");
        assert!(matches!(received.event, AgentEvent::AssistantDelta { .. }));
    }

    #[test]
    fn test_poke_event_roundtrip_serialize() {
        let variants: Vec<PokeEvent> = vec![
            PokeEvent::PokeSend {
                message: "hello poke".to_string(),
            },
            PokeEvent::ChatSend {
                content: "chat message".to_string(),
            },
            PokeEvent::ChatSent {
                content: "sent content".to_string(),
                message_id: "msg_123".to_string(),
            },
            PokeEvent::ChatReceived {
                content: "received".to_string(),
                sender_id: "user_42".to_string(),
            },
            PokeEvent::ReasoningReceived {
                content: "deep thoughts...".to_string(),
                model: "claude-sonnet-4".to_string(),
            },
            PokeEvent::ChatOver {
                reason: "max_iterations".to_string(),
            },
            PokeEvent::ChatHistoryAdd {
                message_ids: vec!["a".to_string(), "b".to_string()],
            },
            PokeEvent::TokenUsed {
                tokens: 1234,
                model: "gpt-4o".to_string(),
                provider: "openai".to_string(),
            },
            PokeEvent::ConfigChangeNeedsRestart {
                fields: vec![
                    "providers.openai.api_key".to_string(),
                    "gateway.port".to_string(),
                ],
            },
        ];

        for variant in variants {
            let json = serde_json::to_value(&variant).unwrap();
            let deserialized: PokeEvent = serde_json::from_value(json).unwrap();

            // Verify variant kind matches by debug formatting
            let original_debug = format!("{:?}", &variant);
            let deserialized_debug = format!("{:?}", &deserialized);
            assert_eq!(
                original_debug, deserialized_debug,
                "Roundtrip failed for {:?}",
                variant
            );
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let bus = MessageBus::new();
        let mut rx1 = bus.subscribe_poke_events();
        let mut rx2 = bus.subscribe_poke_events();

        bus.publish_poke_event(PokeEvent::ChatOver {
            reason: "completed".to_string(),
        })
        .unwrap();

        // Both subscribers should receive the same event
        let received_1 = rx1.try_recv().unwrap();
        let received_2 = rx2.try_recv().unwrap();
        assert!(matches!(received_1, PokeEvent::ChatOver { .. }));
        assert!(matches!(received_2, PokeEvent::ChatOver { .. }));
    }
}
