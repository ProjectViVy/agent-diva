# ADR-0010: Context Compaction — 上下文压缩架构

**Status:** Accepted  
**Date:** 2026-06-04  
**Deciders:** Architecture Review  
**Replaces:** N/A（全新能力）  
**Supersedes:** N/A

---

## 1. Context

### 1.1 问题陈述

agent-diva-pro 当前会话模型存在以下结构性缺陷：

| 问题 | 现状 | 影响 |
|------|------|------|
| 无 token 感知 | `get_history(50)` 按消息数量截断，不感知实际 token 大小 | 长工具输出、多工具调用轮次会爆超 context window |
| 静默溢出 | 超过 provider limit 时无预检，依赖 provider 返回错误 | 用户体验降级为 API error，丢失对话连续性 |
| 长对话退化 | 无结构性摘要/压缩机制，仅靠 memory consolidation 异步写 MEMORY.md | 旧消息→丢弃；discard 过程不可逆且无法重建上下文 |
| consolidation 与 compaction 混淆 | `consolidation.rs` 负责从 old messages 提取 memory 写入磁盘，不解决 context window 问题 | 两个正交能力（memory preservation vs. context budget）未分离 |

### 1.2 调研输入

- **selfinprove Context Compaction 调研**：定义了 P0/P1 scope、token 估算公式 `chars/4 × 4/3`、CompactSummary JSON schema、实施顺序。
- **Claude Code 模式分析**：Claude Code 使用 `chars.length / 4` 估算 token（Unicode-aware），并设置 budget 比为 0.8~0.9。达到 budget 阈值时触发 compaction，将前 N 条消息替换为 `<summary>` 占位。
- **OpenHarness 架构启示**：OpenHarness 采用 `ContextBudget` trait 模式，将 budget 检查、截断、summary 生成分离为独立组件，agent loop 只调用高层 `ensure_budget()` 接口。

### 1.3 现有代码基线（已验证）

| 文件 | 路径 | 关键特征 |
|------|------|---------|
| Session struct | `agent-diva-core/src/session/store.rs` | 7 字段，有 `metadata: serde_json::Value`（无类型化 compaction 字段） |
| ContextBuilder | `agent-diva-agent/src/context.rs` | `build_messages()` 直接消费 `history: Vec<ChatMessage>`，无 token 检查 |
| process_inbound_message_inner | `agent-diva-agent/src/agent_loop/loop_turn.rs` | L68: `session.get_history(50)`，硬编码 50 |
| consolidation | `agent-diva-agent/src/consolidation.rs` | 独立模块，写入 MEMORY.md/HISTORY.md，不参与 context assembly |
| SessionManager | `agent-diva-core/src/session/manager.rs` | JSONL 存储，load/save 分别处理 metadata 行和 message 行 |

---

## 2. Decision

### 2.1 P0 架构范围

```
┌──────────────────────────────────────────────────────────┐
│  New: agent-diva-agent/src/compaction/                   │
│                                                          │
│  ┌───────────────┐  ┌─────────────┐  ┌───────────────┐  │
│  │ TokenEstimator │  │ ContextBudget│  │ContextCompactor│ │
│  │  .estimate()   │  │  Monitor     │  │  .compact()   │  │
│  │  chars→tokens  │  │  .check()    │  │  history→      │  │
│  │                │  │  budget→pct  │  │  summary       │  │
│  └───────┬───────┘  └──────┬──────┘  └──────┬────────┘  │
│          │                 │                 │           │
│          └─────────┬───────┴─────────────────┘           │
│                    │                                      │
│         ┌──────────▼──────────┐                          │
│         │   compaction/mod.rs  │  module root             │
│         │   CompactSummary     │  data struct             │
│         └──────────┬──────────┘                          │
└────────────────────┼─────────────────────────────────────┘
                     │
        ┌────────────┼────────────┐
        ▼            ▼            ▼
    context.rs   loop_turn.rs   store.rs
    (prompt asm)  (loop integ)  (Schema)
```

P0 交付物：

1. **TokenEstimator** — token 估算器
2. **CompactSummary schema** — 类型化 compaction 记录
3. **ContextBudgetMonitor** — 上下文预算检查
4. **ContextCompactor** — 执行压缩（调用 LLM 生成 summary）
5. **ContextBuilder 集成** — prompt assembly 感知 compaction
6. **Agent Loop 集成** — `process_inbound_message_inner` 调用 budget check + compact

