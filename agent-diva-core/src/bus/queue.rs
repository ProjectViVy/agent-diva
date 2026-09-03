//! Broadcast fan-out for AgentEvent projections and identity-only activity.
//!
//! Turn ingress and adapter egress are owned by the typed Channel Fabric. This
//! module intentionally contains no message queues or turn DTOs.

use super::events::{AgentBusEvent, AgentEvent, PokeEvent};
use tokio::sync::broadcast;

/// Event fan-out shared by the agent loop, projections, and observability
/// consumers.
#[derive(Clone)]
pub struct AgentEventBus {
    event_tx: broadcast::Sender<AgentBusEvent>,
    poke_event_tx: broadcast::Sender<PokeEvent>,
}

impl AgentEventBus {
    /// Create a new event fan-out hub.
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);
        let (poke_event_tx, _) = broadcast::channel(1024);
        Self {
            event_tx,
            poke_event_tx,
        }
    }

    /// Publish an event without turn-specific correlation.
    pub fn publish_event(
        &self,
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        event: AgentEvent,
    ) -> crate::Result<()> {
        self.publish_event_inner(AgentBusEvent {
            channel: channel.into(),
            chat_id: chat_id.into(),
            session_key: None,
            request_id: None,
            trace_id: None,
            event,
        })
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
        self.publish_event_inner(AgentBusEvent {
            channel: channel.into(),
            chat_id: chat_id.into(),
            session_key: Some(session_key.into()),
            request_id: Some(request_id.into()),
            trace_id: Some(trace_id.into()),
            event,
        })
    }

    fn publish_event_inner(&self, event: AgentBusEvent) -> crate::Result<()> {
        // A projection subscriber may be absent during startup/shutdown. The
        // event remains best-effort fan-out; durable state owns recovery.
        let _ = self.event_tx.send(event);
        Ok(())
    }

    /// Subscribe to the AgentEvent projection stream.
    pub fn subscribe_events(&self) -> broadcast::Receiver<AgentBusEvent> {
        self.event_tx.subscribe()
    }

    /// Publish a lifecycle or identity-only activity event.
    pub fn publish_poke_event(&self, event: PokeEvent) -> crate::Result<()> {
        let _ = self.poke_event_tx.send(event);
        Ok(())
    }

    /// Subscribe to lifecycle and identity-only activity events.
    pub fn subscribe_poke_events(&self) -> broadcast::Receiver<PokeEvent> {
        self.poke_event_tx.subscribe()
    }
}

impl Default for AgentEventBus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn event_fanout_creation_and_publish() {
        let bus = AgentEventBus::new();
        let mut event_rx = bus.subscribe_events();
        bus.publish_event(
            "ch",
            "chat1",
            AgentEvent::FinalResponse {
                content: "response".to_string(),
            },
        )
        .unwrap();
        let received = event_rx.try_recv().unwrap();
        assert_eq!(received.channel, "ch");
        assert_eq!(received.chat_id, "chat1");
        assert!(matches!(received.event, AgentEvent::FinalResponse { .. }));
    }

    #[tokio::test]
    async fn identity_activity_never_contains_message_content() {
        let bus = AgentEventBus::new();
        let mut poke_rx = bus.subscribe_poke_events();
        bus.publish_poke_event(PokeEvent::UserActivity {
            session_key: "telegram:chat-1".to_string(),
            sender_id: Some("user-1".to_string()),
        })
        .unwrap();
        assert!(matches!(
            poke_rx.try_recv().unwrap(),
            PokeEvent::UserActivity { session_key, sender_id }
                if session_key == "telegram:chat-1" && sender_id.as_deref() == Some("user-1")
        ));
    }

    #[tokio::test]
    async fn event_and_poke_fanout_are_independent() {
        let bus = AgentEventBus::new();
        let mut event_rx = bus.subscribe_events();
        let mut poke_rx = bus.subscribe_poke_events();
        bus.publish_event(
            "ch",
            "chat1",
            AgentEvent::FinalResponse {
                content: "response".to_string(),
            },
        )
        .unwrap();
        bus.publish_poke_event(PokeEvent::PokeSend {
            message: "poke".to_string(),
        })
        .unwrap();
        assert!(matches!(
            event_rx.try_recv().unwrap().event,
            AgentEvent::FinalResponse { .. }
        ));
        assert!(matches!(
            poke_rx.try_recv().unwrap(),
            PokeEvent::PokeSend { .. }
        ));
        assert!(event_rx.try_recv().is_err());
        assert!(poke_rx.try_recv().is_err());
    }

    #[test]
    fn poke_event_variants_roundtrip_serialize() {
        let variants = [
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
            PokeEvent::UserActivity {
                session_key: "session-1".to_string(),
                sender_id: Some("user_42".to_string()),
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
            assert_eq!(format!("{variant:?}"), format!("{deserialized:?}"));
        }
    }
}
