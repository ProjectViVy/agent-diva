//! Canonical checkpoint compaction.
//!
//! A checkpoint is replacement state, not an append-only summary log.  This
//! module owns the complete production path used by automatic, manual, and
//! reactive compaction.

use agent_diva_core::session::{
    bound_checkpoint_body, CanonicalCheckpoint, ChatMessage, CheckpointTrigger, Session,
};
use agent_diva_providers::{LLMProvider, Message};
use chrono::Utc;
use std::sync::Arc;
use tracing::info;

use super::prompt::{checkpoint_request, CHECKPOINT_SYSTEM_PROMPT};
use super::quality::validate_summary;
use crate::context_budget::BudgetConfig;
use crate::token_estimate::estimate_total_tokens;

/// A complete immutable input to one checkpoint attempt.
#[derive(Debug, Clone)]
pub struct CheckpointSnapshot {
    /// The checkpoint currently visible to the session, if any.
    pub previous_checkpoint: Option<CanonicalCheckpoint>,
    /// The complete durable transcript, including messages hidden by the old checkpoint.
    pub durable_messages: Vec<ChatMessage>,
    /// Messages from the active turn that have not yet been durably saved.
    pub current_turn_messages: Vec<ChatMessage>,
    /// A non-compaction floor, such as memory consolidation progress.
    pub durable_start_index: usize,
}

impl CheckpointSnapshot {
    /// Build a snapshot from a session and an optional in-memory turn suffix.
    pub fn from_session(session: &Session, current_turn_messages: Vec<ChatMessage>) -> Self {
        Self {
            previous_checkpoint: session.canonical_checkpoint.clone(),
            durable_messages: session.messages.clone(),
            current_turn_messages,
            durable_start_index: session.last_consolidated,
        }
    }

    /// Construct a full-transcript snapshot for focused callers and tests.
    pub fn new(
        previous_checkpoint: Option<CanonicalCheckpoint>,
        durable_messages: Vec<ChatMessage>,
        current_turn_messages: Vec<ChatMessage>,
    ) -> Self {
        Self {
            previous_checkpoint,
            durable_messages,
            current_turn_messages,
            durable_start_index: 0,
        }
    }

    /// Set the non-compaction durable floor.
    pub fn with_durable_start_index(mut self, index: usize) -> Self {
        self.durable_start_index = index;
        self
    }
}

/// The result held in memory until a reactive turn is finalized.
#[derive(Debug, Clone)]
pub struct PendingCheckpointUpdate {
    /// The replacement checkpoint.  It is never appended to another checkpoint.
    pub checkpoint: CanonicalCheckpoint,
    /// Durable messages that remain active after the selected prefix.
    pub active_durable_messages: Vec<ChatMessage>,
    /// Current-turn messages that remain active after the selected prefix.
    pub active_current_turn_messages: Vec<ChatMessage>,
    /// The source range in the combined snapshot that was folded.
    pub source_start_index: usize,
    pub source_end_index: usize,
    /// Whether this update actually replaced checkpoint state.
    pub changed: bool,
    /// Whether the body contains an unsaved current-turn snapshot.
    pub includes_current_turn: bool,
}

impl PendingCheckpointUpdate {
    /// Finalize a reactive update against the actual post-turn durable length.
    pub fn finalize_for_durable_message_count(&self, count: usize) -> CanonicalCheckpoint {
        if self.includes_current_turn {
            self.checkpoint.with_durable_message_index(count)
        } else {
            self.checkpoint.clone()
        }
    }
}

/// The unique production entry point for checkpoint compaction.
pub struct CheckpointCompactor;

impl CheckpointCompactor {
    /// Compact one durable/current-turn snapshot and return a replacement update.
    ///
    /// `Ok(None)` means the safe boundary had no prefix to fold.  Provider,
    /// empty-response, and quality failures return `Err` and leave the caller's
    /// session untouched.
    pub async fn compact_snapshot(
        snapshot: CheckpointSnapshot,
        config: &BudgetConfig,
        provider: Arc<dyn LLMProvider>,
        model: &str,
        trigger: CheckpointTrigger,
    ) -> Result<Option<PendingCheckpointUpdate>, anyhow::Error> {
        let durable_len = snapshot.durable_messages.len();
        let mut combined = snapshot.durable_messages.clone();
        combined.extend(snapshot.current_turn_messages.clone());

        let checkpoint_floor = snapshot
            .previous_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.durable_message_index)
            .unwrap_or_default();
        let start = snapshot
            .durable_start_index
            .max(checkpoint_floor)
            .min(combined.len());
        let keep = config
            .keep_recent_count
            .min(combined.len().saturating_sub(start));
        let tentative_end = combined.len().saturating_sub(keep);
        let end = select_safe_compaction_end(&combined, start, tentative_end);

