//! Memory consolidation: summarizes old conversation history into long-term memory

use crate::compaction::quality::QualityGate;
use agent_diva_core::memory::{MemoryProvider, SyncTurnRequest, SyncTurnStatus};
use agent_diva_core::session::Session;
use agent_diva_providers::{LLMProvider, Message};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Default number of messages before consolidation triggers
pub const DEFAULT_MEMORY_WINDOW: usize = 100;

const CONSOLIDATION_PROMPT: &str = r#"You are a memory consolidation assistant. Analyze the conversation below and extract important information.

You MUST call the `save_memory` tool with your findings. Do not respond with text.

Guidelines:
- `memory_update`: Updated long-term memory in Markdown. Merge new facts with existing memory. Remove outdated info.
- `history_entry`: A one-line timestamped summary of what happened in this conversation segment."#;

fn save_memory_tool_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "function",
        "function": {
            "name": "save_memory",
            "description": "Save consolidated memory and history entry",
            "parameters": {
                "type": "object",
                "properties": {
                    "memory_update": {
                        "type": "string",
                        "description": "Updated long-term memory content in Markdown"
                    },
                    "history_entry": {
                        "type": "string",
                        "description": "One-line timestamped summary of the conversation segment"
                    }
                },
                "required": ["memory_update", "history_entry"]
            }
        }
    })
}

/// Check if consolidation should run
pub fn should_consolidate(session: &Session, memory_window: usize) -> bool {
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
    let mut best_memory_update: Option<String> = None;
    let mut best_history_entry: Option<String> = None;
    let mut best_score: f64 = 0.0;
    let mut best_issues: Vec<String> = Vec::new();

    for attempt in 0..=max_retry {
        let user_content = if attempt == 0 {
            base_user_content.clone()
        } else {
            // On retry, prepend quality feedback
            format!(
                "注意：上一次整合质量不合格（得分 {:.2}/1.0），原因：{}。\n请生成更详细、更完整的记忆更新，确保覆盖所有关键信息。\n\n{}",
                best_score,
                best_issues.join("；"),
                base_user_content
            )
        };

        let system_msg = Message::system(CONSOLIDATION_PROMPT);
        let user_msg = Message::user(user_content);

        let response = match provider
            .chat(
                vec![system_msg, user_msg],
                Some(tools.clone()),
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

        let memory_update = tc
            .arguments
            .get("memory_update")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let history_entry = tc
            .arguments
            .get("history_entry")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Run quality gate on the memory_update against source messages
        let quality_result = quality_gate.evaluate(&memory_update, old_messages);

        info!(
            "Consolidation attempt {}/{}: quality score={:.2} (completeness={:.2}, keyword_coverage={:.2}), passes={}",
            attempt + 1,
            max_retry + 1,
            quality_result.score,
            quality_result.completeness,
            quality_result.keyword_coverage,
            quality_result.passes,
        );

        // Track the best attempt
        if quality_result.score > best_score {
            best_score = quality_result.score;
            best_issues = quality_result.issues.clone();
            if !memory_update.is_empty() {
                best_memory_update = Some(memory_update);
            }
            if !history_entry.is_empty() {
                best_history_entry = Some(history_entry);
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
    if let Some(ref memory_update) = best_memory_update {
        debug!("Updated MEMORY.md");

        if let Some(ref history_entry) = best_history_entry {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC");
            let entry = format!("[{}] {}", timestamp, history_entry);
            memory_provider
                .sync_turn(SyncTurnRequest {
                    workspace_root: workspace.to_path_buf(),
                    memory_update_markdown: Some(memory_update.clone()),
                    history_entry: Some(entry),
                })
                .await
                .and_then(|response| match response.status {
                    SyncTurnStatus::Persisted | SyncTurnStatus::Noop => Ok(response),
                    SyncTurnStatus::Failed { reason } => {
                        Err(agent_diva_core::Error::Internal(reason))
                    }
                })?;
            debug!("Appended to HISTORY.md");
        } else {
            memory_provider
                .sync_turn(SyncTurnRequest {
                    workspace_root: workspace.to_path_buf(),
                    memory_update_markdown: Some(memory_update.clone()),
                    history_entry: None,
                })
                .await
                .and_then(|response| match response.status {
                    SyncTurnStatus::Persisted | SyncTurnStatus::Noop => Ok(response),
                    SyncTurnStatus::Failed { reason } => {
                        Err(agent_diva_core::Error::Internal(reason))
                    }
                })?;
        }

        info!(
            "Consolidation complete with memory update (best score {:.2})",
            best_score
        );
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
