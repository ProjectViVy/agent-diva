use agent_diva_agent::compaction::QualityGate;
use agent_diva_agent::consolidation::{consolidate_with_gate, DEFAULT_MEMORY_WINDOW};
use agent_diva_core::memory::{
    MemoryAddRequest, MemoryCrudContext, MemoryCrudOutcome, MemoryProvider, PrefetchRequest,
    PrefetchResponse, PrefetchStatus, SessionEndRequest, SessionEndResponse, SessionEndStatus,
    StartupStatus, SyncTurnRequest, SyncTurnResponse, SystemPromptBlock, SystemPromptRequest,
    SystemPromptResponse,
};
use agent_diva_core::session::{ChatMessage, Session};
use agent_diva_laputa::LaputaPaths;
use agent_diva_providers::{
    LLMProvider, LLMResponse, Message, ProviderEventStream, ProviderResult, ToolCallRequest,
    ToolChoiceMode,
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
        last_compacted: 0,
        compaction_history: vec![],
    }
}

struct WorldClaimProvider;

#[async_trait::async_trait]
impl LLMProvider for WorldClaimProvider {
    async fn chat(
        &self,
        _: Vec<Message>,
        _: Option<Vec<serde_json::Value>>,
        _: ToolChoiceMode,
        _: Option<String>,
        _: i32,
        _: f64,
    ) -> ProviderResult<LLMResponse> {
        let world_content = "---\nkind: world\ndomain: language\ntitle: Rust ownership\nsource: session\n---\nRust ownership prevents data races at compile time.";
        Ok(LLMResponse {
            content: None,
            tool_calls: vec![ToolCallRequest {
                id: "call-world".to_string(),
                call_type: "function".to_string(),
                name: "save_memory".to_string(),
                arguments: HashMap::from([
                    (
                        "items".to_string(),
                        serde_json::json!([
                            {"action": "add", "content": world_content}
                        ]),
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

struct NoopMemoryProvider {
    add_calls: AtomicUsize,
}

impl NoopMemoryProvider {
    fn new() -> Self {
        Self {
            add_calls: AtomicUsize::new(0),
        }
    }
}

#[async_trait::async_trait]
impl MemoryProvider for NoopMemoryProvider {
    fn system_prompt_block(
        &self,
        _: &SystemPromptRequest,
    ) -> agent_diva_core::Result<SystemPromptResponse> {
        Ok(SystemPromptResponse {
            status: StartupStatus::Ready,
            prompt_block: Some(SystemPromptBlock {
                shape: agent_diva_core::memory::StartupInjectionShape::CompactRenderedMarkdown,
                markdown: String::new(),
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
        Ok(SyncTurnResponse {
            status: agent_diva_core::memory::SyncTurnStatus::Noop,
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

#[tokio::test]
async fn world_claim_routes_to_world_governance_ledger() {
    let workspace = tempfile::tempdir().unwrap();
    let mut session = sample_session(DEFAULT_MEMORY_WINDOW + 10);
    let provider: Arc<dyn LLMProvider> = Arc::new(WorldClaimProvider);
    let memory = NoopMemoryProvider::new();

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

    assert_eq!(
        memory.add_calls.load(Ordering::SeqCst),
        0,
        "WORLD claims must not reach memory_add"
    );

    let ledger_path = LaputaPaths::new(workspace.path())
        .cognitive_dir()
        .join("world-ledger.jsonl");
    let ledger = std::fs::read_to_string(&ledger_path).unwrap();
    assert!(
        ledger.contains("\"action\":\"submit\""),
        "ledger should contain submit action: {ledger}"
    );
    assert!(
        ledger.contains("\"actor\":\"consolidation\""),
        "ledger should record consolidation as actor: {ledger}"
    );
}
