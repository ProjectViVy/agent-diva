//! Default neuron executor backed by an LLM provider.

use crate::events::NeuronEvent;
use crate::node::{NeuronError, NeuronNode};
use crate::types::{NeuronRequest, NeuronResponse};
use agent_diva_providers::{LLMProvider, LLMResponse, LLMStreamEvent};
use async_trait::async_trait;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

/// Provider-backed neuron implementation.
pub struct LlmNeuron {
    provider: Arc<dyn LLMProvider>,
    neuron_id: String,
}

impl LlmNeuron {
    /// Create a neuron with a generated identifier.
    pub fn new(provider: Arc<dyn LLMProvider>) -> Self {
        Self {
            provider,
            neuron_id: Uuid::new_v4().to_string(),
        }
    }

    /// Create a neuron with a caller-controlled identifier.
    pub fn with_id(provider: Arc<dyn LLMProvider>, neuron_id: impl Into<String>) -> Self {
        Self {
            provider,
            neuron_id: neuron_id.into(),
        }
    }

    /// Return this neuron's stable identifier.
    pub fn neuron_id(&self) -> &str {
        &self.neuron_id
    }
}

#[async_trait]
impl NeuronNode for LlmNeuron {
    async fn run_once(&self, req: NeuronRequest) -> Result<NeuronResponse, NeuronError> {
        self.run_once_stream(req, None).await
    }

    async fn run_once_stream(
        &self,
        req: NeuronRequest,
        event_tx: Option<mpsc::UnboundedSender<NeuronEvent>>,
    ) -> Result<NeuronResponse, NeuronError> {
        if req.messages.is_empty() {
            return Err(NeuronError::InvalidInput(
                "messages cannot be empty".to_string(),
            ));
        }

        let trace_id = Uuid::new_v4().to_string();
        if let Some(tx) = &event_tx {
            let _ = tx.send(NeuronEvent::Started {
                neuron_id: self.neuron_id.clone(),
                trace_id,
            });
        }

        let model = req
            .model
            .clone()
            .unwrap_or_else(|| self.provider.get_default_model());

        let mut stream = self
            .provider
            .chat_stream(
                req.messages,
                None,
                Some(model),
                req.max_tokens,
                req.temperature,
            )
            .await?;

        let mut text = String::new();
        let mut reasoning = String::new();
        let mut completed: Option<LLMResponse> = None;

        while let Some(event) = stream.next().await {
            match event {
                Ok(LLMStreamEvent::TextDelta(delta)) => {
                    text.push_str(&delta);
                    if let Some(tx) = &event_tx {
                        let _ = tx.send(NeuronEvent::TextDelta {
                            neuron_id: self.neuron_id.clone(),
                            delta,
                        });
                    }
                }
                Ok(LLMStreamEvent::ReasoningDelta(delta)) => {
                    reasoning.push_str(&delta);
                    if let Some(tx) = &event_tx {
                        let _ = tx.send(NeuronEvent::ReasoningDelta {
                            neuron_id: self.neuron_id.clone(),
                            delta,
                        });
                    }
                }
                Ok(LLMStreamEvent::ToolCallDelta { .. }) => {
                    // Reserved for higher-level orchestration; ignored in v0.
                }
                Ok(LLMStreamEvent::Completed(done)) => {
                    completed = Some(done);
                    break;
                }
                Err(err) => {
                    if let Some(tx) = &event_tx {
                        let _ = tx.send(NeuronEvent::Failed {
                            neuron_id: self.neuron_id.clone(),
                            error: err.to_string(),
                        });
                    }
                    return Err(NeuronError::Provider(err));
                }
            }
        }

        let response = completed.unwrap_or_else(|| LLMResponse {
            content: if text.is_empty() { None } else { Some(text) },
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: std::collections::HashMap::new(),
            reasoning_content: if reasoning.is_empty() {
                None
            } else {
                Some(reasoning)
            },
        });

        if let Some(tx) = &event_tx {
            let _ = tx.send(NeuronEvent::Completed {
                neuron_id: self.neuron_id.clone(),
                finish_reason: response.finish_reason.clone(),
            });
        }

        Ok(NeuronResponse {
            content: response.content,
            reasoning_content: response.reasoning_content,
            tool_calls: response.tool_calls,
            finish_reason: response.finish_reason,
            usage: response.usage,
            metadata: req.metadata,
        })
    }
}
