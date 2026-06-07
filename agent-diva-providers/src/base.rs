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

/// Conservative feature flags for a model.
///
/// Unknown models default to no optional capabilities. This prevents the
/// provider layer from sending multimodal payloads to text-only models.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub vision: bool,
    pub tools: bool,
    pub reasoning: bool,
    /// Known context window size in tokens, if the model is in the hardcoded table.
    pub context_window: Option<usize>,
}

impl ModelCapabilities {
    pub const fn text_only() -> Self {
        Self {
            vision: false,
            tools: false,
            reasoning: false,
            context_window: None,
        }
    }
}

/// Return conservative capabilities for a model id.
pub fn model_capabilities_for_model(model: &str) -> ModelCapabilities {
    model_capabilities_for_model_with_config(model, None)
}

/// Return capabilities for a model id, optionally using provider-level reasoning config.
///
/// If `reasoning_config` is provided and has a `reasoning_type`, the model is considered
/// reasoning-capable regardless of the hard-coded list. This allows per-provider
/// dynamic configuration without waiting for code updates.
pub fn model_capabilities_for_model_with_config(
    model: &str,
    reasoning_config: Option<&agent_diva_core::reasoning::ReasoningConfig>,
) -> ModelCapabilities {
    let normalized = normalize_model_id(model);
    let mut capabilities = ModelCapabilities::text_only();

    capabilities.vision = matches!(
        normalized.as_str(),
        "gpt-4o" | "gpt-4o-mini" | "gpt-4.1" | "gpt-4.1-mini"
    );

    // Dynamic reasoning detection: if provider has reasoning_config, trust it;
    // otherwise fall back to the hard-coded known-reasoning-model list.
    capabilities.reasoning = if let Some(config) = reasoning_config {
        !config.reasoning_type.is_empty()
    } else {
        is_known_reasoning_model(&normalized)
    };
    capabilities.context_window = context_window_for_model(&normalized);
    capabilities
}

/// Return true when the model is known to support reasoning/thinking content.
fn is_known_reasoning_model(normalized: &str) -> bool {
    matches!(
        normalized,
        // DeepSeek — both chat and reasoner return reasoning_content via API
        "deepseek-chat" | "deepseek-reasoner"
        // Anthropic Claude — extended thinking
        | "claude-3-opus" | "claude-3-5-sonnet" | "claude-3-7-sonnet"
        | "claude-3-5-haiku" | "claude-sonnet-4" | "claude-opus-4"
        // OpenAI reasoning models
        | "o1" | "o1-mini" | "o3-mini" | "o1-pro"
        // Gemini thinking models
        | "gemini-2.0-flash-thinking" | "gemini-2.5-pro" | "gemini-2.5-flash"
        // Qwen thinking-capable models
        | "qwen-max" | "qwen-plus" | "qwq-32b"
        // Doubao/Douyin models with thinking
        | "doubao-pro-32k" | "doubao-lite-32k"
        // DeepSeek R1 (via OpenRouter or native)
        | "deepseek-r1"
    )
}
/// Known context window sizes for popular models (OpenFang-style hardcoded table).
///
/// This is a pragmatic fallback — the "笨办法" (dumb approach) that always works
/// without network calls or heavy dependencies. Unknown models return `None`,
/// and callers fall back to a conservative default (128K).
fn context_window_for_model(normalized: &str) -> Option<usize> {
    match normalized {
        // ── Anthropic ──────────────────────────────────────────────────────
        "claude-sonnet-4-6" | "claude-opus-4-7" => Some(1_000_000),
        "claude-opus-4-6" | "claude-sonnet-4" | "claude-opus-4" => Some(200_000),
        "claude-haiku-4" | "claude-3-5-sonnet" | "claude-3-7-sonnet" => Some(200_000),
        "claude-3-opus" | "claude-3-5-haiku" | "claude-3-haiku" => Some(200_000),
        // ── OpenAI ─────────────────────────────────────────────────────────
        "gpt-4.1" | "gpt-4.1-mini" => Some(1_047_576),
        "gpt-4o" | "gpt-4o-mini" => Some(128_000),
        "gpt-5" => Some(400_000),
        "o1" | "o1-mini" | "o3-mini" | "o1-pro" => Some(128_000),
        // ── DeepSeek ───────────────────────────────────────────────────────
        "deepseek-chat" | "deepseek-coder" => Some(128_000),
        "deepseek-reasoner" | "deepseek-r1" => Some(128_000),
        "deepseek-v4-pro" | "deepseek-v4-chat" => Some(1_000_000),
        // ── Google Gemini ──────────────────────────────────────────────────
        "gemini-2.5-pro" | "gemini-2.5-flash" => Some(1_048_576),
        "gemini-2.0-flash-thinking" => Some(1_048_576),
        // ── xAI ────────────────────────────────────────────────────────────
        "grok-4" => Some(256_000),
        // ── Qwen ───────────────────────────────────────────────────────────
        "qwen-max" | "qwen-plus" | "qwq-32b" | "qwen3-30b-a3b" => Some(262_144),
        // ── MiniMax ────────────────────────────────────────────────────────
        "minimax-text-01" => Some(204_800),
        // ── MiMo ───────────────────────────────────────────────────────────
        "mimo-7b" => Some(262_144),
        // ── Doubao ─────────────────────────────────────────────────────────
        "doubao-pro-32k" | "doubao-lite-32k" => Some(32_000),
        // Unknown model → caller should use a conservative default
        _ => None,
    }
}

