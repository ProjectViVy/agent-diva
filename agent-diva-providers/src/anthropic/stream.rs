//! Anthropic SSE streaming parser.
//!
//! Anthropic SSE differs from OpenAI: it uses **named events** with an
//! explicit `event:` prefix, and usage data is split across
//! `message_start` (input tokens) and `message_delta` (output tokens).

use agent_diva_core::Usage;

use crate::base::{LLMResponse, ToolCallRequest};
use std::collections::HashMap;

use super::dto::{
    SseContentBlockStart, SseDelta, SseEvent, SseInputUsage, SseMessageStartData, SseOutputUsage,
};

/// Active content block being accumulated during streaming.
enum ActiveBlock {
    Text {
        text: String,
    },
    ToolUse {
        id: String,
        name: String,
        partial_json: String,
    },
    Thinking {
        thinking: String,
        signature: String,
    },
}

/// Internal streaming state.
#[derive(Default)]
pub(super) struct StreamState {
    pub(super) message_id: String,
    pub(super) model: String,
    pub(super) role: String,

    /// Accumulated text content (concatenation of text blocks).
    pub(super) text_content: String,

    /// Accumulated reasoning/thinking content.
    pub(super) reasoning_content: String,

    /// Accumulated thinking signatures (preserved for round-trip).
    pub(super) thinking_signatures: Vec<String>,

    /// Active content blocks being assembled (indexed by block index).
    active_blocks: Vec<ActiveBlock>,

    /// Completed tool calls.
    pub(super) tool_calls: Vec<ToolCallRequest>,

    /// Stop reason from `message_delta`.
    pub(super) stop_reason: Option<String>,

    /// Input-side usage from `message_start`.
    pub(super) input_usage: SseInputUsage,

    /// Output-side usage from `message_delta`.
    pub(super) output_usage: SseOutputUsage,
}

impl StreamState {
    /// Process a single parsed SSE event into streaming state.
    pub(super) fn handle_event(&mut self, event: &SseEvent) {
        match event {
            SseEvent::MessageStart { message } => {
                self.handle_message_start(message);
            }
            SseEvent::ContentBlockStart {
                index,
                content_block,
            } => {
                self.handle_content_block_start(*index, content_block);
            }
            SseEvent::ContentBlockDelta { index, delta } => {
                self.handle_content_block_delta(*index, delta);
            }
            SseEvent::ContentBlockStop { index } => {
                self.handle_content_block_stop(*index);
            }
            SseEvent::MessageDelta { delta, usage } => {
                self.stop_reason = delta.stop_reason.clone();
                self.output_usage = usage.clone();
            }
            SseEvent::MessageStop | SseEvent::Ping => {
                // message_stop signals end of stream; ping is a no-op
            }
        }
    }

    fn handle_message_start(&mut self, msg: &SseMessageStartData) {
        self.message_id = msg.id.clone();
        self.model = msg.model.clone();
        self.role = msg.role.clone();
        self.input_usage = msg.usage.clone();
    }

    fn handle_content_block_start(&mut self, index: usize, block: &SseContentBlockStart) {
        // Ensure the active_blocks vec has room for this index
        if self.active_blocks.len() <= index {
            self.active_blocks
                .resize_with(index + 1, || ActiveBlock::Text {
                    text: String::new(),
                });
        }
        match block {
            SseContentBlockStart::Text { text } => {
                self.active_blocks[index] = ActiveBlock::Text { text: text.clone() };
            }
            SseContentBlockStart::ToolUse { id, name } => {
                self.active_blocks[index] = ActiveBlock::ToolUse {
                    id: id.clone(),
                    name: name.clone(),
                    partial_json: String::new(),
                };
            }
            SseContentBlockStart::Thinking {
                thinking,
                signature,
            } => {
                self.active_blocks[index] = ActiveBlock::Thinking {
                    thinking: thinking.clone(),
                    signature: signature.clone(),
                };
            }
        }
    }

