//! Unified prompt representation for complete sanitized tool outputs.

use agent_diva_core::tool_artifact::{
    CanonicalToolResult, ToolArtifactSecurityContext, ToolArtifactStore, ToolResultRef,
    INLINE_THRESHOLD_CHARS,
};
use agent_diva_providers::{Message, MessageContent};
use std::collections::HashMap;
use std::path::Path;

pub(crate) const MICROCOMPACT_THRESHOLD_CHARS: usize = 4_000;

#[derive(Debug, Default, Eq, PartialEq)]
pub(crate) struct MicrocompactReport {
    pub compacted: Vec<String>,
    pub failures: Vec<(String, String)>,
}

pub(crate) async fn canonicalize_tool_result(
    workspace: &Path,
    session_id: &str,
    tool_name: &str,
    tool_call_id: &str,
    status: &str,
    content: String,
) -> CanonicalToolResult {
    if content.chars().count() <= INLINE_THRESHOLD_CHARS {
        return CanonicalToolResult::inline(content, false);
    }

    persist_artifact_result(
        workspace,
        session_id,
        tool_name,
        tool_call_id,
        status,
        content,
    )
    .await
}

async fn persist_artifact_result(
    workspace: &Path,
    session_id: &str,
    tool_name: &str,
    tool_call_id: &str,
    status: &str,
    content: String,
) -> CanonicalToolResult {
    let char_count = content.chars().count();
    let byte_count = content.len();

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
            match CanonicalToolResult::artifact(reference) {
                Ok(result) => result,
                Err(_) => CanonicalToolResult::materialization_failure(
                    "artifact_corrupt",
                    char_count,
                    byte_count,
                ),
            }
        }
        Ok(Err(error)) => {
            CanonicalToolResult::materialization_failure(error.code(), char_count, byte_count)
        }
        Err(_) => {
            CanonicalToolResult::materialization_failure("artifact_io", char_count, byte_count)
        }
    }
}

pub(crate) async fn microcompact_tool_results(
    workspace: &Path,
    session_id: &str,
    messages: &mut [Message],
) -> MicrocompactReport {
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
    let mut report = MicrocompactReport::default();

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
        let canonical = persist_artifact_result(
            workspace,
            session_id,
            &tool_name,
            &tool_call_id,
            "ok",
            content,
        )
        .await;
        if canonical.is_error() {
            report.failures.push((
                tool_call_id,
                canonical.error_code().unwrap_or("artifact_io").to_string(),
            ));
        } else {
            message.content = MessageContent::Text(canonical.into_content());
            report.compacted.push(tool_call_id);
        }
    }
    report
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
        let report = microcompact_tool_results(workspace.path(), "session", &mut messages).await;
        assert_eq!(report.compacted, vec!["old"]);
        assert!(report.failures.is_empty());
        assert!(
            serde_json::from_str::<ToolResultRef>(messages[1].content.as_text().unwrap()).is_ok()
        );
        assert_eq!(messages[3].content.as_text().unwrap(), "新".repeat(4_001));
        assert!(
            microcompact_tool_results(workspace.path(), "session", &mut messages)
                .await
                .compacted
                .is_empty()
        );
    }

    #[tokio::test]
    async fn inline_boundary_and_large_reference_are_stable() {
        let workspace = tempfile::tempdir().unwrap();
        let inline = canonicalize_tool_result(
            workspace.path(),
            "session",
            "exec",
            "inline",
            "ok",
            "x".repeat(INLINE_THRESHOLD_CHARS),
        )
        .await;
        assert!(inline.reference().is_none());
        assert_eq!(inline.content().chars().count(), INLINE_THRESHOLD_CHARS);
        assert!(!inline.is_error());

        let referenced = canonicalize_tool_result(
            workspace.path(),
            "session",
            "exec",
            "large",
            "ok",
            "界".repeat(INLINE_THRESHOLD_CHARS + 1),
        )
        .await;
        let reference = referenced.reference().unwrap();
        assert_eq!(reference.tool_call_id, "large");
        assert_eq!(reference.char_count, INLINE_THRESHOLD_CHARS + 1);
        assert!(reference.truncated);
        assert!(referenced.content().len() < INLINE_THRESHOLD_CHARS);
        assert!(!referenced.is_error());
    }

    #[tokio::test]
    async fn over_single_item_limit_is_an_explicit_materialization_failure() {
        let workspace = tempfile::tempdir().unwrap();
        let result = canonicalize_tool_result(
            workspace.path(),
            "session",
            "exec",
            "huge",
            "ok",
            "x".repeat(agent_diva_core::tool_artifact::MAX_ARTIFACT_BYTES as usize + 1),
        )
        .await;
        assert!(result.is_error());
        assert!(result.reference().is_none());
        assert_eq!(result.error_code(), Some("artifact_capacity_exceeded"));
        assert!(result.content().contains("artifact_capacity_exceeded"));
        assert!(!result.content().contains(&"x".repeat(1_000)));
    }
}
