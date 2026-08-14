//! Session-scoped checkpoint contract.
//!
//! A session checkpoint is volatile, session-scoped, and never authoritative: it is
//! written through the checkpoint tool, injected into live-turn context, and
//! cleared on session end. Long-term promotion happens explicitly through
//! `memory_distill` (see the G1 evidence extension).

use std::path::PathBuf;

/// L0 memory management policy injected into the system prompt.
///
/// Mirrors the GA axioms: Action-Verified writes, no volatile state in
/// long-term memory, and minimal-pointer rendering.
pub const L0_MEMORY_POLICY: &str = "\
You manage memory in layers:
1. Action-Verified writes: persist facts confirmed by tool results or explicit user statements; never write speculation as authority.
2. No volatile state: session progress belongs in the working checkpoint, not in long-term memory.
3. Minimal pointer principle: startup injection carries only a compact index; retrieve full entries on demand with memory_search or memory_list.";

/// Input for rendering the current session's checkpoint block.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionCheckpointRequest {
    /// Workspace root for the active agent session.
    pub workspace_root: PathBuf,
    /// Session-scoped key (for example `channel:chat_id`).
    pub session_id: String,
}

/// Result of rendering the checkpoint block for live-turn assembly.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionCheckpointResponse {
    /// Markdown block to inject after the system prompt, when non-empty.
    pub prompt_block: Option<String>,
}

/// Input for writing the session working checkpoint.
///
/// The checkpoint is volatile: it is stored session-scoped, injected into turn
/// context, and cleared on session end. It never becomes long-term authority.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SessionCheckpointWriteRequest {
    /// Workspace root for the active agent session.
    pub workspace_root: PathBuf,
    /// Session-scoped key (for example `channel:chat_id`).
    pub session_id: String,
    /// Structured key facts of the current task state.
    pub key_info: String,
    /// Related SOP / skill references.
    pub related_sops: Vec<String>,
    /// Free-form checkpoint content.
    pub content: String,
}

/// Render a checkpoint into the structured session checkpoint Markdown block.
pub fn render_session_checkpoint_block(
    key_info: &str,
    related_sops: &[String],
    content: &str,
) -> String {
    let mut block = String::from("## Session Checkpoint\n");
    if !key_info.trim().is_empty() {
        block.push_str(&format!("Key info:\n{}\n", key_info.trim()));
    }
    if !related_sops.is_empty() {
        let refs = related_sops
            .iter()
            .map(|sop| format!("- {}", sop.trim()))
            .collect::<Vec<_>>()
            .join("\n");
        block.push_str(&format!("Related SOPs:\n{refs}\n"));
    }
    if !content.trim().is_empty() {
        block.push_str(&format!("State:\n{}\n", content.trim()));
    }
    block
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_checkpoint_response_has_no_block() {
        let response = SessionCheckpointResponse::default();
        assert_eq!(response.prompt_block, None);
    }

    #[test]
    fn checkpoint_request_roundtrip_serde() {
        let request = SessionCheckpointWriteRequest {
            workspace_root: PathBuf::from("/ws"),
            session_id: "channel:123".to_string(),
            key_info: "migrating service B".to_string(),
            related_sops: vec!["rust-deploy".to_string()],
            content: "port 8080 confirmed".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        let back: SessionCheckpointWriteRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(back, request);
    }

    #[test]
    fn render_checkpoint_block_contains_all_sections() {
        let block = render_session_checkpoint_block(
            "migrating service B",
            &["rust-deploy".to_string()],
            "port 8080 confirmed",
        );
        assert!(block.starts_with("## Session Checkpoint"));
        assert!(block.contains("Key info:"));
        assert!(block.contains("migrating service B"));
        assert!(block.contains("Related SOPs:"));
        assert!(block.contains("- rust-deploy"));
        assert!(block.contains("port 8080 confirmed"));
    }

    #[test]
    fn render_checkpoint_block_omits_empty_sections() {
        let block = render_session_checkpoint_block("", &[], "");
        assert_eq!(block, "## Session Checkpoint\n");
    }
}