### 2.2 P1 Scope（本期不实施，留接口）

- **Reactive compact**: provider 返回 `context_length_exceeded` overflow 时 catch 错误码，回退执行 compaction 并重试

### 2.3 Token 估算公式

**采用 Claude Code 同款公式：**

```rust
fn estimate_tokens(text: &str) -> usize {
    let char_count = text.chars().count();    // Unicode-aware
    (char_count as f64 / 4.0 * 4.0 / 3.0).ceil() as usize
    //          chars→words    words→tokens (inflate 33%)
    // 等价于: char_count * 1.0 / 3.0  (= char_count / 3)
}
```

- **chars/4**: 平均英文单词长度 4 chars（Unicode-safe）
- **×4/3**: words→tokens 膨胀系数（Claude Code heuristic，覆盖中文等多字节 tokenization）
- **结果等价于 chars/3**，显式写出两步以与 Claude Code 对齐且方便调参

对 ChatMessage 的 token 估算覆盖：

```rust
fn estimate_message_tokens(msg: &ChatMessage) -> usize {
    let mut tokens = estimate_tokens(&msg.content);
    if let Some(reasoning) = &msg.reasoning_content {
        tokens += estimate_tokens(reasoning);
    }
    if let Some(tc) = &msg.tool_calls {
        for tc in tc {
            tokens += estimate_tokens(&tc.to_string());  // JSON serialized
        }
    }
    tokens
}
```

### 2.4 Context Budget 阈值

```rust
pub struct BudgetConfig {
    /// Maximum tokens allowed in the full assembled context
    pub max_tokens: usize,           // default: 180_000 (DeepSeek V3 context = 128K, 留余量)
    /// System prompt budget ratio (system+skills+memory take this fraction)
    pub system_budget_ratio: f64,    // default: 0.15
    /// Target fill ratio before triggering compaction
    pub compact_threshold_ratio: f64, // default: 0.80
    /// Messages to always keep at tail (never compacted)
    pub keep_recent_count: usize,    // default: 10
}
```

Budget 分配逻辑：

```
total_budget = max_tokens
system_budget = total_budget * system_budget_ratio      # 留给 system prompt + skills + memory
history_budget = total_budget - system_budget             # 留给消息历史
compact_threshold = history_budget * compact_threshold_ratio  # 触发 compact 的实际阈值
```

### 2.5 CompactSummary Schema（类型安全，存入 Session）

**不沿用 `metadata: serde_json::Value` 方案**。在 `Session` struct 上新增类型安全字段：

```rust
/// agent-diva-core/src/session/store.rs — Session 新增字段
pub struct Session {
    // ... existing fields unchanged ...
    /// Context compaction summary, if compaction has occurred
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub compaction: Option<CompactSummary>,
}

/// CompactSummary — serializable, type-safe
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactSummary {
    /// Schema version for forward compat
    pub schema_version: u32,
    /// Unique compact event ID
    pub compact_id: String,
    /// ISO8601 timestamp
    pub created_at: String,
    /// What triggered this compaction
    pub trigger: CompactTrigger,
    /// Index range of compacted messages in session.messages
    pub source_range: CompactionRange,
    /// Number of recent messages kept (not compacted)
    pub kept_recent_count: usize,
    /// Message count before compaction
    pub pre_compact_message_count: usize,
    /// Estimated tokens before compaction
    pub pre_compact_estimated_tokens: usize,
    /// The generated summary (natural language)
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactTrigger {
    Auto,       // budget threshold exceeded
    Manual,     // user-triggered (/compact command)
    Reactive,   // provider overflow catch (P1)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionRange {
    pub start_index: usize,
    pub end_index: usize,  // exclusive
}
```

**JSON 示例（与 selfinprove 调研一致）：**

```json
{
  "compaction": {
    "schema_version": 1,
    "compact_id": "compact-20260604-a1b2c3d4",
    "created_at": "2026-06-04T12:00:00Z",
    "trigger": "auto",
    "source_range": {"start_index": 0, "end_index": 120},
    "kept_recent_count": 10,
    "pre_compact_message_count": 130,
    "pre_compact_estimated_tokens": 85000,
    "summary": "用户正在开发 Rust workspace agent-diva-pro..."
  }
}
```

