//! HQ-00 characterization tests for the pre-queue AgentLoop contract.
//!
//! These tests intentionally describe the baseline that HQ-02 will replace:
//! Bus turns are globally serial today, direct-call session identity comes from
//! `channel:chat_id`, and Stop is observed while a provider stream is polled.

use super::*;
use agent_diva_providers::{
    LLMResponse, LLMStreamEvent, Message, ProviderError, ProviderEventStream, ProviderResult,
    ToolChoiceMode,
};
use async_trait::async_trait;
use futures::stream;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::time::{timeout, Duration};

const PROGRESS_TIMEOUT: Duration = Duration::from_secs(10);

fn completed_response(content: &str) -> LLMResponse {
    LLMResponse {
        content: Some(content.to_string()),
        tool_calls: Vec::new(),
        finish_reason: "stop".to_string(),
        usage: HashMap::new(),
        reasoning_content: None,
    }
}

struct FirstCallGateProvider {
    calls: AtomicUsize,
    call_tx: mpsc::UnboundedSender<usize>,
    first_call_gate: Arc<Semaphore>,
}

impl FirstCallGateProvider {
    fn new(call_tx: mpsc::UnboundedSender<usize>, first_call_gate: Arc<Semaphore>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            call_tx,
            first_call_gate,
        }
    }
}

#[async_trait]
impl LLMProvider for FirstCallGateProvider {
    async fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        Err(ProviderError::ApiError(
            "chat should not be used".to_string(),
        ))
    }

    async fn chat_stream(
        &self,
        _messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        let _ = self.call_tx.send(call);
        if call == 0 {
            let gate = self.first_call_gate.clone();
            return Ok(Box::pin(stream::once(async move {
                let permit = gate
                    .acquire_owned()
                    .await
                    .map_err(|_| ProviderError::ApiError("first-call gate closed".to_string()))?;
                permit.forget();
                Ok(LLMStreamEvent::Completed(completed_response("first")))
            })));
        }

        Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
            completed_response("next"),
        ))])))
    }

    fn get_default_model(&self) -> String {
        "test-model".to_string()
    }
}

struct ImmediateProvider;

#[async_trait]
impl LLMProvider for ImmediateProvider {
    async fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        Err(ProviderError::ApiError(
            "chat should not be used".to_string(),
        ))
    }

    async fn chat_stream(
        &self,
        _messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
            completed_response("done"),
        ))])))
    }

    fn get_default_model(&self) -> String {
        "test-model".to_string()
    }
}

#[tokio::test]
async fn bus_dispatch_is_globally_serial_before_session_workers() {
    let bus = MessageBus::new();
    let (call_tx, mut call_rx) = mpsc::unbounded_channel();
    let first_call_gate = Arc::new(Semaphore::new(0));
    let provider = Arc::new(FirstCallGateProvider::new(call_tx, first_call_gate.clone()));
    let temp_dir = tempfile::tempdir().unwrap();
    let mut agent = AgentLoop::new(
        bus.clone(),
        provider,
        temp_dir.path().to_path_buf(),
        None,
        Some(1),
    )
    .await
    .unwrap();
    let run = tokio::spawn(async move { agent.run().await.map_err(|error| error.to_string()) });

    bus.publish_inbound(InboundMessage::new("gui", "user", "session-a", "first"))
        .unwrap();
    bus.publish_inbound(InboundMessage::new("gui", "user", "session-b", "second"))
        .unwrap();

    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(0)
    );
    assert!(
        timeout(Duration::from_millis(100), call_rx.recv())
            .await
            .is_err(),
        "a second session must not reach the provider while the first turn is blocked"
    );

    first_call_gate.add_permits(1);
    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(1)
    );

    run.abort();
}

#[tokio::test]
async fn direct_session_argument_is_compatibility_only() {
    let bus = MessageBus::new();
    let provider = Arc::new(ImmediateProvider);
    let temp_dir = tempfile::tempdir().unwrap();
    let mut agent = AgentLoop::new(bus, provider, temp_dir.path().to_path_buf(), None, Some(1))
        .await
        .unwrap();

    let response = agent
        .process_direct("hello", "legacy-explicit-key", "gui", "canonical-chat")
        .await
        .unwrap();

    assert_eq!(response, "done");
    assert!(agent.sessions.get("gui:canonical-chat").is_some());
    assert!(agent.sessions.get("legacy-explicit-key").is_none());
}

#[tokio::test]
async fn stop_session_is_observed_while_provider_stream_is_pending() {
    let bus = MessageBus::new();
    let mut event_rx = bus.subscribe_events();
    let (call_tx, mut call_rx) = mpsc::unbounded_channel();
    let first_call_gate = Arc::new(Semaphore::new(0));
    let provider = Arc::new(FirstCallGateProvider::new(call_tx, first_call_gate));
    let temp_dir = tempfile::tempdir().unwrap();
    let mut agent = AgentLoop::new(
        bus.clone(),
        provider,
        temp_dir.path().to_path_buf(),
        None,
        Some(1),
    )
    .await
    .unwrap();
    let (control_tx, control_rx) = mpsc::unbounded_channel();
    agent.runtime_control_rx = Some(control_rx);
    let run = tokio::spawn(async move { agent.run().await.map_err(|error| error.to_string()) });

    bus.publish_inbound(InboundMessage::new("gui", "user", "stop-target", "wait"))
        .unwrap();
    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(0)
    );

    control_tx
        .send(RuntimeControlCommand::StopSession {
            session_key: "gui:stop-target".to_string(),
        })
        .unwrap();

    let stopped = timeout(PROGRESS_TIMEOUT, async {
        loop {
            let event = event_rx.recv().await.unwrap();
            if event.channel != "gui" || event.chat_id != "stop-target" {
                continue;
            }
            if let AgentEvent::Error { message } = event.event {
                break message;
            }
        }
    })
    .await
    .expect("timed out waiting for StopSession error event");

    assert_eq!(stopped, "Generation stopped by user.");
    run.abort();
}
