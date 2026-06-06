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

pub fn estimate_request_tokens(messages: &[Message], tool_defs: &[serde_json::Value]) -> usize {
    let message_tokens: usize = messages.iter().map(estimate_message_tokens).sum();
    let tool_tokens: usize = tool_defs.iter().map(estimate_serialized_tokens).sum();
    message_tokens + tool_tokens
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
        assert!(provider_error_indicates_context_overflow(&ProviderError::ApiError(
            "This model's maximum context length is 8192 tokens, however you requested 12000 tokens".to_string()
        )));
        assert!(provider_error_indicates_context_overflow(
            &ProviderError::InvalidResponse(
                "prompt is too long; reduce the length and retry".to_string()
            )
        ));
        assert!(!provider_error_indicates_context_overflow(
            &ProviderError::ApiError("rate limit exceeded".to_string())
        ));
    }
}