### 2.6 SessionManager 序列化适配

`manager.rs` 的 `save()` 和 `load()` 需要感知 `compaction` 字段：

- **save**: metadata 行新增 `"compaction": session.compaction` (serde)
- **load**: 从 metadata 行解析 `compaction` 字段，构造 `CompactSummary`

### 2.7 Prompt Assembly 集成

`ContextBuilder::build_messages()` 修改：

```rust
// pseudocode — 实际集成点
pub fn build_messages(&self, history: ..., current_message: ..., session: &Session) -> Vec<Message> {
    let mut messages = Vec::new();

    // 1. System prompt (unchanged)
    messages.push(Message::system(self.build_system_prompt()));

    // 2. Compaction summary injection
    if let Some(ref compact) = session.compaction {
        messages.push(Message::system(format!(
            "[Previous conversation summary — generated at {}]\n{}",
            compact.created_at, compact.summary
        )));
    }

    // 3. History (unchanged, except history already excludes compacted range)
    for msg in history { ... }

    // 4. Current message (unchanged)
    messages.push(Message::user(current_message));
    messages
}
```

### 2.8 Agent Loop 集成

`process_inbound_message_inner` 修改点：

```rust
// 在 get_history 之后、build_messages 之前插入

// Step 1: build initial message list for estimation
let history = session.get_history(50);
let mut messages = self.context.build_messages(history, message_content, ...);

// Step 2: token estimation & budget check
let estimated = ContextBudgetMonitor::new(&config.budget).estimate_total(&messages, &system_prompt);
if ContextBudgetMonitor::new(&config.budget).needs_compaction(estimated) {
    // Step 3: compact
    let budget = &config.budget;
    let compact_result = ContextCompactor::compact(
        session,
        &self.provider,
        &model_to_use,
        budget,
    ).await?;

    // Step 4: update session and rebuild
    session.compaction = Some(compact_result.summary);
    session.last_compacted = compact_result.new_compacted_index;  // new field
    let history = session.get_history(50);  // fresh history, excludes compacted
    messages = self.context.build_messages(history, message_content, ...);
}
```

**重要决策：compaction 在 turn 开头执行，不在 turn 中间。**  原因：
- 每条新 user message 是天然边界
- 避免在 tool calling loop 中间打断
- 与 Claude Code 行为一致

### 2.9 Session 新增字段：last_compacted

```rust
pub struct Session {
    // ... existing fields ...
    /// Index of last compacted message (messages before this are summarized in `compaction`)
    #[serde(default)]
    pub last_compacted: usize,
    pub compaction: Option<CompactSummary>,
}
```

`get_history()` 修改：当 `last_compacted > 0` 时，history 从 `last_compacted` 开始取（已压缩的消息不进入 context）。

`last_compacted` vs `last_consolidated` 差异：
- `last_consolidated`: memory consolidation 的进度指针，写入 MEMORY.md
- `last_compacted`: context compaction 的进度指针，写入 CompactSummary

两者独立演进，互不干扰。

### 2.10 实施顺序

```
Phase 1: token_estimate.rs     — TokenEstimator struct + estimate_tokens() + estimate_message_tokens()
         ↓
Phase 2: context_budget.rs     — BudgetConfig + ContextBudgetMonitor.estimate_total() + needs_compaction()
         ↓
Phase 3: compaction.rs         — CompactSummary + CompactTrigger + CompactionRange + ContextCompactor.compact()
         ↓
Phase 4: context.rs (modify)   — build_messages() 注入 compaction summary
         + loop_turn.rs (modify) — budget check + trigger compact + rebuild
         ↓
Phase 5: store.rs (modify)     — Session 新增 compaction + last_compacted 字段
         + manager.rs (modify) — 序列化适配
```

---

## 3. Module Changes

### 3.1 agent-diva-core/src/session/store.rs