/// Return true when the model is explicitly known to support vision input.
pub fn supports_vision_model(model: &str) -> bool {
    model_capabilities_for_model(model).vision
}

/// Return true when the model is explicitly known to support reasoning/thinking.
pub fn supports_reasoning_model(model: &str) -> bool {
    model_capabilities_for_model(model).reasoning
}

/// Return true when the model supports reasoning given an optional provider-level config.
pub fn supports_reasoning_model_with_config(
    model: &str,
    reasoning_config: Option<&agent_diva_core::reasoning::ReasoningConfig>,
) -> bool {
    model_capabilities_for_model_with_config(model, reasoning_config).reasoning
}

fn normalize_model_id(model: &str) -> String {
    let trimmed = model.trim().to_ascii_lowercase();
    trimmed
        .rsplit_once('/')
        .map(|(_, model)| model.to_string())
        .unwrap_or(trimmed)
}

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
        use serde::ser::Error as _;
        use serde::ser::SerializeStruct;

        #[derive(Serialize)]
        struct Function<'a> {
            name: &'a str,
            arguments: String,
        }

        let arguments = serde_json::to_string(&self.arguments).map_err(|e| {
            S::Error::custom(format!(
                "failed to serialize tool call arguments for {}: {}",
                self.name, e
            ))
        })?;

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
    /// Incremental reasoning content
    ReasoningDelta(String),
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
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_blocks: Option<Vec<serde_json::Value>>,
}

/// Structured content for a chat message.
///
/// Text messages serialize as the legacy JSON string shape, while multimodal
/// messages serialize as an array of content parts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Parts(Vec<MessageContentPart>),
}

