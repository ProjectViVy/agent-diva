//! Memory consolidation: summarizes old conversation history into long-term memory

use crate::compaction::quality::QualityGate;
use agent_diva_core::memory::{
    MemoryAddRequest, MemoryCrudContext, MemoryCrudOutcome, MemoryGetRequest, MemoryProvider,
    MemoryRemoveRequest, MemoryUpdateRequest, SyncTurnRequest, SyncTurnStatus,
};
use agent_diva_core::session::Session;
use agent_diva_laputa::{LaputaPaths, WorldClaimPayload, WorldGovernance, WorldProposalState};
use agent_diva_providers::{LLMProvider, Message};
use agent_diva_tools::distill_guard;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

mod prompt;

/// Default number of messages before consolidation triggers
pub const DEFAULT_MEMORY_WINDOW: usize = 100;

fn save_memory_tool_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "save_memory",
            "description": "Save consolidated memory as itemized operations",
            "parameters": {
                "type": "object",
                "properties": {
                    "items": {
                        "type": "array",
                        "description": "Itemized memory operations",
                        "items": {
                            "type": "object",
                            "properties": {
                                "action": {
                                    "type": "string",
                                    "enum": ["add", "update", "remove"]
                                },
                                "id": {
                                    "type": "string",
                                    "description": "Record id (required for update/remove)"
                                },
                                "content": {
                                    "type": "string",
                                    "description": "Content (required for add/update)"
                                },
                                "reason": {
                                    "type": "string",
                                    "description": "Removal reason (required for remove)"
                                }
                            },
                            "required": ["action"]
                        }
                    },
                    "history_entry": {
                        "type": "string",
                        "description": "One-line summary of the conversation segment"
                    }
                },
                "required": ["items", "history_entry"]
            }
        }
    })
}

/// Check if consolidation should run
pub fn should_consolidate(session: &Session, memory_window: usize) -> bool {
    if distill_guard::distill_ran_this_session() {
        return false;
    }
    let consolidated = session.last_consolidated.min(session.messages.len());
    let unconsolidated = session.messages.len() - consolidated;
    unconsolidated >= memory_window
}

/// Consolidate old messages into long-term memory
pub async fn consolidate(
    session: &mut Session,
    provider: &Arc<dyn LLMProvider>,
    model: &str,
    workspace: &Path,
    memory_provider: &dyn MemoryProvider,
    memory_window: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    consolidate_with_gate(
        session,
        provider,
        model,
        workspace,
        memory_provider,
        memory_window,
        QualityGate::default(),
    )
    .await
}

/// Try to parse a WORLD claim from consolidation item content.
///
/// Returns `None` when the content does not carry a `kind: world` frontmatter
/// marker or lacks the minimum `domain` / `title` fields required for a
/// structured WORLD claim.
fn try_parse_world_claim(content: &str) -> Option<WorldClaimPayload> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after = &trimmed[3..];
    let end = after.find("\n---")?;
    let frontmatter = &after[..end];

    let mut kind_found = false;
    let mut domain = None;
    let mut title = None;
    let mut source = None;
    let mut text = None;

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("kind:") {
            if value.trim() == "world" {
                kind_found = true;
            }
        } else if let Some(value) = line.strip_prefix("domain:") {
            domain = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("title:") {
            title = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("source:") {
            source = Some(value.trim().to_string());
        } else if let Some(value) = line.strip_prefix("text:") {
            text = Some(value.trim().to_string());
        }
    }

    if !kind_found {
        return None;
    }

    let body_start = trimmed[3 + end + 4..].trim_start();
    let body = text.unwrap_or_else(|| body_start.to_string());

    Some(WorldClaimPayload {
        domain: domain?,
        title: title?,
        status: "active".to_string(),
        confidence: "medium".to_string(),
        scopes: vec![],
        source: source.unwrap_or_else(|| "consolidation".to_string()),
        text: body,
    })
}

