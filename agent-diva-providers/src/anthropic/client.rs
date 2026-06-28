//! Anthropic Messages API client — native HTTP implementation.
//!
//! `AnthropicClient` implements [`LLMProvider`] via the Anthropic Messages API
//! (`POST /v1/messages`).  It is a pure-HTTP driver with zero SDK dependency.
//!
//! # Key differences from OpenAI-compatible providers
//!
//! - **System prompt** is a top-level `system` field, NOT a `role: "system"` message.
//! - **Content blocks** — every message content is an array of typed blocks
//!   (`text`, `image`, `tool_use`, `tool_result`, `thinking`).
//! - **Usage** has three disjoint input-token buckets that must be summed.
//! - **SSE** uses named events (`event:` prefix) rather than bare `data:` lines.
//! - **Auth** uses `x-api-key` (or `Authorization: Bearer` for OAuth tokens).
//! - **max_tokens** is mandatory per the Anthropic API.

use agent_diva_core::Usage;
use async_trait::async_trait;
use futures::stream;
use reqwest::header::HeaderValue;
use reqwest::StatusCode;
use std::collections::HashMap;
use std::time::Duration;
use tracing::{debug, error};

use crate::base::{
    LLMProvider, LLMResponse, LLMStreamEvent, Message, MessageContent, MessageContentPart,
    ProviderError, ProviderEventStream, ProviderResult, ToolCallRequest,
};
use crate::http_util::build_api_http_client;

use super::dto::{
    AnthropicErrorEnvelope, AnthropicMessage, AnthropicRequest, AnthropicResponse, AnthropicUsage,
    CacheControl, ContentBlock, ImageSource, SystemPrompt, ThinkingConfig, ToolDefinition,
};
use super::stream::{parse_anthropic_sse, StreamState};

// ── Constants ───────────────────────────────────────────────────────────

/// Anthropic API version string.
const ANTHROPIC_VERSION: &str = "2023-06-01";

/// Default base URL for the Anthropic API.
const DEFAULT_API_BASE: &str = "https://api.anthropic.com";

/// Minimum budget tokens for extended thinking (API constraint).
const MIN_THINKING_BUDGET_TOKENS: u32 = 1024;

/// OAuth/setup token prefix (use Bearer auth instead of x-api-key).
const OAUTH_TOKEN_PREFIX: &str = "sk-ant-oat01-";

/// SSE idle timeout — per-line bound to prevent detached parser leaks.
const SSE_IDLE_TIMEOUT_SECS: u64 = 90;

/// Default max_tokens when none is provided (Anthropic requires this field).
const DEFAULT_MAX_TOKENS: u32 = 4096;

// ── AnthropicClient ─────────────────────────────────────────────────────

/// Native Anthropic Messages API client.
///
/// # Example
///
/// ```ignore
/// let client = AnthropicClient::new(
///     "sk-ant-api03-xxxx".to_string(),
///     None,
///     "claude-sonnet-4-5".to_string(),
///     None,
/// );
/// ```
pub struct AnthropicClient {
    api_key: String,
    api_base: String,
    default_model: String,
    http_client: reqwest::Client,
    extra_headers: HashMap<String, String>,

    /// Whether to enable prompt caching.
    cache_enabled: bool,

    /// Default thinking budget (0 = disabled).
    thinking_budget_tokens: u32,
}

impl AnthropicClient {
    /// Create a new Anthropic client.
    ///
    /// # Arguments
    ///
    /// * `api_key` — Anthropic API key (`sk-ant-api03-...`) or OAuth setup token (`sk-ant-oat01-...`).
    /// * `api_base` — Override the API base URL (default: `https://api.anthropic.com`).
    /// * `default_model` — Model ID to use when none is provided at request time.
    /// * `extra_headers` — Additional HTTP headers to send with every request.
    pub fn new(
        api_key: String,
        api_base: Option<String>,
        default_model: String,
        extra_headers: Option<HashMap<String, String>>,
    ) -> Self {
        let api_base = api_base
            .unwrap_or_else(|| DEFAULT_API_BASE.to_string())
            .trim_end_matches('/')
            .to_string();

        let http_client = build_api_http_client(&api_base, Duration::from_secs(300))
            .expect("failed to build reqwest client for Anthropic");

        Self {
            api_key,
            api_base,
            default_model,
            http_client,
            extra_headers: extra_headers.unwrap_or_default(),
            cache_enabled: true,
            thinking_budget_tokens: 0,
        }
    }

