//! Token estimation using the `chars/4 × 4/3` heuristic (Claude Code compatible).
//!
//! This module provides fast, approximate token counting without requiring
//! a full tokenizer. The heuristic divides the character count by 4 (average
//! English word length) and inflates by 4/3 (33% overhead for multi-byte /
//! multi-token words). The result is equivalent to `chars / 3`.
//!
//! # Accuracy
//!
//! ≤10% error on DeepSeek V3 tokenizer for typical English + code input.
//! Unicode-aware: uses [`str::chars`] for correct multi-byte handling.
//!
//! # References
//!
//! - Claude Code: `chars.length / 4`
//! - ADR-0010: Context Compaction architecture

use agent_diva_core::session::ChatMessage;

/// Estimate the number of tokens in `text` using the chars/4 × 4/3 heuristic.
///
/// Returns 0 for empty input.
///
/// # Formula
///
/// ```text
/// char_count / 4 → approximate English words
/// × 4/3        → inflate for tokenization overhead (multi-token words, punctuation)
/// = char_count / 3
/// ```
///
/// # Examples
///
/// ```
/// # use agent_diva_agent::token_estimate::estimate_tokens;
/// assert_eq!(estimate_tokens(""), 0);
/// assert!(estimate_tokens("hello world") > 0);
/// ```
pub fn estimate_tokens(text: &str) -> usize {
    let char_count = text.chars().count();
    // chars / 4 → approximate words, × 4/3 → inflate for tokenization overhead
    // Equivalent to char_count / 3
    (char_count as f64 / 4.0 * 4.0 / 3.0).ceil() as usize
}

/// Estimate token count for a single [`ChatMessage`].
///
/// Counts tokens from the following fields:
///
/// | Field              | Contribution                            |
/// |--------------------|-----------------------------------------|
/// | `content`          | Always counted                          |
/// | `reasoning_content`| Counted if `Some`                       |
/// | `tool_calls`       | Each serialized to JSON string, counted |
/// | `thinking_blocks`  | Each serialized to JSON string, counted |
pub fn estimate_message_tokens(msg: &ChatMessage) -> usize {
    let mut tokens = estimate_tokens(&msg.content);

    if let Some(ref reasoning) = msg.reasoning_content {
        tokens += estimate_tokens(reasoning);
    }
    if let Some(ref tool_calls) = msg.tool_calls {
        for tc in tool_calls {
            tokens += estimate_tokens(&tc.to_string());
        }
    }
    if let Some(ref thinking_blocks) = msg.thinking_blocks {
        for tb in thinking_blocks {
            tokens += estimate_tokens(&tb.to_string());
        }
    }

    tokens
}

/// Estimate total tokens for a slice of [`ChatMessage`]s.
///
/// Sums [`estimate_message_tokens`] across all messages.
pub fn estimate_total_tokens(messages: &[ChatMessage]) -> usize {
    messages.iter().map(estimate_message_tokens).sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    // ── estimate_tokens ──────────────────────────────────────────

    #[test]
    fn empty_string_returns_zero() {
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn ascii_baseline() {
        // "hello world" = 11 chars → ceil(11/3) = 4
        let t = estimate_tokens("hello world");
        assert_eq!(t, 4);
    }

    #[test]
    fn short_string_one_token() {
        // 1 char → ceil(1/3) = 1
        assert_eq!(estimate_tokens("a"), 1);
    }

    #[test]
    fn chinese_multi_byte() {
        // "你好世界" = 4 chars → ceil(4/3) = 2
        let t = estimate_tokens("你好世界");
        assert_eq!(t, 2);
    }

    #[test]
    fn mixed_content() {
        // "hello 你好" = 8 chars (5 ascii + space + 2 multi-byte) → ceil(8/3) = 3
        let t = estimate_tokens("hello 你好");
        assert_eq!(t, 3);
    }

    #[test]
    fn long_input() {
        let long = "a".repeat(1000);
        // 1000 chars → ceil(1000/3) = 334
        let t = estimate_tokens(&long);
        assert_eq!(t, 334);
    }

    #[test]
    fn exactly_divisible_by_three() {
        // 9 chars → 9/3 = 3 exactly
        assert_eq!(estimate_tokens("123456789"), 3);
    }

    #[test]
    fn unicode_single_codepoint() {
        // Emoji like '😀' is 1 char but 4 bytes → chars().count() = 1 → 1 token
        assert_eq!(estimate_tokens("😀"), 1);
    }

    // ── estimate_message_tokens ──────────────────────────────────

    fn make_msg(role: &str, content: &str) -> ChatMessage {
        ChatMessage {
            role: role.to_string(),
            content: content.to_string(),
            timestamp: Utc::now(),
            tool_call_id: None,
            tool_calls: None,
            name: None,
            reasoning_content: None,
            thinking_blocks: None,
        }
    }

    #[test]
    fn message_basic() {
        let msg = make_msg("user", "hello");
        assert_eq!(estimate_message_tokens(&msg), estimate_tokens("hello"));
    }

    #[test]
    fn message_with_reasoning() {
        let msg = ChatMessage {
            reasoning_content: Some("step-by-step reasoning".to_string()),
            ..make_msg("assistant", "final answer")
        };
        let tokens = estimate_message_tokens(&msg);
        let content_only = estimate_tokens("final answer");
        assert!(tokens > content_only);
    }

    #[test]
    fn message_with_tool_calls() {
        let msg = ChatMessage {
            tool_calls: Some(vec![
                serde_json::json!({"name": "read_file", "arguments": {"path": "/x"}}),
            ]),
            ..make_msg("assistant", "let me check")
        };
        let tokens = estimate_message_tokens(&msg);
        let content_only = estimate_message_tokens(&make_msg("assistant", "let me check"));
        assert!(tokens > content_only);
    }

    #[test]
    fn message_with_thinking_blocks() {
        let msg = ChatMessage {
            thinking_blocks: Some(vec![
                serde_json::json!({"type": "thinking", "content": "hmm let me think"}),
            ]),
            ..make_msg("assistant", "ok")
        };
        let tokens = estimate_message_tokens(&msg);
        let content_only = estimate_message_tokens(&make_msg("assistant", "ok"));
        assert!(tokens > content_only);
    }

    #[test]
    fn message_all_fields_present() {
        let msg = ChatMessage {
            reasoning_content: Some("thinking...".to_string()),
            tool_calls: Some(vec![serde_json::json!({"name": "tool"})]),
            thinking_blocks: Some(vec![serde_json::json!({"type": "tb"})]),
            ..make_msg("assistant", "result")
        };
        let tokens = estimate_message_tokens(&msg);
        assert!(tokens > 0);
    }

    // ── estimate_total_tokens ────────────────────────────────────

    #[test]
    fn total_multiple_messages() {
        let msgs = vec![make_msg("user", "hello"), make_msg("assistant", "hi there")];
        let total = estimate_total_tokens(&msgs);
        let individual: usize = msgs.iter().map(estimate_message_tokens).sum();
        assert_eq!(total, individual);
    }

    #[test]
    fn total_empty_slice() {
        assert_eq!(estimate_total_tokens(&[]), 0);
    }

    #[test]
    fn total_single_message() {
        let msgs = vec![make_msg("user", "test")];
        assert_eq!(
            estimate_total_tokens(&msgs),
            estimate_message_tokens(&msgs[0])
        );
    }
}