    fn handle_content_block_delta(&mut self, index: usize, delta: &SseDelta) {
        // Lazy-resize if needed
        if self.active_blocks.len() <= index {
            self.active_blocks
                .resize_with(index + 1, || ActiveBlock::Text {
                    text: String::new(),
                });
        }
        match delta {
            SseDelta::TextDelta { text: delta_text } => {
                // Emit text for the stream consumer — but for final assembly
                // we collect in text_content.
                if let ActiveBlock::Text { ref mut text } = self.active_blocks[index] {
                    text.push_str(delta_text);
                }
            }
            SseDelta::InputJsonDelta {
                partial_json: json_delta,
            } => {
                if let ActiveBlock::ToolUse {
                    ref mut partial_json,
                    ..
                } = self.active_blocks[index]
                {
                    partial_json.push_str(json_delta);
                }
            }
            SseDelta::ThinkingDelta { thinking } => {
                if let ActiveBlock::Thinking {
                    thinking: ref mut t,
                    ..
                } = self.active_blocks[index]
                {
                    t.push_str(thinking);
                }
            }
            SseDelta::SignatureDelta { signature } => {
                if let ActiveBlock::Thinking {
                    signature: ref mut s,
                    ..
                } = self.active_blocks[index]
                {
                    s.push_str(signature);
                }
            }
        }
    }

    fn handle_content_block_stop(&mut self, index: usize) {
        if index >= self.active_blocks.len() {
            return;
        }
        // Finalize the block
        match &self.active_blocks[index] {
            ActiveBlock::Text { text } => {
                self.text_content.push_str(text);
            }
            ActiveBlock::ToolUse {
                id,
                name,
                partial_json,
            } => {
                let arguments = parse_tool_arguments(partial_json);
                self.tool_calls.push(ToolCallRequest {
                    id: id.clone(),
                    call_type: "function".to_string(),
                    name: name.clone(),
                    arguments,
                });
            }
            ActiveBlock::Thinking {
                thinking,
                signature,
            } => {
                // Collect thinking content for reasoning_content output
                if !thinking.is_empty() {
                    if !self.reasoning_content.is_empty() {
                        self.reasoning_content.push('\n');
                    }
                    self.reasoning_content.push_str(thinking);
                }
                // Preserve signatures for round-trip
                if !signature.is_empty() {
                    self.thinking_signatures.push(signature.clone());
                }
            }
        }
    }

    /// Consume the state and produce a final `LLMResponse`.
    pub(super) fn into_response(self) -> LLMResponse {
        let usage = build_usage(&self.input_usage, &self.output_usage);

        LLMResponse {
            content: if self.text_content.is_empty() {
                None
            } else {
                Some(self.text_content)
            },
            tool_calls: self.tool_calls,
            finish_reason: self.stop_reason.unwrap_or_else(|| "stop".to_string()),
            usage: Some(usage),
            reasoning_content: if self.reasoning_content.is_empty() {
                None
            } else {
                Some(self.reasoning_content)
            },
        }
    }
}

// ── SSE event parsing ──────────────────────────────────────────────────

/// Parse Anthropic named-SSE events from a byte buffer.
///
/// Anthropic SSE uses `event: <name>\ndata: <json>\n\n` format,
/// where the `event:` prefix tells us what event type to expect.
///
/// Returns pairs of (event_name, data_json).
pub(super) fn parse_anthropic_sse(buffer: &mut String) -> Vec<(String, String)> {
    let mut events = Vec::new();
    while let Some(pos) = buffer.find("\n\n") {
        let raw = buffer[..pos].to_string();
        buffer.drain(..pos + 2);

        let mut event_name = String::new();
        let mut data = String::new();

        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("event: ") {
                event_name = rest.trim().to_string();
            } else if let Some(rest) = line.strip_prefix("data: ") {
                data = rest.trim().to_string();
            }
        }

        if !data.is_empty() {
            events.push((event_name, data));
        }
    }
    events
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// Parse streaming tool call JSON into a HashMap.
fn parse_tool_arguments(partial_json: &str) -> HashMap<String, serde_json::Value> {
    serde_json::from_str::<HashMap<String, serde_json::Value>>(partial_json).unwrap_or_else(|_| {
        // Fallback: wrap raw text
        let mut map = HashMap::new();
        map.insert(
            "raw".to_string(),
            serde_json::Value::String(partial_json.to_string()),
        );
        map
    })
}