/// Consolidate old messages into long-term memory with a configurable quality gate.
///
/// After each consolidation attempt, the quality gate validates the `memory_update`
/// against the source messages. If quality is below threshold and retries remain,
/// the consolidation is retried with a feedback prompt. After all retries are
/// exhausted, the best result is used.
pub async fn consolidate_with_gate(
    session: &mut Session,
    provider: &Arc<dyn LLMProvider>,
    model: &str,
    workspace: &Path,
    memory_provider: &dyn MemoryProvider,
    memory_window: usize,
    quality_gate: QualityGate,
) -> Result<(), Box<dyn std::error::Error>> {
    if distill_guard::distill_ran_this_session() {
        info!("Skipping consolidation: explicit distill already ran this session");
        return Ok(());
    }
    let consolidated = session.last_consolidated.min(session.messages.len());
    let unconsolidated_count = session.messages.len() - consolidated;
    if unconsolidated_count < memory_window {
        return Ok(());
    }

    info!(
        "Starting memory consolidation: {} unconsolidated messages",
        unconsolidated_count
    );

    // Keep recent half for context overlap
    let keep_recent = memory_window / 2;
    let consolidate_end = session.messages.len().saturating_sub(keep_recent);
    if consolidate_end <= consolidated {
        return Ok(());
    }

    // Build conversation summary from old messages
    let old_messages = &session.messages[consolidated..consolidate_end];
    let mut conversation = String::new();
    for msg in old_messages {
        let content = if msg.content.chars().count() > 500 {
            format!("{}...", msg.content.chars().take(500).collect::<String>())
        } else {
            msg.content.clone()
        };
        conversation.push_str(&format!("[{}]: {}\n", msg.role, content));
    }

    // Load existing memory for context
    let existing_memory = memory_provider
        .system_prompt_block(&agent_diva_core::memory::SystemPromptRequest {
            workspace_root: workspace.to_path_buf(),
        })?
        .prompt_block
        .map(|block| block.markdown)
        .unwrap_or_default();

    // Build the base user prompt
    let base_user_content = format!(
        "## Existing Memory\n{}\n\n## Conversation to Consolidate\n{}",
        if existing_memory.is_empty() {
            "(none)".to_string()
        } else {
            existing_memory
        },
        conversation,
    );

    let tools = vec![save_memory_tool_schema()];

    // Retry loop: quality gate with configurable max_retry
    let max_retry = quality_gate.max_retry;
    let mut best_items: Option<serde_json::Value> = None;
    let mut best_score: f64 = 0.0;
    let mut best_issues: Vec<String> = Vec::new();

    for attempt in 0..=max_retry {
        let user_content = if attempt == 0 {
            base_user_content.clone()
        } else {
            // On retry, prepend quality feedback
            prompt::retry(best_score, &best_issues, &base_user_content)
        };

        debug!(
            prompt_id = prompt::PROMPT_ID,
            prompt_version = prompt::PROMPT_VERSION,
            "building consolidation prompt"
        );
        let system_msg = Message::system(prompt::SYSTEM);
        let user_msg = Message::user(user_content);

        let response = match provider
            .chat(
                vec![system_msg, user_msg],
                Some(tools.clone()),
                agent_diva_providers::ToolChoiceMode::Auto,
                Some(model.to_string()),
                2048,
                0.3,
            )
            .await
        {
            Ok(resp) => resp,
            Err(e) => {
                warn!(
                    "Consolidation LLM call failed on attempt {}: {}",
                    attempt + 1,
                    e
                );
                continue;
            }
        };

        // Parse the save_memory tool call from the response
        let tool_call = response
            .tool_calls
            .iter()
            .find(|tc| tc.name == "save_memory");

        let Some(tc) = tool_call else {
            warn!(
                "Consolidation LLM call did not return a save_memory tool call on attempt {}",
                attempt + 1
            );
            continue;
        };

        let items = tc.arguments.get("items").cloned();

        let memory_update_text = items
            .as_ref()
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|item| item.get("content").and_then(|c| c.as_str()))
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();

        let quality_result = quality_gate.evaluate(&memory_update_text, old_messages);

        info!(
            "Consolidation attempt {}/{}: quality score={:.2} (completeness={:.2}, keyword_coverage={:.2}), passes={}",
            attempt + 1,
            max_retry + 1,
            quality_result.score,
            quality_result.completeness,
            quality_result.keyword_coverage,
            quality_result.passes,
        );

        // Track the best attempt (>= ensures first attempt is always stored)
        if quality_result.score >= best_score || best_items.is_none() {
            best_score = quality_result.score;
            best_issues = quality_result.issues.clone();
            if items.is_some() {
                best_items = items;
            }
        }

        // Early exit if quality passes
        if quality_result.passes {
            info!(
                "Consolidation quality passed on attempt {} (score {:.2})",
                attempt + 1,
                quality_result.score
            );
            break;
        }

        if attempt < max_retry {
            warn!(
                "Consolidation quality insufficient (score {:.2}), retrying…",
                quality_result.score
            );
        }
    }

    // Apply the best result
    if let Some(ref items_value) = best_items {
        let crud_ctx = MemoryCrudContext {
            workspace_root: workspace.to_path_buf(),
        };

        if let Some(items_arr) = items_value.as_array() {
            let mut applied: u32 = 0;
            let mut proposed: u32 = 0;
            let mut failed: u32 = 0;
            let mut world_governance =
                WorldGovernance::open(LaputaPaths::new(workspace).cognitive_dir()).ok();

            for item in items_arr {
                let action = item.get("action").and_then(|v| v.as_str()).unwrap_or("");
                let content = item.get("content").and_then(|v| v.as_str()).unwrap_or("");
                let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("");
                let reason = item.get("reason").and_then(|v| v.as_str()).unwrap_or("");

                if action == "add" {
                    if let (Some(wg), Some(payload)) =
                        (world_governance.as_mut(), try_parse_world_claim(content))
                    {
                        let claim_id = id.to_string();
                        let claim_id = if claim_id.is_empty() {
                            format!(
                                "consolidation-{}-{}",
                                payload.domain,
                                payload.title.replace(' ', "-")
                            )
                        } else {
                            claim_id
                        };
                        match wg.submit(&claim_id, "consolidation", payload, chrono::Utc::now()) {
                            Ok(WorldProposalState::PendingReview) => {
                                proposed += 1;
                                debug!("Consolidation: WORLD claim {claim_id} queued for review");
                            }
                            Ok(WorldProposalState::Applied) => {
                                applied += 1;
                                debug!("Consolidation: WORLD claim {claim_id} applied");
                            }
                            Ok(other) => {
                                failed += 1;
                                warn!("Consolidation: WORLD claim {claim_id} → {other:?}");
                            }
                            Err(e) => {
                                failed += 1;
                                warn!("Consolidation: WORLD claim {claim_id} error: {e}");
                            }
                        }
                        continue;
                    }
                }

                let result = match action {
                    "add" => {
                        memory_provider
                            .memory_add(
                                &crud_ctx,
                                MemoryAddRequest {
                                    content: content.to_string(),
                                    evidence_refs: vec![],
                                },
                            )
                            .await
                    }
                    "update" => {
                        let base_revision = current_revision(memory_provider, &crud_ctx, id).await;
                        memory_provider
                            .memory_update(
                                &crud_ctx,
                                MemoryUpdateRequest {
                                    record_id: id.to_string(),
                                    content: content.to_string(),
                                    base_revision,
                                    evidence_refs: Vec::new(),
                                },
                            )
                            .await
                    }
                    "remove" => {
                        let base_revision = current_revision(memory_provider, &crud_ctx, id).await;
                        memory_provider
                            .memory_remove(
                                &crud_ctx,
                                MemoryRemoveRequest {
                                    record_id: id.to_string(),
                                    reason: reason.to_string(),
                                    base_revision,
                                },
                            )
                            .await
                    }
                    other => {
                        warn!("Consolidation: unknown action '{other}', skipping");
                        continue;
                    }
                };

                match result {
                    Ok(MemoryCrudOutcome::Applied { .. }) => {
                        applied += 1;
                        debug!("Consolidation: {action} applied");
                    }
                    Ok(MemoryCrudOutcome::ProposalCreated { .. }) => {
                        proposed += 1;
                        debug!("Consolidation: {action} created proposal");
                    }
                    Ok(MemoryCrudOutcome::Failed { reason }) => {
                        failed += 1;
                        warn!("Consolidation: {action} failed: {reason}");
                    }
                    Err(e) => {
                        failed += 1;
                        warn!("Consolidation: {action} error: {e}");
                    }
                    _ => {}
                }
            }

            info!(
                "Consolidation itemized: {applied} applied, {proposed} proposed, {failed} failed"
            );
        } else {
            warn!("consolidation fallback: non-itemized (items is not an array)");
            let memory_update_text = items_value.as_str().unwrap_or("").to_string();
            memory_provider
                .sync_turn(SyncTurnRequest {
                    workspace_root: workspace.to_path_buf(),
                    memory_update_markdown: Some(memory_update_text),
                    history_entry: None,
                })
                .await
                .and_then(|response| match response.status {
                    SyncTurnStatus::Persisted
                    | SyncTurnStatus::ProposalCreated
                    | SyncTurnStatus::Noop => Ok(response),
                    SyncTurnStatus::Failed { reason } => {
                        Err(agent_diva_core::Error::Internal(reason))
                    }
                })?;
        }

        info!("Consolidation complete (best score {:.2})", best_score);
    } else {
        warn!(
            "Consolidation LLM call did not produce any usable result after {} attempts",
            max_retry + 1
        );
    }

    // Always advance the pointer to avoid infinite retry on the same messages.
    // Even if the LLM didn't return the expected tool call, we don't want to
    // re-consolidate the same segment every turn.
    session.last_consolidated = consolidate_end;
    debug!("last_consolidated advanced to {}", consolidate_end);

    Ok(())
}