impl MessageContent {
    /// Return the content when it is the legacy text shape.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(text) => Some(text),
            Self::Parts(_) => None,
        }
    }

    /// Convert structured content to text for providers that only accept text.
    pub fn to_text_lossy(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::Parts(parts) => parts
                .iter()
                .filter_map(|part| match part {
                    MessageContentPart::Text { text } => Some(text.as_str()),
                    MessageContentPart::ImageUrl { .. }
                    | MessageContentPart::ImageFile { .. }
                    | MessageContentPart::ImageData { .. } => None,
                })
                .collect(),
        }
    }

    /// Apply a text-only transform without altering non-text content parts.
    pub fn sanitize_text<F>(&mut self, sanitize: F)
    where
        F: Fn(&str) -> String,
    {
        match self {
            Self::Text(text) => {
                *text = sanitize(text);
            }
            Self::Parts(parts) => {
                for part in parts {
                    if let MessageContentPart::Text { text } = part {
                        *text = sanitize(text);
                    }
                }
            }
        }
    }

    /// Return true when any text segment matches the predicate.
    pub fn text_any<F>(&self, predicate: F) -> bool
    where
        F: Fn(&str) -> bool,
    {
        match self {
            Self::Text(text) => predicate(text),
            Self::Parts(parts) => parts.iter().any(|part| match part {
                MessageContentPart::Text { text } => predicate(text),
                MessageContentPart::ImageUrl { .. }
                | MessageContentPart::ImageFile { .. }
                | MessageContentPart::ImageData { .. } => false,
            }),
        }
    }

    /// Return true when the content contains any image-bearing part.
    pub fn has_image(&self) -> bool {
        match self {
            Self::Text(_) => false,
            Self::Parts(parts) => parts.iter().any(MessageContentPart::is_image),
        }
    }
}

impl From<String> for MessageContent {
    fn from(value: String) -> Self {
        Self::Text(value)
    }
}

impl From<&str> for MessageContent {
    fn from(value: &str) -> Self {
        Self::Text(value.to_string())
    }
}

impl From<&String> for MessageContent {
    fn from(value: &String) -> Self {
        Self::Text(value.clone())
    }
}

impl From<Vec<MessageContentPart>> for MessageContent {
    fn from(value: Vec<MessageContentPart>) -> Self {
        Self::Parts(value)
    }
}

/// A structured content part within a multimodal chat message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrl },
    ImageFile { image_file: ImageFile },
    ImageData { image_data: ImageData },
}

