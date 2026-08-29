//! Native Anthropic Messages API client.

use async_trait::async_trait;
use futures::stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

use crate::base::{
    LLMProvider, LLMResponse, LLMStreamEvent, Message, MessageContent, MessageContentPart,
    ProviderError, ProviderEventStream, ProviderResult, StreamingUtf8Decoder, ToolCallRequest,
};
use crate::http_util::build_api_http_client;
use crate::retry;

const ANTHROPIC_VERSION: &str = "2023-06-01";
const STREAM_IDLE_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: i32,
    temperature: f64,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<Vec<AnthropicSystemBlock>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

#[derive(Debug, Serialize)]
struct AnthropicSystemBlock {
    #[serde(rename = "type")]
    block_type: &'static str,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
enum AnthropicContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: String,
    },
}

#[derive(Debug, Serialize)]
struct AnthropicTool {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    cache_control: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    #[serde(default)]
    content: Vec<AnthropicResponseBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicResponseBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking { thinking: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: Value,
    },
    #[serde(other)]
    Other,
}

#[derive(Debug, Default, Clone, Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: i64,
    #[serde(default)]
    output_tokens: i64,
    #[serde(default)]
    cache_creation_input_tokens: i64,
    #[serde(default)]
    cache_read_input_tokens: i64,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    index: Option<usize>,
    #[serde(default)]
    content_block: Option<AnthropicStreamBlock>,
    #[serde(default)]
    delta: Option<AnthropicStreamDelta>,
    #[serde(default)]
    message: Option<AnthropicStreamMessage>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamBlock {
    #[serde(rename = "tool_use")]
    ToolUse { id: String, name: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum AnthropicStreamDelta {
    #[serde(rename = "text_delta")]
    Text { text: String },
    #[serde(rename = "thinking_delta")]
    Thinking { thinking: String },
    #[serde(rename = "input_json_delta")]
    InputJson { partial_json: String },
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamMessage {
    #[serde(default)]
    usage: Option<AnthropicUsage>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Debug, Default, Clone)]
struct StreamingToolUse {
    id: String,
    name: String,
    input_json: String,
}

pub struct AnthropicClient {
    client: Client,
    api_base: String,
    api_key: Option<String>,
    default_model: String,
    extra_headers: HashMap<String, String>,
    provider_name: Option<String>,
    /// Snapshot by each request to notify retry progress; set per call by the agent.
    retry_listener: std::sync::Mutex<Option<crate::retry::RetryListener>>,
    final_wire_cache_listener: std::sync::Mutex<Option<crate::final_wire::FinalWireCacheListener>>,
}

impl AnthropicClient {
    pub fn new(
        api_key: Option<String>,
        api_base: Option<String>,
        default_model: String,
        extra_headers: Option<HashMap<String, String>>,
        provider_name: Option<String>,
    ) -> Self {
        let api_base = api_base
            .map(|base| base.trim().trim_end_matches('/').to_string())
            .filter(|base| !base.is_empty())
            .unwrap_or_else(|| "https://api.anthropic.com".to_string());
        Self {
            client: build_api_http_client(&api_base, Duration::from_secs(300))
                .unwrap_or_else(|_| Client::new()),
            api_base,
            api_key,
            default_model,
            extra_headers: extra_headers.unwrap_or_default(),
            provider_name,
            retry_listener: std::sync::Mutex::new(None),
            final_wire_cache_listener: std::sync::Mutex::new(None),
        }
    }

    fn provider_name(&self) -> &str {
        self.provider_name.as_deref().unwrap_or("anthropic")
    }

    fn build_request(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Value>>,
        model: String,
        max_tokens: i32,
        temperature: f64,
        stream: bool,
    ) -> ProviderResult<AnthropicRequest> {
        let (system, messages) = convert_messages(messages)?;
        Ok(AnthropicRequest {
            model,
            max_tokens,
            temperature,
            messages,
            system: system.map(|sections| {
                sections
                    .into_iter()
                    .enumerate()
                    .map(|(index, text)| AnthropicSystemBlock {
                        block_type: "text",
                        text,
                        cache_control: (index == 0).then(|| json!({"type": "ephemeral"})),
                    })
                    .collect()
            }),
            tools: convert_tools(tools)?,
            stream: stream.then_some(true),
        })
    }

    fn require_api_key(&self) -> ProviderResult<String> {
        self.api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(ToString::to_string)
            .ok_or_else(|| ProviderError::ConfigError("Anthropic API key is required".to_string()))
    }

    fn apply_headers(
        &self,
        request: reqwest::RequestBuilder,
        api_key: &str,
    ) -> reqwest::RequestBuilder {
        let mut request = request
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json");
        for (key, value) in &self.extra_headers {
            request = request.header(key, value);
        }
        request
    }

    fn parse_response(response: AnthropicResponse) -> LLMResponse {
        let mut text = String::new();
        let mut reasoning = String::new();
        let mut tool_calls = Vec::new();
        for block in response.content {
            match block {
                AnthropicResponseBlock::Text { text: value } => text.push_str(&value),
                AnthropicResponseBlock::Thinking { thinking } => reasoning.push_str(&thinking),
                AnthropicResponseBlock::ToolUse { id, name, input } => {
                    tool_calls.push(ToolCallRequest {
                        id,
                        call_type: "function".to_string(),
                        name,
                        arguments: value_to_arguments(input),
                    });
                }
                AnthropicResponseBlock::Other => {}
            }
        }
        LLMResponse {
            content: (!text.is_empty()).then_some(text),
            tool_calls,
            finish_reason: response.stop_reason.unwrap_or_else(|| "stop".to_string()),
            usage: usage_map(response.usage),
            reasoning_content: (!reasoning.is_empty()).then_some(reasoning),
        }
    }

    fn parse_sse_events(buffer: &mut String) -> Vec<String> {
        let mut events = Vec::new();
        while let Some(idx) = buffer.find("\n\n") {
            let raw = buffer[..idx].to_string();
            *buffer = buffer[idx + 2..].to_string();
            let data = raw
                .lines()
                .filter_map(|line| line.strip_prefix("data:"))
                .map(str::trim)
                .collect::<Vec<_>>()
                .join("\n");
            if !data.is_empty() {
                events.push(data);
            }
        }
        events
    }
}

#[async_trait]
impl LLMProvider for AnthropicClient {
    fn set_retry_listener(&self, listener: Option<crate::retry::RetryListener>) {
        *self.retry_listener.lock().unwrap() = listener;
    }

    fn set_final_wire_cache_listener(
        &self,
        listener: Option<crate::final_wire::FinalWireCacheListener>,
    ) {
        *self.final_wire_cache_listener.lock().unwrap() = listener;
    }

    fn dynamic_context_transport(&self) -> crate::base::DynamicContextTransport {
        crate::base::DynamicContextTransport::UserContextEnvelope
    }

    fn prompt_cache_profile(&self, _model: &str) -> crate::base::PromptCacheProfile {
        crate::base::PromptCacheProfile::ephemeral(self.provider_name())
    }

    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Value>>,
        _tool_choice: crate::base::ToolChoiceMode,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let model = model.unwrap_or_else(|| self.default_model.clone());
        let request = self.build_request(
            messages,
            tools,
            model.clone(),
            max_tokens,
            temperature,
            false,
        )?;
        let api_key = self.require_api_key()?;
        let body_value = serde_json::to_value(&request)?;
        let final_wire_listener = crate::request_observers::current_final_wire_cache_listener()
            .or_else(|| self.final_wire_cache_listener.lock().unwrap().clone());
        if let Some(listener) = final_wire_listener {
            let stable_system = body_value.get("system").cloned().unwrap_or(Value::Null);
            let tools = body_value
                .get("tools")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default();
            listener(crate::final_wire::snapshot_from_wire(
                self.provider_name(),
                model.clone(),
                self.prompt_cache_profile(&model),
                &stable_system,
                tools,
            ));
        }
        let body = serde_json::to_string(&body_value)?;
        let url = format!("{}/v1/messages", self.api_base);
        let retry_listener = crate::request_observers::current_retry_listener()
            .or_else(|| self.retry_listener.lock().unwrap().clone());
        let response = retry::send_with_retry(&model, retry_listener.as_ref(), || {
            let request = self.apply_headers(self.client.post(&url).body(body.clone()), &api_key);
            async move { request.send().await }
        })
        .await?;
        let text = response.text().await?;
        let response: AnthropicResponse = serde_json::from_str(&text)?;
        Ok(Self::parse_response(response))
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<Value>>,
        _tool_choice: crate::base::ToolChoiceMode,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let model = model.unwrap_or_else(|| self.default_model.clone());
        let request = self.build_request(
            messages,
            tools,
            model.clone(),
            max_tokens,
            temperature,
            true,
        )?;
        let api_key = self.require_api_key()?;
        let body_value = serde_json::to_value(&request)?;
        let final_wire_listener = crate::request_observers::current_final_wire_cache_listener()
            .or_else(|| self.final_wire_cache_listener.lock().unwrap().clone());
        if let Some(listener) = final_wire_listener {
            let stable_system = body_value.get("system").cloned().unwrap_or(Value::Null);
            let tools = body_value
                .get("tools")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or_default();
            listener(crate::final_wire::snapshot_from_wire(
                self.provider_name(),
                model.clone(),
                self.prompt_cache_profile(&model),
                &stable_system,
                tools,
            ));
        }
        let body = serde_json::to_string(&body_value)?;
        let url = format!("{}/v1/messages", self.api_base);
        let retry_listener = crate::request_observers::current_retry_listener()
            .or_else(|| self.retry_listener.lock().unwrap().clone());
        let response = retry::send_with_retry(&model, retry_listener.as_ref(), || {
            let request = self.apply_headers(self.client.post(&url).body(body.clone()), &api_key);
            async move { request.send().await }
        })
        .await?;

        let provider = self.provider_name().to_string();
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        tokio::spawn(async move {
            let mut response = response;
            let mut utf8_decoder = StreamingUtf8Decoder::default();
            let mut buffer = String::new();
            let mut text = String::new();
            let mut reasoning = String::new();
            let mut usage = None;
            let mut finish_reason = None;
            let mut tools: HashMap<usize, StreamingToolUse> = HashMap::new();

            loop {
                let chunk = match tokio::time::timeout(STREAM_IDLE_TIMEOUT, response.chunk()).await
                {
                    Ok(Ok(Some(bytes))) => bytes,
                    Ok(Ok(None)) => break,
                    Ok(Err(error)) => {
                        let _ = tx.send(Err(ProviderError::HttpError(error)));
                        return;
                    }
                    Err(_) => {
                        let _ = tx.send(Err(ProviderError::ApiError(format!(
                            "Anthropic stream idle timeout for provider {}",
                            provider
                        ))));
                        return;
                    }
                };
                let decoded_chunk = match utf8_decoder.push(&chunk) {
                    Ok(text) => text,
                    Err(error) => {
                        let _ = tx.send(Err(error));
                        return;
                    }
                };
                buffer.push_str(&decoded_chunk);
                for payload in Self::parse_sse_events(&mut buffer) {
                    let event: AnthropicStreamEvent = match serde_json::from_str(&payload) {
                        Ok(event) => event,
                        Err(error) => {
                            let _ = tx.send(Err(ProviderError::JsonError(error)));
                            return;
                        }
                    };
                    match event.event_type.as_str() {
                        "message_start" => {
                            if let Some(message) = event.message {
                                usage = message.usage;
                                finish_reason = message.stop_reason;
                            }
                        }
                        "content_block_start" => {
                            if let (Some(index), Some(AnthropicStreamBlock::ToolUse { id, name })) =
                                (event.index, event.content_block)
                            {
                                tools.insert(
                                    index,
                                    StreamingToolUse {
                                        id,
                                        name,
                                        input_json: String::new(),
                                    },
                                );
                            }
                        }
                        "content_block_delta" => {
                            if let Some(delta) = event.delta {
                                match delta {
                                    AnthropicStreamDelta::Text { text: delta_text } => {
                                        text.push_str(&delta_text);
                                        let _ = tx.send(Ok(LLMStreamEvent::TextDelta(delta_text)));
                                    }
                                    AnthropicStreamDelta::Thinking { thinking } => {
                                        reasoning.push_str(&thinking);
                                        let _ =
                                            tx.send(Ok(LLMStreamEvent::ReasoningDelta(thinking)));
                                    }
                                    AnthropicStreamDelta::InputJson { partial_json } => {
                                        if let Some(index) = event.index {
                                            if let Some(tool) = tools.get_mut(&index) {
                                                tool.input_json.push_str(&partial_json);
                                                let _ =
                                                    tx.send(Ok(LLMStreamEvent::ToolCallDelta {
                                                        index,
                                                        id: Some(tool.id.clone()),
                                                        name: Some(tool.name.clone()),
                                                        arguments_delta: Some(partial_json),
                                                    }));
                                            }
                                        }
                                    }
                                    AnthropicStreamDelta::Other => {}
                                }
                            }
                        }
                        "message_delta" => {
                            if let Some(message) = event.message {
                                if message.usage.is_some() {
                                    usage = message.usage;
                                }
                                if message.stop_reason.is_some() {
                                    finish_reason = message.stop_reason;
                                }
                            }
                        }
                        "message_stop" => {
                            if let Err(error) = utf8_decoder.finish() {
                                let _ = tx.send(Err(error));
                                return;
                            }
                            let response = LLMResponse {
                                content: (!text.is_empty()).then_some(text.clone()),
                                tool_calls: finalize_streaming_tools(&tools),
                                finish_reason: finish_reason
                                    .clone()
                                    .unwrap_or_else(|| "stop".to_string()),
                                usage: usage_map(usage.clone()),
                                reasoning_content: (!reasoning.is_empty())
                                    .then_some(reasoning.clone()),
                            };
                            let _ = tx.send(Ok(LLMStreamEvent::Completed(response)));
                            return;
                        }
                        _ => {}
                    }
                }
            }
            if let Err(error) = utf8_decoder.finish() {
                let _ = tx.send(Err(error));
                return;
            }
            let response = LLMResponse {
                content: (!text.is_empty()).then_some(text),
                tool_calls: finalize_streaming_tools(&tools),
                finish_reason: finish_reason.unwrap_or_else(|| "stop".to_string()),
                usage: usage_map(usage),
                reasoning_content: (!reasoning.is_empty()).then_some(reasoning),
            };
            let _ = tx.send(Ok(LLMStreamEvent::Completed(response)));
        });
        Ok(Box::pin(stream::unfold(rx, |mut rx| async {
            rx.recv().await.map(|item| (item, rx))
        })))
    }

    fn get_default_model(&self) -> String {
        self.default_model.clone()
    }
}

fn convert_messages(
    messages: Vec<Message>,
) -> ProviderResult<(Option<Vec<String>>, Vec<AnthropicMessage>)> {
    let mut system = Vec::new();
    let mut out: Vec<AnthropicMessage> = Vec::new();
    let mut pending_tool_uses = HashSet::new();
    let mut resolved_tool_uses = HashSet::new();

    for message in messages {
        match message.role.as_str() {
            "system" => system.push(message.content.to_text_lossy()),
            "assistant" => {
                let mut content = content_blocks_from_message_content(&message.content)?;
                if let Some(reasoning) = message.reasoning_content.filter(|value| !value.is_empty())
                {
                    content.push(AnthropicContentBlock::Thinking {
                        thinking: reasoning,
                    });
                }
                if let Some(tool_calls) = message.tool_calls {
                    for tool_call in tool_calls {
                        pending_tool_uses.insert(tool_call.id.clone());
                        content.push(AnthropicContentBlock::ToolUse {
                            id: tool_call.id,
                            name: tool_call.name,
                            input: json!(tool_call.arguments),
                        });
                    }
                }
                push_merged(&mut out, "assistant", content);
            }
            "tool" => {
                let id = message.tool_call_id.clone().ok_or_else(|| {
                    ProviderError::ConfigError(
                        "Anthropic tool messages require tool_call_id".to_string(),
                    )
                })?;
                resolved_tool_uses.insert(id.clone());
                push_merged(
                    &mut out,
                    "user",
                    vec![AnthropicContentBlock::ToolResult {
                        tool_use_id: id,
                        content: message.content.to_text_lossy(),
                    }],
                );
            }
            _ => {
                let role = if message.role == "assistant" {
                    "assistant"
                } else {
                    "user"
                };
                push_merged(
                    &mut out,
                    role,
                    content_blocks_from_message_content(&message.content)?,
                );
            }
        }
    }

    for missing in pending_tool_uses.difference(&resolved_tool_uses) {
        push_merged(
            &mut out,
            "user",
            vec![AnthropicContentBlock::ToolResult {
                tool_use_id: missing.clone(),
                content: "Tool result unavailable in conversation history.".to_string(),
            }],
        );
    }

    Ok((
        (!system.is_empty()).then_some(system),
        out.into_iter()
            .filter(|message| !message.content.is_empty())
            .collect(),
    ))
}

fn content_blocks_from_message_content(
    content: &MessageContent,
) -> ProviderResult<Vec<AnthropicContentBlock>> {
    match content {
        MessageContent::Text(text) => {
            if text.is_empty() {
                Ok(Vec::new())
            } else {
                Ok(vec![AnthropicContentBlock::Text { text: text.clone() }])
            }
        }
        MessageContent::Parts(parts) => parts
            .iter()
            .map(|part| match part {
                MessageContentPart::Text { text } => {
                    Ok(AnthropicContentBlock::Text { text: text.clone() })
                }
                MessageContentPart::ImageUrl { .. }
                | MessageContentPart::ImageFile { .. }
                | MessageContentPart::ImageData { .. } => Err(ProviderError::ConfigError(
                    "Anthropic image input is not implemented yet".to_string(),
                )),
            })
            .collect(),
    }
}

fn push_merged(out: &mut Vec<AnthropicMessage>, role: &str, content: Vec<AnthropicContentBlock>) {
    if content.is_empty() {
        return;
    }
    if let Some(last) = out.last_mut().filter(|message| message.role == role) {
        last.content.extend(content);
    } else {
        out.push(AnthropicMessage {
            role: role.to_string(),
            content,
        });
    }
}

fn convert_tools(tools: Option<Vec<Value>>) -> ProviderResult<Option<Vec<AnthropicTool>>> {
    let Some(tools) = tools else {
        return Ok(None);
    };
    let mut converted = tools
        .into_iter()
        .map(|tool| {
            let cache_control = tool.get("cache_control").cloned();
            let function = tool.get("function").unwrap_or(&tool);
            let name = function
                .get("name")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    ProviderError::ConfigError("Tool definition missing name".to_string())
                })?
                .to_string();
            let description = function
                .get("description")
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let input_schema = function
                .get("parameters")
                .cloned()
                .unwrap_or_else(|| json!({"type": "object", "properties": {}}));
            Ok(AnthropicTool {
                name,
                description,
                input_schema,
                cache_control,
            })
        })
        .collect::<ProviderResult<Vec<_>>>()?;
    if !converted.iter().any(|tool| tool.cache_control.is_some()) {
        if let Some(last_tool) = converted.last_mut() {
            last_tool.cache_control = Some(json!({"type": "ephemeral"}));
        }
    }
    Ok((!converted.is_empty()).then_some(converted))
}

