//! Base trait for LLM providers

use async_trait::async_trait;
use futures::stream::{self, Stream};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashMap;
use std::pin::Pin;
use thiserror::Error;

/// Error type for provider operations
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON parsing failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

pub type ProviderResult<T> = Result<T, ProviderError>;

pub type ProviderEventStream = Pin<Box<dyn Stream<Item = ProviderResult<LLMStreamEvent>> + Send>>;

/// A tool call request from the LLM
#[derive(Debug, Clone)]
pub struct ToolCallRequest {
    pub id: String,
    pub call_type: String,
    pub name: String,
    pub arguments: HashMap<String, serde_json::Value>,
}

impl Serialize for ToolCallRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeStruct;

        #[derive(Serialize)]
        struct Function<'a> {
            name: &'a str,
            arguments: String,
        }

        let arguments = serde_json::to_string(&self.arguments).unwrap_or_else(|_| "{}".to_string());

        let mut state = serializer.serialize_struct("ToolCallRequest", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("type", &self.call_type)?;
        state.serialize_field(
            "function",
            &Function {
                name: &self.name,
                arguments,
            },
        )?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ToolCallRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Function {
            name: String,
            arguments: serde_json::Value,
        }

        #[derive(Deserialize)]
        struct Helper {
            id: String,
            #[serde(rename = "type")]
            call_type: String,
            #[serde(default)]
            function: Option<Function>,
            #[serde(default)]
            name: Option<String>,
            #[serde(default)]
            arguments: Option<serde_json::Value>,
        }

        fn normalize_arguments(value: serde_json::Value) -> HashMap<String, serde_json::Value> {
            match value {
                serde_json::Value::String(raw) => serde_json::from_str::<
                    HashMap<String, serde_json::Value>,
                >(&raw)
                .unwrap_or_else(|_| {
                    let mut map = HashMap::new();
                    map.insert("raw".to_string(), serde_json::Value::String(raw));
                    map
                }),
                serde_json::Value::Object(map) => map.into_iter().collect(),
                _ => HashMap::new(),
            }
        }

        let helper = Helper::deserialize(deserializer)?;
        if let Some(function) = helper.function {
            return Ok(Self {
                id: helper.id,
                call_type: helper.call_type,
                name: function.name,
                arguments: normalize_arguments(function.arguments),
            });
        }

        let name = helper
            .name
            .ok_or_else(|| serde::de::Error::missing_field("function or name"))?;
        let arguments = helper
            .arguments
            .map(normalize_arguments)
            .unwrap_or_default();

        Ok(Self {
            id: helper.id,
            call_type: helper.call_type,
            name,
            arguments,
        })
    }
}

/// Response from an LLM provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LLMResponse {
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCallRequest>,
    #[serde(default = "default_finish_reason")]
    pub finish_reason: String,
    #[serde(default)]
    pub usage: HashMap<String, i64>,
    #[serde(default)]
    pub reasoning_content: Option<String>,
}

fn default_finish_reason() -> String {
    "stop".to_string()
}

impl LLMResponse {
    /// Check if response contains tool calls
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// Streaming event emitted by LLM providers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LLMStreamEvent {
    /// Incremental assistant text output
    TextDelta(String),
    /// Incremental tool-call metadata (reserved for advanced UIs)
    ToolCallDelta {
        index: usize,
        id: Option<String>,
        name: Option<String>,
        arguments_delta: Option<String>,
    },
    /// Final completed response
    Completed(LLMResponse),
}

/// A message in the chat conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallRequest>>,
}

impl Message {
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
        }
    }

    /// Create a tool response message
    pub fn tool(content: impl Into<String>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }
}

/// Trait for LLM providers
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Send a chat completion request
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse>;

    /// Send a streaming chat completion request.
    ///
    /// Default behavior falls back to non-streaming chat and emits one text delta.
    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let response = self
            .chat(messages, tools, model, max_tokens, temperature)
            .await?;

        let mut events = Vec::new();
        if let Some(content) = response.content.clone() {
            if !content.is_empty() {
                events.push(Ok(LLMStreamEvent::TextDelta(content)));
            }
        }
        events.push(Ok(LLMStreamEvent::Completed(response)));

        Ok(Box::pin(stream::iter(events)))
    }

    /// Get the default model for this provider
    fn get_default_model(&self) -> String;
}
