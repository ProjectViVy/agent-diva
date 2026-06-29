use crate::summary_compaction::{SummaryChain, SummaryEngine};
use crate::summary_quality::SummaryQualityGate;
use agent_diva_providers::{
    provider_error_indicates_context_overflow as provider_context_overflow, Message,
    MessageContent, MessageContentPart, ProviderError,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBudgetPolicy {
    pub context_budget_tokens: usize,
    pub reserve_tokens: usize,
    pub overflow_retry_enabled: bool,
}

impl ContextBudgetPolicy {
    pub fn available_context_tokens(&self) -> usize {
        self.context_budget_tokens
            .saturating_sub(self.reserve_tokens)
            .max(1)
    }

    pub const fn history_probe_messages(&self) -> usize {
        200
    }

    pub fn overflow_user_message(&self) -> &'static str {
        "The conversation context is too large for this model. I automatically shrank it once, but it still did not fit. Please start a fresh session or shorten the request."
    }
}

impl Default for ContextBudgetPolicy {
    fn default() -> Self {
        Self {
            context_budget_tokens: 24_000,
            reserve_tokens: 4_000,
            overflow_retry_enabled: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompactionMode {
    Normal,
    OverflowRecovery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextBudgetReport {
    pub mode: CompactionMode,
    pub estimated_tokens_before: usize,
    pub estimated_tokens_after: usize,
    pub available_context_tokens: usize,
    pub removed_history_messages: usize,
    pub truncated_tool_messages: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemPromptBudgetReport {
    pub estimated_tokens: usize,
    pub reserve_tokens: usize,
    pub overflow_tokens: usize,
    pub prompt_chars: usize,
}

impl SystemPromptBudgetReport {
    pub fn exceeds_reserved_budget(&self) -> bool {
        self.overflow_tokens > 0
    }
}

/// Configuration for LLM-based summary compaction.
///
/// Controls whether `compact_with_summary` attempts to summarise older
/// conversation history using an LLM before falling back to message
/// truncation.
#[derive(Debug, Clone)]
pub struct SummaryCompactionConfig {
    /// Whether LLM-based summary compaction is enabled.
    pub llm_summary_enabled: bool,
    /// Minimum quality threshold (0.0 – 1.0) for accepting a summary.
    /// Only used when a [`SummaryQualityGate`] is also provided.
    pub quality_threshold: f64,
    /// Maximum number of retries for summary generation.
    pub max_retries: u32,
}

impl Default for SummaryCompactionConfig {
    fn default() -> Self {
        Self {
            llm_summary_enabled: true,
            quality_threshold: 0.6,
            max_retries: 2,
        }
    }
}

pub fn compact_messages_to_budget(
    messages: &[Message],
    tool_defs: &[serde_json::Value],
    policy: &ContextBudgetPolicy,
    mode: CompactionMode,
) -> (Vec<Message>, ContextBudgetReport) {
    let available_context_tokens = policy.available_context_tokens();
    let estimated_tokens_before = estimate_request_tokens(messages, tool_defs);
    let mut compacted = messages.to_vec();
    let mut truncated_tool_messages = 0;

    let tool_char_limit = match mode {
        CompactionMode::Normal => 12_000,
        CompactionMode::OverflowRecovery => 4_000,
    };

    for message in &mut compacted {
        if message.role == "tool" && trim_message_text(message, tool_char_limit) {
            truncated_tool_messages += 1;
        }
    }

    let mut estimated_tokens_after = estimate_request_tokens(&compacted, tool_defs);
    let mut removed_history_messages = 0;
    while estimated_tokens_after > available_context_tokens {
        let Some(index) = oldest_removable_index(&compacted, mode) else {
            break;
        };
        compacted.remove(index);
        removed_history_messages += 1;
        estimated_tokens_after = estimate_request_tokens(&compacted, tool_defs);
    }

    (
        compacted,
        ContextBudgetReport {
            mode,
            estimated_tokens_before,
            estimated_tokens_after,
            available_context_tokens,
            removed_history_messages,
            truncated_tool_messages,
        },
    )
}

/// Compact messages using LLM summarization before falling back to truncation.
///
/// When LLM summary compaction is enabled and a [`SummaryEngine`] is provided,
/// this function attempts to replace older conversation history with a
/// concise LLM-generated summary. If the optional [`SummaryQualityGate`] is
/// given, the summary is validated before it is accepted.
///
/// On LLM failure, empty results, quality rejection, or when summarization
/// alone does not bring the message list within budget, the function falls
/// back to the standard [`compact_messages_to_budget`] truncation strategy.
pub async fn compact_with_summary(
    messages: &[Message],
    tool_defs: &[serde_json::Value],
    policy: &ContextBudgetPolicy,
    mode: CompactionMode,
    config: Option<&SummaryCompactionConfig>,
    engine: Option<&SummaryEngine>,
    quality_gate: Option<&SummaryQualityGate>,
) -> (Vec<Message>, ContextBudgetReport) {
    let estimated_tokens_before = estimate_request_tokens(messages, tool_defs);
    let available = policy.available_context_tokens();

    // No compaction needed at all.
    if estimated_tokens_before <= available {
        return (
            messages.to_vec(),
            ContextBudgetReport {
                mode,
                estimated_tokens_before,
                estimated_tokens_after: estimated_tokens_before,
                available_context_tokens: available,
                removed_history_messages: 0,
                truncated_tool_messages: 0,
            },
        );
    }

    // Check whether LLM summary is configured and available.
    let llm_summary_enabled = config
        .map(|c| c.llm_summary_enabled)
        .unwrap_or(false);

    if llm_summary_enabled {
        let Some(engine) = engine else { return (messages.to_vec(), fallback_report); };

        let (summarize_start, summarize_end) =
            find_summarizable_range(messages, mode);

        if summarize_start < summarize_end {
            let to_summarize = &messages[summarize_start..summarize_end];

            match engine.summarize(to_summarize).await {
                Ok(Some(summary)) => {
                    // Quality gate check.
                    let quality_ok = quality_gate
                        .map(|gate| gate.evaluate(&summary, to_summarize).passed)
                        .unwrap_or(true);

                    if quality_ok {
                        let mut compacted =
                            Vec::with_capacity(2 + messages.len() - summarize_end);
                        compacted.push(messages[0].clone()); // system
                        compacted.push(Message::user(format!(
                            "[Summary of previous context]: {}",
                            summary.content
                        )));
                        compacted.extend_from_slice(&messages[summarize_end..]);

                        let estimated_after =
                            estimate_request_tokens(&compacted, tool_defs);

                        if estimated_after <= available {
                            return (
                                compacted,
                                ContextBudgetReport {
                                    mode,
                                    estimated_tokens_before,
                                    estimated_tokens_after: estimated_after,
                                    available_context_tokens: available,
                                    removed_history_messages: 0,
                                    truncated_tool_messages: 0,
                                },
                            );
                        }
                    }
                }
                Ok(None) | Err(_) => {
                    // Fall through to truncation fallback.
                }
            }
        }
    }

    // Fall back to standard truncation.
    compact_messages_to_budget(messages, tool_defs, policy, mode)
}

/// Determine the range `(start, end)` of messages that are safe to
/// summarise (excludes the system message and a protected tail of recent
/// messages).
fn find_summarizable_range(messages: &[Message], mode: CompactionMode) -> (usize, usize) {
    if messages.len() <= 3 {
        return (0, 0);
    }

    let protected_tail = match mode {
        CompactionMode::Normal => 3,
        CompactionMode::OverflowRecovery => 1,
    };

    // End index (exclusive): leave `protected_tail` + the last message
    // untouched.
    let end = messages.len().saturating_sub(1 + protected_tail);
    if end > 1 {
        (1, end)
    } else {
        (0, 0)
    }
}

pub fn estimate_request_tokens(messages: &[Message], tool_defs: &[serde_json::Value]) -> usize {
    let message_tokens: usize = messages.iter().map(estimate_message_tokens).sum();
    let tool_tokens: usize = tool_defs.iter().map(estimate_serialized_tokens).sum();
    message_tokens + tool_tokens
}

/// Measurement strategy for token estimation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementStrategy {
    /// Simple char/4 + 2 heuristic estimation.
    CharsDiv4,
    /// Use tiktoken-rs for accurate tokenisation (requires `tiktoken` feature).
    #[cfg(feature = "tiktoken")]
    Tiktoken,
}

impl Default for MeasurementStrategy {
    fn default() -> Self {
        #[cfg(feature = "tiktoken")]
        { MeasurementStrategy::Tiktoken }
        #[cfg(not(feature = "tiktoken"))]
        { MeasurementStrategy::CharsDiv4 }
    }
}

/// Measure the system prompt and produce a budget report.
///
/// `strategy` controls how the prompt is measured:
/// - [`MeasurementStrategy::CharsDiv4`] uses the heuristic `chars/4 + 2`.
/// - [`MeasurementStrategy::Tiktoken`] (requires `tiktoken` feature) uses
///   `tiktoken-rs` with `cl100k_base` encoding.
pub fn measure_system_prompt_budget(
    prompt: &str,
    policy: &ContextBudgetPolicy,
    strategy: MeasurementStrategy,
) -> SystemPromptBudgetReport {
    let estimated_tokens = match strategy {
        MeasurementStrategy::CharsDiv4 => estimate_text_tokens(prompt),
        #[cfg(feature = "tiktoken")]
        MeasurementStrategy::Tiktoken => measure_tiktoken(prompt),
    };
    let overflow_tokens = estimated_tokens.saturating_sub(policy.reserve_tokens);

    SystemPromptBudgetReport {
        estimated_tokens,
        reserve_tokens: policy.reserve_tokens,
        overflow_tokens,
        prompt_chars: prompt.chars().count(),
    }
}

/// Tokenize text with tiktoken-rs, falling back to heuristic on error.
#[cfg(feature = "tiktoken")]
fn measure_tiktoken(text: &str) -> usize {
    tiktoken_rs::cl100k_base()
        .map(|bpe| bpe.encode_with_special_tokens(text).len())
        .unwrap_or_else(|_| estimate_text_tokens(text))
}

pub fn provider_error_indicates_context_overflow(error: &ProviderError) -> bool {
    provider_context_overflow(error)
}

fn oldest_removable_index(messages: &[Message], mode: CompactionMode) -> Option<usize> {
    if messages.len() <= 2 {
        return None;
    }

    let protected_tail_non_system = match mode {
        CompactionMode::Normal => 3,
        CompactionMode::OverflowRecovery => 1,
    };

    let mut protected = vec![false; messages.len()];
    protected[0] = true;
    protected[messages.len() - 1] = true;

    let mut protected_count = 0;
    for index in (0..messages.len().saturating_sub(1)).rev() {
        if messages[index].role == "system" {
            protected[index] = true;
            continue;
        }
        if protected_count < protected_tail_non_system {
            protected[index] = true;
            protected_count += 1;
        } else {
            break;
        }
    }

    (1..messages.len().saturating_sub(1)).find(|index| {
        let message = &messages[*index];
        !protected[*index] && message.role != "system"
    })
}

fn trim_message_text(message: &mut Message, max_chars: usize) -> bool {
    match &mut message.content {
        MessageContent::Text(text) => trim_text(text, max_chars),
        MessageContent::Parts(parts) => {
            let mut changed = false;
            for part in parts {
                if let MessageContentPart::Text { text } = part {
                    changed |= trim_text(text, max_chars);
                }
            }
            changed
        }
    }
}

fn trim_text(text: &mut String, max_chars: usize) -> bool {
    let char_count = text.chars().count();
    if char_count <= max_chars {
        return false;
    }

    let head_chars = max_chars.saturating_sub(96);
    let mut trimmed: String = text.chars().take(head_chars).collect();
    trimmed.push_str(&format!(
        "\n...[context budget trimmed {} chars]...",
        char_count.saturating_sub(max_chars)
    ));
    *text = trimmed;
    true
}

fn estimate_message_tokens(message: &Message) -> usize {
    let base = 12;
    let content_tokens = estimate_content_tokens(&message.content);
    let name_tokens = message
        .name
        .as_deref()
        .map(estimate_text_tokens)
        .unwrap_or(0);
    let tool_call_id_tokens = message
        .tool_call_id
        .as_deref()
        .map(estimate_text_tokens)
        .unwrap_or(0);
    let tool_calls_tokens = message
        .tool_calls
        .as_ref()
        .map(|calls| {
            calls
                .iter()
                .map(|call| {
                    let mut tokens = estimate_text_tokens(&call.id)
                        + estimate_text_tokens(&call.call_type)
                        + estimate_text_tokens(&call.name);
                    tokens += estimate_serialized_tokens(&call.arguments);
                    tokens
                })
                .sum::<usize>()
        })
        .unwrap_or(0);
    let reasoning_tokens = message
        .reasoning_content
        .as_deref()
        .map(estimate_text_tokens)
        .unwrap_or(0);
    let thinking_tokens = message
        .thinking_blocks
        .as_ref()
        .map(estimate_serialized_tokens)
        .unwrap_or(0);

    base + content_tokens
        + name_tokens
        + tool_call_id_tokens
        + tool_calls_tokens
        + reasoning_tokens
        + thinking_tokens
}

fn estimate_content_tokens(content: &MessageContent) -> usize {
    match content {
        MessageContent::Text(text) => estimate_text_tokens(text),
        MessageContent::Parts(parts) => parts
            .iter()
            .map(|part| match part {
                MessageContentPart::Text { text } => estimate_text_tokens(text),
                MessageContentPart::ImageUrl { image_url } => estimate_text_tokens(&image_url.url),
                MessageContentPart::ImageFile { image_file } => {
                    estimate_text_tokens(&image_file.file_id)
                }
                MessageContentPart::ImageData { image_data } => {
                    estimate_text_tokens(&image_data.data_uri)
                }
            })
            .sum(),
    }
}

fn estimate_serialized_tokens<T: serde::Serialize>(value: &T) -> usize {
    serde_json::to_string(value)
        .map(|json| estimate_text_tokens(&json))
        .unwrap_or(64)
}

fn estimate_text_tokens(text: &str) -> usize {
    let chars = text.chars().count();
    (chars / 4).max(1) + 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_providers::{ImageFile, ToolCallRequest};
    use std::collections::HashMap;

    #[test]
    fn compact_messages_trims_tool_results_before_dropping_history() {
        let long_tool_output = "x".repeat(20_000);
        let messages = vec![
            Message::system("system"),
            Message::user("old user"),
            Message::assistant("old assistant"),
            Message::tool(long_tool_output, "call-1"),
            Message::user("current user"),
        ];
        let policy = ContextBudgetPolicy {
            context_budget_tokens: 3_000,
            reserve_tokens: 500,
            overflow_retry_enabled: true,
        };

        let (compacted, report) =
            compact_messages_to_budget(&messages, &[], &policy, CompactionMode::Normal);

        assert!(report.truncated_tool_messages >= 1);
        assert_eq!(
            compacted.last().unwrap().content.as_text(),
            Some("current user")
        );
    }

    #[test]
    fn compact_messages_drops_oldest_history_first() {
        let messages = vec![
            Message::system("system"),
            Message::user("user-1"),
            Message::assistant("assistant-1"),
            Message::user("user-2"),
            Message::assistant("assistant-2"),
            Message::user("current"),
        ];
        let policy = ContextBudgetPolicy {
            context_budget_tokens: 40,
            reserve_tokens: 10,
            overflow_retry_enabled: true,
        };

        let (compacted, report) =
            compact_messages_to_budget(&messages, &[], &policy, CompactionMode::OverflowRecovery);

        assert!(report.removed_history_messages >= 1);
        assert!(!compacted
            .iter()
            .any(|message| message.content.as_text() == Some("user-1")));
        assert_eq!(compacted.last().unwrap().content.as_text(), Some("current"));
    }

    #[test]
    fn estimate_request_tokens_counts_tool_defs_and_calls() {
        let mut call_args = HashMap::new();
        call_args.insert("path".to_string(), serde_json::json!("README.md"));
        let mut assistant = Message::assistant("using tool");
        assistant.tool_calls = Some(vec![ToolCallRequest {
            id: "call-1".to_string(),
            call_type: "function".to_string(),
            name: "read_file".to_string(),
            arguments: call_args,
        }]);
        let messages = vec![
            Message::system("system"),
            assistant,
            Message::user(MessageContent::Parts(vec![
                MessageContentPart::Text {
                    text: "look".to_string(),
                },
                MessageContentPart::ImageFile {
                    image_file: ImageFile {
                        file_id: "sha256:image".to_string(),
                    },
                },
            ])),
        ];
        let tool_defs = vec![serde_json::json!({
            "type": "function",
            "function": {"name": "read_file", "parameters": {"type": "object"}}
        })];

        assert!(estimate_request_tokens(&messages, &tool_defs) > 0);
    }

    #[test]
    fn provider_error_detects_context_overflow() {
        assert!(provider_error_indicates_context_overflow(&ProviderError::api_message(
            "This model's maximum context length is 8192 tokens, however you requested 12000 tokens".to_string()
        )));
        assert!(provider_error_indicates_context_overflow(
            &ProviderError::InvalidResponse(
                "prompt is too long; reduce the length and retry".to_string()
            )
        ));
        assert!(!provider_error_indicates_context_overflow(
            &ProviderError::api_message("rate limit exceeded".to_string())
        ));
    }

    #[test]
    fn measure_system_prompt_budget_uses_rendered_prompt_size() {
        let policy = ContextBudgetPolicy {
            context_budget_tokens: 1_000,
            reserve_tokens: 8,
            overflow_retry_enabled: true,
        };

        let report =
            measure_system_prompt_budget("abcd efgh ijkl", &policy, MeasurementStrategy::CharsDiv4);

        assert_eq!(
            report.estimated_tokens,
            estimate_text_tokens("abcd efgh ijkl")
        );
        assert_eq!(report.reserve_tokens, 8);
        assert_eq!(report.prompt_chars, "abcd efgh ijkl".chars().count());
        assert!(!report.exceeds_reserved_budget());
    }

    #[test]
    fn measure_system_prompt_budget_marks_reserved_overflow() {
        let policy = ContextBudgetPolicy {
            context_budget_tokens: 1_000,
            reserve_tokens: 3,
            overflow_retry_enabled: true,
        };

        let report = measure_system_prompt_budget(
            "abcdefghijklmno",
            &policy,
            MeasurementStrategy::CharsDiv4,
        );

        assert!(report.exceeds_reserved_budget());
        assert!(report.overflow_tokens > 0);
    }

    #[test]
    fn measurement_strategy_default_is_supported() {
        let policy = ContextBudgetPolicy {
            context_budget_tokens: 1_000,
            reserve_tokens: 8,
            overflow_retry_enabled: true,
        };

        // Default strategy must never panic.
        let report = measure_system_prompt_budget(
            "hello world",
            &policy,
            MeasurementStrategy::default(),
        );

        assert!(report.estimated_tokens > 0);
        assert_eq!(report.prompt_chars, "hello world".chars().count());
    }

    #[cfg(feature = "tiktoken")]
    #[test]
    fn measure_system_prompt_budget_with_tiktoken() {
        let policy = ContextBudgetPolicy {
            context_budget_tokens: 1_000,
            reserve_tokens: 128,
            overflow_retry_enabled: true,
        };

        let report =
            measure_system_prompt_budget("hello world", &policy, MeasurementStrategy::Tiktoken);

        // tiktoken should produce a reasonable token count for "hello world".
        assert!(report.estimated_tokens > 0);
        assert!(report.estimated_tokens < 10, "expected ~2 tokens, got {}", report.estimated_tokens);
        assert_eq!(report.prompt_chars, "hello world".chars().count());
    }

    // ── compact_with_summary tests ──────────────────────────────────────

    use crate::summary_compaction::SummaryEngine;
    use agent_diva_providers::{LLMProvider, LLMResponse, ProviderResult};
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    /// A minimal [`LLMProvider`] for testing summary compaction.
    struct MockSummaryProvider {
        succeed: bool,
        call_count: AtomicU32,
    }

    #[async_trait]
    impl LLMProvider for MockSummaryProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            if self.succeed {
                Ok(LLMResponse {
                    content: Some(
                        "Summary: discussed important keywords like conversation and context"
                            .to_string(),
                    ),
                    tool_calls: vec![],
                    finish_reason: "stop".to_string(),
                    usage: None,
                    reasoning_content: None,
                })
            } else {
                Err(ProviderError::Permanent {
                    message: "mock provider failure".to_string(),
                })
            }
        }

        fn get_default_model(&self) -> String {
            "mock".to_string()
        }
    }

    #[tokio::test]
    async fn test_compact_with_summary_succeeds() {
        let engine = SummaryEngine::new(Arc::new(MockSummaryProvider {
            succeed: true,
            call_count: AtomicU32::new(0),
        }));

        // 8 messages: range (1, 4) summarizable → summarized to 6 messages.
        let messages = vec![
            Message::system("sys"),
            Message::user("old message with lots of verbose text about important topics"),
            Message::assistant("old response discussing key points with extra detail"),
            Message::user("another old verbose message with lengthy conversation"),
            Message::assistant("more recent response discussing things"),
            Message::user("current user query"),
            Message::assistant("current assistant reply"),
            Message::user("latest message at the end"),
        ];

        // Budget just tight enough that original overflows but summary fits.
        // Original estimate ~179 tokens, available = 170 → compaction needed.
        // After summary: ~135 tokens, 135 < 170 → fits.
        let policy = ContextBudgetPolicy {
            context_budget_tokens: 200,
            reserve_tokens: 30,
            overflow_retry_enabled: true,
        };

        let config = SummaryCompactionConfig::default();

        let (compacted, report) = compact_with_summary(
            &messages,
            &[],
            &policy,
            CompactionMode::Normal,
            Some(&config),
            Some(&engine),
            None,
        )
        .await;

        // Should have used summary path: no truncation happened.
        assert_eq!(
            report.removed_history_messages, 0,
            "summary path should not report removed messages"
        );
        // The summary message should be present in the output.
        assert!(
            compacted.iter().any(|m| m
                .content
                .as_text()
                .map_or(false, |t| t.contains("Summary"))),
            "compacted messages should contain the summary text"
        );
    }

    #[tokio::test]
    async fn test_compact_fallback_on_llm_failure() {
        let engine = SummaryEngine::new(Arc::new(MockSummaryProvider {
            succeed: false,
            call_count: AtomicU32::new(0),
        }));

        // 6 messages: range (1, 2) summarizable → fails → falls back.
        let messages = vec![
            Message::system("sys"),
            Message::user("old verbose message that takes up lots of tokens to overflow easily"),
            Message::assistant("old detailed response with extra explanations and content"),
            Message::user("another verbose user message with plenty of detail"),
            Message::assistant("detailed response with additional content"),
            Message::user("current user message"),
        ];

        let policy = ContextBudgetPolicy {
            context_budget_tokens: 40,
            reserve_tokens: 10,
            overflow_retry_enabled: true,
        };

        let config = SummaryCompactionConfig::default();

        let (compacted, report) = compact_with_summary(
            &messages,
            &[],
            &policy,
            CompactionMode::Normal,
            Some(&config),
            Some(&engine),
            None,
        )
        .await;

        // Should have fallen back to truncation.
        assert!(
            report.removed_history_messages > 0,
            "fallback should truncate messages"
        );
        assert_eq!(
            compacted.last().unwrap().content.as_text(),
            Some("current user message")
        );
    }

    #[tokio::test]
    async fn test_compact_with_summary_no_config_falls_back() {
        let engine = SummaryEngine::new(Arc::new(MockSummaryProvider {
            succeed: true,
            call_count: AtomicU32::new(0),
        }));

        // 6 messages so Normal-mode truncation can actually remove items.
        let messages = vec![
            Message::system("sys"),
            Message::user("old verbose message that uses many tokens"),
            Message::assistant("old response with detailed explanation"),
            Message::user("another verbose message with lots of content"),
            Message::assistant("detailed response with extra info"),
            Message::user("current user message"),
        ];

        let policy = ContextBudgetPolicy {
            context_budget_tokens: 40,
            reserve_tokens: 10,
            overflow_retry_enabled: true,
        };

        // Passing None for config should skip summary and fall back.
        let (_compacted, report) = compact_with_summary(
            &messages,
            &[],
            &policy,
            CompactionMode::Normal,
            None,
            Some(&engine),
            None,
        )
        .await;

        assert!(
            report.removed_history_messages > 0,
            "no config should fall back to truncation"
        );
    }

    #[tokio::test]
    async fn test_compact_with_summary_no_engine_falls_back() {
        // 6 messages so Normal-mode truncation can actually remove items.
        let messages = vec![
            Message::system("sys"),
            Message::user("old verbose message that uses many tokens"),
            Message::assistant("old response with detailed explanation"),
            Message::user("another verbose message with lots of content"),
            Message::assistant("detailed response with extra info"),
            Message::user("current user message"),
        ];

        let policy = ContextBudgetPolicy {
            context_budget_tokens: 40,
            reserve_tokens: 10,
            overflow_retry_enabled: true,
        };

        let config = SummaryCompactionConfig::default();

        let (_compacted, report) = compact_with_summary(
            &messages,
            &[],
            &policy,
            CompactionMode::Normal,
            Some(&config),
            None, // no engine
            None,
        )
        .await;

        assert!(
            report.removed_history_messages > 0,
            "no engine should fall back to truncation"
        );
    }

    #[tokio::test]
    async fn test_compact_with_summary_no_compaction_needed() {
        let engine = SummaryEngine::new(Arc::new(MockSummaryProvider {
            succeed: true,
            call_count: AtomicU32::new(0),
        }));

        let messages = vec![
            Message::system("system"),
            Message::user("hi"),
            Message::assistant("hello"),
        ];

        let policy = ContextBudgetPolicy {
            context_budget_tokens: 10_000,
            reserve_tokens: 1_000,
            overflow_retry_enabled: true,
        };

        let config = SummaryCompactionConfig::default();

        let (compacted, report) = compact_with_summary(
            &messages,
            &[],
            &policy,
            CompactionMode::Normal,
            Some(&config),
            Some(&engine),
            None,
        )
        .await;

        // No compaction needed.
        assert_eq!(report.removed_history_messages, 0);
        assert_eq!(compacted.len(), messages.len());
    }
}
