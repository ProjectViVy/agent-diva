//! Ollama local provider implementation
//!
//! Supports direct connection to Ollama endpoints with features including:
//! - Non-streaming and streaming chat
//! - Tool/function calling
//! - Reasoning models with thinking

use async_trait::async_trait;
use serde::Deserializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error};

use crate::base::{
    LLMProvider, LLMResponse, LLMStreamEvent, Message, ProviderError, ProviderEventStream,
    ProviderResult, ToolCallRequest,
};
use crate::http_util::build_api_http_client;
use tokio::sync::mpsc;

/// Ollama provider for local model inference
pub struct OllamaProvider {
    base_url: String,
    default_model: String,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<ChatOptions>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "stream_options")]
    stream_options: Option<StreamOptions>,
}

#[derive(Debug, Serialize)]
struct StreamOptions {
    include_usage: bool,
}

#[derive(Debug, Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct ChatOptions {
    temperature: f64,
}

// ─── Streaming Response Structures ─────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct OllamaStreamChunk {
    #[serde(default)]
    message: OllamaStreamMessage,
    #[serde(default)]
    done: bool,
    #[serde(default)]
    prompt_eval_count: Option<i64>,
    #[serde(default)]
    eval_count: Option<i64>,
}

#[derive(Debug, Deserialize, Default)]
struct OllamaStreamMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    thinking: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    tool_calls: Vec<OllamaStreamToolCall>,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamToolCall {
    #[serde(default)]
    id: Option<String>,
    function: OllamaStreamFunction,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamFunction {
    name: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

// ─── Non-streaming Response Structures ──────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ChatResponse {
    message: ResponseMessage,
    #[serde(default)]
    prompt_eval_count: Option<i64>,
    #[serde(default)]
    eval_count: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct OllamaToolCall {
    #[serde(default)]
    id: Option<String>,
    function: OllamaFunction,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    thinking: Option<String>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    tool_calls: Vec<OllamaToolCall>,
}

#[derive(Debug, Deserialize)]
struct OllamaFunction {
    name: String,
    #[serde(default)]
    arguments: serde_json::Value,
}

// ─── Helper Functions ───────────────────────────────────────────────────────────

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

/// Generate a short random hex id (no uuid crate needed)
fn rand_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:016x}", t)
}

impl OllamaProvider {
    /// Normalize the base URL for consistency
    fn normalize_base_url(raw_url: &str) -> String {
        let trimmed = raw_url.trim().trim_end_matches('/');
        if trimmed.is_empty() {
            String::new()
        } else {
            trimmed
                .strip_suffix("/api")
                .unwrap_or(trimmed)
                .trim_end_matches('/')
                .to_string()
        }
    }

    /// Create a new Ollama provider
    pub fn new(base_url: Option<&str>, default_model: String) -> Self {
        Self {
            base_url: Self::normalize_base_url(base_url.unwrap_or("http://localhost:11434")),
            default_model,
        }
    }

    /// Build the chat completion URL
    fn build_chat_url(&self) -> String {
        format!("{}/api/chat", self.base_url)
    }

    /// Extract token usage from Ollama's eval counts.
    ///
    /// Ollama returns `prompt_eval_count` and `eval_count` instead of the
    /// standard `prompt_tokens` / `completion_tokens`. This method normalizes
    /// them to the standard LLMResponse usage HashMap format.
    fn extract_usage(
        prompt_eval_count: Option<i64>,
        eval_count: Option<i64>,
    ) -> HashMap<String, i64> {
        let mut usage = HashMap::new();
        if let Some(prompt_eval) = prompt_eval_count {
            usage.insert("prompt_tokens".to_string(), prompt_eval);
        }
        if let Some(eval) = eval_count {
            usage.insert("completion_tokens".to_string(), eval);
        }
        if prompt_eval_count.is_some() || eval_count.is_some() {
            usage.insert(
                "total_tokens".to_string(),
                prompt_eval_count.unwrap_or(0) + eval_count.unwrap_or(0),
            );
        }
        usage
    }

