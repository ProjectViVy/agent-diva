use agent_diva_neuron::{LlmNeuron, NeuronError, NeuronEvent, NeuronNode, NeuronRequest};
use agent_diva_providers::{
    LLMProvider, LLMResponse, Message, ProviderError, ProviderResult, ToolCallRequest,
};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

struct MockProvider {
    response: LLMResponse,
    default_model: String,
}

#[async_trait]
impl LLMProvider for MockProvider {
    async fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        Ok(self.response.clone())
    }

    fn get_default_model(&self) -> String {
        self.default_model.clone()
    }
}

struct ErrorProvider;

#[async_trait]
impl LLMProvider for ErrorProvider {
    async fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        Err(ProviderError::ApiError("mock failure".to_string()))
    }

    fn get_default_model(&self) -> String {
        "mock-model".to_string()
    }
}

#[tokio::test]
async fn run_once_returns_content() {
    let provider = Arc::new(MockProvider {
        response: LLMResponse {
            content: Some("hello neuron".to_string()),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        },
        default_model: "mock-default".to_string(),
    });
    let neuron = LlmNeuron::with_id(provider, "n-1");
    let req = NeuronRequest::new(vec![Message::user("ping")], 512, 0.2);

    let resp = neuron.run_once(req).await.expect("run_once should succeed");
    assert_eq!(resp.content.as_deref(), Some("hello neuron"));
    assert_eq!(resp.finish_reason, "stop");
}

#[tokio::test]
async fn run_once_passthrough_tool_calls() {
    let tool_call = ToolCallRequest {
        id: "call-1".to_string(),
        call_type: "function".to_string(),
        name: "web_search".to_string(),
        arguments: HashMap::new(),
    };

    let provider = Arc::new(MockProvider {
        response: LLMResponse {
            content: Some("I need tool".to_string()),
            tool_calls: vec![tool_call.clone()],
            finish_reason: "tool_calls".to_string(),
            usage: HashMap::new(),
            reasoning_content: Some("need lookup".to_string()),
        },
        default_model: "mock-default".to_string(),
    });

    let neuron = LlmNeuron::with_id(provider, "n-2");
    let req = NeuronRequest::new(vec![Message::user("search something")], 512, 0.2);

    let resp = neuron.run_once(req).await.expect("run_once should succeed");
    assert_eq!(resp.tool_calls.len(), 1);
    assert_eq!(resp.tool_calls[0].name, tool_call.name);
    assert_eq!(resp.finish_reason, "tool_calls");
}

#[tokio::test]
async fn run_once_stream_emits_event_sequence() {
    let provider = Arc::new(MockProvider {
        response: LLMResponse {
            content: Some("stream text".to_string()),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        },
        default_model: "mock-default".to_string(),
    });

    let neuron = LlmNeuron::with_id(provider, "n-3");
    let req = NeuronRequest::new(vec![Message::user("ping")], 256, 0.5);
    let (tx, mut rx) = mpsc::unbounded_channel();

    let resp = neuron
        .run_once_stream(req, Some(tx))
        .await
        .expect("run_once_stream should succeed");

    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }

    assert!(matches!(events.first(), Some(NeuronEvent::Started { .. })));
    assert!(events
        .iter()
        .any(|evt| matches!(evt, NeuronEvent::TextDelta { delta, .. } if delta == "stream text")));
    assert!(matches!(
        events.last(),
        Some(NeuronEvent::Completed { finish_reason, .. }) if finish_reason == "stop"
    ));
    assert_eq!(resp.content.as_deref(), Some("stream text"));
}

#[tokio::test]
async fn provider_errors_are_mapped() {
    let neuron = LlmNeuron::with_id(Arc::new(ErrorProvider), "n-4");
    let req = NeuronRequest::new(vec![Message::user("ping")], 256, 0.5);

    let err = neuron
        .run_once(req)
        .await
        .expect_err("expected provider error");
    assert!(matches!(err, NeuronError::Provider(_)));
}

#[tokio::test]
async fn empty_messages_is_invalid() {
    let provider = Arc::new(MockProvider {
        response: LLMResponse {
            content: Some("ignored".to_string()),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        },
        default_model: "mock-default".to_string(),
    });
    let neuron = LlmNeuron::with_id(provider, "n-5");
    let req = NeuronRequest::default();

    let err = neuron
        .run_once(req)
        .await
        .expect_err("expected invalid input");
    assert!(matches!(err, NeuronError::InvalidInput(_)));
}