        if end <= start {
            return Ok(None);
        }

        let source = &combined[start..end];
        let formatted = Self::format_messages_for_compaction(source);
        let source_message_count = source.len();
        let source_token_count = estimate_total_tokens(source);
        info!(
            source_message_count,
            source_start = start,
            source_end = end,
            source_token_count,
            "building canonical checkpoint"
        );

        const MAX_RETRIES: u32 = 2;
        const QUALITY_THRESHOLD: f64 = 0.6;
        let prior_body = snapshot
            .previous_checkpoint
            .as_ref()
            .map(|checkpoint| checkpoint.body.as_str());
        let base_prompt = checkpoint_request(prior_body, source_message_count, &formatted);

        let mut best_body = String::new();
        let mut best_score = 0.0;
        let mut best_issues = Vec::new();
        let mut attempts: u32 = 0;

        for attempt in 0..=MAX_RETRIES {
            attempts += 1;
            let user_prompt = if attempt == 0 {
                base_prompt.clone()
            } else {
                format!(
                    "The previous checkpoint body scored {best_score:.2}/1.0 and had these issues: {}.\nProduce a complete replacement with every required section.\n\n{base_prompt}",
                    best_issues.join("; ")
                )
            };
            let response = provider
                .chat(
                    vec![
                        Message::system(CHECKPOINT_SYSTEM_PROMPT),
                        Message::user(user_prompt),
                    ],
                    None,
                    agent_diva_providers::ToolChoiceMode::Unspecified,
                    Some(model.to_string()),
                    4096,
                    0.3,
                )
                .await
                .map_err(|error| anyhow::anyhow!("checkpoint provider call failed: {error}"))?;
            let response_text = response.content.unwrap_or_default();
            if response_text.trim().is_empty() {
                return Err(anyhow::anyhow!(
                    "checkpoint provider returned an empty body"
                ));
            }
            let candidate = Self::extract_checkpoint_body(&response_text);
            let report = validate_summary(&candidate, source);
            if report.score > best_score {
                best_score = report.score;
                best_body = candidate;
                best_issues = report.issues;
            }
            if report.score >= QUALITY_THRESHOLD {
                break;
            }
        }

        if best_body.trim().is_empty() || best_score < QUALITY_THRESHOLD {
            return Err(anyhow::anyhow!(
                "checkpoint quality gate rejected all attempts (best score {best_score:.2})"
            ));
        }

        let body = normalize_checkpoint_body(prior_body, &best_body);
        let checkpoint = CanonicalCheckpoint::new(
            format!(
                "checkpoint-{}-{}",
                Utc::now().format("%Y%m%d-%H%M%S"),
                &uuid::Uuid::new_v4().to_string()[..8]
            ),
            Utc::now().to_rfc3339(),
            trigger,
            end.min(durable_len),
            source_message_count,
            source_token_count,
            Some(best_score),
            best_issues,
            attempts.saturating_sub(1),
            body,
        );

        let current_start = end
            .saturating_sub(durable_len)
            .min(snapshot.current_turn_messages.len());
        let active_durable_messages = if end < durable_len {
            combined[end..durable_len].to_vec()
        } else {
            Vec::new()
        };
        let active_current_turn_messages = if end < combined.len() {
            snapshot.current_turn_messages[current_start..].to_vec()
        } else {
            Vec::new()
        };

