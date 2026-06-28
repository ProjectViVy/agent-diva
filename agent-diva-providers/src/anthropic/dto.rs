//! Anthropic Messages API DTOs — request/response types.
//!
//! These types map directly to the Anthropic Messages API wire format
//! (<https://docs.anthropic.com/en/api/messages>).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Core content block model ───────────────────────────────────────────

/// A typed content block within an Anthropic message.
///
/// Anthropic uses a content-block model rather than simple strings.
/// Every message `content` is an array of these typed blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    /// Plain text content.
    #[serde(rename = "text")]
    Text {
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Base64-encoded image.
    #[serde(rename = "image")]
    Image { source: ImageSource },

    /// Assistant-initiated tool call (history-replay only).
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },

    /// Tool execution result (role="user" in Anthropic).
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Signed extended-thinking block (history-replay only).
    #[serde(rename = "thinking")]
    Thinking { thinking: String, signature: String },
}

#[allow(dead_code)]
impl ContentBlock {
    /// Returns the text content if this is a `Text` block.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text { text, .. } => Some(text),
            _ => None,
        }
    }

    /// Returns true if this is a `ToolUse` block.
    pub fn is_tool_use(&self) -> bool {
        matches!(self, Self::ToolUse { .. })
    }

    /// Returns the tool_use id if this is a `ToolUse` block.
    pub fn tool_use_id(&self) -> Option<&str> {
        match self {
            Self::ToolUse { id, .. } => Some(id),
            _ => None,
        }
    }
}

// ── Supporting types ───────────────────────────────────────────────────

/// Prompt-caching marker for content blocks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub cache_type: String,
}

impl CacheControl {
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
        }
    }
}

/// Image source for `ContentBlock::Image`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    pub media_type: String,
    pub data: String,
}

// ── System prompt ──────────────────────────────────────────────────────

/// Anthropic system prompt — a top-level field, NOT a message role.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// Simple string form.
    Text(String),
    /// Array of text blocks (with optional cache_control).
    Blocks(Vec<SystemTextBlock>),
}

/// A single text block within a system prompt array.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTextBlock {
    #[serde(rename = "type")]
    pub block_type: String,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

#[allow(dead_code)]
impl SystemTextBlock {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: text.into(),
            cache_control: None,
        }
    }

    pub fn text_cached(text: impl Into<String>) -> Self {
        Self {
            block_type: "text".to_string(),
            text: text.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }
    }
}

// ── Messages ───────────────────────────────────────────────────────────

/// An Anthropic-native message (role + array of content blocks).
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicMessage {
    pub role: String,
    pub content: Vec<ContentBlock>,
}

impl AnthropicMessage {
    pub fn user(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: "user".to_string(),
            content: blocks,
        }
    }

    pub fn assistant(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: blocks,
        }
    }
}

// ── Tool definitions ───────────────────────────────────────────────────

/// Anthropic tool definition (uses `input_schema`, not `parameters`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

/// Anthropic tool choice (object form, not flat string).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolChoice {
    #[serde(rename = "type")]
    pub choice_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disable_parallel_tool_use: Option<bool>,
}

// ── Thinking configuration ─────────────────────────────────────────────

/// Extended-thinking configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThinkingConfig {
    #[serde(rename = "type")]
    pub thinking_type: String,
    pub budget_tokens: u32,
}

impl ThinkingConfig {
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            thinking_type: "enabled".to_string(),
            budget_tokens,
        }
    }
}

// ── Top-level request / response ──────────────────────────────────────

/// Anthropic Messages API request body.
#[derive(Debug, Clone, Serialize)]
pub struct AnthropicRequest {
    pub model: String,
    pub messages: Vec<AnthropicMessage>,
    pub max_tokens: u32,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

/// Anthropic Messages API response body (non-streaming).
#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicResponse {
    #[allow(dead_code)]
    pub id: String,
    #[allow(dead_code)]
    pub model: String,
    #[allow(dead_code)]
    pub role: String,
    pub content: Vec<ContentBlock>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    pub stop_sequence: Option<String>,
    pub usage: AnthropicUsage,
}

/// Anthropic token usage with three disjoint input-token buckets.
///
/// **Normalization required**: the real total input is
/// `input_tokens + cache_creation_input_tokens + cache_read_input_tokens`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct AnthropicUsage {
    /// Tokens after the last cache breakpoint (NOT the total input).
    #[serde(default)]
    pub input_tokens: Option<u64>,

    /// Tokens in the model's response.
    #[serde(default)]
    pub output_tokens: Option<u64>,

    /// Tokens written to cache this request (billed at higher rate).
    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,

    /// Tokens served from cache (billed at discounted rate).
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