    /// Enable or disable prompt caching (default: enabled).
    pub fn with_cache(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    /// Set a default thinking budget token count (0 = disabled).
    pub fn with_thinking(mut self, budget_tokens: u32) -> Self {
        self.thinking_budget_tokens = budget_tokens;
        self
    }

    // ── Auth ───────────────────────────────────────────────────────────

    /// Determine whether this API key is an OAuth/setup token.
    fn is_oauth_token(key: &str) -> bool {
        key.starts_with(OAUTH_TOKEN_PREFIX)
    }

    /// Build authentication headers for the request.
    fn auth_headers(&self) -> HashMap<String, String> {
        let mut headers = self.extra_headers.clone();
        if Self::is_oauth_token(&self.api_key) {
            headers.insert(
                "Authorization".to_string(),
                format!("Bearer {}", self.api_key),
            );
        } else {
            headers.insert("x-api-key".to_string(), self.api_key.clone());
        }
        headers
    }

    /// Apply all required headers to a request builder.
    fn apply_headers(&self, mut builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        let auth_headers = self.auth_headers();
        for (key, value) in &auth_headers {
            if let Ok(v) = HeaderValue::from_str(value) {
                builder = builder.header(key.as_str(), v);
            }
        }

        builder = builder
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("content-type", "application/json");

        builder
    }

    // ── Request building ──────────────────────────────────────────────

    /// Build the full request URL.
    fn request_url(&self) -> String {
        format!("{}/v1/messages", self.api_base)
    }

    /// Build an AnthropicRequest from agent-diva messages and parameters.
    fn build_request(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: String,
        max_tokens: i32,
        temperature: f64,
        stream: bool,
        thinking: Option<ThinkingConfig>,
    ) -> AnthropicRequest {
        let (system, anthropic_messages) = Self::convert_messages(messages);

        let tools: Option<Vec<ToolDefinition>> = tools.map(|raw_tools| {
            let mut defs: Vec<ToolDefinition> = raw_tools
                .iter()
                .filter_map(|t| Self::convert_tool_definition(t))
                .collect();

            // Apply cache control to last tool if caching is enabled
            if self.cache_enabled && !defs.is_empty() {
                let last = defs.last_mut().unwrap();
                last.cache_control = Some(CacheControl::ephemeral());
            }
            defs
        });

        let effective_temperature = if thinking.is_some() {
            // Temperature must be 1.0 when native thinking is enabled
            1.0
        } else {
            temperature
        };

        let max_tokens_u32 = if max_tokens > 0 {
            max_tokens as u32
        } else {
            DEFAULT_MAX_TOKENS
        };

        AnthropicRequest {
            model,
            messages: anthropic_messages,
            max_tokens: max_tokens_u32,
            system,
            tools,
            tool_choice: None,
            thinking,
            stream: if stream { Some(true) } else { None },
            temperature: Some(effective_temperature),
            metadata: None,
        }
    }

    // ── Message conversion ────────────────────────────────────────────

    /// Convert agent-diva `Message` vec into an Anthropic (SystemPrompt, Vec<AnthropicMessage>).
    ///
    /// Rules (from research doc §8):
    /// 1. System messages → top-level `system` field (NOT a message role).
    /// 2. Tool results → `role: "user"` with `tool_result` content blocks, merged with
    ///    adjacent user messages.
    /// 3. Adjacent same-role messages are merged (Anthropic rejects user/user pairs).
    /// 4. Thinking blocks from `thinking_blocks` / `reasoning_content` are preserved
    ///    in assistant messages with tool_use.
    /// 5. Orphaned `tool_use` blocks are backfilled with stub `tool_result`.
    fn convert_messages(messages: Vec<Message>) -> (Option<SystemPrompt>, Vec<AnthropicMessage>) {
        let mut system_parts: Vec<String> = Vec::new();
        let mut converted: Vec<AnthropicMessage> = Vec::new();

        for msg in messages {
            match msg.role.as_str() {
                "system" => {
                    system_parts.push(msg.content.to_text_lossy());
                }
                "user" => {
                    let blocks = Self::convert_user_content(&msg.content);
                    // Merge with previous user message if adjacent
                    if let Some(last) = converted.last_mut() {
                        if last.role == "user" {
                            last.content.extend(blocks);
                            continue;
                        }
                    }
                    converted.push(AnthropicMessage::user(blocks));
                }
                "assistant" => {
                    let blocks = Self::convert_assistant_content(&msg);
                    // Merge with previous assistant if adjacent
                    if let Some(last) = converted.last_mut() {
                        if last.role == "assistant" {
                            last.content.extend(blocks);
                            continue;
                        }
                    }
                    converted.push(AnthropicMessage::assistant(blocks));
                }
                "tool" => {
                    let blocks = Self::convert_tool_result(&msg);
                    // Tool results are role="user" in Anthropic; merge with prev user
                    if let Some(last) = converted.last_mut() {
                        if last.role == "user" {
                            last.content.extend(blocks);
                            continue;
                        }
                    }
                    converted.push(AnthropicMessage::user(blocks));
                }
                _ => {
                    // Unknown role → treat as user
                    let blocks = Self::convert_user_content(&msg.content);
                    converted.push(AnthropicMessage::user(blocks));
                }
            }
        }

        // Build system prompt
        let system = if system_parts.is_empty() {
            None
        } else {
            let combined = system_parts.join("\n\n");
            Some(SystemPrompt::Text(combined))
        };

        // Backfill orphaned tool_use blocks
        Self::backfill_orphaned_tool_use(&mut converted);

        (system, converted)
    }

    /// Convert agent-diva `MessageContent` into Anthropic content blocks for a user message.
    fn convert_user_content(content: &MessageContent) -> Vec<ContentBlock> {
        match content {
            MessageContent::Text(text) => {
                if text.is_empty() {
                    return vec![ContentBlock::Text {
                        text: ".".to_string(),
                        cache_control: None,
                    }];
                }
                vec![ContentBlock::Text {
                    text: text.clone(),
                    cache_control: None,
                }]
            }
            MessageContent::Parts(parts) => {
                let mut blocks = Vec::new();
                for part in parts {
                    match part {
                        MessageContentPart::Text { text } => {
                            blocks.push(ContentBlock::Text {
                                text: text.clone(),
                                cache_control: None,
                            });
                        }
                        MessageContentPart::ImageUrl { image_url } => {
                            // Parse data URI or URL for base64 data
                            if let Some(data) = Self::extract_base64_from_data_uri(&image_url.url) {
                                let media_type = Self::guess_media_type(&image_url.url);
                                blocks.push(ContentBlock::Image {
                                    source: ImageSource {
                                        source_type: "base64".to_string(),
                                        media_type,
                                        data,
                                    },
                                });
                            }
                        }
                        MessageContentPart::ImageData { image_data } => {
                            if let Some(data) =
                                Self::extract_base64_from_data_uri(&image_data.data_uri)
                            {
                                let media_type = Self::guess_media_type(&image_data.data_uri);
                                blocks.push(ContentBlock::Image {
                                    source: ImageSource {
                                        source_type: "base64".to_string(),
                                        media_type,
                                        data,
                                    },
                                });
                            }
                        }
                        MessageContentPart::ImageFile { .. } => {
                            // ImageFile references local files — skip for API calls
                            debug!("Skipping ImageFile reference in Anthropic message");
                        }
                    }
                }
                if blocks.is_empty() {
                    blocks.push(ContentBlock::Text {
                        text: ".".to_string(),
                        cache_control: None,
                    });
                }
                blocks
            }
        }
    }

    /// Convert an agent-diva assistant message into Anthropic content blocks.
    ///
    /// Handles:
    /// - Text content
    /// - Thinking blocks from `thinking_blocks` or `reasoning_content`
    /// - Tool calls from `tool_calls`
    fn convert_assistant_content(msg: &Message) -> Vec<ContentBlock> {
        let mut blocks = Vec::new();

        // 1. Thinking blocks (must come first, before any tool_use)
        if let Some(ref thinking_blocks) = msg.thinking_blocks {
            for tb in thinking_blocks {
                if let Some(block) = Self::parse_thinking_block(tb) {
                    blocks.push(block);
                }
            }
        } else if let Some(ref reasoning) = msg.reasoning_content {
            // Fallback: treat reasoning_content as a single thinking block
            // (without signature — only for display, not for replay)
            if !reasoning.is_empty() {
                // We don't have a signature, so just put it as text for now
                // The actual thinking block requires a signature for replay
                if msg.tool_calls.as_ref().map_or(true, |tc| tc.is_empty()) {
                    // No tool calls — safe to include reasoning as text
                    blocks.push(ContentBlock::Text {
                        text: reasoning.clone(),
                        cache_control: None,
                    });
                }
            }
        }

        // 2. Text content
        let text_content = msg.content.to_text_lossy();
        if !text_content.is_empty() {
            blocks.push(ContentBlock::Text {
                text: text_content,
                cache_control: None,
            });
        }

        // 3. Tool calls → tool_use blocks
        if let Some(ref tool_calls) = msg.tool_calls {
            for tc in tool_calls {
                blocks.push(ContentBlock::ToolUse {
                    id: tc.id.clone(),
                    name: tc.name.clone(),
                    input: serde_json::Value::Object(
                        tc.arguments
                            .iter()
                            .map(|(k, v)| (k.clone(), v.clone()))
                            .collect(),
                    ),
                });
            }
        }

        blocks
    }

    /// Convert an agent-diva tool result into Anthropic `tool_result` blocks.
    fn convert_tool_result(msg: &Message) -> Vec<ContentBlock> {
        let tool_use_id = msg
            .tool_call_id
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let content_text = msg.content.to_text_lossy();

        vec![ContentBlock::ToolResult {
            tool_use_id,
            content: serde_json::Value::String(content_text),
            cache_control: None,
        }]
    }

    /// Try to parse a `thinking_blocks` JSON value into a `ContentBlock::Thinking`.
    fn parse_thinking_block(raw: &serde_json::Value) -> Option<ContentBlock> {
        let thinking = raw.get("thinking")?.as_str()?;
        let signature = raw.get("signature")?.as_str()?;
        Some(ContentBlock::Thinking {
            thinking: thinking.to_string(),
            signature: signature.to_string(),
        })
    }

    /// Backfill orphaned `tool_use` blocks with stub `tool_result` blocks.
    ///
    /// Anthropic rejects messages where a tool_use has no corresponding tool_result
    /// in the same conversation turn. We insert a stub to prevent 400 errors.
    fn backfill_orphaned_tool_use(messages: &mut Vec<AnthropicMessage>) {
        for i in 0..messages.len() {
            if messages[i].role != "assistant" {
                continue;
            }
            let has_tool_use = messages[i].content.iter().any(|b| b.is_tool_use());
            if !has_tool_use {
                continue;
            }
            // Check if the next message has tool_results for all tool_uses
            let tool_use_ids: Vec<String> = messages[i]
                .content
                .iter()
                .filter_map(|b| b.tool_use_id().map(|s| s.to_string()))
                .collect();

            if tool_use_ids.is_empty() {
                continue;
            }

            // Check next message
            let has_result = if i + 1 < messages.len() && messages[i + 1].role == "user" {
                tool_use_ids.iter().all(|id| {
                    messages[i + 1].content.iter().any(|b| match b {
                        ContentBlock::ToolResult { tool_use_id, .. } => tool_use_id == id,
                        _ => false,
                    })
                })
            } else {
                false
            };

            if !has_result {
                // Insert stub tool_results as a user message after this assistant
                let stubs: Vec<ContentBlock> = tool_use_ids
                    .iter()
                    .map(|id| ContentBlock::ToolResult {
                        tool_use_id: id.clone(),
                        content: serde_json::Value::String("[tool result omitted]".to_string()),
                        cache_control: None,
                    })
                    .collect();
                messages.insert(i + 1, AnthropicMessage::user(stubs));
            }
        }
    }

    // ── Tool conversion ───────────────────────────────────────────────

    /// Convert an OpenAI-style tool JSON to an Anthropic `ToolDefinition`.
    ///
    /// OpenAI format: `{type: "function", function: {name, description, parameters}}`
    /// Anthropic format: `{name, description, input_schema}`
    fn convert_tool_definition(raw: &serde_json::Value) -> Option<ToolDefinition> {
        let func = raw.get("function")?;
        let name = func.get("name")?.as_str()?.to_string();
        let description = func
            .get("description")
            .and_then(|d| d.as_str())
            .map(|s| s.to_string());
        let parameters = func
            .get("parameters")
            .cloned()
            .unwrap_or(serde_json::json!({"type": "object", "properties": {}}));

        Some(ToolDefinition {
            name,
            description,
            input_schema: parameters,
            cache_control: None,
        })
    }

    // ── Response parsing ──────────────────────────────────────────────

    /// Parse an Anthropic non-streaming response into an `LLMResponse`.
    fn parse_response(resp: AnthropicResponse) -> ProviderResult<LLMResponse> {
        let mut text_parts = Vec::new();
        let mut reasoning_parts = Vec::new();
        let mut tool_calls = Vec::new();

        for block in &resp.content {
            match block {
                ContentBlock::Text { text, .. } => {
                    text_parts.push(text.clone());
                }
                ContentBlock::Thinking {
                    thinking,
                    signature,
                } => {
                    reasoning_parts.push(thinking.clone());
                    // Preserve thinking blocks for round-trip
                    // (stored as JSON in reasoning_content for now;
                    //  full thinking_blocks support is a future enhancement)
                    let _ = signature;
                }
                ContentBlock::ToolUse { id, name, input } => {
                    let arguments = match input {
                        serde_json::Value::Object(map) => map.clone().into_iter().collect(),
                        _ => {
                            let mut fallback = HashMap::new();
                            fallback.insert("raw".to_string(), input.clone());
                            fallback
                        }
                    };
                    tool_calls.push(ToolCallRequest {
                        id: id.clone(),
                        call_type: "function".to_string(),
                        name: name.clone(),
                        arguments,
                    });
                }
                ContentBlock::ToolResult { .. } | ContentBlock::Image { .. } => {
                    // These shouldn't appear in assistant responses
                }
            }
        }

        let content = if text_parts.is_empty() {
            None
        } else {
            Some(text_parts.join(""))
        };

        let reasoning = if reasoning_parts.is_empty() {
            None
        } else {
            Some(reasoning_parts.join("\n"))
        };

        let usage = resp.usage.to_normalized_usage();

        Ok(LLMResponse {
            content,
            tool_calls,
            finish_reason: resp.stop_reason.unwrap_or_else(|| "stop".to_string()),
            usage: Some(usage),
            reasoning_content: reasoning,
        })
    }

    // ── Error handling ────────────────────────────────────────────────

    /// Build a `ProviderApiError` from an HTTP error response.
    fn build_api_error(status: StatusCode, error_text: String, _model: &str) -> ProviderError {
        // Try to parse as Anthropic error envelope
        if let Ok(envelope) = serde_json::from_str::<AnthropicErrorEnvelope>(&error_text) {
            let message = envelope.error.message;
            match status.as_u16() {
                401 | 403 => ProviderError::Auth { message },
                429 => ProviderError::RateLimited { retry_after: None },
                500 | 502 | 503 | 504 => ProviderError::Transient { message },
                400 | 404 | 422 => ProviderError::Permanent { message },
                _ => ProviderError::api_message(format!("{} (HTTP {})", message, status.as_u16())),
            }
        } else {
            // Non-JSON error body — classify by status code
            let msg = format!("HTTP {} — {}", status.as_u16(), error_text);
            match status.as_u16() {
                401 | 403 => ProviderError::Auth { message: msg },
                429 => ProviderError::RateLimited { retry_after: None },
                500 | 502 | 503 | 504 => ProviderError::Transient { message: msg },
                400 | 402 | 404 | 405 | 422 => ProviderError::Permanent { message: msg },
                _ => ProviderError::api_message(format!(
                    "Anthropic API error (HTTP {}): {}",
                    status.as_u16(),
                    error_text
                )),
            }
        }
    }
}

// ── LLMProvider trait implementation ──────────────────────────────────

#[async_trait]
impl LLMProvider for AnthropicClient {
    async fn chat(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let resolved_model = model.unwrap_or_else(|| self.default_model.clone());
        let url = self.request_url();

        // Determine thinking config
        let thinking = if self.thinking_budget_tokens >= MIN_THINKING_BUDGET_TOKENS {
            Some(ThinkingConfig::enabled(self.thinking_budget_tokens))
        } else {
            None
        };

        let request = self.build_request(
            messages,
            tools,
            resolved_model.clone(),
            max_tokens,
            temperature,
            false,
            thinking,
        );

        let body_json = serde_json::to_string(&request)
            .map_err(|e| ProviderError::InvalidResponse(format!("Serialize error: {}", e)))?;

        debug!(
            "Anthropic chat: model={}, url={}, body_bytes={}",
            resolved_model,
            url,
            body_json.len()
        );

        let req_builder = self.apply_headers(self.http_client.post(&url).body(body_json));

        let response = req_builder.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!("Anthropic chat error: HTTP {} — {}", status, error_text);
            return Err(Self::build_api_error(status, error_text, &resolved_model));
        }

