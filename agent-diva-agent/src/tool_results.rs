//! Unified prompt representation for complete sanitized tool outputs.

use agent_diva_core::security::{truncate_tool_result, MAX_TOOL_RESULT_CHARS};
use agent_diva_core::tool_artifact::{
    ToolArtifactSecurityContext, ToolArtifactStore, ToolResultRef, INLINE_THRESHOLD_CHARS,
};
use agent_diva_providers::{Message, MessageContent};
use std::collections::HashMap;
use std::path::Path;

pub(crate) const MICROCOMPACT_THRESHOLD_CHARS: usize = 4_000;

pub(crate) struct PromptToolResult {
    pub content: String,
    pub reference: Option<ToolResultRef>,
    pub degraded: bool,
}

pub(crate) async fn prepare_prompt_tool_result(
    workspace: &Path,
    session_id: &str,
    tool_name: &str,
    tool_call_id: &str,
    status: &str,
    content: String,
) -> PromptToolResult {
    if content.chars().count() <= INLINE_THRESHOLD_CHARS {
        return PromptToolResult {
            content,
            reference: None,
            degraded: false,
        };
    }

    let store = ToolArtifactStore::new(workspace);
    let context = ToolArtifactSecurityContext::new(workspace, session_id);
    let persisted_content = content.clone();
    let tool_name_owned = tool_name.to_string();
    let tool_call_id_owned = tool_call_id.to_string();
    let status_owned = status.to_string();
    match tokio::task::spawn_blocking(move || {
        store.put(
            &context,
            &tool_name_owned,
            &tool_call_id_owned,
            &status_owned,
            &persisted_content,
        )
    })
    .await
    {
        Ok(Ok(metadata)) => {
            let reference = ToolResultRef::from_metadata(&metadata, &content);
            let rendered = reference.render().unwrap_or_else(|_| {
                "{\"error\":\"artifact_corrupt\",\"truncated\":true}".to_string()
            });
            PromptToolResult {
                content: rendered,
                reference: Some(reference),
                degraded: false,
            }
        }
        Ok(Err(error)) => PromptToolResult {
            content: format!(
                "{}\n[artifact unavailable: {}; legacy safety fallback applied]",
                truncate_tool_result(&content, MAX_TOOL_RESULT_CHARS),
                error.code()
            ),
            reference: None,
            degraded: true,
        },
        Err(_) => PromptToolResult {
            content: format!(
                "{}\n[artifact unavailable: artifact_io; legacy safety fallback applied]",
                truncate_tool_result(&content, MAX_TOOL_RESULT_CHARS)
            ),
            reference: None,
            degraded: true,
        },
    }
}

pub(crate) async fn microcompact_tool_results(
    workspace: &Path,
    session_id: &str,
    messages: &mut [Message],
) -> Vec<String> {
    let protected_start = messages
        .iter()
        .rposition(|message| message.role == "assistant" && message.tool_calls.is_some())
        .unwrap_or(messages.len());
    let tool_names = messages
        .iter()
        .filter_map(|message| message.tool_calls.as_ref())
        .flatten()
        .map(|call| (call.id.clone(), call.name.clone()))
        .collect::<HashMap<_, _>>();
    let store = ToolArtifactStore::new(workspace);
    let context = ToolArtifactSecurityContext::new(workspace, session_id);
    let mut compacted = Vec::new();

    for (index, message) in messages.iter_mut().enumerate() {
        if index >= protected_start || message.role != "tool" {
            continue;
        }
        let Some(tool_call_id) = message.tool_call_id.clone() else {
            continue;
        };
        let Some(content) = message.content.as_text().map(str::to_string) else {
            continue;
        };
        if content.starts_with("Error:")
            || content.chars().count() <= MICROCOMPACT_THRESHOLD_CHARS
            || serde_json::from_str::<ToolResultRef>(&content).is_ok()
        {
            continue;
        }
        let tool_name = tool_names
            .get(&tool_call_id)
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let store = store.clone();
        let context = context.clone();
        let content_for_store = content.clone();
        let call_for_store = tool_call_id.clone();
        let tool_for_store = tool_name.clone();
        let persisted = tokio::task::spawn_blocking(move || {
            store.put(
                &context,
                &tool_for_store,
                &call_for_store,
                "ok",
                &content_for_store,
            )
        })
        .await;
        if let Ok(Ok(metadata)) = persisted {
            let reference = ToolResultRef::from_metadata(&metadata, &content);
            if let Ok(rendered) = reference.render() {
                message.content = MessageContent::Text(rendered);
                compacted.push(tool_call_id);
            }
        }
    }
    compacted
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_providers::ToolCallRequest;

    #[tokio::test]
    async fn microcompact_is_oldest_first_idempotent_and_preserves_current_group() {
        let workspace = tempfile::tempdir().unwrap();
        let call = |id: &str| ToolCallRequest {
            id: id.to_string(),
            call_type: "function".to_string(),
            name: "read_file".to_string(),
            arguments: HashMap::new(),
        };
        let mut old_assistant = Message::assistant("");
        old_assistant.tool_calls = Some(vec![call("old")]);
        let mut current_assistant = Message::assistant("");
        current_assistant.tool_calls = Some(vec![call("current")]);
        let mut messages = vec![
            old_assistant,
            Message::tool("旧".repeat(4_001), "old".to_string()),
            current_assistant,
            Message::tool("新".repeat(4_001), "current".to_string()),
        ];
        assert_eq!(
            microcompact_tool_results(workspace.path(), "session", &mut messages).await,
            vec!["old"]
        );
        assert!(
            serde_json::from_str::<ToolResultRef>(messages[1].content.as_text().unwrap()).is_ok()
        );
        assert_eq!(messages[3].content.as_text().unwrap(), "新".repeat(4_001));
        assert!(
            microcompact_tool_results(workspace.path(), "session", &mut messages)
                .await
                .is_empty()
        );
    }

    #[tokio::test]
    async fn inline_boundary_and_large_reference_are_stable() {
        let workspace = tempfile::tempdir().unwrap();
        let inline = prepare_prompt_tool_result(
            workspace.path(),
            "session",
            "exec",
            "inline",
            "ok",
            "x".repeat(INLINE_THRESHOLD_CHARS),
        )
        .await;
        assert!(inline.reference.is_none());
        assert_eq!(inline.content.chars().count(), INLINE_THRESHOLD_CHARS);

        let referenced = prepare_prompt_tool_result(
            workspace.path(),
            "session",
            "exec",
            "large",
            "ok",
            "界".repeat(INLINE_THRESHOLD_CHARS + 1),
        )
        .await;
        let reference = referenced.reference.unwrap();
        assert_eq!(reference.tool_call_id, "large");
        assert_eq!(reference.char_count, INLINE_THRESHOLD_CHARS + 1);
        assert!(reference.truncated);
        assert!(referenced.content.len() < INLINE_THRESHOLD_CHARS);
    }

    #[tokio::test]
    async fn over_single_item_limit_uses_explicit_legacy_fallback() {
        let workspace = tempfile::tempdir().unwrap();
        let result = prepare_prompt_tool_result(
            workspace.path(),
            "session",
            "exec",
            "huge",
            "ok",
            "x".repeat(agent_diva_core::tool_artifact::MAX_ARTIFACT_BYTES as usize + 1),
        )
        .await;
        assert!(result.degraded);
        assert!(result.reference.is_none());
        assert!(result.content.contains("artifact_capacity_exceeded"));
        assert!(result.content.contains("80000 chars"));
    }
}
