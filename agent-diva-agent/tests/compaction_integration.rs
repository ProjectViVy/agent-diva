//! C5b canonical checkpoint integration contracts.

use agent_diva_agent::compaction::{CheckpointCompactor, CheckpointSnapshot};
use agent_diva_agent::context::ContextBuilder;
use agent_diva_agent::context_budget::BudgetConfig;
use agent_diva_core::session::{
    CanonicalCheckpoint, ChatMessage, CheckpointTrigger, Session, CANONICAL_CHECKPOINT_MAX_CHARS,
};
use agent_diva_providers::{LLMProvider, LLMResponse, Message, ProviderResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

struct StableCheckpointProvider;

#[async_trait]
impl LLMProvider for StableCheckpointProvider {
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
            content: Some(
                "<checkpoint>\n## 目标与约束\n保留 task constraint decision。\n\n## 已完成事项\n已完成 task。\n\n## 关键决定\n继续当前 decision。\n\n## 当前状态\n任务当前状态稳定。\n\n## 未解决问题\n无。\n\n## 下一步\n继续完成 task。\n\n## 保留标识符与 artifact 引用\n保留 artifact://checkpoint。\n</checkpoint>"
                    .into(),
            ),
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

fn session_with_messages(count: usize) -> Session {
    let mut session = Session::new("integration:checkpoint");
    for index in 0..count {
        session.add_message("user", format!("task constraint decision message {index}"));
    }
    session
}

fn config() -> BudgetConfig {
    BudgetConfig {
        keep_recent_count: 2,
        ..BudgetConfig::default()
    }
}

async fn compact(
    session: &Session,
    trigger: CheckpointTrigger,
) -> agent_diva_agent::compaction::PendingCheckpointUpdate {
    CheckpointCompactor::compact_snapshot(
        CheckpointSnapshot::from_session(session, Vec::new()),
        &config(),
        Arc::new(StableCheckpointProvider),
        "mock",
        trigger,
    )
    .await
    .unwrap()
    .unwrap()
}

#[tokio::test]
async fn repeated_compaction_replaces_one_bounded_checkpoint() {
    let mut session = session_with_messages(12);
    let first = compact(&session, CheckpointTrigger::Auto).await;
    session.canonical_checkpoint = Some(first.checkpoint.clone());
    assert_eq!(first.checkpoint.durable_message_index, 10);

    for index in 12..16 {
        session.add_message("user", format!("task constraint decision message {index}"));
    }
    let second = compact(&session, CheckpointTrigger::Manual).await;
    session.canonical_checkpoint = Some(second.checkpoint.clone());

    assert_eq!(second.source_start_index, 10);
    assert_eq!(second.source_end_index, 14);
    assert_eq!(
        session
            .canonical_checkpoint
            .as_ref()
            .unwrap()
            .schema_version,
        1
    );
    assert!(
        session
            .canonical_checkpoint
            .as_ref()
            .unwrap()
            .body
            .chars()
            .count()
            <= CANONICAL_CHECKPOINT_MAX_CHARS
    );
    assert_eq!(session.get_history(usize::MAX).len(), 2);
}

#[tokio::test]
async fn trigger_kind_does_not_change_snapshot_boundary_or_order() {
    let session = session_with_messages(12);
    let auto = compact(&session, CheckpointTrigger::Auto).await;
    let manual = compact(&session, CheckpointTrigger::Manual).await;
    let reactive = compact(&session, CheckpointTrigger::Reactive).await;

    assert_eq!(auto.source_start_index, manual.source_start_index);
    assert_eq!(auto.source_end_index, manual.source_end_index);
    assert_eq!(auto.source_end_index, reactive.source_end_index);
    assert!(auto.checkpoint.body.contains("## 目标与约束"));
    assert!(auto.checkpoint.body.contains("## 当前状态"));
}

#[test]
fn context_injects_one_checkpoint_between_stable_prefix_and_history() {
    let builder = ContextBuilder::new(PathBuf::from("."));
    let checkpoint = CanonicalCheckpoint::new(
        "checkpoint-test",
        "2026-01-01T00:00:00Z",
        CheckpointTrigger::Auto,
        1,
        1,
        10,
        Some(1.0),
        Vec::new(),
        0,
        "## 目标与约束\nkeep it bounded\n## 已完成事项\ndone\n## 关键决定\nkeep\n## 当前状态\nready\n## 未解决问题\nnone\n## 下一步\ncontinue\n## 保留标识符与 artifact 引用\nartifact://checkpoint",
    );
    let messages = builder.build_messages(
        vec![ChatMessage::new("user", "active history")],
        "current user".into(),
        None,
        None,
        Some(&checkpoint),
    );
    let checkpoint_blocks = messages
        .iter()
        .filter(|message| {
            message
                .content
                .as_text()
                .is_some_and(|content| content.contains("Canonical Checkpoint"))
        })
        .count();
    assert_eq!(checkpoint_blocks, 1);
    assert!(messages[2]
        .content
        .as_text()
        .is_some_and(|content| content.contains("active history")));
}

#[test]
fn new_session_has_no_checkpoint_and_reset_clears_it() {
    let mut session = Session::new("reset");
    assert!(session.canonical_checkpoint.is_none());
    session.canonical_checkpoint = Some(CanonicalCheckpoint::new(
        "checkpoint",
        "2026-01-01T00:00:00Z",
        CheckpointTrigger::Manual,
        0,
        0,
        0,
        None,
        Vec::new(),
        0,
        "## 当前状态\nready",
    ));
    session.clear();
    assert!(session.canonical_checkpoint.is_none());
}

#[test]
fn checkpoint_roundtrips_through_session_manager() {
    let temp = TempDir::new().unwrap();
    let mut manager = agent_diva_core::session::SessionManager::new(temp.path());
    let session = manager.get_or_create("persisted");
    session.canonical_checkpoint = Some(CanonicalCheckpoint::new(
        "checkpoint-persisted",
        "2026-01-01T00:00:00Z",
        CheckpointTrigger::Auto,
        3,
        3,
        21,
        Some(0.8),
        Vec::new(),
        1,
        "## 当前状态\npersisted",
    ));
    let snapshot = manager.get("persisted").unwrap().clone();
    manager.save(&snapshot).unwrap();

    let mut restored_manager = agent_diva_core::session::SessionManager::new(temp.path());
    let restored = restored_manager.get_or_load("persisted").unwrap();
    assert_eq!(
        restored
            .canonical_checkpoint
            .as_ref()
            .unwrap()
            .checkpoint_id,
        "checkpoint-persisted"
    );
}
