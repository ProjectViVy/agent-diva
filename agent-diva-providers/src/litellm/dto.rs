use serde::de::Deserializer;
use serde::{Deserialize, Serialize};

use crate::base::Message;

/// LiteLLM API request format
#[derive(Debug, Serialize)]
pub(super) struct ChatCompletionRequest {
    pub(super) model: String,
    pub(super) messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) stream: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) reasoning_effort: Option<String>,
    pub(super) max_tokens: i32,
    pub(super) temperature: f64,
}

/// LiteLLM API response format
#[derive(Debug, Deserialize)]
pub(super) struct ChatCompletionResponse {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(super) choices: Vec<Choice>,
    #[serde(default)]
    pub(super) usage: Usage,
}

#[derive(Debug, Deserialize)]
pub(super) struct OpenAiErrorEnvelope {
    pub(super) error: Option<OpenAiErrorBody>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OpenAiErrorBody {
    pub(super) message: Option<String>,
    #[serde(rename = "type")]
    pub(super) error_type: Option<String>,
    pub(super) code: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Choice {
    pub(super) message: ResponseMessage,
    pub(super) finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ResponseMessage {
    #[serde(default)]
    pub(super) content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(super) tool_calls: Vec<ToolCall>,
    #[serde(default)]
    pub(super) reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ToolCall {
    pub(super) id: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub(super) call_type: String,
    pub(super) function: Function,
}

#[derive(Debug, Deserialize)]
pub(super) struct Function {
    pub(super) name: String,
    pub(super) arguments: String,
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct Usage {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(super) prompt_tokens: i64,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(super) completion_tokens: i64,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(super) total_tokens: i64,
}

#[derive(Debug, Deserialize)]
pub(super) struct StreamChunk {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(super) choices: Vec<StreamChoice>,
    #[serde(default)]
    pub(super) usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub(super) struct StreamChoice {
    #[serde(default)]
    pub(super) delta: StreamDelta,
    #[serde(default)]
    pub(super) finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct StreamDelta {
    #[serde(default)]
    pub(super) content: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub(super) tool_calls: Vec<StreamToolCall>,
    #[serde(default)]
    pub(super) reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct StreamToolCall {
    #[serde(default)]
    pub(super) index: usize,
    #[serde(default)]
    pub(super) id: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    #[allow(dead_code)]
    pub(super) call_type: Option<String>,
    #[serde(default)]
    pub(super) function: Option<StreamFunction>,
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct StreamFunction {
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) arguments: Option<String>,
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}