impl AnthropicUsage {
    /// Sum the three disjoint input-token buckets to get the true total input.
    pub fn total_input_tokens(&self) -> u64 {
        let a = self.input_tokens.unwrap_or(0);
        let b = self.cache_creation_input_tokens.unwrap_or(0);
        let c = self.cache_read_input_tokens.unwrap_or(0);
        a.saturating_add(b).saturating_add(c)
    }

    /// Compute the total tokens (input + output).
    ///
    /// Used by `to_normalized_usage_map()` for the `total_tokens` key.
    pub fn total_tokens(&self) -> u64 {
        self.total_input_tokens()
            .saturating_add(self.output_tokens.unwrap_or(0))
    }
}

// ── Anthropic error envelope ───────────────────────────────────────────

/// Anthropic error envelope (`{"type": "error", "error": {...}}`).
#[derive(Debug, Deserialize)]
pub struct AnthropicErrorEnvelope {
    pub error: AnthropicErrorBody,
}

#[derive(Debug, Deserialize)]
pub struct AnthropicErrorBody {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub error_type: String,
    pub message: String,
}

// ── SSE streaming event types ──────────────────────────────────────────

/// Parsed Anthropic SSE event (from `event:` + `data:` pairs).
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SseEvent {
    #[serde(rename = "message_start")]
    MessageStart { message: SseMessageStartData },

    #[serde(rename = "content_block_start")]
    ContentBlockStart {
        index: usize,
        content_block: SseContentBlockStart,
    },

    #[serde(rename = "content_block_delta")]
    ContentBlockDelta { index: usize, delta: SseDelta },

    #[serde(rename = "content_block_stop")]
    ContentBlockStop { index: usize },

    #[serde(rename = "message_delta")]
    MessageDelta {
        delta: SseMessageDelta,
        usage: SseOutputUsage,
    },

    #[serde(rename = "message_stop")]
    MessageStop,

    #[serde(rename = "ping")]
    Ping,
}

/// Data from the `message_start` SSE event.
#[derive(Debug, Clone, Deserialize)]
pub struct SseMessageStartData {
    pub id: String,
    pub model: String,
    pub role: String,
    pub usage: SseInputUsage,
}

/// Input-side usage from `message_start`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SseInputUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,

    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,

    #[serde(default)]
    pub cache_creation_input_tokens: Option<u64>,
}

/// Content block announcement from `content_block_start`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SseContentBlockStart {
    #[serde(rename = "text")]
    Text { text: String },

    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String },

    #[serde(rename = "thinking")]
    Thinking { thinking: String, signature: String },
}

/// Incremental delta from `content_block_delta`.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum SseDelta {
    #[serde(rename = "text_delta")]
    TextDelta { text: String },

    #[serde(rename = "input_json_delta")]
    InputJsonDelta { partial_json: String },

    #[serde(rename = "thinking_delta")]
    ThinkingDelta { thinking: String },

    #[serde(rename = "signature_delta")]
    SignatureDelta { signature: String },
}

/// Stop reason delta from `message_delta`.
#[derive(Debug, Clone, Deserialize)]
pub struct SseMessageDelta {
    pub stop_reason: Option<String>,
}