        Ok(Some(PendingCheckpointUpdate {
            checkpoint,
            active_durable_messages,
            active_current_turn_messages,
            source_start_index: start,
            source_end_index: end,
            changed: true,
            includes_current_turn: end > durable_len,
        }))
    }

    /// Mechanically fold completed tool groups before the semantic call.
    pub fn format_messages_for_compaction(messages: &[ChatMessage]) -> String {
        let mut output = String::new();
        let mut index = 0;
        while index < messages.len() {
            if let Some(group) = completed_tool_group(messages, index) {
                let tool_names = group.tool_names.to_vec().join(", ");
                let artifacts = group.artifact_refs.to_vec().join(", ");
                output.push_str(&format!(
                    "[tool-group completed] tools={tool_names}; status=completed; artifact_refs={}\n",
                    if artifacts.is_empty() { "none" } else { &artifacts }
                ));
                index = group.end;
                continue;
            }

            let message = &messages[index];
            let content = message.content.chars().take(2_000).collect::<String>();
            let suffix = if message.content.chars().count() > 2_000 {
                "…[truncated]"
            } else {
                ""
            };
            output.push_str(&format!(
                "[{}. {}] {content}{suffix}\n",
                index + 1,
                message.role
            ));
            index += 1;
        }
        output
    }

    fn extract_checkpoint_body(response: &str) -> String {
        let response = response.trim();
        if let Some(start) = response.find("<checkpoint>") {
            let content = &response[start + "<checkpoint>".len()..];
            if let Some(end) = content.find("</checkpoint>") {
                return content[..end].trim().to_string();
            }
        }
        if let Some(start) = response.find("<summary>") {
            let content = &response[start + "<summary>".len()..];
            if let Some(end) = content.find("</summary>") {
                return content[..end].trim().to_string();
            }
        }
        response
            .replace("<analysis>", "")
            .replace("</analysis>", "")
            .trim()
            .to_string()
    }
}

const CHECKPOINT_SECTIONS: [&str; 7] = [
    "目标与约束",
    "已完成事项",
    "关键决定",
    "当前状态",
    "未解决问题",
    "下一步",
    "保留标识符与 artifact 引用",
];

fn normalize_checkpoint_body(previous: Option<&str>, candidate: &str) -> String {
    let has_all_sections = CHECKPOINT_SECTIONS
        .iter()
        .all(|section| candidate.contains(section));
    if has_all_sections {
        return bound_checkpoint_body(candidate.trim());
    }

    let inherited = previous.unwrap_or("无历史检查点。");
    bound_checkpoint_body(&format!(
        "## 目标与约束\n从历史检查点继承，除非当前状态明确覆盖。\n\n## 已完成事项\n{candidate}\n\n## 关键决定\n{inherited}\n\n## 当前状态\n{candidate}\n\n## 未解决问题\n未在本次压缩输入中确认。\n\n## 下一步\n依据当前状态继续处理用户请求。\n\n## 保留标识符与 artifact 引用\n保留原文中的路径、ID 和 artifact 引用。\n"
    ))
}

#[derive(Debug)]
struct ToolGroup {
    end: usize,
    complete: bool,
    tool_names: Vec<String>,
    artifact_refs: Vec<String>,
}