        let response_text = response.text().await?;
        let response_data: AnthropicResponse =
            serde_json::from_str(&response_text).map_err(|e| {
                error!("Failed to parse Anthropic response: {}", e);
                ProviderError::JsonError(e)
            })?;

        Self::parse_response(response_data)
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
        model: Option<String>,
        max_tokens: i32,
        temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let resolved_model = model.unwrap_or_else(|| self.default_model.clone());
        let url = self.request_url();

        // Determine thinking config
        let thinking = if self.thinking_budget_tokens >= MIN_THINKING_BUDGET_TOKENS {
            Some(ThinkingConfig::enabled(self.thinking_budget_tokens))
        } else {
            None
        };

        // When thinking is enabled, fall back to non-streaming to preserve
        // signed thinking blocks, then synthesize a stream from the response.
        if thinking.is_some() {
            let response = self
                .chat(
                    messages,
                    tools,
                    Some(resolved_model),
                    max_tokens,
                    temperature,
                )
                .await?;

            let mut events: Vec<ProviderResult<LLMStreamEvent>> = Vec::new();

            // Emit reasoning delta if present
            if let Some(ref reasoning) = response.reasoning_content {
                if !reasoning.is_empty() {
                    events.push(Ok(LLMStreamEvent::ReasoningDelta(reasoning.clone())));
                }
            }

            // Emit text delta
            if let Some(ref content) = response.content {
                if !content.is_empty() {
                    events.push(Ok(LLMStreamEvent::TextDelta(content.clone())));
                }
            }

            // Emit tool call deltas
            for (i, tc) in response.tool_calls.iter().enumerate() {
                events.push(Ok(LLMStreamEvent::ToolCallDelta {
                    index: i,
                    id: Some(tc.id.clone()),
                    name: Some(tc.name.clone()),
                    arguments_delta: None,
                }));
            }

            events.push(Ok(LLMStreamEvent::Completed(response)));

            return Ok(Box::pin(stream::iter(events)));
        }

