use std::collections::HashMap;

use crate::base::{LLMResponse, ToolCallRequest};

use super::dto::Usage;

#[derive(Debug, Default, Clone)]
pub(super) struct PartialToolCall {
    pub(super) id: Option<String>,
    pub(super) call_type: String,
    pub(super) name: String,
    pub(super) arguments: String,
}

pub(super) fn finalize_partial_response(
    content: String,
    reasoning_content: String,
    partial_calls: &[PartialToolCall],
    finish_reason: Option<String>,
    usage: Option<Usage>,
) -> LLMResponse {
    let mut tool_calls = Vec::new();
    for (i, call) in partial_calls.iter().enumerate() {
        let id = call
            .id
            .clone()
            .unwrap_or_else(|| format!("stream_tool_call_{}", i));
        let call_type = if call.call_type.is_empty() {
            "function".to_string()
        } else {
            call.call_type.clone()
        };

        let arguments = serde_json::from_str::<HashMap<String, serde_json::Value>>(&call.arguments)
            .unwrap_or_else(|_| {
                // Try unwrapping double-encoded JSON string
                if let Ok(inner) = serde_json::from_str::<String>(&call.arguments) {
                    serde_json::from_str::<HashMap<String, serde_json::Value>>(&inner)
                        .unwrap_or_else(|_| {
                            HashMap::from([("raw".into(), serde_json::Value::String(inner))])
                        })
                } else {
                    HashMap::from([(
                        "raw".into(),
                        serde_json::Value::String(call.arguments.clone()),
                    )])
                }
            });

        tool_calls.push(ToolCallRequest {
            id,
            call_type,
            name: call.name.clone(),
            arguments,
        });
    }

    let mut usage_map = HashMap::new();
    if let Some(usage) = usage {
        usage_map.insert("prompt_tokens".to_string(), usage.prompt_tokens);
        usage_map.insert("completion_tokens".to_string(), usage.completion_tokens);
        usage_map.insert("total_tokens".to_string(), usage.total_tokens);
    }

    LLMResponse {
        content: if content.is_empty() {
            None
        } else {
            Some(content)
        },
        tool_calls,
        finish_reason: finish_reason.unwrap_or_else(|| "stop".to_string()),
        usage: usage_map,
        reasoning_content: if reasoning_content.is_empty() {
            None
        } else {
            Some(reasoning_content)
        },
    }
}

pub(super) fn parse_sse_events(buffer: &mut String) -> Vec<String> {
    let mut events = Vec::new();
    while let Some(pos) = buffer.find("\n\n") {
        let raw = buffer[..pos].to_string();
        buffer.drain(..pos + 2);

        let mut data_lines = Vec::new();
        for line in raw.lines() {
            if let Some(rest) = line.strip_prefix("data:") {
                data_lines.push(rest.trim().to_string());
            }
        }

        if !data_lines.is_empty() {
            events.push(data_lines.join("\n"));
        }
    }

    events
}