impl MessageContentPart {
    /// Return true for all image-bearing content part variants.
    pub fn is_image(&self) -> bool {
        matches!(
            self,
            Self::ImageUrl { .. } | Self::ImageFile { .. } | Self::ImageData { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageUrl {
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageFile {
    pub file_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImageData {
    pub data_uri: String,
}

impl Message {
    /// Create a user message
    pub fn user(content: impl Into<MessageContent>) -> Self {
        Self {
            role: "user".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
            thinking_blocks: None,
        }
    }

    /// Return true when this message contains image-bearing content.
    pub fn has_image_content(&self) -> bool {
        self.content.has_image()
    }

    /// Create a system message
    pub fn system(content: impl Into<MessageContent>) -> Self {
        Self {
            role: "system".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
            thinking_blocks: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<MessageContent>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
            thinking_blocks: None,
        }
    }

    /// Create a tool response message
    pub fn tool(content: impl Into<MessageContent>, tool_call_id: impl Into<String>) -> Self {
        Self {
            role: "tool".to_string(),
            content: content.into(),
            name: None,
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
            reasoning_content: None,
            thinking_blocks: None,
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn message_content_reads_legacy_string_content() {
        let message: Message = serde_json::from_value(json!({
            "role": "user",
            "content": "hello"
        }))
        .unwrap();

        assert_eq!(message.content, MessageContent::Text("hello".to_string()));
    }

    #[test]
    fn message_content_writes_legacy_string_content() {
        let message = Message::user("hello");
        let json = serde_json::to_value(&message).unwrap();

        assert_eq!(json["content"], "hello");
    }

    #[test]
    fn message_content_detects_image_parts() {
        assert!(!Message::user("hello").has_image_content());

        let message = Message::user(MessageContent::Parts(vec![
            MessageContentPart::Text {
                text: "look".to_string(),
            },
            MessageContentPart::ImageFile {
                image_file: ImageFile {
                    file_id: "sha256:image".to_string(),
                },
            },
        ]));

        assert!(message.has_image_content());
    }

    #[test]
    fn vision_capabilities_are_conservative() {
        assert!(!supports_vision_model("unknown-model"));
        assert!(!supports_vision_model("deepseek-chat"));
        assert!(supports_vision_model("gpt-4o"));
        assert!(supports_vision_model("openai/gpt-4.1-mini"));

        assert_eq!(
            model_capabilities_for_model("unknown-model"),
            ModelCapabilities::text_only()
        );
    }

    #[test]
    fn reasoning_capabilities_are_conservative() {
        // Unknown models: no reasoning
        assert!(!supports_reasoning_model("unknown-model"));
        assert!(!supports_reasoning_model("gpt-4o"));

        // DeepSeek models
        assert!(supports_reasoning_model("deepseek-chat"));
        assert!(supports_reasoning_model("deepseek-reasoner"));
        assert!(supports_reasoning_model("deepseek/deepseek-r1"));

        // Anthropic Claude
        assert!(supports_reasoning_model("claude-3-5-sonnet"));
        assert!(supports_reasoning_model("claude-sonnet-4"));

        // OpenAI reasoning models
        assert!(supports_reasoning_model("o1"));
        assert!(supports_reasoning_model("o3-mini"));

        // Gemini
        assert!(supports_reasoning_model("gemini-2.5-pro"));

        // Unknown model is still text_only
        assert_eq!(
            model_capabilities_for_model("unknown-model"),
            ModelCapabilities::text_only()
        );
    }

    #[test]
    fn message_content_reads_and_writes_structured_parts() {
        let content = MessageContent::Parts(vec![
            MessageContentPart::Text {
                text: "look".to_string(),
            },
            MessageContentPart::ImageUrl {
                image_url: ImageUrl {
                    url: "https://example.com/cat.png".to_string(),
                },
            },
            MessageContentPart::ImageFile {
                image_file: ImageFile {
                    file_id: "file_local_123".to_string(),
                },
            },
            MessageContentPart::ImageData {
                image_data: ImageData {
                    data_uri: "data:image/png;base64,AAAA".to_string(),
                },
            },
        ]);
        let message = Message::user(content.clone());
        let json = serde_json::to_value(&message).unwrap();

        assert_eq!(json["content"][0]["type"], "text");
        assert_eq!(json["content"][0]["text"], "look");
        assert_eq!(
            json["content"][1]["image_url"]["url"],
            "https://example.com/cat.png"
        );
        assert_eq!(
            json["content"][2]["image_file"]["file_id"],
            "file_local_123"
        );
        assert_eq!(
            json["content"][3]["image_data"]["data_uri"],
            "data:image/png;base64,AAAA"
        );

        let round_trip: Message = serde_json::from_value(json).unwrap();
        assert_eq!(round_trip.content, content);
    }

    #[test]
    fn message_content_to_text_lossy_keeps_only_text_parts() {
        let content = MessageContent::Parts(vec![
            MessageContentPart::Text {
                text: "hello ".to_string(),
            },
            MessageContentPart::ImageFile {
                image_file: ImageFile {
                    file_id: "file_local_123".to_string(),
                },
            },
            MessageContentPart::Text {
                text: "world".to_string(),
            },
        ]);

        assert_eq!(content.as_text(), None);
        assert_eq!(content.to_text_lossy(), "hello world");
    }

    #[test]
    fn dynamic_reasoning_with_config_enables_reasoning_for_any_model() {
        use agent_diva_core::reasoning::ReasoningConfig;

        // Unknown model without config: no reasoning
        assert!(!supports_reasoning_model("unknown-custom-model"));

        // Unknown model with reasoning config: reasoning enabled
        let config = ReasoningConfig {
            reasoning_type: "openai-chat".to_string(),
            thinking_token_limits: None,
            supported_efforts: None,
            default_effort: None,
        };
        assert!(supports_reasoning_model_with_config(
            "unknown-custom-model",
            Some(&config)
        ));

        // Known reasoning model still works without config
        assert!(supports_reasoning_model("deepseek-chat"));

        // Known reasoning model with config still works
        assert!(supports_reasoning_model_with_config(
            "deepseek-chat",
            Some(&config)
        ));
    }

    #[test]
    fn dynamic_reasoning_with_empty_type_disables_reasoning() {
        use agent_diva_core::reasoning::ReasoningConfig;

        let config = ReasoningConfig {
            reasoning_type: "".to_string(),
            thinking_token_limits: None,
            supported_efforts: None,
            default_effort: None,
        };
        // Empty reasoning_type means no reasoning capability
        assert!(!supports_reasoning_model_with_config(
            "any-model",
            Some(&config)
        ));
    }

    #[test]
    fn model_capabilities_with_config_reflects_dynamic_reasoning() {
        use agent_diva_core::reasoning::ReasoningConfig;

        let config = ReasoningConfig {
            reasoning_type: "anthropic".to_string(),
            thinking_token_limits: None,
            supported_efforts: None,
            default_effort: Some("high".to_string()),
        };

        let caps = model_capabilities_for_model_with_config("my-custom-model", Some(&config));
        assert!(caps.reasoning);
        assert!(!caps.vision); // Still conservative for unknown models
    }

    #[test]
    fn model_capabilities_without_config_falls_back_to_hardcoded() {
        // Known reasoning model
        let caps = model_capabilities_for_model("deepseek-chat");
        assert!(caps.reasoning);

        // Unknown model
        let caps = model_capabilities_for_model("unknown-model");
        assert!(!caps.reasoning);
        assert!(!caps.vision);
        assert!(!caps.tools);
    }
    #[test]
    fn context_window_for_known_models() {
        // Anthropic 1M models
        assert_eq!(
            model_capabilities_for_model("claude-sonnet-4-6").context_window,
            Some(1_000_000)
        );
        assert_eq!(
            model_capabilities_for_model("claude-opus-4-7").context_window,
            Some(1_000_000)
        );
        // Anthropic 200K models
        assert_eq!(
            model_capabilities_for_model("claude-sonnet-4").context_window,
            Some(200_000)
        );
        assert_eq!(
            model_capabilities_for_model("claude-haiku-4").context_window,
            Some(200_000)
        );
        // OpenAI 1M models
        assert_eq!(
            model_capabilities_for_model("gpt-4.1").context_window,
            Some(1_047_576)
        );
        // OpenAI 128K models
        assert_eq!(
            model_capabilities_for_model("gpt-4o").context_window,
            Some(128_000)
        );
        // OpenAI 400K model
        assert_eq!(
            model_capabilities_for_model("gpt-5").context_window,
            Some(400_000)
        );
        // DeepSeek 128K
        assert_eq!(
            model_capabilities_for_model("deepseek-chat").context_window,
            Some(128_000)
        );
        // DeepSeek 1M
        assert_eq!(
            model_capabilities_for_model("deepseek-v4-pro").context_window,
            Some(1_000_000)
        );
        // Gemini 1M
        assert_eq!(
            model_capabilities_for_model("gemini-2.5-pro").context_window,
            Some(1_048_576)
        );
        // xAI
        assert_eq!(
            model_capabilities_for_model("grok-4").context_window,
            Some(256_000)
        );
        // Qwen
        assert_eq!(
            model_capabilities_for_model("qwen-max").context_window,
            Some(262_144)
        );
    }
    #[test]
    fn context_window_for_unknown_model_is_none() {
        assert_eq!(
            model_capabilities_for_model("unknown-model").context_window,
            None
        );
        assert_eq!(
            model_capabilities_for_model("some-random-llm").context_window,
            None
        );
    }
    #[test]
    fn context_window_with_provider_prefix() {
        // Provider prefix should be stripped by normalize_model_id
        assert_eq!(
            model_capabilities_for_model("openai/gpt-4o").context_window,
            Some(128_000)
        );
        assert_eq!(
            model_capabilities_for_model("anthropic/claude-sonnet-4-6").context_window,
            Some(1_000_000)
        );
    }
}