    /// Convert internal Message format to Ollama's native format
    fn convert_messages(messages: &[Message]) -> Vec<OllamaMessage> {
        messages
            .iter()
            .map(|msg| {
                // Handle assistant messages with tool_calls
                if msg.role == "assistant" {
                    // For now, just use the content field
                    // Tool calls are handled separately in the request
                    return OllamaMessage {
                        role: msg.role.clone(),
                        content: msg.content.to_text_lossy(),
                    };
                }

                // Handle tool messages
                if msg.role == "tool" {
                    // Tool results go in the content field
                    return OllamaMessage {
                        role: "tool".to_string(),
                        content: msg.content.to_text_lossy(),
                    };
                }

                // User and system messages pass through
                OllamaMessage {
                    role: msg.role.clone(),
                    content: msg.content.to_text_lossy(),
                }
            })
            .collect()
    }

    /// Parse tool arguments safely
    fn parse_tool_arguments(arguments: &serde_json::Value) -> HashMap<String, serde_json::Value> {
        if let serde_json::Value::Object(map) = arguments {
            map.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        } else {
            HashMap::new()
        }
    }

    /// Parse SSE event payload
    fn parse_sse_events(buffer: &mut String) -> Vec<String> {
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

    /// Convert Ollama tool call to standard ToolCallRequest format
    fn convert_tool_call(tool_call: &OllamaStreamToolCall) -> ToolCallRequest {
        let id = tool_call
            .id
            .clone()
            .unwrap_or_else(|| format!("call_{}", rand_id()));
        let name = tool_call.function.name.clone();
        let arguments = tool_call.function.arguments.clone();

        ToolCallRequest {
            id,
            call_type: "function".to_string(),
            name,
            arguments: Self::parse_tool_arguments(&arguments),
        }
    }

    /// Convert non-streaming Ollama tool call to standard ToolCallRequest format
    fn convert_stream_tool_call(tool_call: &OllamaToolCall) -> ToolCallRequest {
        let id = tool_call
            .id
            .clone()
            .unwrap_or_else(|| format!("call_{}", rand_id()));
        let name = tool_call.function.name.clone();
        let arguments = tool_call.function.arguments.clone();

        ToolCallRequest {
            id,
            call_type: "function".to_string(),
            name,
            arguments: Self::parse_tool_arguments(&arguments),
        }
    }
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        _max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let resolved_model = model.unwrap_or_else(|| self.default_model.clone());
        let url = self.build_chat_url();

        let client = build_api_http_client(&self.base_url, Duration::from_secs(300))
            .map_err(ProviderError::HttpError)?;

        let ollama_messages = Self::convert_messages(&messages);

        // Build request with tools if provided
        let request = ChatRequest {
            model: resolved_model.clone(),
            messages: ollama_messages,
            stream: false,
            options: Some(ChatOptions { temperature }),
            stream_options: None,
        };

        // Add tools to request if provided

        debug!(
            "Sending chat request to Ollama: model={}, url={}, tools={}",
            resolved_model,
            url,
            tools.as_ref().map_or(0, |t| t.len())
        );

        let mut post_request = client.post(&url).json(&request);

        // Manually add tools field if present
        if let Some(tools_list) = &tools {
            let tools_json = serde_json::to_value(&request)
                .map_err(|e| ProviderError::InvalidResponse(format!("Serialize error: {}", e)))?;

            let mut tools_map = tools_json
                .as_object()
                .ok_or_else(|| {
                    ProviderError::InvalidResponse("Request is not an object".to_string())
                })?
                .clone();

            tools_map.insert(
                "tools".to_string(),
                serde_json::to_value(tools_list).map_err(|e| {
                    ProviderError::InvalidResponse(format!("Tools serialize error: {}", e))
                })?,
            );

            post_request = client.post(&url).json(&tools_map);
        }

        let response = post_request.send().await.map_err(|e| {
            error!("Ollama HTTP error: {}", e);
            ProviderError::HttpError(e)
        })?;

        let chat_response: ChatResponse = response.json::<ChatResponse>().await.map_err(|e| {
            error!("Failed to parse Ollama response: {}", e);
            ProviderError::InvalidResponse(format!("Failed to parse response: {}", e))
        })?;

        // Handle tool calls if present
        let mut tool_calls = Vec::new();
        for tc in &chat_response.message.tool_calls {
            let tool_call = Self::convert_stream_tool_call(tc);
            tool_calls.push(tool_call);
        }

        let content = if chat_response.message.content.trim().is_empty() {
            if tool_calls.is_empty() {
                // No content and no tool calls - error case
                if let Some(thinking) = &chat_response.message.thinking {
                    if !thinking.trim().is_empty() {
                        format!(
                            "I was thinking: {}... but couldn't complete the response.",
                            thinking.chars().take(100).collect::<String>()
                        )
                    } else {
                        "I couldn't generate a response. Please try again.".to_string()
                    }
                } else {
                    "I couldn't generate a response. Please try again.".to_string()
                }
            } else {
                // Tool calls present - content may be empty, that's OK
                String::new()
            }
        } else {
            chat_response.message.content
        };

        Ok(LLMResponse {
            content: if content.is_empty() {
                None
            } else {
                Some(content)
            },
            tool_calls,
            finish_reason: "stop".to_string(),
            usage: Self::extract_usage(chat_response.prompt_eval_count, chat_response.eval_count),
            reasoning_content: chat_response.message.thinking,
        })
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        _max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let resolved_model = model.unwrap_or_else(|| self.default_model.clone());
        let url = self.build_chat_url();

        let client = build_api_http_client(&self.base_url, Duration::from_secs(300))
            .map_err(ProviderError::HttpError)?;

        let ollama_messages = Self::convert_messages(&messages);
        let request = ChatRequest {
            model: resolved_model.clone(),
            messages: ollama_messages,
            stream: true,
            options: Some(ChatOptions { temperature }),
            stream_options: Some(StreamOptions { include_usage: true }),
        };

        debug!(
            "Sending streaming chat request to Ollama: model={}, url={}",
            resolved_model, url
        );

        let mut response = client.post(&url).json(&request).send().await.map_err(|e| {
            error!("Ollama HTTP error: {}", e);
            ProviderError::HttpError(e)
        })?;

        let (tx, rx) = mpsc::channel::<ProviderResult<LLMStreamEvent>>(100);

        tokio::spawn(async move {
            let mut buffer = String::new();
            let mut content = String::new();
            let mut reasoning_content = String::new();
            let mut tool_calls: Vec<ToolCallRequest> = Vec::new();

            loop {
                let chunk = match response.chunk().await {
                    Ok(Some(bytes)) => bytes,
                    Ok(None) => break,
                    Err(err) => {
                        error!("Stream error: {}", err);
                        let _ = tx.send(Err(ProviderError::HttpError(err))).await;
                        return;
                    }
                };

                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                for payload in Self::parse_sse_events(&mut buffer) {
                    if payload == "[DONE]" {
                        debug!("Stream received [DONE]");
                        continue;
                    }

                    match serde_json::from_str::<OllamaStreamChunk>(&payload) {
                        Ok(chunk) => {
                            // Handle content delta
                            if !chunk.message.content.is_empty() {
                                content.push_str(&chunk.message.content);
                                let _ = tx
                                    .send(Ok(LLMStreamEvent::TextDelta(chunk.message.content)))
                                    .await;
                            }

                            // Handle thinking delta
                            if let Some(thinking) = chunk.message.thinking {
                                if !thinking.is_empty() {
                                    reasoning_content.push_str(&thinking);
                                    let _ =
                                        tx.send(Ok(LLMStreamEvent::ReasoningDelta(thinking))).await;
                                }
                            }

                            // Handle tool calls
                            for tc in &chunk.message.tool_calls {
                                let tool_call = Self::convert_tool_call(tc);
                                tool_calls.push(tool_call.clone());
                                let _ = tx
                                    .send(Ok(LLMStreamEvent::ToolCallDelta {
                                        index: tool_calls.len() - 1,
                                        id: Some(tool_call.id),
                                        name: Some(tool_call.name),
                                        arguments_delta: None,
                                    }))
                                    .await;
                            }

                            if chunk.done {
                                // Extract usage from the final done chunk
                                let final_usage =
                                    Self::extract_usage(chunk.prompt_eval_count, chunk.eval_count);
                                // Send completed event with usage data
                                let _ = tx
                                    .send(Ok(LLMStreamEvent::Completed(LLMResponse {
                                        content: if content.is_empty() {
                                            None
                                        } else {
                                            Some(content.clone())
                                        },
                                        tool_calls: tool_calls.clone(),
                                        finish_reason: "stop".to_string(),
                                        usage: final_usage,
                                        reasoning_content: if reasoning_content.is_empty() {
                                            None
                                        } else {
                                            Some(reasoning_content.clone())
                                        },
                                    })))
                                    .await;
                                return;
                            }
                        }
                        Err(e) => {
                            debug!("Failed to parse SSE chunk: {} - payload: {}", e, payload);
                        }
                    }
                }
            }

            // Send completed response
            let final_response = LLMResponse {
                content: if content.is_empty() {
                    None
                } else {
                    Some(content)
                },
                tool_calls,
                finish_reason: "stop".to_string(),
                usage: Default::default(),
                reasoning_content: if reasoning_content.is_empty() {
                    None
                } else {
                    Some(reasoning_content)
                },
            };

            let _ = tx.send(Ok(LLMStreamEvent::Completed(final_response))).await;
        });

        Ok(Box::pin(futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })))
    }

    fn get_default_model(&self) -> String {
        self.default_model.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_usage_with_both_counts() {
        let usage = OllamaProvider::extract_usage(Some(42), Some(21));
        assert_eq!(usage.get("prompt_tokens"), Some(&42));
        assert_eq!(usage.get("completion_tokens"), Some(&21));
        assert_eq!(usage.get("total_tokens"), Some(&63));
    }

    #[test]
    fn test_extract_usage_prompt_only() {
        let usage = OllamaProvider::extract_usage(Some(100), None);
        assert_eq!(usage.get("prompt_tokens"), Some(&100));
        assert_eq!(usage.get("completion_tokens"), None);
        assert_eq!(usage.get("total_tokens"), Some(&100));
    }

    #[test]
    fn test_extract_usage_completion_only() {
        let usage = OllamaProvider::extract_usage(None, Some(50));
        assert_eq!(usage.get("prompt_tokens"), None);
        assert_eq!(usage.get("completion_tokens"), Some(&50));
        assert_eq!(usage.get("total_tokens"), Some(&50));
    }

    #[test]
    fn test_extract_usage_both_none() {
        let usage = OllamaProvider::extract_usage(None, None);
        assert!(usage.is_empty());
    }

    #[test]
    fn test_extract_usage_zero_counts() {
        let usage = OllamaProvider::extract_usage(Some(0), Some(0));
        assert_eq!(usage.get("prompt_tokens"), Some(&0));
        assert_eq!(usage.get("completion_tokens"), Some(&0));
        assert_eq!(usage.get("total_tokens"), Some(&0));
    }

    #[test]
    fn test_chat_response_deserialize_with_usage() {
        let json = r#"{
            "message": {"content": "Hello!", "thinking": null, "tool_calls": []},
            "prompt_eval_count": 42,
            "eval_count": 21
        }"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.message.content, "Hello!");
        assert_eq!(resp.prompt_eval_count, Some(42));
        assert_eq!(resp.eval_count, Some(21));
    }

    #[test]
    fn test_chat_response_deserialize_without_usage() {
        let json = r#"{
            "message": {"content": "Hi", "thinking": null, "tool_calls": []}
        }"#;
        let resp: ChatResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.message.content, "Hi");
        assert_eq!(resp.prompt_eval_count, None);
        assert_eq!(resp.eval_count, None);
    }

    #[test]
    fn test_stream_chunk_deserialize_with_usage() {
        let json = r#"{
            "message": {"content": "", "thinking": null, "tool_calls": []},
            "done": true,
            "prompt_eval_count": 100,
            "eval_count": 50
        }"#;
        let chunk: OllamaStreamChunk = serde_json::from_str(json).unwrap();
        assert!(chunk.done);
        assert_eq!(chunk.prompt_eval_count, Some(100));
        assert_eq!(chunk.eval_count, Some(50));
    }

    #[test]
    fn test_chat_request_streaming_includes_stream_options() {
        let request = ChatRequest {
            model: "llama3".to_string(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: true,
            options: Some(ChatOptions { temperature: 0.7 }),
            stream_options: Some(StreamOptions { include_usage: true }),
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "llama3");
        assert_eq!(json["stream"], true);
        assert!(json["stream_options"].is_object());
        assert_eq!(json["stream_options"]["include_usage"], true);
    }

    #[test]
    fn test_chat_request_non_streaming_omits_stream_options() {
        let request = ChatRequest {
            model: "llama3".to_string(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
            }],
            stream: false,
            options: None,
            stream_options: None,
        };
        let json = serde_json::to_value(&request).unwrap();
        assert_eq!(json["model"], "llama3");
        assert_eq!(json["stream"], false);
        assert!(!json.as_object().unwrap().contains_key("stream_options"));
        assert!(!json.as_object().unwrap().contains_key("options"));
    }
}