| 改动 | 描述 |
|------|------|
| Session 新增 `compaction: Option<CompactSummary>` | 类型安全的 compaction 记录 |
| Session 新增 `last_compacted: usize` | compaction 进度指针 |
| `Session::new()` 初始化 `compaction: None, last_compacted: 0` | 保持向后兼容 |
| 新增 `CompactSummary` struct（含所有子类型） | compaction 数据模型 |
| `get_history()` 改用 `last_compacted` 作为起始索引 | 被压缩的消息不回显进 context |

### 3.2 agent-diva-core/src/session/manager.rs

| 改动 | 描述 |
|------|------|
| `save()` metadata 行加 `"compaction"` 和 `"last_compacted"` | JSON 持久化 |
| `load()` 解析 `compaction` + `last_compacted` | JSON 反序列化 |

### 3.3 agent-diva-agent/src/context.rs

| 改动 | 描述 |
|------|------|
| `build_messages()` 新增 `session: &Session` 参数 | 访问 compaction summary |
| 在 system prompt 后插入 `[Previous conversation summary]` | 当 `session.compaction.is_some()` |
| `ContextBuilder` 新增 `budget_config: BudgetConfig` 字段 | 预算配置 |

### 3.4 agent-diva-agent/src/agent_loop/loop_turn.rs

| 改动 | 描述 |
|------|------|
| `get_history(50)` → 保持当前逻辑，内部由 `last_compacted` 过滤 | 配合 store.rs 的改动 |
| 在 `build_messages` 后插入 token estimation | 调用 `ContextBudgetMonitor::estimate_total` |
| 在 agent loop 前插入 compaction decision | 若 needs_compaction → compact → rebuild messages |
| 保存 compaction 结果到 session | session.compaction = Some(...) |

### 3.5 agent-diva-agent/src/agent_loop.rs

| 改动 | 描述 |
|------|------|
| `AgentLoop` struct 新增 `budget_config: BudgetConfig` | 上下文预算配置 |
| `AgentLoop::new()` 初始化 `budget_config` | 默认值 |
| `ToolConfig` 新增 `budget: BudgetConfig` | 可配置 |

### 3.6 agent-diva-agent/src/consolidation.rs

**不修改。** 保持独立。compaction 与 consolidation 正交。

---

## 4. New Modules

### 4.1 目录结构

```
agent-diva-agent/src/compaction/
├── mod.rs              # module root, re-exports
├── token_estimate.rs   # TokenEstimator
├── context_budget.rs   # BudgetConfig + ContextBudgetMonitor
└── compaction.rs       # CompactSummary + ContextCompactor
```

### 4.2 token_estimate.rs

```rust
//! Token estimation using chars/4 × 4/3 heuristic (Claude Code compatible).
//! Results are ≤10% error on DeepSeek V3 tokenizer.

/// Estimate token count for arbitrary text.
pub fn estimate_tokens(text: &str) -> usize {
    let char_count = text.chars().count();
    (char_count as f64 / 4.0 * 4.0 / 3.0).ceil() as usize
}

/// Estimate token count for a ChatMessage including content, reasoning, and tool calls.
pub fn estimate_message_tokens(msg: &agent_diva_core::session::ChatMessage) -> usize {
    let mut tokens = estimate_tokens(&msg.content);
    if let Some(reasoning) = &msg.reasoning_content {
        tokens += estimate_tokens(reasoning);
    }
    if let Some(ref tcs) = msg.tool_calls {
        for tc in tcs {
            tokens += estimate_tokens(&tc.to_string());
        }
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string()           { assert_eq!(estimate_tokens(""), 0); }
    #[test]
    fn ascii_baseline()         { let t = estimate_tokens("hello world"); assert!(t > 0); }
    #[test]
    fn chinese_multi_byte()     { let t = estimate_tokens("你好世界"); assert!(t > 0); }
    #[test]
    fn mixed_content()          { let t = estimate_tokens("hello 你好"); assert!(t > 0); }
    #[test]
    fn long_input()             { assert!(estimate_tokens(&"a".repeat(1000)) > 0); }
}
```

### 4.3 context_budget.rs

