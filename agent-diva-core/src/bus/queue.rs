//! Async message queue implementation

use super::events::{InboundMessage, OutboundMessage};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
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
    /// Running state
    running: Arc<RwLock<bool>>,
}

impl MessageBus {
    /// Create a new message bus
    pub fn new() -> Self {
        let (inbound_tx, inbound_rx) = mpsc::unbounded_channel();
        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();

        Self {
            inbound_tx,
            inbound_rx: Arc::new(RwLock::new(Some(inbound_rx))),
            outbound_tx,
            outbound_rx: Arc::new(RwLock::new(Some(outbound_rx))),
            subscribers: Arc::new(RwLock::new(HashMap::new())),
            running: Arc::new(RwLock::new(false)),
        }
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
}
