//! Epic 7 集成测试 — Context Compaction
//!
//! 测试覆盖:
//! 1. CompactSummary 序列化/反序列化 roundtrip
//! 2. 无 compaction 的 Session 向后兼容加载
//! 3. last_compacted 正确截断 get_history 返回
//! 4. build_messages 有 compaction 时注入 boundary + summary
//! 5. build_messages 无 compaction 时行为不变
//! 6. budget check — 低于阈值不触发 compaction
//! 7. budget check — 高于阈值触发 compaction
//! 8. token 估算精度验证
//! 9. context overflow 检测

use agent_diva_agent::context::ContextBuilder;
use agent_diva_agent::context_budget::{self, BudgetConfig};
use agent_diva_agent::token_estimate::{estimate_tokens, estimate_total_tokens};
use agent_diva_core::session::{ChatMessage, CompactSummary, CompactTrigger, CompactionRange, Session};
use chrono::Utc;
use serde_json;

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

fn make_msg(role: &str, content: &str) -> ChatMessage {
    ChatMessage {
        role: role.to_string(),
        content: content.to_string(),
        timestamp: Utc::now(),
        tool_call_id: None,
        tool_calls: None,
        name: None,
        reasoning_content: None,
        thinking_blocks: None,
    }
}

fn make_session(key: &str) -> Session {
    Session::new(key)
}