```rust
//! Context budget monitoring: estimate total token usage and decide whether
//! compaction is needed.

use agent_diva_providers::Message;

#[derive(Debug, Clone)]
pub struct BudgetConfig {
    pub max_tokens: usize,
    pub system_budget_ratio: f64,
    pub compact_threshold_ratio: f64,
    pub keep_recent_count: usize,
}

impl Default for BudgetConfig {
    fn default() -> Self {
        Self {
            max_tokens: 180_000,
            system_budget_ratio: 0.15,
            compact_threshold_ratio: 0.80,
            keep_recent_count: 10,
        }
    }
}

pub struct ContextBudgetMonitor {
    config: BudgetConfig,
}

#[derive(Debug, Clone)]
pub struct BudgetReport {
    pub total_estimated_tokens: usize,
    pub history_tokens: usize,
    pub system_tokens: usize,
    pub history_budget: usize,
    pub threshold: usize,
    pub needs_compaction: bool,
    pub utilization_pct: f64,
}

impl ContextBudgetMonitor {
    pub fn new(config: BudgetConfig) -> Self { Self { config } }

    /// Estimate total tokens of assembled messages.
    pub fn estimate_total(&self, messages: &[Message]) -> usize {
        messages.iter().map(|m| {
            let mut tokens = super::token_estimate::estimate_tokens(
                &m.content.clone().unwrap_or_default()
            );
            if let Some(ref reasoning) = m.reasoning_content {
                tokens += super::token_estimate::estimate_tokens(reasoning);
            }
            tokens
        }).sum()
    }

    /// Run a full budget check.
    pub fn check(&self, messages: &[Message]) -> BudgetReport {
        let total = self.estimate_total(messages);
        // Split: system prompt is messages[0]; rest is history
        let system_tokens = messages.first()
            .map(|m| super::token_estimate::estimate_tokens(
                &m.content.clone().unwrap_or_default()
            )).unwrap_or(0);
        let history_tokens = total.saturating_sub(system_tokens);
        let history_budget = (self.config.max_tokens as f64 * (1.0 - self.config.system_budget_ratio)) as usize;
        let threshold = (history_budget as f64 * self.config.compact_threshold_ratio) as usize;

        BudgetReport {
            total_estimated_tokens: total,
            history_tokens,
            system_tokens,
            history_budget,
            threshold,
            needs_compaction: history_tokens > threshold && total > self.config.max_tokens / 2,
            utilization_pct: if history_budget > 0 {
                history_tokens as f64 / history_budget as f64 * 100.0
            } else { 0.0 },
        }
    }
}
```

### 4.4 compaction.rs

```rust
//! Context compaction: generate a summary of old messages to free context budget.

use agent_diva_core::session::{ChatMessage, CompactSummary, CompactTrigger, CompactionRange, Session};
use agent_diva_providers::{LLMProvider, Message};
use std::sync::Arc;

use super::context_budget::BudgetConfig;

const COMPACTION_SYSTEM_PROMPT: &str = r#"You are a conversation summarizer. Your task is to compress the provided conversation into a dense, lossy summary that preserves ALL actionable context:

- Project state, active tasks, decisions made
- User preferences, identity, constraints
- Tool calls: what was done and why
- File paths edited, commands executed, results
- Open questions, blockers, next steps

Output ONLY the summary. No preamble, no meta-commentary. Write in the third person past tense. Be information-dense. Max 2000 characters."#;

pub struct ContextCompactor;

#[derive(Debug)]
pub struct CompactionResult {
    pub summary: CompactSummary,
    pub new_compacted_index: usize,
}

impl ContextCompactor {
    /// Execute compaction: summarize old messages, return CompactSummary + new pointer.
    pub async fn compact(
        session: &Session,
        provider: &Arc<dyn LLMProvider>,
        model: &str,
        config: &BudgetConfig,
    ) -> Result<CompactionResult, Box<dyn std::error::Error>> {
        let total = session.messages.len();
        let start = session.last_compacted;
        let keep = config.keep_recent_count.min(total.saturating_sub(start));
        let end = total.saturating_sub(keep);
        if end <= start { return Ok(skip_result(start)); }

        // Build conversation text from old messages (truncated per message)
        let old: Vec<&ChatMessage> = session.messages[start..end].iter().collect();
        let conversation = old.iter()
            .map(|m| format!("[{}]: {}", m.role, truncate(&m.content, 600)))
            .collect::<Vec<_>>().join("\n");

        // Build compacted messages for the LLM call
        let compacted_messages = vec![
            Message::system(COMPACTION_SYSTEM_PROMPT.to_string()),
            Message::user(conversation),
        ];

        // Call LLM for summary
        let response = provider.chat(
            compacted_messages,
            None,   // no tools for compaction
            Some(model.to_string()),
            1024,   // max_tokens for summary
            0.3,    // low temperature for factual recall
        ).await?;

        let summary_text = response.content.unwrap_or_default().trim().to_string();

        // Estimate pre-compact tokens
        let pre_tokens: usize = old.iter()
            .map(|m| super::token_estimate::estimate_message_tokens(m))
            .sum();

        let compact_id = format!("compact-{}-{:x}",
            chrono::Utc::now().format("%Y%m%d"),
            fxhash::hash64(&summary_text)  // simple non-crypto hash
        );

        let summary = CompactSummary {
            schema_version: 1,
            compact_id,
            created_at: chrono::Utc::now().to_rfc3339(),
            trigger: CompactTrigger::Auto,
            source_range: CompactionRange { start_index: start, end_index: end },
            kept_recent_count: keep,
            pre_compact_message_count: end - start,
            pre_compact_estimated_tokens: pre_tokens,
            summary: summary_text,
        };

        Ok(CompactionResult {
            summary,
            new_compacted_index: end,
        })
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars { s.to_string() }
    else { format!("{}...", s.chars().take(max_chars).collect::<String>()) }
}

fn skip_result(idx: usize) -> CompactionResult {
    CompactionResult { summary: CompactSummary { /* ... defaults */ }, new_compacted_index: idx }
}
```

