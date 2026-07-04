//! Context compaction — generate a summary of old messages to free context budget.
//!
//! The compactor selects messages to compact (everything except the recent tail),
//! calls the LLM with a structured compaction prompt, and produces a
//! `CompactSummary` that is stored in the session.

use agent_diva_core::session::{
    ChatMessage, CompactSummary, CompactTrigger, CompactionRange, Session,
};
use agent_diva_providers::{LLMProvider, Message};
use chrono::Utc;
use std::sync::Arc;
use tracing::{info, warn};

use super::meta::MetaCompactor;
use super::prompt::{COMPACTION_SYSTEM_PROMPT, PRIOR_SUMMARIES_PREFIX};
use super::quality::{validate_summary, QualityGate};
use crate::context_budget::BudgetConfig;
use crate::token_estimate::estimate_total_tokens;

// ---------------------------------------------------------------------------
// ContextCompactor
// ---------------------------------------------------------------------------

/// Executes context compaction by calling an LLM to summarize old messages.
pub struct ContextCompactor;

/// Result of a successful compaction.
#[derive(Debug)]
pub struct CompactionResult {
    /// The generated summary record
    pub summary: CompactSummary,
    /// New value for `session.last_compacted`
    pub new_compacted_index: usize,
}

impl ContextCompactor {
    /// Execute true LLM-driven compaction.
    ///
    /// 1. Selects messages in `[session.last_compacted .. messages.len() - keep_recent_count]`
    /// 2. Formats messages for the LLM
    /// 3. Calls the provider with [`COMPACTION_SYSTEM_PROMPT`]
    /// 4. Extracts `<summary>` from the LLM response
    /// 5. Validates summary quality, retries if below threshold (max 2 retries)
    /// 6. Returns a [`CompactionResult`] with a full [`CompactSummary`]
    ///
    /// # Errors
    ///
    /// Returns `Err` when the provider call fails or when the response is empty.
    /// Callers should log the error and continue (compaction is best-effort).
    pub async fn compact(
        session: &Session,
        config: &BudgetConfig,
        provider: Arc<dyn LLMProvider>,
        model: &str,
        trigger: CompactTrigger,
        prior_summaries: &[CompactSummary],
    ) -> Result<CompactionResult, anyhow::Error> {
        let keep = config.keep_recent_count.min(session.messages.len());
        let start = session.last_compacted;
        let end = session.messages.len().saturating_sub(keep);

        // Nothing to compact — return empty placeholder
        if start >= end {
            return Ok(CompactionResult {
                summary: CompactSummary {
                    schema_version: 1,
                    compact_id: String::new(),
                    created_at: String::new(),
                    trigger: trigger.clone(),
                    source_range: CompactionRange {
                        start_index: start,
                        end_index: end,
                    },
                    kept_recent_count: keep,
                    pre_compact_message_count: 0,
                    pre_compact_estimated_tokens: 0,
                    summary: String::new(),
                    quality_score: None,
                    retry_count: 0,
                },
                new_compacted_index: session.last_compacted,
            });
        }

        let range = &session.messages[start..end];
        let pre_compact_message_count = range.len();
        let pre_compact_estimated_tokens = estimate_total_tokens(range);

        info!(
            "Compacting {} messages (indices {}-{}, ~{} tokens)",
            pre_compact_message_count, start, end, pre_compact_estimated_tokens
        );

        // Format messages for the LLM
        let formatted = Self::format_messages_for_compaction(range);

        // Build prior summaries context (if any)
        // Apply meta-compaction if prior summaries exceed budget
        let mut prior_summary_texts: Vec<String> =
            prior_summaries.iter().map(|s| s.summary.clone()).collect();

        // Meta-compaction: keep prior summaries within a reasonable token budget.
        // The budget is a fraction of the overall context budget, reserved for
        // prior summaries.  Default: 10% of max_tokens, capped at 2000 tokens.
        let max_prior_summary_tokens = config.max_tokens / 10;
        let prior_tokens: usize = prior_summary_texts
            .iter()
            .map(|s| crate::token_estimate::estimate_tokens(s))
            .sum();

        if prior_tokens > max_prior_summary_tokens {
            info!(
                "Prior summaries token count ({} tokens) exceeds budget ({} tokens); triggering meta-compaction",
                prior_tokens, max_prior_summary_tokens
            );
            let quality_gate = QualityGate::default();
            let meta_compactor = MetaCompactor::new(quality_gate);
            if let Err(e) =
                meta_compactor.compact(&mut prior_summary_texts, max_prior_summary_tokens)
            {
                warn!("Meta-compaction failed: {}; falling back to truncation", e);
            }
        }

        let prior_context = if prior_summary_texts.is_empty() {
            String::new()
        } else {
            let combined = prior_summary_texts
                .iter()
                .enumerate()
                .map(|(i, s)| format!("[{}/{}] {}", i + 1, prior_summary_texts.len(), s))
                .collect::<Vec<_>>()
                .join("\n\n");
            format!(
                "{}\n",
                PRIOR_SUMMARIES_PREFIX.replace("{prior_summaries}", &combined)
            )
        };

        // Build the base user prompt
        let base_user_prompt = format!(
            "{}请压缩以下 {} 条对话消息：\n\n{}",
            prior_context, pre_compact_message_count, formatted
        );

        // Retry loop: up to 3 attempts (1 initial + 2 retries)
        const MAX_RETRIES: u32 = 2;
        const QUALITY_THRESHOLD: f64 = 0.6;

        let mut best_summary_text = String::new();
        let mut best_score: f64 = 0.0;
        let mut best_report_issues: Vec<String> = Vec::new();
        let mut attempts = 0u32;

        for attempt in 0..=MAX_RETRIES {
            attempts += 1;

            // On retry, prepend quality feedback to the user prompt
            let user_prompt = if attempt == 0 {
                base_user_prompt.clone()
            } else {
                format!(
                    "注意：上一次生成的摘要质量不合格（得分 {:.2}/1.0），原因：{}。\n请生成更详细、更完整的摘要，确保覆盖所有关键信息。\n\n{}",
                    best_score,
                    best_report_issues.join("；"),
                    base_user_prompt
                )
            };

            // Build messages for the LLM call
            let messages = vec![
                Message::system(COMPACTION_SYSTEM_PROMPT),
                Message::user(user_prompt),
            ];

            // Call LLM (non-streaming, synchronous compaction)
            let response = match provider
                .chat(messages, None, Some(model.to_string()), 4096, 0.3)
                .await
            {
                Ok(resp) => resp,
                Err(e) => {
                    warn!(
                        "LLM compaction call failed on attempt {}: {}",
                        attempt + 1,
                        e
                    );
                    continue;
                }
            };

            let response_text = response.content.unwrap_or_default();

            if response_text.trim().is_empty() {
                warn!(
                    "LLM returned empty compaction response on attempt {}",
                    attempt + 1
                );
                continue;
            }

            // Parse <summary> from response
            let summary_text = Self::extract_summary(&response_text);

            // Validate quality
            let report = validate_summary(&summary_text, range);

            info!(
                "Compaction attempt {}/{}: score={:.2} (len={:.2}, kw={:.2}, comp={:.2}), issues={:?}",
                attempt + 1,
                MAX_RETRIES + 1,
                report.score,
                report.length_score,
                report.keyword_score,
                report.completeness_score,
                report.issues
            );

            // Track the best attempt
            if report.score > best_score {
                best_score = report.score;
                best_summary_text = summary_text;
                best_report_issues = report.issues.clone();
            }

            // Early exit if quality is acceptable
            if report.score >= QUALITY_THRESHOLD {
                info!(
                    "Compaction quality acceptable on attempt {} (score {:.2} >= {})",
                    attempt + 1,
                    report.score,
                    QUALITY_THRESHOLD
                );
                break;
            }

            if attempt < MAX_RETRIES {
                info!(
                    "Compaction quality insufficient (score {:.2} < {}), retrying…",
                    report.score, QUALITY_THRESHOLD
                );
            }
        }

        if best_summary_text.is_empty() {
            return Err(anyhow::anyhow!(
                "All compaction attempts produced empty summaries"
            ));
        }

        let retry_count = attempts.saturating_sub(1);

        info!(
            "Compaction complete: {} messages → {} chars summary (score={:.2}, retries={})",
            pre_compact_message_count,
            best_summary_text.len(),
            best_score,
            retry_count
        );

        // Generate a unique compact_id
        let compact_id = format!(
            "compact-{}-{}",
            Utc::now().format("%Y%m%d-%H%M%S"),
            &uuid::Uuid::new_v4().to_string()[..8]
        );

        let summary = CompactSummary {
            schema_version: 1,
            compact_id,
            created_at: Utc::now().to_rfc3339(),
            trigger: trigger.clone(),
            source_range: CompactionRange {
                start_index: start,
                end_index: end,
            },
            kept_recent_count: keep,
            pre_compact_message_count,
            pre_compact_estimated_tokens,
            summary: best_summary_text,
            quality_score: Some(best_score),
            retry_count,
        };

        Ok(CompactionResult {
            summary,
            new_compacted_index: end,
        })
    }

