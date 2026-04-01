//! Memory consolidation: summarizes old conversation history into long-term memory

use agent_diva_core::memory::MemoryManager;
use agent_diva_core::person_seam::PersonSeamVisibility;
use agent_diva_core::session::Session;
use agent_diva_providers::{LLMProvider, Message};
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
    memory_manager: &MemoryManager,
    memory_window: usize,
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
        if msg.person_seam == Some(PersonSeamVisibility::Internal) {
            continue;
        }
        let content = if msg.content.chars().count() > 500 {
            format!("{}...", msg.content.chars().take(500).collect::<String>())
        } else {
            msg.content.clone()
        };
        conversation.push_str(&format!("[{}]: {}\n", msg.role, content));
    }

    // Load existing memory for context
    let existing_memory = memory_manager.get_memory_context();

    // Build the LLM request
    let system_msg = Message::system(CONSOLIDATION_PROMPT);
    let user_content = format!(
        "## Existing Memory\n{}\n\n## Conversation to Consolidate\n{}",
        if existing_memory.is_empty() {
            "(none)".to_string()
        } else {
            existing_memory
        },
        conversation,
    );
    let user_msg = Message::user(user_content);

    let tools = vec![save_memory_tool_schema()];
    let response = provider
        .chat(
            vec![system_msg, user_msg],
            Some(tools),
            Some(model.to_string()),
            2048,
            0.3,
        )
        .await?;

    // Parse the save_memory tool call from the response
    let tool_call = response
        .tool_calls
        .iter()
        .find(|tc| tc.name == "save_memory");

    if let Some(tc) = tool_call {
        let memory_update = tc
            .arguments
            .get("memory_update")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let history_entry = tc
            .arguments
            .get("history_entry")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if !memory_update.is_empty() {
            let memory = agent_diva_core::memory::Memory::with_content(memory_update);
            memory_manager.save_memory(&memory)?;
            debug!("Updated MEMORY.md");
        }

        if !history_entry.is_empty() {
            let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M UTC");
            let entry = format!("[{}] {}", timestamp, history_entry);
            memory_manager.append_history(&entry)?;
            debug!("Appended to HISTORY.md");
        }

        info!("Consolidation complete with memory update");
    } else {
        warn!(
            "Consolidation LLM call did not return a save_memory tool call, skipping memory write"
        );
    }

    // Always advance the pointer to avoid infinite retry on the same messages.
    // Even if the LLM didn't return the expected tool call, we don't want to
    // re-consolidate the same segment every turn.
    session.last_consolidated = consolidate_end;
    debug!("last_consolidated advanced to {}", consolidate_end);

    Ok(())
}