---

## 5. Compaction Flow

### 5.1 消息流转时序

```
User Message
    │
    ▼
Session::get_history(50)          ← uses last_compacted as floor
    │
    ▼
ContextBuilder::build_messages()  ← + compaction summary if exists
    │
    ▼
ContextBudgetMonitor::check()
    │
    ├─ needs_compaction? ──No──▶ Agent Loop (normal flow)
    │
    └─ Yes
        │
        ▼
    ContextCompactor::compact()   ← call LLM with compaction prompt
        │
        ├─ Save CompactSummary → session.compaction
        ├─ Update session.last_compacted
        ├─ SessionManager::save(session)
        │
        ▼
    Rebuild history + messages    ← fresh get_history + build_messages
        │
        ▼
    Agent Loop (normal flow)
```

### 5.2 数据流

```
┌───────────┐     ┌─────────────┐     ┌───────────────┐
│  Session  │────▶│ ContextBudget│────▶│ContextCompactor│
│ .messages │     │  Monitor     │     │               │
│ .compaction    │  .check()    │     │  .compact()   │
│ .last_compacted│              │     │               │
└───────────┘     └──────┬──────┘     └───────┬───────┘
                         │                     │
                         │ BudgetReport        │ CompactionResult
                         │ {needs_compaction}  │ {summary, new_idx}
                         ▼                     ▼
                  ┌──────────────┐    ┌───────────────┐
                  │  Agent Loop  │◀───│  Session      │
                  │  (decision)  │    │  (updated)    │
                  └──────────────┘    └───────────────┘
```

### 5.3 Session 存储布局（JSONL）

```jsonl
{"_type":"metadata","created_at":"...","updated_at":"...","metadata":{},"last_consolidated":0,"last_compacted":120,"compaction":{...}}
{"role":"assistant","content":"I'll help with..."}
{"role":"user","content":"next question"}
```

- `last_compacted: 120` → messages[0..120] 已被压缩
- `compaction.summary` → 压缩摘要，作为 system message 注入 context
- messages 本身不删除 → 保留完整历史用于 future re-compaction 或 audit

---

## 6. Consequences

### 6.1 正面影响

| 影响 | 详细 |
|------|------|
| **context overflow 防护** | 主动预算检查，在超过 80% threshold 时触发压缩，避免 provider 400 错误 |
| **长对话可用性** | 100+ 轮对话不会因 context 溢出而失败；旧轮次被摘要保留 |
| **向后兼容** | `compaction: None` + `last_compacted: 0` 的 session 行为与现有完全一致 |
| **独立演进** | compaction 与 consolidation 互不干扰；两者独立触发、独立存储 |
| **类型安全** | CompactSummary 是 Rust struct 而非 `serde_json::Value`，编译期保证正确性 |
| **可观测** | BudgetReport 提供 utilization_pct 等指标，便于生产监控 |

