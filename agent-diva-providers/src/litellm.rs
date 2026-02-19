//! LiteLLM HTTP client implementation

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

use crate::base::{
    LLMProvider, LLMResponse, LLMStreamEvent, Message, ProviderError, ProviderEventStream,
    ProviderResult, ToolCallRequest,
};
use crate::registry::{ProviderRegistry, ProviderSpec};

/// LiteLLM API request format
#[derive(Debug, Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_choice: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
    max_tokens: i32,
    temperature: f64,
}

/// LiteLLM API response format
#[derive(Debug, Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseMessage {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<ToolCall>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    call_type: String,
    function: Function,
}

#[derive(Debug, Deserialize)]
struct Function {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize, Default)]
struct Usage {
    #[serde(default)]
    prompt_tokens: i64,
    #[serde(default)]
    completion_tokens: i64,
    #[serde(default)]
    total_tokens: i64,
}

#[derive(Debug, Deserialize)]
struct StreamChunk {
    choices: Vec<StreamChoice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    #[serde(default)]
    delta: StreamDelta,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamDelta {
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<StreamToolCall>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamToolCall {
    #[serde(default)]
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    #[allow(dead_code)]
    call_type: Option<String>,
    #[serde(default)]
    function: Option<StreamFunction>,
}

#[derive(Debug, Default, Deserialize)]
struct StreamFunction {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct PartialToolCall {
    id: Option<String>,
    call_type: String,
    name: String,
    arguments: String,
}

/// LiteLLM provider client
pub struct LiteLLMClient {
    client: Client,
    api_base: String,
    api_key: Option<String>,
    default_model: String,
    extra_headers: HashMap<String, String>,
    registry: ProviderRegistry,
    gateway: Option<ProviderSpec>,
}

impl LiteLLMClient {
    /// Create a new LiteLLM client
    pub fn new(
        api_key: Option<String>,
        api_base: Option<String>,
        default_model: String,
        extra_headers: Option<HashMap<String, String>>,
        provider_name: Option<String>,
    ) -> Self {
        let registry = ProviderRegistry::new();
        let gateway = registry
            .find_gateway(
                provider_name.as_deref(),
                api_key.as_deref(),
                api_base.as_deref(),
            )
            .cloned();

        let api_base = api_base.and_then(|base| {
            if base.trim().is_empty() {
                None
            } else {
                Some(base)
            }
        });
        let api_base = api_base
            .or_else(|| {
                provider_name.as_deref().and_then(|name| {
                    registry.find_by_name(name).and_then(|spec| {
                        if spec.default_api_base.is_empty() {
                            None
                        } else {
                            Some(spec.default_api_base.clone())
                        }
                    })
                })
            })
            .unwrap_or_else(|| "http://localhost:4000".to_string());

        Self {
            client: Client::builder()
                .http1_only() // Force HTTP/1.1 to avoid issues with some local servers
                .build()
                .unwrap_or_else(|_| Client::new()),
            api_base,
            api_key,
            default_model,
            extra_headers: extra_headers.unwrap_or_default(),
            registry,
            gateway,
        }
    }

    /// Resolve model name by applying provider/gateway prefixes
    fn resolve_model(&self, model: &str) -> String {
        if let Some(gateway) = &self.gateway {
            // Gateway mode
            let mut resolved = model.to_string();
            if gateway.strip_model_prefix {
                if let Some(stripped) = model.split('/').next_back() {
                    resolved = stripped.to_string();
                }
            }
            if !gateway.litellm_prefix.is_empty()
                && !resolved.starts_with(&format!("{}/", gateway.litellm_prefix))
            {
                resolved = format!("{}/{}", gateway.litellm_prefix, resolved);
            }
            debug!("Resolved model (gateway): {} -> {}", model, resolved);
            return resolved;
        }

        // Standard mode: auto-prefix for known providers
        if let Some(spec) = self.registry.find_by_model(model) {
            if !spec.litellm_prefix.is_empty() {
                let has_skip_prefix = spec
                    .skip_prefixes
                    .iter()
                    .any(|prefix| model.starts_with(prefix));
                if !has_skip_prefix {
                    let resolved = format!("{}/{}", spec.litellm_prefix, model);
                    debug!("Resolved model (standard): {} -> {}", model, resolved);
                    return resolved;
                }
            }
        }

        debug!("Model unchanged: {}", model);
        model.to_string()
    }

    /// Apply model-specific parameter overrides from the registry
    fn apply_model_overrides(&self, model: &str, kwargs: &mut HashMap<String, serde_json::Value>) {
        let model_lower = model.to_lowercase();
        if let Some(spec) = self.registry.find_by_model(model) {
            for (pattern, overrides) in &spec.model_overrides {
                if model_lower.contains(pattern) {
                    for (key, value) in overrides {
                        kwargs.insert(key.clone(), value.clone());
                    }
                    debug!("Applied model overrides for {}: {:?}", pattern, overrides);
                    return;
                }
            }
        }
    }

    /// Parse LiteLLM response into our standard format
    fn parse_response(&self, response: ChatCompletionResponse) -> ProviderResult<LLMResponse> {
        let choice = response
            .choices
            .first()
            .ok_or_else(|| ProviderError::InvalidResponse("No choices in response".to_string()))?;

        let mut tool_calls = Vec::new();
        for tc in &choice.message.tool_calls {
            // Parse arguments from JSON string
            let arguments = match serde_json::from_str::<HashMap<String, serde_json::Value>>(
                &tc.function.arguments,
            ) {
                Ok(args) => args,
                Err(e) => {
                    warn!("Failed to parse tool call arguments: {}", e);
                    let mut map = HashMap::new();
                    map.insert(
                        "raw".to_string(),
                        serde_json::Value::String(tc.function.arguments.clone()),
                    );
                    map
                }
            };

            tool_calls.push(ToolCallRequest {
                id: tc.id.clone(),
                call_type: tc.call_type.clone(),
                name: tc.function.name.clone(),
                arguments,
            });
        }

        let mut usage = HashMap::new();
        usage.insert("prompt_tokens".to_string(), response.usage.prompt_tokens);
        usage.insert(
            "completion_tokens".to_string(),
            response.usage.completion_tokens,
        );
        usage.insert("total_tokens".to_string(), response.usage.total_tokens);

        Ok(LLMResponse {
            content: choice.message.content.clone(),
            tool_calls,
            finish_reason: choice
                .finish_reason
                .clone()
                .unwrap_or_else(|| "stop".to_string()),
            usage,
            reasoning_content: choice.message.reasoning_content.clone(),
        })
    }

    fn build_request(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        resolved_model: String,
        max_tokens: i32,
        temperature: f64,
        stream: bool,
    ) -> ChatCompletionRequest {
        let mut request = ChatCompletionRequest {
            model: resolved_model,
            messages,
            tools: None,
            tool_choice: None,
            stream: if stream { Some(true) } else { None },
            max_tokens,
            temperature,
        };

        if let Some(tools_list) = tools {
            request.tools = Some(tools_list);
            request.tool_choice = Some("auto".to_string());
        }

        request
    }

    fn apply_headers(&self, mut req_builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(api_key) = &self.api_key {
            req_builder = req_builder.header("Authorization", format!("Bearer {}", api_key));
        }

        for (key, value) in &self.extra_headers {
            req_builder = req_builder.header(key, value);
        }

        req_builder
    }

    fn finalize_partial_response(
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

            let arguments =
                serde_json::from_str::<HashMap<String, serde_json::Value>>(&call.arguments)
                    .unwrap_or_else(|_| {
                        let mut map = HashMap::new();
                        map.insert(
                            "raw".to_string(),
                            serde_json::Value::String(call.arguments.clone()),
                        );
                        map
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
}

#[async_trait]
impl LLMProvider for LiteLLMClient {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let model = model.unwrap_or_else(|| self.default_model.clone());
        let resolved_model = self.resolve_model(&model);

        let mut kwargs = HashMap::new();
        self.apply_model_overrides(&model, &mut kwargs);

        // Build request
        let request = self.build_request(
            messages,
            tools,
            resolved_model.clone(),
            max_tokens,
            kwargs
                .get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(temperature),
            false,
        );

        debug!(
            "Sending chat request to {} with model {}",
            self.api_base, resolved_model
        );

        // Build HTTP request
        let url = format!("{}/chat/completions", self.api_base);
        let req_builder = self.apply_headers(self.client.post(&url).json(&request));

        // Send request
        let response = req_builder.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let response_data: ChatCompletionResponse = response.json().await?;
        self.parse_response(response_data)
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let model = model.unwrap_or_else(|| self.default_model.clone());
        let resolved_model = self.resolve_model(&model);

        let mut kwargs = HashMap::new();
        self.apply_model_overrides(&model, &mut kwargs);

        let request = self.build_request(
            messages,
            tools,
            resolved_model.clone(),
            max_tokens,
            kwargs
                .get("temperature")
                .and_then(|v| v.as_f64())
                .unwrap_or(temperature),
            true,
        );

        debug!(
            "Sending streaming chat request to {} with model {}",
            self.api_base, resolved_model
        );

        let url = format!("{}/chat/completions", self.api_base);
        let req_builder = self.apply_headers(self.client.post(&url).json(&request));
        let response = req_builder.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ProviderError::ApiError(format!(
                "HTTP {}: {}",
                status, error_text
            )));
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut response = response;
            let mut buffer = String::new();
            let mut content = String::new();
            let mut reasoning_content = String::new();
            let mut finish_reason: Option<String> = None;
            let mut usage: Option<Usage> = None;
            let mut partial_calls: Vec<PartialToolCall> = Vec::new();

            loop {
                let chunk = match response.chunk().await {
                    Ok(Some(bytes)) => bytes,
                    Ok(None) => break,
                    Err(err) => {
                        let _ = tx.send(Err(ProviderError::HttpError(err)));
                        return;
                    }
                };

                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                for payload in Self::parse_sse_events(&mut buffer) {
                    if payload == "[DONE]" {
                        let final_response = Self::finalize_partial_response(
                            content.clone(),
                            reasoning_content.clone(),
                            &partial_calls,
                            finish_reason.clone(),
                            usage.take(),
                        );
                        let _ = tx.send(Ok(LLMStreamEvent::Completed(final_response)));
                        return;
                    }

                    let parsed = match serde_json::from_str::<StreamChunk>(&payload) {
                        Ok(chunk) => chunk,
                        Err(err) => {
                            let _ = tx.send(Err(ProviderError::JsonError(err)));
                            return;
                        }
                    };

                    if parsed.choices.is_empty() {
                        usage = parsed.usage;
                        continue;
                    }

                    if let Some(choice) = parsed.choices.first() {
                        if let Some(reason) = &choice.finish_reason {
                            finish_reason = Some(reason.clone());
                        }
                        let delta = &choice.delta;
                        if let Some(delta_text) = &delta.content {
                            content.push_str(delta_text);
                            let _ = tx.send(Ok(LLMStreamEvent::TextDelta(delta_text.clone())));
                        }
                        if let Some(reasoning) = &delta.reasoning_content {
                            reasoning_content.push_str(reasoning);
                        }

                        for tool_call in &delta.tool_calls {
                            let index = tool_call.index;
                            if partial_calls.len() <= index {
                                partial_calls.resize_with(index + 1, PartialToolCall::default);
                            }
                            let entry = &mut partial_calls[index];
                            if let Some(id) = &tool_call.id {
                                entry.id = Some(id.clone());
                            }
                            if let Some(call_type) = &tool_call.call_type {
                                entry.call_type = call_type.clone();
                            }
                            if let Some(function) = &tool_call.function {
                                if let Some(name) = &function.name {
                                    entry.name.push_str(name);
                                }
                                if let Some(arguments_delta) = &function.arguments {
                                    entry.arguments.push_str(arguments_delta);
                                    let _ = tx.send(Ok(LLMStreamEvent::ToolCallDelta {
                                        index,
                                        id: entry.id.clone(),
                                        name: if entry.name.is_empty() {
                                            None
                                        } else {
                                            Some(entry.name.clone())
                                        },
                                        arguments_delta: Some(arguments_delta.clone()),
                                    }));
                                }
                            }
                        }
                    }
                }
            }

            let final_response = Self::finalize_partial_response(
                content,
                reasoning_content,
                &partial_calls,
                finish_reason,
                usage,
            );
            let _ = tx.send(Ok(LLMStreamEvent::Completed(final_response)));
        });

        Ok(Box::pin(futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })))
    }