/// Build a normalized `Usage` from split Anthropic SSE usage data.
fn build_usage(input: &SseInputUsage, output: &SseOutputUsage) -> Usage {
    let prompt_tokens = (input.input_tokens.unwrap_or(0)
        + input.cache_creation_input_tokens.unwrap_or(0)
        + input.cache_read_input_tokens.unwrap_or(0)) as i64;

    let completion_tokens = output.output_tokens.unwrap_or(0) as i64;

    Usage::new(prompt_tokens, completion_tokens)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::dto::*;
    use super::*;
    use crate::anthropic::dto::SseMessageDelta;

    // ── SSE parsing ──────────────────────────────────────────────────

    #[test]
    fn parse_anthropic_sse_single_event() {
        let mut buffer =
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hello\"}}\n\n"
                .to_string();
        let events = parse_anthropic_sse(&mut buffer);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].0, "content_block_delta");
        assert!(events[0].1.contains("text_delta"));
        assert!(buffer.is_empty());
    }

    #[test]
    fn parse_anthropic_sse_multiple_events() {
        let mut buffer = concat!(
            "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"model\":\"claude\",\"role\":\"assistant\",\"usage\":{\"input_tokens\":10}}}\n\n",
            "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"text_delta\",\"text\":\"Hi\"}}\n\n",
            "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}}\n\n",
        )
        .to_string();

        let events = parse_anthropic_sse(&mut buffer);
        assert_eq!(events.len(), 4);
        assert_eq!(events[0].0, "message_start");
        assert_eq!(events[1].0, "content_block_start");
        assert_eq!(events[2].0, "content_block_delta");
        assert_eq!(events[3].0, "content_block_stop");
    }

    #[test]
    fn parse_anthropic_sse_partial_buffer() {
        let mut buffer = "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"model\":\"claude\",\"role\":\"assistant\",\"usage\":{\"input_tokens\":10}}}\n\ntrailing".to_string();
        let events = parse_anthropic_sse(&mut buffer);
        assert_eq!(events.len(), 1);
        assert_eq!(buffer, "trailing");
    }

    #[test]
    fn parse_anthropic_sse_incomplete_event_ignored() {
        // No double-newline means no complete event yet
        let mut buffer = "event: message_start\ndata: {\"type\":\"message_start\"".to_string();
        let events = parse_anthropic_sse(&mut buffer);
        assert_eq!(events.len(), 0);
        assert!(!buffer.is_empty());
    }

    // ── StreamState full lifecycle ───────────────────────────────────

    #[test]
    fn stream_state_text_only_response() {
        let mut state = StreamState::default();

        state.handle_event(&SseEvent::MessageStart {
            message: SseMessageStartData {
                id: "msg_1".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                role: "assistant".to_string(),
                usage: SseInputUsage {
                    input_tokens: Some(100),
                    ..Default::default()
                },
            },
        });

        state.handle_event(&SseEvent::ContentBlockStart {
            index: 0,
            content_block: SseContentBlockStart::Text {
                text: String::new(),
            },
        });

        state.handle_event(&SseEvent::ContentBlockDelta {
            index: 0,
            delta: SseDelta::TextDelta {
                text: "Hello,".to_string(),
            },
        });
        state.handle_event(&SseEvent::ContentBlockDelta {
            index: 0,
            delta: SseDelta::TextDelta {
                text: " world!".to_string(),
            },
        });

        state.handle_event(&SseEvent::ContentBlockStop { index: 0 });

        state.handle_event(&SseEvent::MessageDelta {
            delta: SseMessageDelta {
                stop_reason: Some("end_turn".to_string()),
            },
            usage: SseOutputUsage {
                output_tokens: Some(50),
            },
        });

        let resp = state.into_response();
        assert_eq!(resp.content, Some("Hello, world!".to_string()));
        assert_eq!(resp.finish_reason, "end_turn");
        assert_eq!(resp.tool_calls.len(), 0);

        let usage = resp.usage.as_ref().unwrap();
        assert_eq!(usage.prompt_tokens, 100);
        assert_eq!(usage.completion_tokens, 50);
        assert_eq!(usage.total_tokens, 150);
    }

    #[test]
    fn stream_state_tool_use_response() {
        let mut state = StreamState::default();

        state.handle_event(&SseEvent::MessageStart {
            message: SseMessageStartData {
                id: "msg_2".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                role: "assistant".to_string(),
                usage: SseInputUsage {
                    input_tokens: Some(200),
                    ..Default::default()
                },
            },
        });

        state.handle_event(&SseEvent::ContentBlockStart {
            index: 0,
            content_block: SseContentBlockStart::ToolUse {
                id: "call_1".to_string(),
                name: "get_weather".to_string(),
            },
        });

        state.handle_event(&SseEvent::ContentBlockDelta {
            index: 0,
            delta: SseDelta::InputJsonDelta {
                partial_json: "{\"loc".to_string(),
            },
        });
        state.handle_event(&SseEvent::ContentBlockDelta {
            index: 0,
            delta: SseDelta::InputJsonDelta {
                partial_json: "ation\":\"NYC\"}".to_string(),
            },
        });

        state.handle_event(&SseEvent::ContentBlockStop { index: 0 });

        state.handle_event(&SseEvent::MessageDelta {
            delta: SseMessageDelta {
                stop_reason: Some("tool_use".to_string()),
            },
            usage: SseOutputUsage {
                output_tokens: Some(60),
            },
        });

        let resp = state.into_response();
        assert!(resp.content.is_none() || resp.content == Some(String::new()));
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].id, "call_1");
        assert_eq!(resp.tool_calls[0].name, "get_weather");
        assert_eq!(
            resp.tool_calls[0]
                .arguments
                .get("location")
                .and_then(|v| v.as_str()),
            Some("NYC")
        );
        assert_eq!(resp.finish_reason, "tool_use");
    }

    #[test]
    fn stream_state_usage_with_cache_buckets() {
        let mut state = StreamState::default();

        state.handle_event(&SseEvent::MessageStart {
            message: SseMessageStartData {
                id: "msg_3".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                role: "assistant".to_string(),
                usage: SseInputUsage {
                    input_tokens: Some(500),
                    cache_read_input_tokens: Some(3000),
                    cache_creation_input_tokens: Some(200),
                },
            },
        });

        state.handle_event(&SseEvent::ContentBlockStart {
            index: 0,
            content_block: SseContentBlockStart::Text {
                text: String::new(),
            },
        });

        state.handle_event(&SseEvent::ContentBlockDelta {
            index: 0,
            delta: SseDelta::TextDelta {
                text: "ok".to_string(),
            },
        });

        state.handle_event(&SseEvent::ContentBlockStop { index: 0 });

        state.handle_event(&SseEvent::MessageDelta {
            delta: SseMessageDelta {
                stop_reason: Some("end_turn".to_string()),
            },
            usage: SseOutputUsage {
                output_tokens: Some(3),
            },
        });

        let resp = state.into_response();
        let usage = resp.usage.as_ref().unwrap();

        // prompt_tokens = 500 + 200 + 3000 = 3700
        assert_eq!(usage.prompt_tokens, 3700);
        assert_eq!(usage.completion_tokens, 3);
        assert_eq!(usage.total_tokens, 3703);
    }

    #[test]
    fn stream_state_multiple_content_blocks() {
        let mut state = StreamState::default();

        state.handle_event(&SseEvent::MessageStart {
            message: SseMessageStartData {
                id: "msg_4".to_string(),
                model: "claude-sonnet-4-5".to_string(),
                role: "assistant".to_string(),
                usage: SseInputUsage::default(),
            },
        });

        // Block 0: text
        state.handle_event(&SseEvent::ContentBlockStart {
            index: 0,
            content_block: SseContentBlockStart::Text {
                text: String::new(),
            },
        });
        state.handle_event(&SseEvent::ContentBlockDelta {
            index: 0,
            delta: SseDelta::TextDelta {
                text: "First block. ".to_string(),
            },
        });
        state.handle_event(&SseEvent::ContentBlockStop { index: 0 });

        // Block 1: another text
        state.handle_event(&SseEvent::ContentBlockStart {
            index: 1,
            content_block: SseContentBlockStart::Text {
                text: String::new(),
            },
        });
        state.handle_event(&SseEvent::ContentBlockDelta {
            index: 1,
            delta: SseDelta::TextDelta {
                text: "Second block.".to_string(),
            },
        });
        state.handle_event(&SseEvent::ContentBlockStop { index: 1 });

        state.handle_event(&SseEvent::MessageDelta {
            delta: SseMessageDelta {
                stop_reason: Some("end_turn".to_string()),
            },
            usage: SseOutputUsage::default(),
        });

        let resp = state.into_response();
        assert_eq!(resp.content, Some("First block. Second block.".to_string()));
    }

    // ── parse_tool_arguments ────────────────────────────────────────

    #[test]
    fn parse_tool_arguments_valid_json() {
        let args = parse_tool_arguments("{\"key\": \"value\"}");
        assert_eq!(args.get("key").and_then(|v| v.as_str()), Some("value"));
    }

    #[test]
    fn parse_tool_arguments_invalid_json_fallbacks_to_raw() {
        let args = parse_tool_arguments("not json");
        assert_eq!(args.get("raw").and_then(|v| v.as_str()), Some("not json"));
    }
}