/// Output-side usage from `message_delta`.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct SseOutputUsage {
    #[serde(default)]
    pub output_tokens: Option<u64>,
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── ContentBlock serialization ────────────────────────────────

    #[test]
    fn content_block_text_serializes_correctly() {
        let block = ContentBlock::Text {
            text: "Hello, world!".to_string(),
            cache_control: None,
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["text"], "Hello, world!");
        assert!(json.get("cache_control").is_none());
    }

    #[test]
    fn content_block_text_with_cache_control() {
        let block = ContentBlock::Text {
            text: "cached prompt".to_string(),
            cache_control: Some(CacheControl::ephemeral()),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "text");
        assert_eq!(json["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn content_block_image_serializes_correctly() {
        let block = ContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type: "image/jpeg".to_string(),
                data: "/9j/4AAQ".to_string(),
            },
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "image");
        assert_eq!(json["source"]["type"], "base64");
        assert_eq!(json["source"]["media_type"], "image/jpeg");
    }

    #[test]
    fn content_block_tool_use_serializes_correctly() {
        let block = ContentBlock::ToolUse {
            id: "call_1".to_string(),
            name: "get_weather".to_string(),
            input: serde_json::json!({"location": "NYC"}),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "tool_use");
        assert_eq!(json["id"], "call_1");
        assert_eq!(json["name"], "get_weather");
        assert_eq!(json["input"]["location"], "NYC");
    }

    #[test]
    fn content_block_tool_result_serializes_correctly() {
        let block = ContentBlock::ToolResult {
            tool_use_id: "call_1".to_string(),
            content: serde_json::Value::String("72 degrees".to_string()),
            cache_control: None,
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "tool_result");
        assert_eq!(json["tool_use_id"], "call_1");
        assert_eq!(json["content"], "72 degrees");
    }

    #[test]
    fn content_block_thinking_serializes_correctly() {
        let block = ContentBlock::Thinking {
            thinking: "Let me think...".to_string(),
            signature: "sig_abc".to_string(),
        };
        let json = serde_json::to_value(&block).unwrap();
        assert_eq!(json["type"], "thinking");
        assert_eq!(json["thinking"], "Let me think...");
        assert_eq!(json["signature"], "sig_abc");
    }

    #[test]
    fn content_block_round_trip_text() {
        let original = ContentBlock::Text {
            text: "round trip test".to_string(),
            cache_control: None,
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.as_text(), Some("round trip test"));
    }

    #[test]
    fn content_block_round_trip_tool_use() {
        let original = ContentBlock::ToolUse {
            id: "call_1".to_string(),
            name: "search".to_string(),
            input: serde_json::json!({"q": "rust", "limit": 10}),
        };
        let json = serde_json::to_string(&original).unwrap();
        let parsed: ContentBlock = serde_json::from_str(&json).unwrap();
        match parsed {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_1");
                assert_eq!(name, "search");
                assert_eq!(input["q"], "rust");
                assert_eq!(input["limit"], 10);
            }
            other => panic!("expected ToolUse, got {:?}", other),
        }
    }

    // ── AnthropicRequest serialization ─────────────────────────────

    #[test]
    fn anthropic_request_minimal() {
        let req = AnthropicRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![AnthropicMessage::user(vec![ContentBlock::Text {
                text: "Hello".to_string(),
                cache_control: None,
            }])],
            max_tokens: 4096,
            system: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            stream: None,
            temperature: None,
            metadata: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["model"], "claude-sonnet-4-5");
        assert_eq!(json["max_tokens"], 4096);
        assert!(json.get("system").is_none());
        assert!(json.get("tools").is_none());
        assert_eq!(json["messages"][0]["role"], "user");
    }

    #[test]
    fn anthropic_request_with_system_string() {
        let req = AnthropicRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![],
            max_tokens: 4096,
            system: Some(SystemPrompt::Text("You are helpful.".to_string())),
            tools: None,
            tool_choice: None,
            thinking: None,
            stream: None,
            temperature: None,
            metadata: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["system"], "You are helpful.");
    }

    #[test]
    fn anthropic_request_with_system_blocks() {
        let req = AnthropicRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![],
            max_tokens: 4096,
            system: Some(SystemPrompt::Blocks(vec![SystemTextBlock::text_cached(
                "You are helpful.",
            )])),
            tools: None,
            tool_choice: None,
            thinking: None,
            stream: None,
            temperature: None,
            metadata: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        let system = json["system"].as_array().unwrap();
        assert_eq!(system[0]["type"], "text");
        assert_eq!(system[0]["text"], "You are helpful.");
        assert_eq!(system[0]["cache_control"]["type"], "ephemeral");
    }

    #[test]
    fn anthropic_request_with_thinking() {
        let req = AnthropicRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![],
            max_tokens: 16384,
            system: None,
            tools: None,
            tool_choice: None,
            thinking: Some(ThinkingConfig::enabled(16000)),
            stream: None,
            temperature: None,
            metadata: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["thinking"]["type"], "enabled");
        assert_eq!(json["thinking"]["budget_tokens"], 16000);
    }

    #[test]
    fn anthropic_request_stream_true() {
        let req = AnthropicRequest {
            model: "claude-sonnet-4-5".to_string(),
            messages: vec![],
            max_tokens: 4096,
            system: None,
            tools: None,
            tool_choice: None,
            thinking: None,
            stream: Some(true),
            temperature: None,
            metadata: None,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["stream"], true);
    }

    // ── AnthropicResponse deserialization ──────────────────────────

    #[test]
    fn anthropic_response_deserializes_text() {
        let json = serde_json::json!({
            "id": "msg_01",
            "model": "claude-sonnet-4-5",
            "role": "assistant",
            "content": [
                {"type": "text", "text": "Hello, human!"}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50
            }
        });
        let resp: AnthropicResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.id, "msg_01");
        assert_eq!(resp.model, "claude-sonnet-4-5");
        assert_eq!(resp.role, "assistant");
        assert_eq!(resp.content.len(), 1);
        assert_eq!(resp.content[0].as_text(), Some("Hello, human!"));
        assert_eq!(resp.stop_reason, Some("end_turn".to_string()));
    }

    #[test]
    fn anthropic_response_deserializes_tool_use() {
        let json = serde_json::json!({
            "id": "msg_02",
            "model": "claude-sonnet-4-5",
            "role": "assistant",
            "content": [
                {"type": "tool_use", "id": "call_1", "name": "get_weather", "input": {"location": "NYC"}}
            ],
            "stop_reason": "tool_use",
            "usage": {
                "input_tokens": 80,
                "output_tokens": 30
            }
        });
        let resp: AnthropicResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.content.len(), 1);
        assert!(resp.content[0].is_tool_use());
        assert_eq!(resp.stop_reason, Some("tool_use".to_string()));
    }

    #[test]
    fn anthropic_response_deserializes_thinking() {
        let json = serde_json::json!({
            "id": "msg_03",
            "model": "claude-sonnet-4-5",
            "role": "assistant",
            "content": [
                {"type": "thinking", "thinking": "Let me analyze...", "signature": "sig_xyz"},
                {"type": "text", "text": "Here's the answer."}
            ],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 200,
                "output_tokens": 100
            }
        });
        let resp: AnthropicResponse = serde_json::from_value(json).unwrap();
        assert_eq!(resp.content.len(), 2);
        // First block is thinking
        match &resp.content[0] {
            ContentBlock::Thinking {
                thinking,
                signature,
                ..
            } => {
                assert_eq!(thinking, "Let me analyze...");
                assert_eq!(signature, "sig_xyz");
            }
            other => panic!("expected Thinking, got {:?}", other),
        }
    }

    // ── AnthropicUsage normalization ────────────────────────────────

    #[test]
    fn anthropic_usage_normalizes_input_buckets() {
        let usage = AnthropicUsage {
            input_tokens: Some(500),
            cache_creation_input_tokens: Some(200),
            cache_read_input_tokens: Some(300),
            output_tokens: Some(400),
        };
        assert_eq!(usage.total_input_tokens(), 1000); // 500 + 200 + 300
        assert_eq!(usage.total_tokens(), 1400); // 1000 + 400
    }

    #[test]
    fn anthropic_usage_with_missing_buckets() {
        let usage = AnthropicUsage {
            input_tokens: Some(500),
            output_tokens: Some(200),
            ..Default::default()
        };
        assert_eq!(usage.total_input_tokens(), 500);
        assert_eq!(usage.total_tokens(), 700);
    }

    #[test]
    fn anthropic_usage_all_none() {
        let usage = AnthropicUsage::default();
        assert_eq!(usage.total_input_tokens(), 0);
        assert_eq!(usage.total_tokens(), 0);
    }

    // ── SSE event deserialization ──────────────────────────────────

    #[test]
    fn sse_event_message_start() {
        let json = serde_json::json!({
            "type": "message_start",
            "message": {
                "id": "msg_01",
                "model": "claude-sonnet-4-5",
                "role": "assistant",
                "usage": {
                    "input_tokens": 100,
                    "cache_read_input_tokens": 200
                }
            }
        });
        let event: SseEvent = serde_json::from_value(json).unwrap();
        match event {
            SseEvent::MessageStart { message } => {
                assert_eq!(message.id, "msg_01");
                assert_eq!(message.model, "claude-sonnet-4-5");
                assert_eq!(message.usage.input_tokens, Some(100));
                assert_eq!(message.usage.cache_read_input_tokens, Some(200));
            }
            other => panic!("expected MessageStart, got {:?}", other),
        }
    }

    #[test]
    fn sse_event_content_block_start_text() {
        let json = serde_json::json!({
            "type": "content_block_start",
            "index": 0,
            "content_block": {
                "type": "text",
                "text": ""
            }
        });
        let event: SseEvent = serde_json::from_value(json).unwrap();
        match event {
            SseEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                assert_eq!(index, 0);
                match content_block {
                    SseContentBlockStart::Text { text } => assert_eq!(text, ""),
                    other => panic!("expected Text, got {:?}", other),
                }
            }
            other => panic!("expected ContentBlockStart, got {:?}", other),
        }
    }

    #[test]
    fn sse_event_content_block_start_tool_use() {
        let json = serde_json::json!({
            "type": "content_block_start",
            "index": 1,
            "content_block": {
                "type": "tool_use",
                "id": "call_1",
                "name": "get_weather"
            }
        });
        let event: SseEvent = serde_json::from_value(json).unwrap();
        match event {
            SseEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                assert_eq!(index, 1);
                match content_block {
                    SseContentBlockStart::ToolUse { id, name } => {
                        assert_eq!(id, "call_1");
                        assert_eq!(name, "get_weather");
                    }
                    other => panic!("expected ToolUse, got {:?}", other),
                }
            }
            other => panic!("expected ContentBlockStart, got {:?}", other),
        }
    }

    #[test]
    fn sse_event_content_block_delta_text() {
        let json = serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "text_delta",
                "text": "Hello"
            }
        });
        let event: SseEvent = serde_json::from_value(json).unwrap();
        match event {
            SseEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                match delta {
                    SseDelta::TextDelta { text } => assert_eq!(text, "Hello"),
                    other => panic!("expected TextDelta, got {:?}", other),
                }
            }
            other => panic!("expected ContentBlockDelta, got {:?}", other),
        }
    }

    #[test]
    fn sse_event_content_block_delta_json() {
        let json = serde_json::json!({
            "type": "content_block_delta",
            "index": 0,
            "delta": {
                "type": "input_json_delta",
                "partial_json": "{\"loc"
            }
        });
        let event: SseEvent = serde_json::from_value(json).unwrap();
        match event {
            SseEvent::ContentBlockDelta { index, delta } => {
                assert_eq!(index, 0);
                match delta {
                    SseDelta::InputJsonDelta { partial_json } => {
                        assert_eq!(partial_json, "{\"loc")
                    }
                    other => panic!("expected InputJsonDelta, got {:?}", other),
                }
            }
            other => panic!("expected ContentBlockDelta, got {:?}", other),
        }
    }

    #[test]
    fn sse_event_message_delta() {
        let json = serde_json::json!({
            "type": "message_delta",
            "delta": {
                "stop_reason": "end_turn"
            },
            "usage": {
                "output_tokens": 50
            }
        });
        let event: SseEvent = serde_json::from_value(json).unwrap();
        match event {
            SseEvent::MessageDelta { delta, usage } => {
                assert_eq!(delta.stop_reason, Some("end_turn".to_string()));
                assert_eq!(usage.output_tokens, Some(50));
            }
            other => panic!("expected MessageDelta, got {:?}", other),
        }
    }

    #[test]
    fn sse_event_message_stop() {
        let json = serde_json::json!({"type": "message_stop"});
        let event: SseEvent = serde_json::from_value(json).unwrap();
        assert!(matches!(event, SseEvent::MessageStop));
    }

    // ── AnthropicErrorEnvelope ──────────────────────────────────────

    #[test]
    fn anthropic_error_deserialization() {
        let json = serde_json::json!({
            "type": "error",
            "error": {
                "type": "invalid_request_error",
                "message": "model not found"
            }
        });
        let envelope: AnthropicErrorEnvelope = serde_json::from_value(json).unwrap();
        assert_eq!(envelope.error.error_type, "invalid_request_error");
        assert_eq!(envelope.error.message, "model not found");
    }

    // ── ToolChoice serialization ────────────────────────────────────

    #[test]
    fn tool_choice_auto() {
        let choice = ToolChoice {
            choice_type: "auto".to_string(),
            name: None,
            disable_parallel_tool_use: None,
        };
        let json = serde_json::to_value(&choice).unwrap();
        assert_eq!(json["type"], "auto");
        assert!(json.get("name").is_none());
    }

    #[test]
    fn tool_choice_specific_tool() {
        let choice = ToolChoice {
            choice_type: "tool".to_string(),
            name: Some("get_weather".to_string()),
            disable_parallel_tool_use: Some(true),
        };
        let json = serde_json::to_value(&choice).unwrap();
        assert_eq!(json["type"], "tool");
        assert_eq!(json["name"], "get_weather");
        assert_eq!(json["disable_parallel_tool_use"], true);
    }

    // ── SystemPrompt serialization ──────────────────────────────────

    #[test]
    fn system_prompt_text_serializes_as_string() {
        let sp = SystemPrompt::Text("You are helpful.".to_string());
        let json = serde_json::to_value(&sp).unwrap();
        assert_eq!(json, "You are helpful.");
    }

    #[test]
    fn system_prompt_blocks_serializes_as_array() {
        let sp = SystemPrompt::Blocks(vec![
            SystemTextBlock::text("First block."),
            SystemTextBlock::text_cached("Second block (cached)."),
        ]);
        let json = serde_json::to_value(&sp).unwrap();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["text"], "First block.");
        assert!(arr[0].get("cache_control").is_none());
        assert_eq!(arr[1]["text"], "Second block (cached).");
        assert_eq!(arr[1]["cache_control"]["type"], "ephemeral");
    }
}
