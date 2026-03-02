//! Public request/response types for neuron execution.

use agent_diva_providers::{Message, ToolCallRequest};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single neuron invocation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuronRequest {
    /// Full message context used for one LLM call.
    pub messages: Vec<Message>,
    /// Optional model override. Falls back to provider default.
    #[serde(default)]
    pub model: Option<String>,
    /// Max output tokens for this invocation.
    pub max_tokens: i32,
    /// Sampling temperature.
    pub temperature: f64,
    /// Future-proof metadata for graph executors.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl NeuronRequest {
    /// Create a request with explicit generation limits.
    pub fn new(messages: Vec<Message>, max_tokens: i32, temperature: f64) -> Self {
        Self {
            messages,
            model: None,
            max_tokens,
            temperature,
            metadata: HashMap::new(),
        }
    }

    /// Apply a model override.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
}

impl Default for NeuronRequest {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            model: None,
            max_tokens: 4096,
            temperature: 0.7,
            metadata: HashMap::new(),
        }
    }
}

/// A single neuron invocation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuronResponse {
    /// Final assistant content.
    pub content: Option<String>,
    /// Optional reasoning stream aggregation.
    #[serde(default)]
    pub reasoning_content: Option<String>,
    /// Tool intents are passed through only; execution belongs to upper layers.
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRequest>,
    /// Provider finish reason.
    pub finish_reason: String,
    /// Usage metrics as reported by provider.
    #[serde(default)]
    pub usage: HashMap<String, i64>,
    /// Future-proof metadata for graph executors.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}
