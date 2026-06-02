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
                        content: msg.content.clone(),
                    };
                }

                // Handle tool messages
                if msg.role == "tool" {
                    // Tool results go in the content field
                    return OllamaMessage {
                        role: "tool".to_string(),
                        content: msg.content.clone(),
                    };
                }

                // User and system messages pass through
                OllamaMessage {
                    role: msg.role.clone(),
                    content: msg.content.clone(),
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
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
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
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
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
            usage: Default::default(),
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
                                debug!("Stream chunk marked as done");
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