fn completed_tool_group(messages: &[ChatMessage], start: usize) -> Option<ToolGroup> {
    let assistant = messages.get(start)?;
    if assistant.role != "assistant" {
        return None;
    }
    let calls = assistant.tool_calls.as_ref()?.iter().collect::<Vec<_>>();
    if calls.is_empty() {
        return None;
    }
    let ids = calls
        .iter()
        .filter_map(|call| call.get("id").and_then(|id| id.as_str()))
        .collect::<Vec<_>>();
    if ids.len() != calls.len() {
        return None;
    }
    let tool_names = calls
        .iter()
        .filter_map(|call| {
            call.get("function")
                .and_then(|function| function.get("name"))
                .and_then(|name| name.as_str())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    let mut matched = Vec::new();
    let mut artifacts = Vec::new();
    let mut end = start + 1;
    while end < messages.len() && messages[end].role == "tool" {
        if let Some(id) = messages[end].tool_call_id.as_deref() {
            if ids.contains(&id) {
                matched.push(id);
                artifacts.extend(extract_artifact_refs(&messages[end].content));
            }
        }
        end += 1;
    }
    matched.sort_unstable();
    matched.dedup();
    artifacts.sort_unstable();
    artifacts.dedup();
    Some(ToolGroup {
        end,
        complete: matched.len() == ids.len(),
        tool_names,
        artifact_refs: artifacts,
    })
}

fn extract_artifact_refs(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .filter(|token| token.contains("artifact://") || token.starts_with("[artifact:"))
        .map(|token| {
            token
                .trim_matches(|ch: char| ",.;)]}".contains(ch))
                .to_string()
        })
        .collect()
}

/// Select a prefix that ends at a complete conversation/tool boundary.
pub fn select_safe_compaction_end(
    messages: &[ChatMessage],
    start: usize,
    tentative_end: usize,
) -> usize {
    let mut end = tentative_end.min(messages.len());
    if end <= start {
        return start;
    }

    for index in start..end {
        if let Some(group) = completed_tool_group(messages, index) {
            if !group.complete || (index < end && end < group.end) {
                end = index;
                break;
            }
        }
    }

    // A regular user turn is also indivisible.  The active tail must start at
    // a user boundary, never at an assistant/tool half-turn.
    if end > start && end < messages.len() && messages[end].role != "user" {
        if let Some(previous_user) = (start..end)
            .rev()
            .find(|index| messages[*index].role == "user")
        {
            end = previous_user;
        }
    }
    end.max(start)
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_providers::{LLMResponse, ProviderError, ProviderResult};
    use async_trait::async_trait;
    use std::collections::HashMap;

    struct GoodProvider;

    #[async_trait]
    impl LLMProvider for GoodProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _tool_choice: agent_diva_providers::ToolChoiceMode,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            Ok(LLMResponse {
                content: Some("目标与约束：保留 task constraint decision。\n已完成事项：完成 task。\n关键决定：继续当前 decision。\n当前状态：任务可继续。\n未解决问题：无。\n下一步：继续执行。\n保留标识符与 artifact 引用：artifact://one。".into()),
                tool_calls: Vec::new(),
                finish_reason: "stop".into(),
                usage: HashMap::new(),
                reasoning_content: None,
            })
        }

        fn get_default_model(&self) -> String {
            "mock".into()
        }
    }

    struct FailingProvider;

    #[async_trait]
    impl LLMProvider for FailingProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _tool_choice: agent_diva_providers::ToolChoiceMode,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            Err(ProviderError::ApiError("failed".into()))
        }

        fn get_default_model(&self) -> String {
            "mock".into()
        }
    }

    fn msg(role: &str, content: &str) -> ChatMessage {
        ChatMessage::new(role, content)
    }

    fn assistant_with_tool(id: &str) -> ChatMessage {
        ChatMessage::with_tool_metadata(
            "assistant",
            "",
            None,
            Some(vec![serde_json::json!({
                "id": id,
                "type": "function",
                "function": {"name": "read_file", "arguments": "{}"}
            })]),
            None,
        )
    }

    #[test]
    fn completed_tool_group_is_folded_without_result_body() {
        let messages = vec![
            msg("user", "read src/main.rs"),
            assistant_with_tool("call-1"),
            ChatMessage::with_tool_metadata(
                "tool",
                "large result artifact://one",
                Some("call-1".into()),
                None,
                Some("read_file".into()),
            ),
        ];
        let formatted = CheckpointCompactor::format_messages_for_compaction(&messages);
        assert!(formatted.contains("tool-group completed"));
        assert!(formatted.contains("artifact://one"));
        assert!(!formatted.contains("large result"));
    }

    #[test]
    fn incomplete_tool_group_stays_out_of_prefix() {
        let messages = vec![msg("user", "read"), assistant_with_tool("call-1")];
        assert_eq!(select_safe_compaction_end(&messages, 0, messages.len()), 0);
    }

    #[tokio::test]
    async fn failed_provider_does_not_create_update() {
        let snapshot = CheckpointSnapshot::new(
            None,
            (0..10).map(|i| msg("user", &i.to_string())).collect(),
            vec![],
        );
        let config = BudgetConfig {
            keep_recent_count: 2,
            ..BudgetConfig::default()
        };
        let error = CheckpointCompactor::compact_snapshot(
            snapshot,
            &config,
            Arc::new(FailingProvider),
            "mock",
            CheckpointTrigger::Reactive,
        )
        .await
        .unwrap_err();
        assert!(error.to_string().contains("provider"));
    }

    #[tokio::test]
    async fn checkpoint_body_is_fixed_and_bounded() {
        let durable = (0..12)
            .map(|_| msg("user", "task constraint decision"))
            .collect();
        let config = BudgetConfig {
            keep_recent_count: 2,
            ..BudgetConfig::default()
        };
        let update = CheckpointCompactor::compact_snapshot(
            CheckpointSnapshot::new(None, durable, vec![]),
            &config,
            Arc::new(GoodProvider),
            "mock",
            CheckpointTrigger::Auto,
        )
        .await
        .unwrap()
        .unwrap();
        assert!(update.checkpoint.body.chars().count() <= 8_000);
        for section in CHECKPOINT_SECTIONS {
            assert!(update.checkpoint.body.contains(section));
        }
    }
}