async fn current_revision(
    memory_provider: &dyn MemoryProvider,
    context: &MemoryCrudContext,
    record_id: &str,
) -> i64 {
    match memory_provider
        .memory_get(
            context,
            MemoryGetRequest {
                record_id: record_id.to_string(),
            },
        )
        .await
    {
        Ok(MemoryCrudOutcome::Listed { entries }) => {
            entries.first().map_or(0, |entry| entry.revision)
        }
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::memory::{
        PrefetchRequest, PrefetchResponse, PrefetchStatus, SessionEndRequest, SessionEndResponse,
        SessionEndStatus, StartupStatus, SyncTurnRequest, SyncTurnResponse, SystemPromptBlock,
        SystemPromptRequest, SystemPromptResponse,
    };
    use agent_diva_core::session::ChatMessage;
    use agent_diva_providers::{LLMResponse, ProviderResult, ToolCallRequest, ToolChoiceMode};
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    /// Memory provider whose `sync_turn` reports a created proposal.
    struct ProposalCreatingProvider {
        sync_calls: AtomicUsize,
    }

    impl ProposalCreatingProvider {
        fn new() -> Self {
            Self {
                sync_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl MemoryProvider for ProposalCreatingProvider {
        fn system_prompt_block(
            &self,
            _request: &SystemPromptRequest,
        ) -> agent_diva_core::Result<SystemPromptResponse> {
            Ok(SystemPromptResponse {
                status: StartupStatus::Ready,
                prompt_block: Some(SystemPromptBlock {
                    shape: agent_diva_core::memory::StartupInjectionShape::CompactRenderedMarkdown,
                    markdown: "Existing memory.".to_string(),
                }),
            })
        }

        async fn prefetch(
            &self,
            _request: PrefetchRequest,
        ) -> agent_diva_core::Result<PrefetchResponse> {
            Ok(PrefetchResponse {
                status: PrefetchStatus::SkippedNoIntent,
                prompt_block: None,
            })
        }

        async fn sync_turn(
            &self,
            _request: SyncTurnRequest,
        ) -> agent_diva_core::Result<SyncTurnResponse> {
            self.sync_calls.fetch_add(1, Ordering::SeqCst);
            Ok(SyncTurnResponse {
                status: SyncTurnStatus::ProposalCreated,
            })
        }

        async fn on_session_end(
            &self,
            _request: SessionEndRequest,
        ) -> agent_diva_core::Result<SessionEndResponse> {
            Ok(SessionEndResponse {
                status: SessionEndStatus::Noop,
            })
        }
    }

    /// Provider that always returns a `save_memory` tool call on `chat`.
    struct SaveMemoryProvider;

    #[async_trait::async_trait]
    impl LLMProvider for SaveMemoryProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _tool_choice: ToolChoiceMode,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            Ok(LLMResponse {
                content: None,
                tool_calls: vec![ToolCallRequest {
                    id: "save-memory-call".to_string(),
                    call_type: "function".to_string(),
                    name: "save_memory".to_string(),
                    arguments: HashMap::from([(
                        "items".to_string(),
                        serde_json::json!("Updated continuity."),
                    )]),
                }],
                finish_reason: "tool_calls".to_string(),
                usage: HashMap::new(),
                reasoning_content: None,
            })
        }

        async fn chat_stream(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _tool_choice: ToolChoiceMode,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<agent_diva_providers::ProviderEventStream> {
            unimplemented!("not used by consolidation")
        }

        fn get_default_model(&self) -> String {
            "test-model".to_string()
        }
    }

    fn sample_session(len: usize) -> Session {
        let now = Utc::now();
        Session {
            key: "test:chat".to_string(),
            messages: (0..len)
                .map(|i| ChatMessage {
                    role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                    content: format!("continuity message {i}"),
                    timestamp: now,
                    tool_call_id: None,
                    tool_calls: None,
                    name: None,
                    reasoning_content: None,
                    thinking_blocks: None,
                    metadata: None,
                    token_usage: None,
                })
                .collect(),
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
            title: None,
            last_consolidated: 0,
            canonical_checkpoint: None,
        }
    }

    #[tokio::test]
    async fn sync_turn_proposal_created_is_treated_as_success() {
        let workspace = tempfile::tempdir().unwrap();
        let mut session = sample_session(DEFAULT_MEMORY_WINDOW + 10);
        let provider: Arc<dyn LLMProvider> = Arc::new(SaveMemoryProvider);
        let memory = ProposalCreatingProvider::new();

        let gate = QualityGate {
            min_completeness: 0.0,
            min_keyword_coverage: 0.0,
            min_score: 0.0,
            max_retry: 0,
        };

        consolidate_with_gate(
            &mut session,
            &provider,
            "test-model",
            workspace.path(),
            &memory,
            DEFAULT_MEMORY_WINDOW,
            gate,
        )
        .await
        .unwrap();

        assert_eq!(memory.sync_calls.load(Ordering::SeqCst), 1);
    }
}

#[cfg(test)]
mod wave5_tests {
    use super::*;
    use agent_diva_core::memory::{
        PrefetchRequest, PrefetchResponse, PrefetchStatus, SessionEndRequest, SessionEndResponse,
        SessionEndStatus, StartupStatus, SyncTurnRequest, SyncTurnResponse, SystemPromptBlock,
        SystemPromptRequest, SystemPromptResponse,
    };
    use agent_diva_core::session::ChatMessage;
    use agent_diva_providers::{
        LLMResponse, ProviderEventStream, ProviderResult, ToolCallRequest, ToolChoiceMode,
    };
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    fn sample_session(len: usize) -> Session {
        let now = Utc::now();
        Session {
            key: "test:chat".to_string(),
            messages: (0..len)
                .map(|i| ChatMessage {
                    role: if i % 2 == 0 { "user" } else { "assistant" }.to_string(),
                    content: format!("continuity message {i}"),
                    timestamp: now,
                    tool_call_id: None,
                    tool_calls: None,
                    name: None,
                    reasoning_content: None,
                    thinking_blocks: None,
                    metadata: None,
                    token_usage: None,
                })
                .collect(),
            created_at: now,
            updated_at: now,
            metadata: serde_json::json!({}),
            title: None,
            last_consolidated: 0,
            canonical_checkpoint: None,
        }
    }

    struct AppliedMemoryProvider {
        add_calls: AtomicUsize,
        sync_calls: AtomicUsize,
    }

    impl AppliedMemoryProvider {
        fn new() -> Self {
            Self {
                add_calls: AtomicUsize::new(0),
                sync_calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait::async_trait]
    impl MemoryProvider for AppliedMemoryProvider {
        fn system_prompt_block(
            &self,
            _: &SystemPromptRequest,
        ) -> agent_diva_core::Result<SystemPromptResponse> {
            Ok(SystemPromptResponse {
                status: StartupStatus::Ready,
                prompt_block: Some(SystemPromptBlock {
                    shape: agent_diva_core::memory::StartupInjectionShape::CompactRenderedMarkdown,
                    markdown: "existing".to_string(),
                }),
            })
        }

        async fn prefetch(&self, _: PrefetchRequest) -> agent_diva_core::Result<PrefetchResponse> {
            Ok(PrefetchResponse {
                status: PrefetchStatus::SkippedNoIntent,
                prompt_block: None,
            })
        }

        async fn sync_turn(&self, _: SyncTurnRequest) -> agent_diva_core::Result<SyncTurnResponse> {
            self.sync_calls.fetch_add(1, Ordering::SeqCst);
            Ok(SyncTurnResponse {
                status: SyncTurnStatus::Persisted,
            })
        }

        async fn memory_add(
            &self,
            _: &MemoryCrudContext,
            _: MemoryAddRequest,
        ) -> agent_diva_core::Result<MemoryCrudOutcome> {
            self.add_calls.fetch_add(1, Ordering::SeqCst);
            Ok(MemoryCrudOutcome::Applied {
                entry: None,
                evidence_advisory: None,
            })
        }

        async fn on_session_end(
            &self,
            _: SessionEndRequest,
        ) -> agent_diva_core::Result<SessionEndResponse> {
            Ok(SessionEndResponse {
                status: SessionEndStatus::Noop,
            })
        }
    }

    struct ItemizedLLMProvider;

    #[async_trait::async_trait]
    impl LLMProvider for ItemizedLLMProvider {
        async fn chat(
            &self,
            _: Vec<Message>,
            _: Option<Vec<serde_json::Value>>,
            _: ToolChoiceMode,
            _: Option<String>,
            _: i32,
            _: f64,
        ) -> ProviderResult<LLMResponse> {
            Ok(LLMResponse {
                content: None,
                tool_calls: vec![ToolCallRequest {
                    id: "call-1".to_string(),
                    call_type: "function".to_string(),
                    name: "save_memory".to_string(),
                    arguments: HashMap::from([
                        (
                            "items".to_string(),
                            serde_json::json!([{"action": "add", "content": "important fact"}]),
                        ),
                        (
                            "history_entry".to_string(),
                            serde_json::Value::String("summary".to_string()),
                        ),
                    ]),
                }],
                finish_reason: "tool_calls".to_string(),
                usage: HashMap::new(),
                reasoning_content: None,
            })
        }

        async fn chat_stream(
            &self,
            _: Vec<Message>,
            _: Option<Vec<serde_json::Value>>,
            _: ToolChoiceMode,
            _: Option<String>,
            _: i32,
            _: f64,
        ) -> ProviderResult<ProviderEventStream> {
            unimplemented!()
        }

        fn get_default_model(&self) -> String {
            "test".to_string()
        }
    }

    struct NonArrayItemsProvider;

    #[async_trait::async_trait]
    impl LLMProvider for NonArrayItemsProvider {
        async fn chat(
            &self,
            _: Vec<Message>,
            _: Option<Vec<serde_json::Value>>,
            _: ToolChoiceMode,
            _: Option<String>,
            _: i32,
            _: f64,
        ) -> ProviderResult<LLMResponse> {
            Ok(LLMResponse {
                content: None,
                tool_calls: vec![ToolCallRequest {
                    id: "call-1".to_string(),
                    call_type: "function".to_string(),
                    name: "save_memory".to_string(),
                    arguments: HashMap::from([
                        (
                            "items".to_string(),
                            serde_json::Value::String("not-an-array".to_string()),
                        ),
                        (
                            "history_entry".to_string(),
                            serde_json::Value::String("summary".to_string()),
                        ),
                    ]),
                }],
                finish_reason: "tool_calls".to_string(),
                usage: HashMap::new(),
                reasoning_content: None,
            })
        }

        async fn chat_stream(
            &self,
            _: Vec<Message>,
            _: Option<Vec<serde_json::Value>>,
            _: ToolChoiceMode,
            _: Option<String>,
            _: i32,
            _: f64,
        ) -> ProviderResult<ProviderEventStream> {
            unimplemented!()
        }

        fn get_default_model(&self) -> String {
            "test".to_string()
        }
    }

    #[test]
    fn skip_when_distill_ran() {
        distill_guard::reset_distill_flag();
        let session = sample_session(DEFAULT_MEMORY_WINDOW + 10);
        assert!(should_consolidate(&session, DEFAULT_MEMORY_WINDOW));
        distill_guard::mark_distill_ran();
        assert!(!should_consolidate(&session, DEFAULT_MEMORY_WINDOW));
        distill_guard::reset_distill_flag();
    }

    #[tokio::test]
    async fn itemized_apply_dispatches_memory_add() {
        distill_guard::reset_distill_flag();
        let workspace = tempfile::tempdir().unwrap();
        let mut session = sample_session(DEFAULT_MEMORY_WINDOW + 10);
        let provider: Arc<dyn LLMProvider> = Arc::new(ItemizedLLMProvider);
        let memory = AppliedMemoryProvider::new();

        let gate = QualityGate {
            min_completeness: 0.0,
            min_keyword_coverage: 0.0,
            min_score: 0.0,
            max_retry: 0,
        };

        consolidate_with_gate(
            &mut session,
            &provider,
            "test",
            workspace.path(),
            &memory,
            DEFAULT_MEMORY_WINDOW,
            gate,
        )
        .await
        .unwrap();

        assert_eq!(memory.add_calls.load(Ordering::SeqCst), 1);
        // History file appends are retired: itemized consolidation only
        // dispatches memory adds.
        assert_eq!(memory.sync_calls.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn itemized_fallback_to_sync_turn_on_non_array() {
        distill_guard::reset_distill_flag();
        let workspace = tempfile::tempdir().unwrap();
        let mut session = sample_session(DEFAULT_MEMORY_WINDOW + 10);
        let provider: Arc<dyn LLMProvider> = Arc::new(NonArrayItemsProvider);
        let memory = AppliedMemoryProvider::new();

        let gate = QualityGate {
            min_completeness: 0.0,
            min_keyword_coverage: 0.0,
            min_score: 0.0,
            max_retry: 0,
        };

        consolidate_with_gate(
            &mut session,
            &provider,
            "test",
            workspace.path(),
            &memory,
            DEFAULT_MEMORY_WINDOW,
            gate,
        )
        .await
        .unwrap();

        assert_eq!(memory.add_calls.load(Ordering::SeqCst), 0);
        // fallback sync_turn for non-array items; history sync_turn is retired
        assert_eq!(memory.sync_calls.load(Ordering::SeqCst), 1);
    }
}
