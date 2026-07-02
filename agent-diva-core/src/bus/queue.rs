//! Async message queue implementation

use super::events::{AgentBusEvent, AgentEvent, InboundMessage, OutboundMessage, PokeEvent};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::debug;

/// Type alias for message channel senders
pub type OutboundSender = mpsc::UnboundedSender<OutboundMessage>;
pub type OutboundReceiver = mpsc::UnboundedReceiver<OutboundMessage>;

type OutboundCallback = Arc<
    dyn Fn(OutboundMessage) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>
        + Send
        + Sync,
>;

/// Async message bus that decouples chat channels from the agent core
///
/// Channels push messages to the inbound queue, and the agent processes
/// them and pushes responses to the outbound queue.
#[derive(Clone)]
pub struct MessageBus {
    /// Inbound messages from channels
    inbound_tx: mpsc::UnboundedSender<InboundMessage>,
    inbound_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<InboundMessage>>>>,
    /// Outbound messages to channels
    outbound_tx: mpsc::UnboundedSender<OutboundMessage>,
    outbound_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<OutboundMessage>>>>,
    /// Outbound subscribers by channel
    subscribers: Arc<RwLock<HashMap<String, Vec<OutboundCallback>>>>,
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
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let (event_tx, _) = broadcast::channel(1024);
        let (poke_event_tx, _) = broadcast::channel(1024);

        Self {
            inbound_tx,
            inbound_rx: Arc::new(RwLock::new(Some(inbound_rx))),
            outbound_tx,
            outbound_rx: Arc::new(RwLock::new(Some(outbound_rx))),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
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
            event,
        };
        // We ignore the error if there are no receivers
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
    pub async fn take_inbound_receiver(&self) -> Option<mpsc::UnboundedReceiver<InboundMessage>> {
        self.inbound_rx.write().await.take()
    }

    /// Take the outbound receiver (can only be called once)
    pub async fn take_outbound_receiver(&self) -> Option<mpsc::UnboundedReceiver<OutboundMessage>> {
        self.outbound_rx.write().await.take()
    }

    /// Publish a message from a channel to the agent
    pub fn publish_inbound(&self, msg: InboundMessage) -> crate::Result<()> {
        self.inbound_tx
            .send(msg)
            .map_err(|_| crate::Error::Channel("Inbound channel closed".to_string()))
    }

    /// Publish a response from the agent to channels
    pub fn publish_outbound(&self, msg: OutboundMessage) -> crate::Result<()> {
        self.outbound_tx
            .send(msg)
            .map_err(|_| crate::Error::Channel("Outbound channel closed".to_string()))
    }

    /// Subscribe to outbound messages for a specific channel with a callback
    pub async fn subscribe_outbound<F, Fut>(&self, channel: impl Into<String>, callback: F)
    where
        F: Fn(OutboundMessage) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let channel = channel.into();
        let wrapped: OutboundCallback = Arc::new(move |msg| Box::pin(callback(msg)));

        let mut subscribers = self.subscribers.write().await;
        subscribers.entry(channel).or_default().push(wrapped);
    }

    /// Dispatch outbound messages to subscribed channels
    /// Run this as a background task
    pub async fn dispatch_outbound_loop(&self) {
        let mut outbound_rx = match self.take_outbound_receiver().await {
            Some(rx) => rx,
            None => {
                debug!("Outbound receiver already taken");
                return;
            }
        };

        *self.running.write().await = true;
        debug!("Starting outbound dispatcher");

        while *self.running.read().await {
            tokio::select! {
                Some(msg) = outbound_rx.recv() => {
                    let channel = msg.channel.clone();
                    let subscribers = self.subscribers.read().await;

                    if let Some(callbacks) = subscribers.get(&channel) {
                        for callback in callbacks {
                            let msg_clone = msg.clone();
                            let future = callback(msg_clone);
                            // Spawn to avoid blocking
                            tokio::spawn(async move {
                                future.await;
                            });
                        }
                    } else {
                        debug!("No subscribers for channel: {}", channel);
                    }
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
                    // Check running state periodically
                    continue;
                }
            }
        }

        debug!("Outbound dispatcher stopped");
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
    async fn test_subscribe_outbound() {
        let bus = MessageBus::new();

        bus.subscribe_outbound("telegram", |_msg| async move {
            // Callback function
        })
        .await;

        // Check bus is not running yet
        assert!(!bus.is_running().await);
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
    async fn test_backward_compat_publish_event() {
        let bus = MessageBus::new();
        let mut event_rx = bus.subscribe_events();

        // Existing publish_event must still work with AgentBusEvent wrapping
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