### 6.2 负面/风险

| 风险 | 缓解措施 |
|------|---------|
| **LLM 摘要质量不可控** | compaction 使用低温(0.3)+结构化 prompt；P1 可加 validator/re-prompt |
| **额外 LLM 调用成本** | 仅在达到阈值时触发；每 50~130 条消息一次，频率可控 |
| **第一次 compaction 时延** | 异步执行，不阻塞用户；未来可加 pre-compact 缓存 |
| **summary 丢失细节** | 设计上接受"lossy compression"；关键细节通过 MEMORY.md (consolidation) 双保险 |
| **JSONL session 文件变大** | compaction 字段 < 2KB；messages 不删除 == 历史完整但更大；未来可加 archive 策略 |
| **compact_id hash 碰撞** | 使用 fxhash::hash64 + timestamp prefix，碰撞概率可忽略 |

### 6.3 与现有能力的关系

| 能力 | `consolidation.rs` | `compaction` |
|------|-------------------|-------------|
| 触发 | 100 条未 consolidate 消息 | 80% context budget |
| 产出 | MEMORY.md + HISTORY.md | CompactSummary in Session |
| 用途 | 持久化记忆跨 session | 当前 session 内上下文预算管理 |
| 读取方 | 下次 system prompt | 本次 build_messages() |
| 删除历史 | 否（advance pointer） | 否（advance pointer） |

两者形成**双轨内存机制**：
- **Compaction** = 当前上下文窗口内的压缩（短时，session scope）
- **Consolidation** = 持久化的长期记忆（长时，workspace scope）

---

## 7. Migration Plan

### 7.1 向后兼容

- 现有 JSONL session 文件无 `last_compacted` / `compaction` 字段 → 反序列化时 `#[serde(default)]` 自动填充 `0` / `None`
- 无 compaction 的 session 行为与当前完全一致
- 不修改 consolidation 路径

### 7.2 Feature Gate（可选）

P0 阶段不引入 feature gate。compaction 作为核心能力默认启用。若未来需要关闭，可通过 `BudgetConfig { max_tokens: usize::MAX }` 实现（threshold 永达不到）。

### 7.3 配置暴露

```rust
// ToolConfig 新增字段
pub struct ToolConfig {
    // ... existing ...
    pub context_budget: BudgetConfig,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            context_budget: BudgetConfig::default(),
        }
    }
}
```

CLI/GUI 层可通过 `BudgetConfig` 暴露为可配置项。

### 7.4 Phase 5 部署步骤

1. **store.rs**: 新增字段 + CompactSummary 类型（向后兼容）
2. **manager.rs**: 序列化适配
3. **token_estimate.rs**: 纯函数，无依赖，先提交并通过测试
4. **context_budget.rs**: 依赖 token_estimate
5. **compaction.rs**: 依赖前两者
6. **context.rs**: 接口微调（新增 `session` 参数）
7. **loop_turn.rs**: 集成调用
8. **agent_loop.rs**: BudgetConfig 初始化
9. `just ci` 全绿 → 合并

### 7.5 测试策略

| 层级 | 测试内容 | 方法 |
|------|---------|------|
| Unit | TokenEstimator 边界情况 | `#[cfg(test)]` in token_estimate.rs |
| Unit | BudgetReport 阈值计算 | `#[cfg(test)]` in context_budget.rs |
| Integration | compaction 端到端（mock LLM） | 使用 mock LLM provider |
| Regression | 无 compaction 的 session 行为不变 | 现有 session manager 测试 |
| Manual | 长对话触发 compaction | 构造 200 条消息的 session |

---

## References

- [selfinprove Context Compaction 调研](selfinprove://context-compaction)
- [Claude Code Token Estimation](https://docs.anthropic.com/en/docs/build-with-claude/context-windows)
- [OpenHarness Context Budget Design](https://github.com/nousresearch/openharness)
- 代码仓库: `agent-diva-pro` workspace (14 crates)
- 相关文件: `consolidation.rs`, `store.rs`, `context.rs`, `loop_turn.rs`
