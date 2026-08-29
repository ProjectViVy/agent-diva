//! Session-admission and actor-dispatch characterization tests.
//!
//! Bus turns are concurrent across sessions, direct-call session identity comes
//! from `channel:chat_id`, and Stop is observed while a provider stream is polled.

use super::*;
use agent_diva_core::bus::SessionAdmissionCode;
use agent_diva_core::config::schema::SessionAdmissionConfig;
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

fn inbound_with_request(chat_id: &str, content: &str, request_id: &str) -> InboundMessage {
    let mut message = InboundMessage::new("gui", "user", chat_id, content);
    message.metadata.insert(
        REQUEST_ID_METADATA_KEY.to_string(),
        serde_json::Value::String(request_id.to_string()),
    );
    message
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

struct PanicFirstProvider {
    calls: AtomicUsize,
    call_tx: mpsc::UnboundedSender<usize>,
    panic_gate: Arc<Semaphore>,
}

#[async_trait]
impl LLMProvider for PanicFirstProvider {
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
            let permit = self
                .panic_gate
                .clone()
                .acquire_owned()
                .await
                .map_err(|_| ProviderError::ApiError("panic gate closed".to_string()))?;
            permit.forget();
            panic!("injected session worker failure");
        }
        Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
            completed_response("recovered"),
        ))])))
    }

    fn get_default_model(&self) -> String {
        "test-model".to_string()
    }
}

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
async fn bus_dispatch_runs_different_sessions_concurrently() {
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
    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(1)
    );
    first_call_gate.add_permits(1);

    run.abort();
}

#[tokio::test]
async fn bus_queue_full_rejects_before_provider_side_effects() {
    let bus = MessageBus::new();
    let mut event_rx = bus.subscribe_events();
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
    agent
        .configure_session_admission(SessionAdmissionConfig {
            max_queue_depth: 1,
            wait_timeout: 10,
            idle_ttl: 60,
        })
        .unwrap();
    let run = tokio::spawn(async move { agent.run().await.map_err(|error| error.to_string()) });

    bus.publish_inbound(inbound_with_request("queue-full", "first", "request-a"))
        .unwrap();
    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(0)
    );
    bus.publish_inbound(inbound_with_request("queue-full", "second", "request-b"))
        .unwrap();
    timeout(PROGRESS_TIMEOUT, async {
        loop {
            let event = event_rx.recv().await.unwrap();
            if event.request_id.as_deref() == Some("request-b")
                && matches!(
                    event.event,
                    AgentEvent::SessionAdmission { ref observation }
                        if observation.phase == SessionAdmissionPhase::Queued
                )
            {
                break;
            }
        }
    })
    .await
    .expect("second request was not queued");

    bus.publish_inbound(inbound_with_request("queue-full", "third", "request-c"))
        .unwrap();
    let rejected = timeout(PROGRESS_TIMEOUT, async {
        loop {
            let event = event_rx.recv().await.unwrap();
            if event.request_id.as_deref() == Some("request-c") {
                if let AgentEvent::SessionAdmission { observation } = event.event {
                    if observation.code == Some(SessionAdmissionCode::SessionQueueFull) {
                        break observation;
                    }
                }
            }
        }
    })
    .await
    .expect("queue-full rejection was not observed");
    assert_eq!(rejected.queue_depth, 1);
    assert!(timeout(Duration::from_millis(100), call_rx.recv())
        .await
        .is_err());

    first_call_gate.add_permits(1);
    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(1)
    );
    run.abort();
}

#[tokio::test]
async fn bus_wait_timeout_releases_slot_without_provider_side_effects() {
    let bus = MessageBus::new();
    let mut event_rx = bus.subscribe_events();
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
    agent
        .configure_session_admission(SessionAdmissionConfig {
            max_queue_depth: 1,
            wait_timeout: 1,
            idle_ttl: 60,
        })
        .unwrap();
    let run = tokio::spawn(async move { agent.run().await.map_err(|error| error.to_string()) });

    bus.publish_inbound(inbound_with_request("wait-timeout", "first", "request-a"))
        .unwrap();
    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(0)
    );
    bus.publish_inbound(inbound_with_request("wait-timeout", "second", "request-b"))
        .unwrap();
    timeout(PROGRESS_TIMEOUT, async {
        loop {
            let event = event_rx.recv().await.unwrap();
            if event.request_id.as_deref() == Some("request-b") {
                if let AgentEvent::SessionAdmission { observation } = event.event {
                    if observation.code == Some(SessionAdmissionCode::SessionQueueWaitTimeout) {
                        break;
                    }
                }
            }
        }
    })
    .await
    .expect("wait-timeout rejection was not observed");
    assert!(timeout(Duration::from_millis(100), call_rx.recv())
        .await
        .is_err());

    first_call_gate.add_permits(1);
    bus.publish_inbound(inbound_with_request("wait-timeout", "third", "request-c"))
        .unwrap();
    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(1),
        "timed-out waiter must not retain a lease"
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

    let (reply_tx, _reply_rx) = tokio::sync::oneshot::channel();
    control_tx
        .send(RuntimeControlCommand::StopSession {
            session_key: "gui:stop-target".to_string(),
            request_id: None,
            reply_tx,
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

#[tokio::test]
async fn failed_worker_drains_waiters_and_next_request_recovers() {
    let bus = MessageBus::new();
    let mut event_rx = bus.subscribe_events();
    let (call_tx, mut call_rx) = mpsc::unbounded_channel();
    let panic_gate = Arc::new(Semaphore::new(0));
    let provider = Arc::new(PanicFirstProvider {
        calls: AtomicUsize::new(0),
        call_tx,
        panic_gate: panic_gate.clone(),
    });
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

    let running = inbound_with_request("worker-failure", "first", "request-running");
    bus.publish_inbound(running).unwrap();
    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(0)
    );

    let queued = inbound_with_request("worker-failure", "second", "request-queued");
    bus.publish_inbound(queued).unwrap();
    timeout(PROGRESS_TIMEOUT, async {
        loop {
            let event = event_rx.recv().await.unwrap();
            if event.request_id.as_deref() == Some("request-queued")
                && matches!(
                    event.event,
                    AgentEvent::SessionAdmission { ref observation }
                        if observation.phase == SessionAdmissionPhase::Queued
                )
            {
                break;
            }
        }
    })
    .await
    .expect("queued request was not observed");

    panic_gate.add_permits(1);
    let unavailable = timeout(PROGRESS_TIMEOUT, async {
        let mut request_ids = std::collections::HashSet::new();
        while request_ids.len() < 2 {
            let event = event_rx.recv().await.unwrap();
            if let AgentEvent::SessionAdmission { observation } = event.event {
                if observation.code == Some(SessionAdmissionCode::SessionWorkerUnavailable) {
                    request_ids.insert(observation.request_id);
                }
            }
        }
        request_ids
    })
    .await
    .expect("worker failure did not drain accepted requests");
    assert_eq!(
        unavailable,
        ["request-running".to_string(), "request-queued".to_string()]
            .into_iter()
            .collect()
    );

    let recovered = inbound_with_request("worker-failure", "third", "request-recovered");
    bus.publish_inbound(recovered).unwrap();
    assert_eq!(
        timeout(PROGRESS_TIMEOUT, call_rx.recv()).await.unwrap(),
        Some(1),
        "a fresh worker should execute after the failed generation is removed"
    );
    run.abort();
}