fn make_compact_summary(summary_text: &str, trigger: CompactTrigger, start: usize, end: usize, kept: usize, pre_count: usize) -> CompactSummary {
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
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 1: CompactSummary 序列化/反序列化 roundtrip
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_compact_summary_roundtrip() {
    let original = make_compact_summary(
        "用户询问了关于 Rust 异步编程的问题，助手提供了详细解释。",
        CompactTrigger::Auto,
        0,
        10,
        5,
        15,
    );

    // 序列化
    let json = serde_json::to_string(&original).expect("序列化成功");
    assert!(!json.is_empty());
    assert!(json.contains("compact_id"));
    assert!(json.contains("auto")); // trigger
    assert!(json.contains("Rust"));

    // 反序列化
    let restored: CompactSummary = serde_json::from_str(&json).expect("反序列化成功");

    // 验证字段一致
    assert_eq!(restored.schema_version, original.schema_version);
    assert_eq!(restored.compact_id, original.compact_id);
    assert_eq!(restored.created_at, original.created_at);
    assert_eq!(restored.summary, original.summary);
    assert_eq!(restored.source_range.start_index, original.source_range.start_index);
    assert_eq!(restored.source_range.end_index, original.source_range.end_index);
    assert_eq!(restored.kept_recent_count, original.kept_recent_count);
    assert_eq!(restored.pre_compact_message_count, original.pre_compact_message_count);

    // 验证 trigger 枚举 roundtrip
    let trigger_json = serde_json::to_string(&restored.trigger).unwrap();
    let trigger_back: CompactTrigger = serde_json::from_str(&trigger_json).unwrap();
    assert!(matches!(trigger_back, CompactTrigger::Auto));
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 2: 无 compaction 的 Session 向后兼容
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_backward_compat_session_without_compaction() {
    let mut session = make_session("telegram:test-backward");
    session.add_message("user", "你好");
    session.add_message("assistant", "你好！有什么可以帮你的？");
    session.add_message("user", "今天天气怎么样？");
    session.add_message("assistant", "抱歉，我无法获取实时天气信息。");

    // 验证 compaction 字段为 None（向后兼容）
    assert!(session.compaction.is_none());
    assert_eq!(session.last_compacted, 0);

    // get_history 应该正常工作
    let history = session.get_history(50);
    assert_eq!(history.len(), 4);
    assert_eq!(history[0].role, "user");
    assert_eq!(history[0].content, "你好");
    assert_eq!(history[3].content, "抱歉，我无法获取实时天气信息。");

    // 序列化后不含 compaction 字段
    let json = serde_json::to_value(&session).unwrap();
    assert!(json.get("compaction").is_none() || json.get("compaction").unwrap().is_null());
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 3: last_compacted 正确截断 get_history
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_session_get_history_respects_last_compacted() {
    let mut session = make_session("telegram:test-compacted");

    // 添加 20 条消息 (交替 user/assistant)
    for i in 0..20 {
        let role = if i % 2 == 0 { "user" } else { "assistant" };
        session.add_message(role, format!("消息 {}", i));
    }

    // 设置 last_compacted = 10 (前 10 条已压缩)
    session.last_compacted = 10;

    // get_history(50) 应该只返回索引 10..20 的消息 (10 条)
    let history = session.get_history(50);
    assert_eq!(history.len(), 10, "应该只返回未压缩的 10 条消息");
    assert_eq!(history[0].content, "消息 10");
    assert_eq!(history[9].content, "消息 19");

    // 限制 max_messages 应该正确截取尾部
    // 注意: get_history 会在截取后删除开头的非 user 消息
    // index 10:user, 11:assistant, ... 17:assistant, 18:user, 19:assistant
    // get_history(3) → window[7..] = [17:assistant, 18:user, 19:assistant]
    // → 删除开头非user → [18:user, 19:assistant] = 2 条
    let short_history = session.get_history(3);
    assert_eq!(short_history.len(), 2);
    assert_eq!(short_history[0].content, "消息 18");
    assert_eq!(short_history[1].content, "消息 19");

    // last_compacted 超出消息总数时不应 panic
    session.last_compacted = 100;
    let empty_history = session.get_history(50);
    assert!(empty_history.is_empty());

    // last_compacted 与 last_consolidated 取 max 作为 floor
    session.last_compacted = 5;
    session.last_consolidated = 12;
    let history = session.get_history(50);
    assert_eq!(history.len(), 8, "floor = max(5,12) = 12, 剩余 8 条");
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 4: build_messages 有 compaction 时注入 boundary + summary
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_build_messages_with_compaction() {
    let builder = ContextBuilder::default();

    // 构造 compact summary
    let summary = make_compact_summary(
        "早期对话摘要：用户询问了天气、新闻和体育比分。助手逐一回应。",
        CompactTrigger::Auto,
        0,
        5,
        5,
        10,
    );

    // 构造历史消息（未压缩的尾部）
    let history: Vec<ChatMessage> = vec![
        make_msg("user", "最近有什么新电影？"),
        make_msg("assistant", "最近上映了《星际穿越2》和《赛博朋克2077》电影版。"),
        make_msg("user", "推荐看哪一部？"),
        make_msg("assistant", "如果你喜欢科幻，推荐《星际穿越2》。"),
    ];

    let messages = builder.build_messages(
        history,
        "你说得对，科幻确实不错。".to_string(),
        Some("telegram"),
        Some("12345"),
        Some(&summary),
    );

    // 验证总消息数
    // system (1) + boundary start (1) + summary (1) + boundary end (1) + 4 history + 1 current user = 9
    assert_eq!(messages.len(), 9, "应有 9 条消息");

    // 验证 boundary markers
    let boundary_start = messages[1].content.to_text_lossy();
    assert!(boundary_start.contains("Context Compaction Boundary"), "应包含 boundary start marker");
    assert!(boundary_start.contains("compacted context start"), "应包含 compacted context start");

    let summary_msg = messages[2].content.to_text_lossy();
    assert!(summary_msg.contains("早期对话摘要"));
    assert!(summary_msg.contains("天气"));

    let boundary_end = messages[3].content.to_text_lossy();
    assert!(boundary_end.contains("compacted context end"), "应包含 compacted context end");

    // 验证历史消息在 boundary 之后
    assert_eq!(messages[4].content.to_text_lossy(), "最近有什么新电影？");
    assert_eq!(messages[5].content.to_text_lossy(), "最近上映了《星际穿越2》和《赛博朋克2077》电影版。");

    // 验证当前消息在最后
    assert_eq!(messages[8].content.to_text_lossy(), "你说得对，科幻确实不错。");

    // 验证 system prompt 存在并包含必要信息
    let system = messages[0].content.to_text_lossy();
    assert!(system.contains("agent"), "应包含 identity header");
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 5: build_messages 无 compaction 时行为不变
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_build_messages_without_compaction() {
    let builder = ContextBuilder::default();

    let history: Vec<ChatMessage> = vec![
        make_msg("user", "你好"),
        make_msg("assistant", "你好！需要什么帮助？"),
    ];

    let messages = builder.build_messages(
        history,
        "帮我查一下天气。".to_string(),
        None,
        None,
        None, // 无 compaction
    );

    // 验证总消息数: system (1) + 2 history + 1 current = 4
    assert_eq!(messages.len(), 4, "无 compaction 时应只有 4 条消息");

    // 验证没有 compaction boundary 注入
    for msg in &messages {
        let content = msg.content.to_text_lossy();
        assert!(
            !content.contains("Context Compaction Boundary"),
            "不应包含 compaction boundary: {}",
            content
        );
        assert!(
            !content.contains("compacted context"),
            "不应包含 compacted context: {}",
            content
        );
    }

    // 验证历史消息顺序正确
    assert_eq!(messages[1].content.to_text_lossy(), "你好");
    assert_eq!(messages[2].content.to_text_lossy(), "你好！需要什么帮助？");
    assert_eq!(messages[3].content.to_text_lossy(), "帮我查一下天气。");
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 6: budget check — 低于阈值不触发 compaction
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_budget_check_below_threshold() {
    let config = BudgetConfig::default();

    // 少量消息 — 远低于 144000 tokens 阈值
    let history: Vec<ChatMessage> = (0..10)
        .map(|i| make_msg("user", &format!("这是一条测试消息 {}", i)))
        .collect();

    let report = context_budget::check_budget(&history, &config);

    assert!(!report.should_compact, "少量消息不应触发 compaction");
    assert!(report.pressure_ratio < 0.80, "压力比应低于 0.80");
    assert!(report.history_estimated < 144_000, "估算 tokens 应远低于阈值");
    assert!(report.pressure_ratio >= 0.0, "压力比不应为负");

    // 验证 system_estimated 和 total_estimated 的一致性
    assert_eq!(
        report.total_estimated,
        report.history_estimated.saturating_add(report.system_estimated),
        "total = history + system"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 7: budget check — 高于阈值触发 compaction
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_budget_check_above_threshold() {
    // 用小 budget 加速测试 — 使用与单元测试相近的策略
    let config = BudgetConfig {
        max_tokens: 10_000,
        system_budget_ratio: 0.0,
        compact_threshold_ratio: 0.80,
        keep_recent_count: 10,
    };

    // 每条消息约 450 chars → ceil(450/3) = 150 tokens
    // 需要 > 10_000 * 0.80 = 8000 tokens threshold
    // 需要 8000/150 = ~54 条消息
    let content = "数据".repeat(225); // 450 chars
    let history: Vec<ChatMessage> = (0..60)
        .map(|i| {
            let role = if i % 2 == 0 { "user" } else { "assistant" };
            make_msg(role, &format!("[{}] {}", i, content))
        })
        .collect();

    let report = context_budget::check_budget(&history, &config);

    assert!(report.should_compact, "高于阈值时应触发 compaction");
    assert!(report.pressure_ratio > 0.80, "压力比应高于 0.80");
    assert!(report.history_estimated > 8000, "估算 tokens 应高于 compact threshold");

    // 验证空历史不触发（即使 budget 很小）
    let empty_report = context_budget::check_budget(&[], &config);
    assert!(!empty_report.should_compact, "空历史不应触发 compaction");
    assert_eq!(empty_report.history_estimated, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 8: token 估算精度验证
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_token_estimation() {
    // ── 基本公式验证 ──
    // 0 chars → 0 tokens
    assert_eq!(estimate_tokens(""), 0);

    // 3 chars → ceil(3/3) = 1
    assert_eq!(estimate_tokens("abc"), 1);

    // 4 chars → ceil(4/3) = 2
    assert_eq!(estimate_tokens("abcd"), 2);

    // 6 chars → ceil(6/3) = 2
    assert_eq!(estimate_tokens("abcdef"), 2);

    // ── 英文文本估算 ──
    let english = "The quick brown fox jumps over the lazy dog"; // 43 chars
    let tokens = estimate_tokens(english);
    assert_eq!(tokens, 15); // ceil(43/3) = 15
    // 实际 GPT/DeepSeek tokenizer 通常 ~9-10 tokens
    // 我们的启发式允许一定误差，但应在一个数量级内
    assert!(tokens >= 1);
    assert!(tokens <= english.len()); // 每个 token 至少 1 个字符

    // ── 中文文本估算 ──
    let chinese = "你好世界这是一个测试"; // 10 chars
    let cn_tokens = estimate_tokens(chinese);
    assert_eq!(cn_tokens, 4); // ceil(10/3) = 4
    assert!(cn_tokens >= 1);

    // ── 长文本单调性 ──
    let short = estimate_tokens("hello");
    let long = estimate_tokens("hello world this is a longer message");
    assert!(long > short, "长文本 token 数应大于短文本");

    // ── estimate_total_tokens 求和正确 ──
    let msgs: Vec<ChatMessage> = vec![
        make_msg("user", "hello"),  // 5 chars → ceil(5/3) = 2
        make_msg("assistant", "hi there"), // 8 chars → ceil(8/3) = 3
        make_msg("user", "how are you?"), // 12 chars → ceil(12/3) = 4
    ];
    let total = estimate_total_tokens(&msgs);
    assert_eq!(total, 2 + 3 + 4); // = 9

    // 空列表返回 0
    assert_eq!(estimate_total_tokens(&[]), 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Test 9: context overflow 检测
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn test_is_context_overflow_detection() {
    // 构造一个极度受限的 budget 来测试 overflow 检测
    let config = BudgetConfig {
        max_tokens: 1_000,
        system_budget_ratio: 0.2,    // 200 tokens for system
        compact_threshold_ratio: 0.80,
        keep_recent_count: 5,
    };

    // history_budget = 1000 - 200 = 800
    // compact_threshold = 800 * 0.80 = 640 tokens
    // overflow = history_estimated > 800

    // ── 场景 1: 正常范围内（不 overflow） ──
    let normal_history: Vec<ChatMessage> = (0..3)
        .map(|i| make_msg("user", &format!("short msg {}", i)))
        .collect();
    let normal_report = context_budget::check_budget(&normal_history, &config);
    assert!(!normal_report.should_compact);
    assert!(normal_report.pressure_ratio < 1.0, "应在 budget 内");

    // ── 场景 2: 超过 budget（overflow） ──
    // 每条消息约 600 chars → ceil(600/3) = 200 tokens
    // 5 条消息 → 1000 tokens > history_budget (800)
    let overflow_content = "x".repeat(600);
    let overflow_history: Vec<ChatMessage> = (0..5)
        .map(|i| make_msg("user", &format!("[{}]{}", i, overflow_content)))
        .collect();
    let overflow_report = context_budget::check_budget(&overflow_history, &config);
    assert!(overflow_report.should_compact, "超 budget 应触发 compaction");
    assert!(
        overflow_report.pressure_ratio > 1.0,
        "压力比应 > 1.0（overflow），实际: {}",
        overflow_report.pressure_ratio
    );
    assert!(
        overflow_report.history_estimated > 800,
        "history estimated 应超过 history_budget (800)，实际: {}",
        overflow_report.history_estimated
    );

    // ── 场景 3: 远超 budget（严重 overflow） ──
    let severe_content = "x".repeat(600);
    let severe_history: Vec<ChatMessage> = (0..20)
        .map(|i| make_msg("user", &format!("[{}]{}", i, severe_content)))
        .collect();
    let severe_report = context_budget::check_budget(&severe_history, &config);
    assert!(severe_report.should_compact, "严重 overflow 应触发 compaction");
    assert!(
        severe_report.pressure_ratio > 1.0,
        "严重 overflow 压力比应 > 1.0，实际: {}",
        severe_report.pressure_ratio
    );

    // ── 场景 4: 空历史 zero pressure ──
    let empty_report = context_budget::check_budget(&[], &config);
    assert!(!empty_report.should_compact);
    assert_eq!(empty_report.history_estimated, 0);
    assert!(empty_report.pressure_ratio == 0.0);
}