    /// Format a slice of chat messages as a text block suitable for the LLM.
    fn format_messages_for_compaction(messages: &[ChatMessage]) -> String {
        let mut out = String::new();
        for (i, msg) in messages.iter().enumerate() {
            let role_label = match msg.role.as_str() {
                "user" => "用户",
                "assistant" => "助手",
                "tool" => "工具",
                "system" => "系统",
                other => other,
            };

            // Truncate very long messages to avoid blowing the compaction prompt
            let content = if msg.content.len() > 2000 {
                format!(
                    "{}…[truncated, {} chars total]",
                    &msg.content[..2000],
                    msg.content.len()
                )
            } else {
                msg.content.clone()
            };

            out.push_str(&format!("[{}. {}] {}\n", i + 1, role_label, content));
        }
        out
    }

    /// Extract the `<summary>...</summary>` section from the LLM response.
    ///
    /// Returns the inner text of the first `<summary>` tag pair.
    /// If no `<summary>` tags are found, returns the raw response text as-is
    /// (degraded mode — the prompt told the model to output structured format).
    fn extract_summary(response: &str) -> String {
        // Try to extract <summary>...</summary> using simple string search
        let start_tag = "<summary>";
        let end_tag = "</summary>";

        if let Some(start_pos) = response.find(start_tag) {
            let after_start = &response[start_pos + start_tag.len()..];
            if let Some(end_pos) = after_start.find(end_tag) {
                let summary = after_start[..end_pos].trim();
                if !summary.is_empty() {
                    return summary.to_string();
                }
            }
        }

        // Degraded mode: return the raw text (stripped of any trailing tags)
        warn!("Failed to extract <summary> from compaction response; using raw text");
        // Strip common tags from the raw response for a cleaner fallback
        let cleaned = response
            .replace("<analysis>", "")
            .replace("</analysis>", "")
            .replace("<summary>", "")
            .replace("</summary>", "")
            .trim()
            .to_string();
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::session::ChatMessage;
    use agent_diva_providers::{LLMProvider, LLMResponse, ProviderError, ProviderResult};
    use async_trait::async_trait;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

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
            token_usage: None,
        }
    }

    struct AlwaysFailProvider;

    #[async_trait]
    impl LLMProvider for AlwaysFailProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            Err(ProviderError::ApiError(
                "forced provider failure".to_string(),
            ))
        }

        fn get_default_model(&self) -> String {
            "mock-model".to_string()
        }
    }

    struct LowQualityProvider {
        call_count: AtomicUsize,
    }

    impl LowQualityProvider {
        fn new() -> Self {
            Self {
                call_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl LLMProvider for LowQualityProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            Ok(LLMResponse {
                content: Some("<summary>too short</summary>".to_string()),
                tool_calls: Vec::new(),
                finish_reason: "stop".to_string(),
                usage: HashMap::new(),
                reasoning_content: None,
            })
        }

        fn get_default_model(&self) -> String {
            "mock-model".to_string()
        }
    }

    #[test]
    fn test_extract_summary_normal() {
        let response = "<analysis>\n一些分析内容\n</analysis>\n<summary>\n这是摘要内容\n</summary>";
        let summary = ContextCompactor::extract_summary(response);
        assert_eq!(summary, "这是摘要内容");
    }

    #[test]
    fn test_extract_summary_multiline() {
        let response = "<analysis>分析</analysis>\n<summary>\n第一行\n第二行\n第三行\n</summary>";
        let summary = ContextCompactor::extract_summary(response);
        assert_eq!(summary, "第一行\n第二行\n第三行");
    }

    #[test]
    fn test_extract_summary_degraded() {
        let response = "这是一段没有标签的原始回复";
        let summary = ContextCompactor::extract_summary(response);
        // Degraded mode: returns the raw text
        assert!(!summary.is_empty());
        assert!(summary.contains("原始回复"));
    }

    #[test]
    fn test_extract_summary_empty_tags() {
        let response = "<analysis></analysis>\n<summary></summary>";
        let summary = ContextCompactor::extract_summary(response);
        // Empty tags with no content → result is empty (nothing to extract)
        assert!(
            summary.is_empty(),
            "Empty tags with no content should produce empty summary, got: {:?}",
            summary
        );
    }

    #[test]
    fn test_format_messages_for_compaction() {
        let msgs = vec![
            make_msg("user", "你好"),
            make_msg("assistant", "你好！有什么可以帮助你的？"),
            make_msg("user", "今天天气怎么样？"),
        ];
        let formatted = ContextCompactor::format_messages_for_compaction(&msgs);
        assert!(formatted.contains("[1. 用户]"));
        assert!(formatted.contains("[2. 助手]"));
        assert!(formatted.contains("[3. 用户]"));
        assert!(formatted.contains("你好"));
    }

    #[test]
    fn test_format_messages_truncates_long() {
        let long_content = "x".repeat(3000);
        let msgs = vec![make_msg("user", &long_content)];
        let formatted = ContextCompactor::format_messages_for_compaction(&msgs);
        assert!(formatted.contains("[truncated"));
        assert!(formatted.len() < long_content.len() + 100);
    }

    #[tokio::test]
    async fn test_compact_returns_error_when_provider_fails_every_attempt() {
        let mut session = Session::new("provider-failure");
        for idx in 0..20 {
            session.add_message(
                if idx % 2 == 0 { "user" } else { "assistant" },
                format!("message {} with enough context to compact", idx),
            );
        }

        let config = BudgetConfig {
            max_tokens: 5_000,
            system_budget_ratio: 0.0,
            compact_threshold_ratio: 0.8,
            keep_recent_count: 4,
        };

        let err = ContextCompactor::compact(
            &session,
            &config,
            Arc::new(AlwaysFailProvider),
            "mock-model",
            CompactTrigger::Auto,
            &[],
        )
        .await
        .unwrap_err();

        assert!(err.to_string().contains("empty summaries"));
    }

    #[tokio::test]
    async fn test_compact_keeps_best_effort_summary_after_quality_gate_rejects_all_attempts() {
        let mut session = Session::new("quality-degraded");
        for idx in 0..24 {
            session.add_message(
                if idx % 2 == 0 { "user" } else { "assistant" },
                format!(
                    "turn {} discusses provider routing, retries, and persistence ordering",
                    idx
                ),
            );
        }

        let config = BudgetConfig {
            max_tokens: 5_000,
            system_budget_ratio: 0.0,
            compact_threshold_ratio: 0.8,
            keep_recent_count: 6,
        };
        let provider = Arc::new(LowQualityProvider::new());

        let result = ContextCompactor::compact(
            &session,
            &config,
            provider.clone(),
            "mock-model",
            CompactTrigger::Auto,
            &[],
        )
        .await
        .expect("best-effort compaction should still succeed");

        assert_eq!(provider.call_count.load(Ordering::SeqCst), 3);
        assert_eq!(result.summary.retry_count, 2);
        assert_eq!(result.summary.summary, "too short");
        assert!(
            result.summary.quality_score.unwrap_or_default() < 0.6,
            "degraded summary should keep the best attempt even below the quality gate"
        );
    }
}
