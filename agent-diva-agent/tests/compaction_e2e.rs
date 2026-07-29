//! CC-P6: Compaction End-to-End Production Validation
//!
//! Tests the full compaction lifecycle in production-like scenarios:
//! 1. 200+ turn session with auto-trigger
//! 2. Multi-compaction chain (3 rounds)
//! 3. Reactive compact (provider overflow simulation)
//! 4. Post-compaction continuity (summary boundary injection)
//! 5. Token savings measurement
//! 6. Quality validation integration with retry
//! 7. Edge cases

use agent_diva_agent::compaction::ContextCompactor;
use agent_diva_agent::context::ContextBuilder;
use agent_diva_agent::context_budget::{self, BudgetConfig};
use agent_diva_agent::token_estimate::{estimate_tokens, estimate_total_tokens};
use agent_diva_agent::{AgentLoop, ToolConfig};
use agent_diva_core::bus::MessageBus;
use agent_diva_core::session::SessionManager;
use agent_diva_core::session::{
    ChatMessage, CompactSummary, CompactTrigger, CompactionRange, Session,
};
use agent_diva_files::{FileConfig, FileManager};
use agent_diva_providers::{
    LLMProvider, LLMResponse, LLMStreamEvent, Message, ProviderError, ProviderEventStream,
    ProviderResult, ToolChoiceMode,
};
use async_trait::async_trait;
use chrono::Utc;
use futures::stream;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::sync::Mutex;

// ═══════════════════════════════════════════════════════════════════════════
// Mock LLM Provider
// ═══════════════════════════════════════════════════════════════════════════

/// A mock LLM provider that returns configurable compaction summaries.
struct MockCompactionProvider {
    /// Number of times `chat()` was called
    call_count: AtomicUsize,
    /// The summary text to return (wrapped in <summary> tags)
    summary_text: String,
}