    fn get_default_model(&self) -> String {
        self.default_model.clone()
    }
}

impl Default for LiteLLMClient {
    fn default() -> Self {
        Self::new(
            None,
            None,
            "anthropic/claude-opus-4-5".to_string(),
            None,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_model() {
        let client = LiteLLMClient::new(None, None, "claude-3-opus".to_string(), None, None);

        // DeepSeek should get prefixed
        assert_eq!(
            client.resolve_model("deepseek-chat"),
            "deepseek/deepseek-chat"
        );

        // Claude should not get prefixed (LiteLLM knows it)
        assert_eq!(client.resolve_model("claude-3-opus"), "claude-3-opus");

        // Qwen should get prefixed
        assert_eq!(client.resolve_model("qwen-max"), "dashscope/qwen-max");
    }

    #[test]
    fn test_gateway_model_resolution() {
        // OpenRouter gateway
        let client = LiteLLMClient::new(
            Some("sk-or-test".to_string()),
            Some("https://openrouter.ai/api/v1".to_string()),
            "claude-3-opus".to_string(),
            None,
            None,
        );
        assert_eq!(
            client.resolve_model("claude-3-opus"),
            "openrouter/claude-3-opus"
        );

        // AiHubMix gateway with strip_model_prefix
        let client = LiteLLMClient::new(
            Some("test-key".to_string()),
            Some("https://aihubmix.com/v1".to_string()),
            "claude-3-opus".to_string(),
            None,
            None,
        );
        // anthropic/claude-3-opus -> claude-3-opus -> openai/claude-3-opus
        assert_eq!(
            client.resolve_model("anthropic/claude-3-opus"),
            "openai/claude-3-opus"
        );
    }

    #[test]
    fn test_parse_sse_events() {
        let mut buffer =
            "data: {\"a\":1}\n\ndata: {\"b\":2}\n\ndata: [DONE]\n\ntrailing".to_string();
        let events = LiteLLMClient::parse_sse_events(&mut buffer);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0], "{\"a\":1}");
        assert_eq!(events[1], "{\"b\":2}");
        assert_eq!(events[2], "[DONE]");
        assert_eq!(buffer, "trailing");
    }
}
