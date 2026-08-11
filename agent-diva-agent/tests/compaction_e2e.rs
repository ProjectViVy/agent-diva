//! C5b reactive and failure-path acceptance tests.

use agent_diva_agent::compaction::{CheckpointCompactor, CheckpointSnapshot};
use agent_diva_agent::context_budget::BudgetConfig;
use agent_diva_core::session::{CanonicalCheckpoint, ChatMessage, CheckpointTrigger, Session};
use agent_diva_providers::{LLMProvider, LLMResponse, Message, ProviderError, ProviderResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;

struct CheckpointProvider;

#[async_trait]
impl LLMProvider for CheckpointProvider {
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
                "目标与约束 task constraint。已完成事项 done。关键决定 decision。当前状态 ready。未解决问题 none。下一步 continue。保留标识符与 artifact 引用 artifact://reactive。".into(),
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

struct FailedProvider;

#[async_trait]
impl LLMProvider for FailedProvider {
    async fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: agent_diva_providers::ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        Err(ProviderError::ApiError(
            "overflow retry provider failed".into(),
        ))
    }

    fn get_default_model(&self) -> String {
        "mock".into()
    }
}

fn durable_session() -> Session {
    let mut session = Session::new("e2e:checkpoint");
    for index in 0..12 {
        session.add_message("user", format!("task constraint decision durable-{index}"));
    }
    session
}

#[tokio::test]
async fn reactive_snapshot_includes_current_turn_but_does_not_mutate_session() {
    let session = durable_session();
    let before = session.clone();
    let current_turn = vec![
        ChatMessage::new("user", "current task constraint"),
        ChatMessage::new("assistant", "working decision"),
    ];
    let config = BudgetConfig {
        keep_recent_count: 0,
        ..BudgetConfig::default()
    };
    let pending = CheckpointCompactor::compact_snapshot(
        CheckpointSnapshot::from_session(&session, current_turn),
        &config,
        Arc::new(CheckpointProvider),
        "mock",
        CheckpointTrigger::Reactive,
    )
    .await
    .unwrap()
    .unwrap();

    assert!(pending.includes_current_turn);
    assert_eq!(
        pending.checkpoint.durable_message_index,
        before.messages.len()
    );
    assert!(session.canonical_checkpoint.is_none());
    let finalized = pending.finalize_for_durable_message_count(before.messages.len() + 2);
    assert_eq!(finalized.durable_message_index, before.messages.len() + 2);
}

#[tokio::test]
async fn reactive_provider_failure_keeps_existing_checkpoint_unchanged() {
    let mut session = durable_session();
    let old = CanonicalCheckpoint::new(
        "old-checkpoint",
        "2026-01-01T00:00:00Z",
        CheckpointTrigger::Auto,
        6,
        6,
        42,
        Some(0.9),
        Vec::new(),
        0,
        "## 当前状态\nold state",
    );
    session.canonical_checkpoint = Some(old.clone());
    let error = CheckpointCompactor::compact_snapshot(
        CheckpointSnapshot::from_session(&session, vec![ChatMessage::new("user", "new task")]),
        &BudgetConfig {
            keep_recent_count: 1,
            ..BudgetConfig::default()
        },
        Arc::new(FailedProvider),
        "mock",
        CheckpointTrigger::Reactive,
    )
    .await
    .unwrap_err();

    assert!(error.to_string().contains("provider"));
    assert_eq!(session.canonical_checkpoint, Some(old));
}

#[test]
fn checkpoint_is_the_only_model_visible_summary_shape() {
    let checkpoint = CanonicalCheckpoint::new(
        "one",
        "2026-01-01T00:00:00Z",
        CheckpointTrigger::Reactive,
        10,
        10,
        100,
        Some(0.8),
        Vec::new(),
        0,
        "## 目标与约束\nkeep\n## 已完成事项\ndone\n## 关键决定\nkeep\n## 当前状态\nready\n## 未解决问题\nnone\n## 下一步\ncontinue\n## 保留标识符与 artifact 引用\nartifact://one",
    );
    let rendered = checkpoint.render_for_context();
    assert_eq!(rendered.matches("Canonical Checkpoint").count(), 1);
    assert!(!rendered.contains("Compaction record"));
}