impl MockCompactionProvider {
    fn new(summary_text: &str) -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            summary_text: summary_text.to_string(),
        }
    }

    fn call_count(&self) -> usize {
        self.call_count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl LLMProvider for MockCompactionProvider {
    async fn chat(
        &self,
        _messages: Vec<agent_diva_providers::Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        self.call_count.fetch_add(1, Ordering::Relaxed);
        let response_text = format!(
            "<analysis>\n对话包含多轮交互，涉及项目开发、配置修改、bug修复等。\n</analysis>\n<summary>\n{}\n</summary>",
            self.summary_text
        );
        Ok(LLMResponse {
            content: Some(response_text),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        })
    }

    fn get_default_model(&self) -> String {
        "mock-model".to_string()
    }
}

/// A mock provider that returns progressively better summaries on retry.
struct RetryMockProvider {
    call_count: AtomicUsize,
    /// First attempt: very short summary (below quality threshold)
    first_summary: String,
    /// Second attempt: detailed summary (above quality threshold)
    second_summary: String,
}

impl RetryMockProvider {
    fn new() -> Self {
        Self {
            call_count: AtomicUsize::new(0),
            first_summary: "太短了".to_string(),
            second_summary: "用户与助手进行了多轮深入的技术讨论。用户请求修改 src/main.rs 配置文件中的数据库连接参数，助手成功执行了修改并验证编译通过。随后用户报告了一个空指针异常的 bug，助手定位到问题在 utils.rs 第 42 行，修复后所有测试通过。整个开发过程使用了 Rust + Tokio 异步技术栈。".to_string(),
        }
    }

    fn call_count(&self) -> usize {
        self.call_count.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl LLMProvider for RetryMockProvider {
    async fn chat(
        &self,
        _messages: Vec<agent_diva_providers::Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let count = self.call_count.fetch_add(1, Ordering::Relaxed);
        let summary = if count == 0 {
            &self.first_summary
        } else {
            &self.second_summary
        };
        let response_text = format!(
            "<analysis>\n分析内容\n</analysis>\n<summary>\n{}\n</summary>",
            summary
        );
        Ok(LLMResponse {
            content: Some(response_text),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        })
    }

    fn get_default_model(&self) -> String {
        "mock-model".to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

fn make_session(key: &str, count: usize, content_template: &str) -> Session {
    let mut session = Session::new(key);
    for i in 0..count {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        session.add_message(role, format!("[turn-{}] {}", i, content_template));
    }
    session
}

fn make_compact_summary(
    summary_text: &str,
    trigger: CompactTrigger,
    start: usize,
    end: usize,
    kept: usize,
    pre_count: usize,
) -> CompactSummary {
    CompactSummary {
        schema_version: 1,
        compact_id: uuid::Uuid::new_v4().to_string(),
        created_at: Utc::now().to_rfc3339(),
        trigger,
        source_range: CompactionRange {
            start_index: start,
            end_index: end,
        },
        kept_recent_count: kept,
        pre_compact_message_count: pre_count,
        pre_compact_estimated_tokens: 0,
        summary: summary_text.to_string(),
        quality_score: None,
        retry_count: 0,
    }
}

/// Small budget config for testing — triggers compaction quickly
fn small_budget() -> BudgetConfig {
    BudgetConfig {
        max_tokens: 5_000,
        system_budget_ratio: 0.0,
        compact_threshold_ratio: 0.80,
        keep_recent_count: 10,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test A: 200+ turn session auto-trigger
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_e2e_200_plus_turn_auto_trigger() {
    // Create a 220-turn session with realistic content
    let content = "用户讨论了项目架构设计，包括 Rust 后端、Tauri 桌面端、Vue 前端的技术选型。助手建议使用 workspace 模式管理多个 crate。";
    let mut session = make_session("e2e:auto-trigger", 220, content);

    // Verify budget check at various points
    let config = small_budget();

    // Early in the session — should NOT trigger
    let early_history: Vec<ChatMessage> = session.messages[..20].to_vec();
    let early_report = context_budget::check_budget(&early_history, &config);
    assert!(
        !early_report.should_compact,
        "20 messages should not trigger compaction, pressure={:.2}",
        early_report.pressure_ratio
    );

    // Full session — SHOULD trigger
    let full_history = session.get_history(200);
    let full_report = context_budget::check_budget(&full_history, &config);
    assert!(
        full_report.should_compact,
        "220 messages should trigger compaction, pressure={:.2}",
        full_report.pressure_ratio
    );

    // Run compaction with mock provider
    let summary_text = "用户与助手进行了 220 轮深入讨论。涵盖：项目架构设计（Rust + Tauri + Vue），workspace 多 crate 管理策略，数据库 schema 迁移方案，API 接口设计规范，性能优化建议（异步 I/O、连接池），以及 12 个 bug 的定位与修复。用户偏好简洁的代码风格，项目使用 Cargo workspace 组织。";
    let provider = Arc::new(MockCompactionProvider::new(summary_text));

    let result = ContextCompactor::compact(
        &session,
        &config,
        provider.clone(),
        "mock-model",
        CompactTrigger::Auto,
        &session.compaction_history,
    )
    .await
    .expect("compaction should succeed");

    // Verify result
    assert!(
        !result.summary.summary.is_empty(),
        "summary should not be empty"
    );
    assert!(
        result.summary.summary.contains("220 轮"),
        "summary should reference conversation scale"
    );
    assert!(
        matches!(result.summary.trigger, CompactTrigger::Auto),
        "trigger should be Auto"
    );
    assert!(
        result.summary.pre_compact_message_count > 0,
        "should have compacted messages"
    );
    assert!(
        result.summary.pre_compact_estimated_tokens > 0,
        "should have token estimate"
    );
    assert!(
        result.new_compacted_index > session.last_compacted,
        "compacted index should advance"
    );

    // Apply to session
    session.last_compacted = result.new_compacted_index;
    session.compaction_history.push(result.summary.clone());

    // Verify get_history returns only the tail
    let tail = session.get_history(50);
    assert!(
        tail.len() <= 10,
        "after compaction, tail should be at most keep_recent_count, got {}",
        tail.len()
    );

    // Verify provider was called exactly once
    assert_eq!(provider.call_count(), 1);

    println!(
        "Test A passed: {} messages → {} chars summary, {} tokens saved",
        result.summary.pre_compact_message_count,
        result.summary.summary.len(),
        result.summary.pre_compact_estimated_tokens
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Test B: Multi-compaction chain (3 rounds)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_e2e_multi_compaction_chain() {
    let config = small_budget();
    let mut session = Session::new("e2e:multi-chain");

    // Round 1: 150 messages → compact
    for i in 0..150 {
        session.add_message(
            if i % 2 == 0 { "user" } else { "assistant" },
            format!(
                "[r1-turn-{}] 讨论项目初始化、依赖配置、CI/CD 流水线搭建。",
                i
            ),
        );
    }

    let provider1 = Arc::new(MockCompactionProvider::new(
        "第一轮压缩摘要：完成项目初始化（Cargo workspace 12 crates），配置 CI/CD（GitHub Actions），搭建依赖管理（cargo-deny, audit）。选择 Rust 2021 edition，tokio 异步运行时。",
    ));

    let r1 = ContextCompactor::compact(
        &session,
        &config,
        provider1,
        "mock-model",
        CompactTrigger::Auto,
        &session.compaction_history,
    )
    .await
    .expect("round 1 compaction should succeed");

    session.last_compacted = r1.new_compacted_index;
    session.compaction_history.push(r1.summary);

    // Round 2: add 100 more messages → compact again
    for i in 0..100 {
        session.add_message(
            if i % 2 == 0 { "user" } else { "assistant" },
            format!("[r2-turn-{}] 讨论 API 设计、数据库 schema、路由实现。", i),
        );
    }

    let provider2 = Arc::new(MockCompactionProvider::new(
        "第二轮压缩摘要（融合第一轮）：在项目初始化基础上，完成 RESTful API 设计（12 个端点），PostgreSQL schema 迁移（3 次 migration），Axum 路由实现，中间件认证（JWT + RBAC）。",
    ));

    let r2 = ContextCompactor::compact(
        &session,
        &config,
        provider2,
        "mock-model",
        CompactTrigger::Auto,
        &session.compaction_history,
    )
    .await
    .expect("round 2 compaction should succeed");

    session.last_compacted = r2.new_compacted_index;
    session.compaction_history.push(r2.summary);

    // Round 3: add 80 more messages → compact again
    for i in 0..80 {
        session.add_message(
            if i % 2 == 0 { "user" } else { "assistant" },
            format!("[r3-turn-{}] 讨论测试策略、性能优化、bug 修复。", i),
        );
    }

    let provider3 = Arc::new(MockCompactionProvider::new(
        "第三轮压缩摘要（融合前两轮）：在 API 和 DB 基础上，编写 45 个单元测试 + 12 个集成测试，性能优化（查询 N+1 → JOIN，响应时间降低 60%），修复 8 个 P1 bug。项目进入 beta 阶段。",
    ));

    let r3 = ContextCompactor::compact(
        &session,
        &config,
        provider3,
        "mock-model",
        CompactTrigger::Auto,
        &session.compaction_history,
    )
    .await
    .expect("round 3 compaction should succeed");

    session.last_compacted = r3.new_compacted_index;
    session.compaction_history.push(r3.summary);

    // Verify chain
    assert_eq!(
        session.compaction_history.len(),
        3,
        "should have 3 compaction records"
    );

    // Verify contiguous ranges
    let ranges: Vec<_> = session
        .compaction_history
        .iter()
        .map(|c| (c.source_range.start_index, c.source_range.end_index))
        .collect();
    assert_eq!(ranges[0].1, ranges[1].0, "chain should be contiguous (1→2)");
    assert_eq!(ranges[1].1, ranges[2].0, "chain should be contiguous (2→3)");

    // Verify all_summaries_text
    let all = session.all_summaries_text();
    assert!(all.contains("Compaction record 1/3"));
    assert!(all.contains("Compaction record 2/3"));
    assert!(all.contains("Compaction record 3/3"));
    assert!(all.contains("第一轮"));
    assert!(all.contains("第二轮"));
    assert!(all.contains("第三轮"));

    // Verify latest_compaction
    let latest = session.latest_compaction().unwrap();
    assert!(latest.summary.contains("beta 阶段"));

    // Verify serialization roundtrip
    let json = serde_json::to_string(&session).unwrap();
    let restored: Session = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.compaction_history.len(), 3);
    assert_eq!(restored.last_compacted, session.last_compacted);

    println!(
        "Test B passed: 3-round chain, ranges={:?}, total_compacted={}",
        ranges, session.last_compacted
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Test C: Reactive compact (provider overflow simulation)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_e2e_reactive_compact() {
    // Simulate: session has 100 messages, budget is tight, provider returns overflow error
    let content = "讨论微服务架构设计，包括服务发现机制选型（Consul vs etcd vs Nginx），负载均衡策略（轮询、加权、最少连接），熔断器模式（Hystrix vs Resilience4j），分布式追踪系统（Jaeger vs Zipkin）集成方案。";
    let mut session = make_session("e2e:reactive", 100, content);

    let config = BudgetConfig {
        max_tokens: 1_000,
        system_budget_ratio: 0.0,
        compact_threshold_ratio: 0.80,
        keep_recent_count: 10,
    };

    // Verify the session is over budget
    let history = session.get_history(50);
    let report = context_budget::check_budget(&history, &config);
    assert!(
        report.should_compact,
        "session should be over budget for reactive test"
    );

    // Simulate reactive compaction (trigger = Reactive)
    let provider = Arc::new(MockCompactionProvider::new(
        "用户与助手讨论了微服务架构。关键决策：使用 Consul 做服务发现，Nginx 做负载均衡，Hystrix 熔断器，Jaeger 分布式追踪。部署方案：Kubernetes + Helm charts。",
    ));

    let result = ContextCompactor::compact(
        &session,
        &config,
        provider,
        "mock-model",
        CompactTrigger::Reactive,
        &session.compaction_history,
    )
    .await
    .expect("reactive compaction should succeed");

    // Verify trigger type is Reactive
    assert!(
        matches!(result.summary.trigger, CompactTrigger::Reactive),
        "trigger should be Reactive"
    );

    // Apply and verify
    session.last_compacted = result.new_compacted_index;
    session.compaction_history.push(result.summary);

    // After reactive compact, the session should be lean enough
    let post_history = session.get_history(50);
    let post_report = context_budget::check_budget(&post_history, &config);
    assert!(
        !post_report.should_compact,
        "after reactive compact, session should be below threshold"
    );

    println!(
        "Test C passed: reactive compact, trigger=Reactive, post_pressure={:.2}",
        post_report.pressure_ratio
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Test D: Post-compaction continuity (build_messages boundary injection)
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_e2e_post_compaction_continuity() {
    let mut session = make_session("e2e:continuity", 50, "讨论前端 Vue 组件设计。");

    // Manually set up a compaction
    let summary = CompactSummary {
        schema_version: 1,
        compact_id: "test-compact-001".to_string(),
        created_at: Utc::now().to_rfc3339(),
        trigger: CompactTrigger::Auto,
        source_range: CompactionRange {
            start_index: 0,
            end_index: 40,
        },
        kept_recent_count: 10,
        pre_compact_message_count: 40,
        pre_compact_estimated_tokens: 2000,
        summary: "早期对话摘要：用户与助手讨论了 Vue 3 组件架构、Pinia 状态管理、Vue Router 配置。用户偏好 Composition API + script setup 语法。".to_string(),
        quality_score: Some(0.85),
        retry_count: 0,
    };
    session.compaction_history.push(summary);
    session.last_compacted = 40;

    // Get post-compaction history
    let history = session.get_history(50);

    // Build messages
    let builder = ContextBuilder::default();
    let messages = builder.build_messages(
        history.clone(),
        "继续开发前端组件。".to_string(),
        Some("telegram"),
        Some("continuity-test"),
        &session.compaction_history,
    );

    // Verify structure
    // system (1) + boundary_start (1) + summary (1) + boundary_end (1) + history (10) + current (1) = 15
    assert_eq!(
        messages.len(),
        15,
        "expected 15 messages, got {}",
        messages.len()
    );

    // Verify boundary markers
    let boundary_start = messages[1].content.to_text_lossy();
    assert!(
        boundary_start.contains("Context Compaction Boundary"),
        "should have boundary start marker"
    );
    assert!(
        boundary_start.contains("compacted context start"),
        "should have compacted context start"
    );

    // Verify summary content
    let summary_msg = messages[2].content.to_text_lossy();
    assert!(
        summary_msg.contains("Vue 3"),
        "summary should contain key context"
    );
    assert!(
        summary_msg.contains("Pinia"),
        "summary should retain technical terms"
    );

    // Verify boundary end
    let boundary_end = messages[3].content.to_text_lossy();
    assert!(
        boundary_end.contains("compacted context end"),
        "should have boundary end marker"
    );

    // Verify recent messages are after boundary
    let first_recent = messages[4].content.to_text_lossy();
    assert!(
        first_recent.contains("turn-40"),
        "first recent message should be from index 40"
    );

    // Verify current message is last
    let current = messages.last().unwrap().content.to_text_lossy();
    assert!(
        current.contains("继续开发前端组件"),
        "current message should be at the end"
    );

    println!(
        "Test D passed: post-compaction continuity verified, {} messages in context",
        messages.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Test E: Token savings measurement
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_e2e_token_savings() {
    let content = "用户询问 Rust 异步编程的最佳实践，包括 tokio 运行时配置、错误处理策略、并发模式选择。助手详细解释了 async/await 语法、Pin/Unpin 概念、以及 Future trait 的实现细节。";
    let mut session = make_session("e2e:token-savings", 200, content);

    let config = small_budget();

    // Measure pre-compaction tokens
    let pre_history = session.get_history(200);
    let pre_tokens = estimate_total_tokens(&pre_history);
    assert!(pre_tokens > 0, "pre-compaction tokens should be > 0");

    // Run compaction
    let provider = Arc::new(MockCompactionProvider::new(
        "用户深入学习了 Rust 异步编程。关键收获：tokio 运行时配置（多线程 vs 当前线程），错误处理（anyhow + thiserror），并发模式（select!、join!），Pin/Unpin 安全性，Future trait 自实现。项目采用 tokio 多线程运行时。",
    ));

    let result = ContextCompactor::compact(
        &session,
        &config,
        provider,
        "mock-model",
        CompactTrigger::Auto,
        &session.compaction_history,
    )
    .await
    .expect("compaction should succeed");

    // Apply compaction
    session.last_compacted = result.new_compacted_index;
    session.compaction_history.push(result.summary.clone());

    // Measure post-compaction tokens
    let post_history = session.get_history(50);
    let post_tokens = estimate_total_tokens(&post_history);

    // Calculate savings
    let summary_tokens = estimate_tokens(&result.summary.summary);
    let effective_tokens = post_tokens + summary_tokens;
    let savings_ratio = 1.0 - (effective_tokens as f64 / pre_tokens as f64);

    println!(
        "Token savings: pre={}, post={}, summary={}, effective={}, savings={:.1}%",
        pre_tokens,
        post_tokens,
        summary_tokens,
        effective_tokens,
        savings_ratio * 100.0
    );

    // Verify savings are meaningful (>40% for 200 messages)
    assert!(
        savings_ratio > 0.4,
        "savings ratio should be > 40%, got {:.1}%",
        savings_ratio * 100.0
    );

    // Verify the pre_compact_estimated_tokens in summary matches our measurement
    assert!(
        result.summary.pre_compact_estimated_tokens > 0,
        "summary should record pre-compact token count"
    );

    println!(
        "Test E passed: {:.1}% token savings ({}→{} tokens)",
        savings_ratio * 100.0,
        pre_tokens,
        effective_tokens
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Test F: Quality validation integration with retry
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_e2e_quality_retry() {
    let content = "用户请求修改 src/main.rs 文件中的数据库配置参数，助手已执行修改并验证编译通过。用户随后报告了 utils.rs 中的空指针异常 bug。";
    let session = make_session("e2e:quality-retry", 50, content);
    let config = small_budget();

    // Use retry provider: first attempt too short, second attempt good
    let provider = Arc::new(RetryMockProvider::new());

    let result = ContextCompactor::compact(
        &session,
        &config,
        provider.clone(),
        "mock-model",
        CompactTrigger::Auto,
        &session.compaction_history,
    )
    .await
    .expect("compaction should succeed after retry");

    // Verify the provider was called at least twice (initial + retry)
    assert!(
        provider.call_count() >= 2,
        "should have retried at least once, got {} calls",
        provider.call_count()
    );

    // Verify the final summary is the good one
    assert!(
        result.summary.summary.contains("src/main.rs"),
        "final summary should contain key context from retry"
    );
    assert!(
        result.summary.summary.contains("空指针"),
        "final summary should contain bug details"
    );

    // Verify retry_count > 0
    assert!(
        result.summary.retry_count > 0,
        "retry_count should be > 0, got {}",
        result.summary.retry_count
    );

    // Verify quality score is above threshold
    if let Some(score) = result.summary.quality_score {
        assert!(
            score >= 0.6,
            "quality score should be >= 0.6, got {:.2}",
            score
        );
    }

    println!(
        "Test F passed: {} retries, quality_score={:?}, retry_count={}",
        provider.call_count(),
        result.summary.quality_score,
        result.summary.retry_count
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Test G: Edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn test_e2e_edge_empty_session() {
    let session = Session::new("e2e:empty");
    let config = small_budget();
    let provider = Arc::new(MockCompactionProvider::new("should not be called"));

    let result = ContextCompactor::compact(
        &session,
        &config,
        provider.clone(),
        "mock-model",
        CompactTrigger::Auto,
        &[],
    )
    .await
    .expect("empty session compaction should return empty placeholder");

    // Should return empty placeholder without calling LLM
    assert_eq!(
        provider.call_count(),
        0,
        "should not call LLM for empty session"
    );
    assert!(
        result.summary.summary.is_empty(),
        "empty session should produce empty summary"
    );
    assert_eq!(
        result.summary.pre_compact_message_count, 0,
        "empty session should have 0 pre-compact messages"
    );
}

#[tokio::test]
async fn test_e2e_edge_only_keep_recent() {
    let mut session = Session::new("e2e:only-keep-recent");
    // Add exactly keep_recent_count messages
    for i in 0..10 {
        session.add_message("user", format!("message {}", i));
    }
    let config = small_budget();
    let provider = Arc::new(MockCompactionProvider::new("should not be called"));

    let result = ContextCompactor::compact(
        &session,
        &config,
        provider.clone(),
        "mock-model",
        CompactTrigger::Auto,
        &[],
    )
    .await
    .expect("should return empty placeholder");

    // With only keep_recent messages, nothing to compact
    assert_eq!(
        result.summary.pre_compact_message_count, 0,
        "should have nothing to compact"
    );
}

#[tokio::test]
async fn test_e2e_edge_long_message_truncation() {
    let mut session = Session::new("e2e:truncation");
    // Add a message with > 2000 chars
    let long_content = "x".repeat(3000);
    session.add_message("user", &long_content);
    session.add_message("assistant", "收到，已处理。");
    // Add enough messages to trigger compaction
    for i in 0..30 {
        session.add_message("user", format!("short message {}", i));
        session.add_message("assistant", format!("reply {}", i));
    }

    let config = small_budget();
    let provider = Arc::new(MockCompactionProvider::new(
        "用户发送了一条超长消息（3000字符），被截断处理。随后进行了 30 轮简短对话。",
    ));

    let result = ContextCompactor::compact(
        &session,
        &config,
        provider,
        "mock-model",
        CompactTrigger::Auto,
        &[],
    )
    .await
    .expect("compaction with long message should succeed");

    // The compaction should succeed even with long messages
    assert!(
        !result.summary.summary.is_empty(),
        "should produce a summary even with long messages"
    );
    assert!(
        result.summary.pre_compact_message_count > 0,
        "should have compacted messages"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Test H: build_messages with multiple compaction boundaries
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_e2e_build_messages_multi_boundary() {
    let mut session = Session::new("e2e:multi-boundary");

    // Add 300 messages
    for i in 0..300 {
        session.add_message(
            if i % 2 == 0 { "user" } else { "assistant" },
            format!("消息 {}", i),
        );
    }

    // Simulate 3 compactions
    let c1 = CompactSummary {
        schema_version: 1,
        compact_id: "c1".to_string(),
        created_at: Utc::now().to_rfc3339(),
        trigger: CompactTrigger::Auto,
        source_range: CompactionRange {
            start_index: 0,
            end_index: 200,
        },
        kept_recent_count: 10,
        pre_compact_message_count: 200,
        pre_compact_estimated_tokens: 10000,
        summary: "第一阶段摘要：项目启动、需求分析、技术选型。".to_string(),
        quality_score: Some(0.85),
        retry_count: 0,
    };
    let c2 = CompactSummary {
        schema_version: 1,
        compact_id: "c2".to_string(),
        created_at: Utc::now().to_rfc3339(),
        trigger: CompactTrigger::Auto,
        source_range: CompactionRange {
            start_index: 200,
            end_index: 270,
        },
        kept_recent_count: 10,
        pre_compact_message_count: 70,
        pre_compact_estimated_tokens: 3500,
        summary: "第二阶段摘要：核心模块开发完成。".to_string(),
        quality_score: Some(0.80),
        retry_count: 0,
    };
    let c3 = CompactSummary {
        schema_version: 1,
        compact_id: "c3".to_string(),
        created_at: Utc::now().to_rfc3339(),
        trigger: CompactTrigger::Reactive,
        source_range: CompactionRange {
            start_index: 270,
            end_index: 290,
        },
        kept_recent_count: 10,
        pre_compact_message_count: 20,
        pre_compact_estimated_tokens: 1000,
        summary: "第三阶段摘要：测试与优化。".to_string(),
        quality_score: Some(0.90),
        retry_count: 1,
    };

    session.compaction_history = vec![c1, c2, c3];
    session.last_compacted = 290;

    let history = session.get_history(50);
    let builder = ContextBuilder::default();
    let messages = builder.build_messages(
        history,
        "继续工作。".to_string(),
        Some("telegram"),
        Some("multi-boundary-test"),
        &session.compaction_history,
    );

    // Verify 3 boundary groups are injected
    // Each compaction: boundary_start + summary + boundary_end = 3 messages
    // Total: system(1) + 3*3 + 10 history + 1 current = 21
    assert_eq!(messages.len(), 21, "expected 21 messages");

    // Verify first boundary has full markers
    let b1 = messages[1].content.to_text_lossy();
    assert!(b1.contains("Context Compaction Boundary"));
    assert!(b1.contains("compacted context start"));

    // Verify subsequent boundaries have numbered markers
    let b2 = messages[4].content.to_text_lossy();
    assert!(b2.contains("Context Compaction #2"));
    let b3 = messages[7].content.to_text_lossy();
    assert!(b3.contains("Context Compaction #3"));

    // Verify summary content
    assert!(messages[2].content.to_text_lossy().contains("第一阶段"));
    assert!(messages[5].content.to_text_lossy().contains("核心模块"));
    assert!(messages[8].content.to_text_lossy().contains("测试与优化"));

    println!("Test H passed: 3 boundary groups injected correctly in build_messages");
}

#[derive(Debug, Default, Clone)]
struct ProviderObservation {
    persisted_last_compacted: usize,
    persisted_compaction_count: usize,
    saw_boundary: bool,
    first_history_text: Option<String>,
}

fn session_file_path(workspace: &Path, session_key: &str) -> PathBuf {
    let safe_key = session_key.replace([':', '/', '\\'], "_");
    workspace
        .join("sessions")
        .join(format!("{}.jsonl", safe_key))
}

fn persisted_compaction_state(path: &Path) -> (usize, usize) {
    let content = std::fs::read_to_string(path).expect("session file should exist");
    let metadata = content
        .lines()
        .next()
        .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .expect("metadata line should deserialize");
    let last_compacted = metadata
        .get("last_compacted")
        .and_then(|value| value.as_u64())
        .unwrap_or_default() as usize;
    let compaction_count = metadata
        .get("compaction_history")
        .and_then(|value| value.as_array())
        .map(|entries| entries.len())
        .unwrap_or_default();
    (last_compacted, compaction_count)
}

fn first_non_system_message(messages: &[Message], current_user: &str) -> Option<String> {
    messages
        .iter()
        .find(|message| message.role != "system" && message.content.to_text_lossy() != current_user)
        .map(|message| message.content.to_text_lossy())
}

struct OrderingStreamProvider {
    workspace: PathBuf,
    session_key: String,
    current_user: String,
    observations: Mutex<Vec<ProviderObservation>>,
    responses: Mutex<Vec<Result<String, ProviderError>>>,
}

impl OrderingStreamProvider {
    fn new(
        workspace: PathBuf,
        session_key: impl Into<String>,
        current_user: impl Into<String>,
        responses: Vec<Result<String, ProviderError>>,
    ) -> Self {
        Self {
            workspace,
            session_key: session_key.into(),
            current_user: current_user.into(),
            observations: Mutex::new(Vec::new()),
            responses: Mutex::new(responses),
        }
    }

    fn observations(&self) -> Vec<ProviderObservation> {
        self.observations.lock().unwrap().clone()
    }
}

#[async_trait]
impl LLMProvider for OrderingStreamProvider {
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
            content: Some(
                "<summary>compacted history keeps only the recent tail for provider calls</summary>"
                    .to_string(),
            ),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        })
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        let path = session_file_path(&self.workspace, &self.session_key);
        let (persisted_last_compacted, persisted_compaction_count) =
            persisted_compaction_state(&path);
        let observation = ProviderObservation {
            persisted_last_compacted,
            persisted_compaction_count,
            saw_boundary: messages.iter().any(|message| {
                message.role == "system"
                    && message
                        .content
                        .to_text_lossy()
                        .contains("Context Compaction Boundary")
            }),
            first_history_text: first_non_system_message(&messages, &self.current_user),
        };
        self.observations.lock().unwrap().push(observation);

        let next = self.responses.lock().unwrap().remove(0);
        match next {
            Ok(content) => Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
                LLMResponse {
                    content: Some(content),
                    tool_calls: Vec::new(),
                    finish_reason: "stop".to_string(),
                    usage: HashMap::new(),
                    reasoning_content: None,
                },
            ))]))),
            Err(err) => Err(err),
        }
    }

    fn get_default_model(&self) -> String {
        "mock-model".to_string()
    }
}

async fn make_agent_for_ordering_test(
    workspace: &Path,
    provider: Arc<dyn LLMProvider>,
    budget: BudgetConfig,
) -> AgentLoop {
    let file_manager = Arc::new(
        FileManager::new(FileConfig::with_path(workspace.join("files")))
            .await
            .unwrap(),
    );
    let tool_config = ToolConfig {
        budget,
        ..ToolConfig::default()
    };

    AgentLoop::with_tools(
        MessageBus::new(),
        provider,
        workspace.to_path_buf(),
        None,
        Some(1),
        tool_config,
        None,
        file_manager,
    )
    .await
    .unwrap()
}

fn seed_session(
    workspace: &Path,
    session_key: &str,
    message_count: usize,
    content: impl Fn(usize) -> String,
) {
    let mut manager = SessionManager::new(workspace);
    let session = manager.get_or_create(session_key);
    for idx in 0..message_count {
        session.add_message("user", content(idx));
    }
    let snapshot = manager.get(session_key).unwrap().clone();
    manager.save(&snapshot).unwrap();
}

#[tokio::test]
async fn test_e2e_auto_compaction_persists_before_first_provider_call() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    let session_key = "gui:chat-auto";
    let current_user = "current auto turn";
    seed_session(&workspace, session_key, 60, |idx| {
        format!("[seed-{idx}] {}", "x".repeat(360))
    });

    let budget = BudgetConfig {
        max_tokens: 3_000,
        system_budget_ratio: 0.0,
        compact_threshold_ratio: 0.8,
        keep_recent_count: 8,
    };
    let provider = Arc::new(OrderingStreamProvider::new(
        workspace.clone(),
        session_key,
        current_user,
        vec![Ok("done".to_string())],
    ));
    let mut agent = make_agent_for_ordering_test(&workspace, provider.clone(), budget).await;

    let response = agent
        .process_direct(current_user, "session-1", "gui", "chat-auto")
        .await
        .unwrap();

    assert_eq!(response, "done");
    let observations = provider.observations();
    assert_eq!(observations.len(), 1);
    let first = &observations[0];
    assert!(
        first.saw_boundary,
        "provider should receive compacted boundary markers"
    );
    assert_eq!(first.persisted_compaction_count, 1);
    assert!(
        first.persisted_last_compacted >= 52,
        "compaction should be persisted before provider call"
    );
    let expected_first = format!("[seed-52] {}", "x".repeat(360));
    assert_eq!(
        first.first_history_text.as_deref(),
        Some(expected_first.as_str())
    );
}

#[tokio::test]
async fn test_e2e_reactive_compaction_persists_then_retries_once() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    let session_key = "gui:chat-reactive";
    let current_user = "current reactive turn";
    seed_session(&workspace, session_key, 24, |idx| {
        format!("[seed-{idx}] {}", "y".repeat(60))
    });

    let budget = BudgetConfig {
        max_tokens: 20_000,
        system_budget_ratio: 0.0,
        compact_threshold_ratio: 0.95,
        keep_recent_count: 6,
    };
    let provider = Arc::new(OrderingStreamProvider::new(
        workspace.clone(),
        session_key,
        current_user,
        vec![
            Err(ProviderError::ApiError(
                "context_length_exceeded: mocked overflow".to_string(),
            )),
            Ok("done".to_string()),
        ],
    ));
    let mut agent = make_agent_for_ordering_test(&workspace, provider.clone(), budget).await;

    let response = agent
        .process_direct(current_user, "session-1", "gui", "chat-reactive")
        .await
        .unwrap();

    assert_eq!(response, "done");
    let observations = provider.observations();
    assert_eq!(
        observations.len(),
        2,
        "overflow path should retry exactly once"
    );
    assert_eq!(observations[0].persisted_compaction_count, 0);
    assert!(!observations[0].saw_boundary);
    assert_eq!(observations[1].persisted_compaction_count, 1);
    assert!(observations[1].persisted_last_compacted >= 18);
    assert!(observations[1].saw_boundary);
    let expected_first = format!("[seed-18] {}", "y".repeat(60));
    assert_eq!(
        observations[1].first_history_text.as_deref(),
        Some(expected_first.as_str())
    );
}

#[tokio::test]
async fn test_e2e_reactive_overflow_retries_only_once_on_second_failure() {
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    let session_key = "gui:chat-reactive-fail";
    let current_user = "current overflow turn";
    seed_session(&workspace, session_key, 18, |idx| {
        format!("[seed-{idx}] {}", "z".repeat(80))
    });

    let provider = Arc::new(OrderingStreamProvider::new(
        workspace.clone(),
        session_key,
        current_user,
        vec![
            Err(ProviderError::ApiError(
                "context_length_exceeded: first overflow".to_string(),
            )),
            Err(ProviderError::ApiError(
                "context_length_exceeded: second overflow".to_string(),
            )),
        ],
    ));
    let mut agent = make_agent_for_ordering_test(
        &workspace,
        provider.clone(),
        BudgetConfig {
            max_tokens: 50_000,
            system_budget_ratio: 0.0,
            compact_threshold_ratio: 0.99,
            keep_recent_count: 4,
        },
    )
    .await;

    let err = agent
        .process_direct(current_user, "session-1", "gui", "chat-reactive-fail")
        .await
        .unwrap_err();

    assert!(err.to_string().contains("second overflow"));
    assert_eq!(provider.observations().len(), 2);
}

struct PromptCaptureProvider {
    prompt: Mutex<Option<String>>,
}

impl PromptCaptureProvider {
    fn new() -> Self {
        Self {
            prompt: Mutex::new(None),
        }
    }

    fn prompt(&self) -> String {
        self.prompt.lock().unwrap().clone().unwrap()
    }
}

#[async_trait]
impl LLMProvider for PromptCaptureProvider {
    async fn chat(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        let user_prompt = messages
            .iter()
            .find(|message| message.role == "user")
            .map(|message| message.content.to_text_lossy())
            .unwrap_or_default();
        *self.prompt.lock().unwrap() = Some(user_prompt);
        Ok(LLMResponse {
            content: Some("<summary>meta compacted summary</summary>".to_string()),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        })
    }

    fn get_default_model(&self) -> String {
        "mock-model".to_string()
    }
}

#[tokio::test]
async fn test_e2e_meta_compaction_truncation_preserves_newer_prior_facts() {
    let mut session = Session::new("e2e:meta-priors");
    for idx in 0..20 {
        session.add_message("user", format!("[turn-{idx}] {}", "message ".repeat(20)));
    }

    let prior_summaries = vec![
        make_compact_summary(
            "older fact: deprecated endpoint",
            CompactTrigger::Auto,
            0,
            5,
            5,
            5,
        ),
        make_compact_summary(
            "newer fact: keep timeout=30s and preserve retry once semantics",
            CompactTrigger::Auto,
            5,
            10,
            5,
            5,
        ),
        make_compact_summary(
            "latest fact: rebuild messages from the post-compaction snapshot before provider retry",
            CompactTrigger::Reactive,
            10,
            15,
            5,
            5,
        ),
    ];

    let provider = Arc::new(PromptCaptureProvider::new());
    let result = ContextCompactor::compact(
        &session,
        &BudgetConfig {
            max_tokens: 500,
            system_budget_ratio: 0.0,
            compact_threshold_ratio: 0.8,
            keep_recent_count: 4,
        },
        provider.clone(),
        "mock-model",
        CompactTrigger::Auto,
        &prior_summaries,
    )
    .await
    .unwrap();

    assert_eq!(result.summary.summary, "meta compacted summary");
    let prompt = provider.prompt();
    assert!(prompt.contains("timeout=30s"));
    assert!(prompt.contains("retry once semantics"));
    assert!(prompt.contains("post-compaction"));
}

#[test]
fn test_e2e_session_compaction_compat_tolerates_unknown_fields_and_future_schema() {
    let session_json = serde_json::json!({
        "key": "gui:future-compaction",
        "messages": [],
        "created_at": "2026-01-01T00:00:00Z",
        "updated_at": "2026-01-01T00:00:00Z",
        "metadata": {
            "room": "wave-d",
            "unknown_meta_flag": true
        },
        "last_consolidated": 0,
        "last_compacted": 9,
        "compaction_history": [
            {
                "schema_version": 77,
                "compact_id": "future-001",
                "created_at": "2026-01-01T00:00:00Z",
                "trigger": "reactive",
                "source_range": { "start_index": 0, "end_index": 9 },
                "kept_recent_count": 4,
                "pre_compact_message_count": 9,
                "pre_compact_estimated_tokens": 900,
                "summary": "future schema compaction summary",
                "quality_score": 0.95,
                "retry_count": 1,
                "unknown_future_field": {
                    "nested": true
                }
            }
        ],
        "top_level_future_field": "ignored"
    });

    let session: Session = serde_json::from_value(session_json).unwrap();
    assert_eq!(session.last_compacted, 9);
    assert_eq!(session.compaction_history.len(), 1);
    assert_eq!(session.compaction_history[0].schema_version, 77);
    assert!(matches!(
        session.compaction_history[0].trigger,
        CompactTrigger::Reactive
    ));
    assert_eq!(
        session
            .metadata
            .get("unknown_meta_flag")
            .and_then(|v| v.as_bool()),
        Some(true)
    );
}