fn value_to_arguments(value: Value) -> HashMap<String, Value> {
    match value {
        Value::Object(map) => map.into_iter().collect(),
        other => HashMap::from([("value".to_string(), other)]),
    }
}

fn usage_map(usage: Option<AnthropicUsage>) -> HashMap<String, i64> {
    let Some(usage) = usage else {
        return HashMap::new();
    };
    let prompt =
        usage.input_tokens + usage.cache_creation_input_tokens + usage.cache_read_input_tokens;
    let completion = usage.output_tokens;
    HashMap::from([
        ("prompt_tokens".to_string(), prompt),
        ("completion_tokens".to_string(), completion),
        ("total_tokens".to_string(), prompt + completion),
        (
            "cache_creation_input_tokens".to_string(),
            usage.cache_creation_input_tokens,
        ),
        (
            "cache_read_input_tokens".to_string(),
            usage.cache_read_input_tokens,
        ),
    ])
}

fn finalize_streaming_tools(tools: &HashMap<usize, StreamingToolUse>) -> Vec<ToolCallRequest> {
    let mut ordered = tools.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|(index, _)| *index);
    ordered
        .into_iter()
        .map(|(_, tool)| ToolCallRequest {
            id: tool.id.clone(),
            call_type: "function".to_string(),
            name: tool.name.clone(),
            arguments: serde_json::from_str::<Value>(&tool.input_json)
                .map(value_to_arguments)
                .unwrap_or_default(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::{ImageUrl, MessageContentPart};

    #[test]
    fn dynamic_context_uses_safe_user_envelope() {
        let client = AnthropicClient::new(None, None, "claude-sonnet-4-5".to_string(), None, None);
        assert_eq!(
            client.dynamic_context_transport(),
            crate::base::DynamicContextTransport::UserContextEnvelope
        );
    }

    #[test]
    fn converts_system_messages_to_top_level_system_and_keeps_raw_model() {
        let client = AnthropicClient::new(None, None, "claude-sonnet-4-5".to_string(), None, None);
        let request = client
            .build_request(
                vec![Message::system("sys"), Message::user("hello")],
                None,
                "claude-sonnet-4-5".to_string(),
                100,
                0.7,
                false,
            )
            .unwrap();
        assert_eq!(request.model, "claude-sonnet-4-5");
        let system = request.system.as_ref().unwrap();
        assert_eq!(system[0].text, "sys");
        assert_eq!(
            system[0].cache_control.as_ref().unwrap()["type"],
            "ephemeral"
        );
        assert_eq!(request.messages[0].role, "user");
    }

    #[test]
    fn c1d_t8_preserves_explicit_core_tool_anchor() {
        let tools = vec![
            json!({"type":"function","function":{"name":"core_a","parameters":{"type":"object"}}}),
            json!({"type":"function","function":{"name":"core_b","parameters":{"type":"object"}},"cache_control":{"type":"ephemeral"}}),
            json!({"type":"function","function":{"name":"mcp_a","parameters":{"type":"object"}}}),
        ];
        let converted = convert_tools(Some(tools)).unwrap().unwrap();
        assert!(converted[0].cache_control.is_none());
        assert_eq!(
            converted[1].cache_control.as_ref().unwrap()["type"],
            "ephemeral"
        );
        assert!(converted[2].cache_control.is_none());
    }

    #[test]
    fn user_context_envelope_stays_after_history_and_before_current_user() {
        let messages = vec![
            Message::system("stable"),
            Message::user("history-user"),
            Message::assistant("history-assistant"),
            Message::user("<agent_diva_context>runtime</agent_diva_context>"),
            Message::user("current-user"),
        ];

        let (system, converted) = convert_messages(messages).unwrap();
        assert_eq!(system.as_deref(), Some(["stable".to_string()].as_slice()));
        assert_eq!(converted.len(), 3);
        assert_eq!(converted[1].role, "assistant");
        assert_eq!(converted[2].role, "user");
        assert_eq!(converted[2].content.len(), 2);
        assert!(matches!(
            &converted[2].content[0],
            AnthropicContentBlock::Text { text } if text.contains("runtime")
        ));
        assert!(matches!(
            &converted[2].content[1],
            AnthropicContentBlock::Text { text } if text == "current-user"
        ));
    }

    #[test]
    fn c1d_t8_marks_only_first_anthropic_system_block() {
        let client = AnthropicClient::new(None, None, "claude-sonnet-4-5".to_string(), None, None);
        let request = client
            .build_request(
                vec![
                    Message::system("stable"),
                    Message::system("volatile compaction"),
                    Message::user("current"),
                ],
                None,
                "claude-sonnet-4-5".to_string(),
                100,
                0.7,
                false,
            )
            .unwrap();
        let system = request.system.unwrap();
        assert_eq!(system.len(), 2);
        assert_eq!(
            system[0].cache_control.as_ref().unwrap()["type"],
            "ephemeral"
        );
        assert!(system[1].cache_control.is_none());
    }

    #[test]
    fn converts_tool_use_tool_result_and_merges_adjacent_user_blocks() {
        let tool_call = ToolCallRequest {
            id: "toolu_1".to_string(),
            call_type: "function".to_string(),
            name: "search".to_string(),
            arguments: HashMap::from([("query".to_string(), json!("rust"))]),
        };
        let mut assistant = Message::assistant("");
        assistant.tool_calls = Some(vec![tool_call]);
        let messages = vec![
            Message::user("first"),
            Message::user("second"),
            assistant,
            Message::tool("result", "toolu_1"),
        ];
        let (_, converted) = convert_messages(messages).unwrap();
        assert_eq!(converted[0].role, "user");
        assert_eq!(converted[0].content.len(), 2);
        assert_eq!(converted[1].role, "assistant");
        assert!(matches!(
            converted[1].content[0],
            AnthropicContentBlock::ToolUse { .. }
        ));
        assert_eq!(converted[2].role, "user");
        assert!(matches!(
            converted[2].content[0],
            AnthropicContentBlock::ToolResult { .. }
        ));
    }

    #[test]
    fn fills_orphan_tool_use_with_stub_tool_result() {
        let mut assistant = Message::assistant("");
        assistant.tool_calls = Some(vec![ToolCallRequest {
            id: "toolu_missing".to_string(),
            call_type: "function".to_string(),
            name: "search".to_string(),
            arguments: HashMap::new(),
        }]);
        let (_, converted) = convert_messages(vec![assistant]).unwrap();
        assert_eq!(converted.len(), 2);
        assert!(matches!(
            converted[1].content[0],
            AnthropicContentBlock::ToolResult { .. }
        ));
    }

    #[test]
    fn image_input_returns_config_error() {
        let message = Message::user(MessageContent::Parts(vec![MessageContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "data:image/png;base64,AAAA".to_string(),
            },
        }]));
        let result = convert_messages(vec![message]);
        assert!(matches!(result, Err(ProviderError::ConfigError(_))));
    }

    #[test]
    fn parses_text_thinking_tool_use_and_usage() {
        let response = AnthropicResponse {
            content: vec![
                AnthropicResponseBlock::Text {
                    text: "hi".to_string(),
                },
                AnthropicResponseBlock::Thinking {
                    thinking: "think".to_string(),
                },
                AnthropicResponseBlock::ToolUse {
                    id: "toolu_1".to_string(),
                    name: "search".to_string(),
                    input: json!({"query":"rust"}),
                },
            ],
            stop_reason: Some("tool_use".to_string()),
            usage: Some(AnthropicUsage {
                input_tokens: 10,
                output_tokens: 5,
                cache_creation_input_tokens: 2,
                cache_read_input_tokens: 3,
            }),
        };
        let parsed = AnthropicClient::parse_response(response);
        assert_eq!(parsed.content.as_deref(), Some("hi"));
        assert_eq!(parsed.reasoning_content.as_deref(), Some("think"));
        assert_eq!(parsed.tool_calls[0].arguments["query"], "rust");
        assert_eq!(parsed.usage["prompt_tokens"], 15);
        assert_eq!(parsed.usage["total_tokens"], 20);
        assert_eq!(parsed.usage["cache_creation_input_tokens"], 2);
        assert_eq!(parsed.usage["cache_read_input_tokens"], 3);
    }

    #[test]
    fn parses_sse_payloads() {
        let mut buffer =
            "event: content_block_delta\ndata: {\"type\":\"content_block_delta\"}\n\n".to_string();
        assert_eq!(
            AnthropicClient::parse_sse_events(&mut buffer),
            vec!["{\"type\":\"content_block_delta\"}".to_string()]
        );
        assert!(buffer.is_empty());
    }
}