        let request = self.build_request(
            messages,
            tools,
            resolved_model.clone(),
            max_tokens,
            temperature,
            true,
            None,
        );

        let body_json = serde_json::to_string(&request)
            .map_err(|e| ProviderError::InvalidResponse(format!("Serialize error: {}", e)))?;

        debug!(
            "Anthropic chat_stream: model={}, url={}, body_bytes={}",
            resolved_model,
            url,
            body_json.len()
        );

        let req_builder = self.apply_headers(self.http_client.post(&url).body(body_json));

        let response = req_builder.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            error!(
                "Anthropic chat_stream error: HTTP {} — {}",
                status, error_text
            );
            return Err(Self::build_api_error(status, error_text, &resolved_model));
        }

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let idle_timeout = Duration::from_secs(SSE_IDLE_TIMEOUT_SECS);

        tokio::spawn(async move {
            let mut resp_stream = response;
            let mut buffer = String::new();
            let mut state = StreamState::default();
            let mut last_text_len: usize = 0;
            let mut last_reasoning_len: usize = 0;
            let mut last_tool_count: usize = 0;

            loop {
                let chunk = match tokio::time::timeout(idle_timeout, resp_stream.chunk()).await {
                    Ok(Ok(Some(bytes))) => bytes,
                    Ok(Ok(None)) => break,
                    Ok(Err(err)) => {
                        error!("Anthropic stream read error: {}", err);
                        let _ = tx.send(Err(ProviderError::HttpError(err)));
                        return;
                    }
                    Err(_elapsed) => {
                        error!(
                            "Anthropic SSE stream idle timeout after {}s",
                            SSE_IDLE_TIMEOUT_SECS
                        );
                        let _ = tx.send(Err(ProviderError::InvalidResponse(
                            "SSE stream idle timeout".to_string(),
                        )));
                        return;
                    }
                };

                let text = String::from_utf8_lossy(&chunk);
                buffer.push_str(&text);

                for (event_name, data) in parse_anthropic_sse(&mut buffer) {
                    // Parse the data JSON as an SSE event
                    let event = match serde_json::from_str::<super::dto::SseEvent>(&data) {
                        Ok(ev) => ev,
                        Err(e) => {
                            debug!(
                                "Failed to parse Anthropic SSE event '{}': {} — data: {}",
                                event_name, e, data
                            );
                            continue;
                        }
                    };

                    state.handle_event(&event);

                    // Emit incremental deltas to the consumer
                    match &event {
                        super::dto::SseEvent::ContentBlockDelta { delta, .. } => match delta {
                            super::dto::SseDelta::TextDelta { text } => {
                                let _ = tx.send(Ok(LLMStreamEvent::TextDelta(text.clone())));
                            }
                            super::dto::SseDelta::InputJsonDelta { partial_json } => {
                                let _ = tx.send(Ok(LLMStreamEvent::ToolCallDelta {
                                    index: state.tool_calls.len().saturating_sub(1),
                                    id: None,
                                    name: None,
                                    arguments_delta: Some(partial_json.clone()),
                                }));
                            }
                            super::dto::SseDelta::ThinkingDelta { thinking } => {
                                let _ =
                                    tx.send(Ok(LLMStreamEvent::ReasoningDelta(thinking.clone())));
                            }
                            _ => {}
                        },
                        super::dto::SseEvent::MessageStop => {
                            break;
                        }
                        _ => {}
                    }

                    // Update tracking
                    let current_text = state.text_content.len();
                    if current_text > last_text_len {
                        last_text_len = current_text;
                    }
                    let current_reasoning = state.reasoning_content.len();
                    if current_reasoning > last_reasoning_len {
                        last_reasoning_len = current_reasoning;
                    }
                    let current_tools = state.tool_calls.len();
                    if current_tools > last_tool_count {
                        last_tool_count = current_tools;
                    }
                }

                // Check if we've received a message_stop event marker
                if buffer.contains("\"message_stop\"") {
                    break;
                }
            }

            // Send completed response
            let final_response = state.into_response();
            let _ = tx.send(Ok(LLMStreamEvent::Completed(final_response)));
        });

        Ok(Box::pin(stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|item| (item, rx))
        })))
    }

    fn get_default_model(&self) -> String {
        self.default_model.clone()
    }
}

// ── AnthropicUsage normalization ─────────────────────────────────────

impl AnthropicUsage {
    /// Convert AnthropicUsage into a normalized `Usage` struct.
    fn to_normalized_usage(&self) -> Usage {
        let prompt = self.total_input_tokens() as i64;
        let completion = self.output_tokens.unwrap_or(0) as i64;

        Usage::new(prompt, completion)
    }
}

// ── Image helpers ─────────────────────────────────────────────────────

impl AnthropicClient {
    /// Extract base64 data from a `data:` URI or raw base64 string.
    fn extract_base64_from_data_uri(uri: &str) -> Option<String> {
        if let Some(comma_pos) = uri.find(',') {
            Some(uri[comma_pos + 1..].to_string())
        } else if !uri.contains("://") && !uri.contains("data:") {
            // Looks like plain base64 (no URI scheme or path prefix)
            Some(uri.to_string())
        } else {
            None
        }
    }

    /// Guess the media type from a data URI or URL.
    fn guess_media_type(uri: &str) -> String {
        if uri.starts_with("data:") {
            if let Some(end) = uri.find(';') {
                return uri["data:".len()..end].to_string();
            }
        }
        // Check common image extensions in the URI
        let lower = uri.to_ascii_lowercase();
        if lower.contains(".png") || lower.contains("image/png") {
            "image/png".to_string()
        } else if lower.contains(".webp") || lower.contains("image/webp") {
            "image/webp".to_string()
        } else if lower.contains(".gif") || lower.contains("image/gif") {
            "image/gif".to_string()
        } else {
            "image/jpeg".to_string()
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::base::{ImageData, ImageFile, ImageUrl, MessageContent, MessageContentPart};

    // ── Constructor tests ───────────────────────────────────────────

    #[test]
    fn anthropic_client_new_defaults() {
        let client = AnthropicClient::new(
            "sk-ant-api03-test".to_string(),
            None,
            "claude-sonnet-4-5".to_string(),
            None,
        );
        assert_eq!(client.default_model, "claude-sonnet-4-5");
        assert_eq!(client.api_base, DEFAULT_API_BASE);
        assert!(!client.api_key.is_empty());
    }

    #[test]
    fn anthropic_client_custom_base_url() {
        let client = AnthropicClient::new(
            "sk-ant-api03-test".to_string(),
            Some("https://custom.anthropic.com".to_string()),
            "claude-sonnet-4-5".to_string(),
            None,
        );
        assert_eq!(client.api_base, "https://custom.anthropic.com");
        assert_eq!(
            client.request_url(),
            "https://custom.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn anthropic_client_base_url_strips_trailing_slash() {
        let client = AnthropicClient::new(
            "sk-ant-api03-test".to_string(),
            Some("https://api.anthropic.com/".to_string()),
            "claude-sonnet-4-5".to_string(),
            None,
        );
        assert_eq!(client.api_base, "https://api.anthropic.com");
    }

    #[test]
    fn anthropic_client_get_default_model() {
        let client = AnthropicClient::new(
            "sk-ant-api03-test".to_string(),
            None,
            "claude-opus-4-5".to_string(),
            None,
        );
        assert_eq!(client.get_default_model(), "claude-opus-4-5");
    }

    // ── Auth detection tests ────────────────────────────────────────

    #[test]
    fn is_oauth_token_detects_setup_token() {
        assert!(AnthropicClient::is_oauth_token("sk-ant-oat01-xxxx"));
        assert!(!AnthropicClient::is_oauth_token("sk-ant-api03-xxxx"));
        assert!(!AnthropicClient::is_oauth_token(""));
    }

    #[test]
    fn auth_headers_uses_x_api_key_for_standard_tokens() {
        let client = AnthropicClient::new(
            "sk-ant-api03-test".to_string(),
            None,
            "claude-sonnet-4-5".to_string(),
            None,
        );
        let headers = client.auth_headers();
        assert!(headers.contains_key("x-api-key"));
        assert_eq!(headers.get("x-api-key").unwrap(), "sk-ant-api03-test");
        assert!(!headers.contains_key("Authorization"));
    }

    #[test]
    fn auth_headers_uses_bearer_for_oauth_tokens() {
        let client = AnthropicClient::new(
            "sk-ant-oat01-test".to_string(),
            None,
            "claude-sonnet-4-5".to_string(),
            None,
        );
        let headers = client.auth_headers();
        assert!(headers.contains_key("Authorization"));
        assert_eq!(
            headers.get("Authorization").unwrap(),
            "Bearer sk-ant-oat01-test"
        );
        assert!(!headers.contains_key("x-api-key"));
    }

    // ── Message conversion: user content ───────────────────────────

    #[test]
    fn convert_user_content_plain_text() {
        let content = MessageContent::Text("Hello, Claude!".to_string());
        let blocks = AnthropicClient::convert_user_content(&content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].as_text(), Some("Hello, Claude!"));
    }

    #[test]
    fn convert_user_content_empty_text_gets_dot_placeholder() {
        let content = MessageContent::Text(String::new());
        let blocks = AnthropicClient::convert_user_content(&content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].as_text(), Some("."));
    }

    #[test]
    fn convert_user_content_with_image_data_uri() {
        let content = MessageContent::Parts(vec![
            MessageContentPart::Text {
                text: "Look at this:".to_string(),
            },
            MessageContentPart::ImageData {
                image_data: ImageData {
                    data_uri: "data:image/png;base64,iVBORw0KGgo".to_string(),
                },
            },
        ]);
        let blocks = AnthropicClient::convert_user_content(&content);
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].as_text(), Some("Look at this:"));
        match &blocks[1] {
            ContentBlock::Image { source } => {
                assert_eq!(source.source_type, "base64");
                assert_eq!(source.media_type, "image/png");
                assert_eq!(source.data, "iVBORw0KGgo");
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }

    #[test]
    fn convert_user_content_image_file_skipped() {
        let content = MessageContent::Parts(vec![MessageContentPart::ImageFile {
            image_file: ImageFile {
                file_id: "file_local_123".to_string(),
            },
        }]);
        let blocks = AnthropicClient::convert_user_content(&content);
        // ImageFile is skipped; should get placeholder text
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].as_text(), Some("."));
    }

    #[test]
    fn convert_user_content_image_url() {
        let content = MessageContent::Parts(vec![MessageContentPart::ImageUrl {
            image_url: ImageUrl {
                url: "data:image/jpeg;base64,/9j/4AAQ".to_string(),
            },
        }]);
        let blocks = AnthropicClient::convert_user_content(&content);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            ContentBlock::Image { source } => {
                assert_eq!(source.media_type, "image/jpeg");
                assert_eq!(source.data, "/9j/4AAQ");
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }

    // ── Message conversion: assistant content ───────────────────────

    #[test]
    fn convert_assistant_content_text_only() {
        let msg = Message::assistant("I am Claude.");
        let blocks = AnthropicClient::convert_assistant_content(&msg);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].as_text(), Some("I am Claude."));
    }

    #[test]
    fn convert_assistant_content_with_tool_calls() {
        let mut msg = Message::assistant("");
        msg.tool_calls = Some(vec![ToolCallRequest {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            name: "get_weather".to_string(),
            arguments: {
                let mut m = HashMap::new();
                m.insert(
                    "location".to_string(),
                    serde_json::Value::String("NYC".to_string()),
                );
                m
            },
        }]);
        let blocks = AnthropicClient::convert_assistant_content(&msg);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            ContentBlock::ToolUse { id, name, input } => {
                assert_eq!(id, "call_1");
                assert_eq!(name, "get_weather");
                assert_eq!(input["location"], "NYC");
            }
            other => panic!("expected ToolUse, got {:?}", other),
        }
    }

    #[test]
    fn convert_assistant_content_with_thinking_blocks() {
        let mut msg = Message::assistant("The answer is 42.");
        msg.thinking_blocks = Some(vec![serde_json::json!({
            "type": "thinking",
            "thinking": "Let me calculate...",
            "signature": "sig_abc123"
        })]);
        let blocks = AnthropicClient::convert_assistant_content(&msg);
        assert_eq!(blocks.len(), 2);
        match &blocks[0] {
            ContentBlock::Thinking {
                thinking,
                signature,
            } => {
                assert_eq!(thinking, "Let me calculate...");
                assert_eq!(signature, "sig_abc123");
            }
            other => panic!("expected Thinking, got {:?}", other),
        }
        assert_eq!(blocks[1].as_text(), Some("The answer is 42."));
    }

    #[test]
    fn convert_assistant_content_with_reasoning_no_tool_calls() {
        let mut msg = Message::assistant("Answer.");
        msg.reasoning_content = Some("Step 1: think. Step 2: answer.".to_string());
        let blocks = AnthropicClient::convert_assistant_content(&msg);
        // Without tool_calls, reasoning is included as text
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[1].as_text(), Some("Answer."));
    }

    // ── Message conversion: tool result ─────────────────────────────

    #[test]
    fn convert_tool_result() {
        let msg = Message::tool("72 degrees", "call_1");
        let blocks = AnthropicClient::convert_tool_result(&msg);
        assert_eq!(blocks.len(), 1);
        match &blocks[0] {
            ContentBlock::ToolResult {
                tool_use_id,
                content,
                ..
            } => {
                assert_eq!(tool_use_id, "call_1");
                assert_eq!(content, "72 degrees");
            }
            other => panic!("expected ToolResult, got {:?}", other),
        }
    }

    // ── Message conversion: full pipeline ───────────────────────────

    #[test]
    fn convert_messages_system_extracted_to_top_level() {
        let messages = vec![
            Message::system("You are a helpful assistant."),
            Message::user("Hello"),
        ];
        let (system, anthropic_msgs) = AnthropicClient::convert_messages(messages);
        match system {
            Some(SystemPrompt::Text(text)) => {
                assert_eq!(text, "You are a helpful assistant.");
            }
            other => panic!("expected Text system prompt, got {:?}", other),
        }
        assert_eq!(anthropic_msgs.len(), 1);
        assert_eq!(anthropic_msgs[0].role, "user");
    }

    #[test]
    fn convert_messages_merges_adjacent_user_messages() {
        let messages = vec![
            Message::user("First question."),
            Message::user("Second question."),
        ];
        let (_, anthropic_msgs) = AnthropicClient::convert_messages(messages);
        // Should be merged into one user message with 2 content blocks
        assert_eq!(anthropic_msgs.len(), 1);
        assert_eq!(anthropic_msgs[0].role, "user");
        assert_eq!(anthropic_msgs[0].content.len(), 2);
        assert_eq!(
            anthropic_msgs[0].content[0].as_text(),
            Some("First question.")
        );
        assert_eq!(
            anthropic_msgs[0].content[1].as_text(),
            Some("Second question.")
        );
    }

    #[test]
    fn convert_messages_merges_user_and_tool_result() {
        // Simulate assistant with tool_use
        let mut assistant = Message::assistant("");
        assistant.tool_calls = Some(vec![ToolCallRequest {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            name: "get_weather".to_string(),
            arguments: HashMap::new(),
        }]);

        let messages = vec![
            Message::user("What's the weather?"),
            assistant,
            Message::tool("72 degrees", "call_1"),
        ];

        let (_, anthropic_msgs) = AnthropicClient::convert_messages(messages);
        // We get: user ("What's the weather?"), assistant (with tool_use), user (tool_result)
        // Tool results are role "user" but since previous message is assistant, no merge.
        assert_eq!(anthropic_msgs.len(), 3);
        assert_eq!(anthropic_msgs[0].role, "user");
        assert_eq!(anthropic_msgs[1].role, "assistant");
        assert_eq!(anthropic_msgs[2].role, "user");
        // The tool_result user message should contain the result
        assert!(anthropic_msgs[2].content.iter().any(|b| {
            matches!(b, ContentBlock::ToolResult { tool_use_id, .. } if tool_use_id == "call_1")
        }));
    }

    #[test]
    fn convert_messages_backfills_orphaned_tool_use() {
        let mut assistant = Message::assistant("");
        assistant.tool_calls = Some(vec![ToolCallRequest {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            name: "get_weather".to_string(),
            arguments: HashMap::new(),
        }]);

        let messages = vec![Message::user("Hi"), assistant];
        let (_, anthropic_msgs) = AnthropicClient::convert_messages(messages);

        // Should have user, assistant (with tool_use), and backfilled user (with tool_result)
        assert!(anthropic_msgs.len() >= 3);
        // Check that the backfilled tool_result exists
        let has_tool_result = anthropic_msgs.iter().any(|m| {
            m.role == "user"
                && m.content.iter().any(|b| {
                    matches!(b, ContentBlock::ToolResult { tool_use_id, .. } if tool_use_id == "call_1")
                })
        });
        assert!(has_tool_result, "orphaned tool_use should be backfilled");
    }

    // ── Tool definition conversion ──────────────────────────────────

    #[test]
    fn convert_tool_definition_openai_to_anthropic() {
        let raw = serde_json::json!({
            "type": "function",
            "function": {
                "name": "get_weather",
                "description": "Get the current weather",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "location": {"type": "string", "description": "City name"}
                    },
                    "required": ["location"]
                }
            }
        });
        let def = AnthropicClient::convert_tool_definition(&raw).unwrap();
        assert_eq!(def.name, "get_weather");
        assert_eq!(def.description, Some("Get the current weather".to_string()));
        assert!(def.input_schema.get("properties").is_some());
    }

    #[test]
    fn convert_tool_definition_returns_none_for_non_function() {
        let raw = serde_json::json!({"type": "unknown"});
        assert!(AnthropicClient::convert_tool_definition(&raw).is_none());
    }

    // ── Image helper tests ──────────────────────────────────────────

    #[test]
    fn extract_base64_from_data_uri_standard() {
        let result =
            AnthropicClient::extract_base64_from_data_uri("data:image/png;base64,iVBORw0KGgo");
        assert_eq!(result, Some("iVBORw0KGgo".to_string()));
    }

    #[test]
    fn extract_base64_from_data_uri_plain_base64() {
        let result = AnthropicClient::extract_base64_from_data_uri("/9j/4AAQSkZJRg");
        assert_eq!(result, Some("/9j/4AAQSkZJRg".to_string()));
    }

    #[test]
    fn guess_media_type_from_data_uri() {
        assert_eq!(
            AnthropicClient::guess_media_type("data:image/png;base64,xxx"),
            "image/png"
        );
        assert_eq!(
            AnthropicClient::guess_media_type("data:image/webp;base64,xxx"),
            "image/webp"
        );
    }

    #[test]
    fn guess_media_type_defaults_to_jpeg() {
        assert_eq!(
            AnthropicClient::guess_media_type("https://example.com/photo"),
            "image/jpeg"
        );
    }

    // ── Usage normalization ─────────────────────────────────────────

    #[test]
    fn anthropic_usage_to_normalized_usage() {
        let usage = AnthropicUsage {
            input_tokens: Some(500),
            cache_creation_input_tokens: Some(200),
            cache_read_input_tokens: Some(300),
            output_tokens: Some(400),
        };
        let u = usage.to_normalized_usage();
        assert_eq!(u.prompt_tokens, 1000); // 500 + 200 + 300
        assert_eq!(u.completion_tokens, 400);
        assert_eq!(u.total_tokens, 1400);
    }

    #[test]
    fn anthropic_usage_to_normalized_usage_no_cache() {
        let usage = AnthropicUsage {
            input_tokens: Some(100),
            output_tokens: Some(50),
            ..Default::default()
        };
        let u = usage.to_normalized_usage();
        assert_eq!(u.prompt_tokens, 100);
        assert_eq!(u.completion_tokens, 50);
        assert_eq!(u.total_tokens, 150);
    }
}
