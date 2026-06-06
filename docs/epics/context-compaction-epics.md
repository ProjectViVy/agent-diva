# Context Compaction — Epic & Story 分解

> 依据 ADR-0010：Context Compaction 上下文压缩架构
> 日期：2026-06-04
> 版本：1.0

---

## 概述

本文档将 ADR-0010 的 P0 范围拆解为 7 个 Epic，每个 Epic 包含 3-5 个可独立交付的 Story。
Epic 按照 ADR 第 2.10 节「实施顺序」排列，确保依赖关系正确。

| Epic | 名称 | 对应模块 | Story 数 |
|------|------|---------|----------|
| Epic 1 | Token 估算基础设施 | `token_estimate.rs` | 3 |
| Epic 2 | 上下文预算监控 | `context_budget.rs` | 4 |
| Epic 3 | Session compaction 数据模型 | `store.rs` + `manager.rs` | 4 |
| Epic 4 | 上下文压缩器 | `compaction.rs` + prompt | 5 |
| Epic 5 | 上下文组装改造 | `context.rs` + `loop_turn.rs` | 5 |
| Epic 6 | Reactive compact 安全网 | Agent Loop 错误处理 | 3 |
| Epic 7 | 集成测试 | 全模块回归 | 5 |

---

## Epic 1: Token 估算基础设施

**目标**：实现 `token_estimate.rs` 模块，提供字符→token 估算能力，为后续预算监控和压缩决策提供基础。

**对应 ADR**：第 2.3 节 Token 估算公式，第 4.2 节 token_estimate.rs

**依赖**：无（纯函数，零外部依赖）

**产出**：
- `agent-diva-agent/src/compaction/token_estimate.rs`
- `agent-diva-agent/src/compaction/mod.rs`（模块骨架）

---

### Story 1.1: 实现 estimate_tokens() 纯函数

**作为** agent 核心开发者
**我想要** 一个接受任意文本并返回估算 token 数的函数
**以便** 后续所有模块都能基于统一的 token 估算标准进行预算计算

**验收标准**：

- **Given** 当传入空字符串 `""` 时
  **When** 调用 `estimate_tokens("")`
  **Then** 返回 `0`

- **Given** 当传入英文字符串 `"hello world"` 时
  **When** 调用 `estimate_tokens("hello world")`
  **Then** 返回结果 > 0，且数值等于 `(11 / 4.0 * 4.0 / 3.0).ceil()`（11 chars → ~4 tokens）

- **Given** 当传入中文字符串 `"你好世界"` 时（每个汉字 3 bytes，但 `.chars()` 计为 1）
  **When** 调用 `estimate_tokens("你好世界")`
  **Then** 返回结果 > 0，Unicode-aware 正确处理多字节字符（4 chars → ~2 tokens）

- **Given** 当传入混合内容 `"hello 你好"` 时
  **When** 调用 `estimate_tokens("hello 你好")`
  **Then** 正确处理中英混合（8 chars → ~3 tokens），不产生 panic

- **Given** 当传入 1000 个字符的长文本
  **When** 调用 `estimate_tokens(&"a".repeat(1000))`
  **Then** 返回结果 > 0，不溢出，不 panic

- **Given** 函数签名
  **When** 查看 `estimate_tokens` 的函数签名
  **Then** 为 `pub fn estimate_tokens(text: &str) -> usize`，位于 `agent-diva-agent/src/compaction/token_estimate.rs`

---

### Story 1.2: 实现 estimate_message_tokens() 函数

**作为** agent 核心开发者
**我想要** 一个可对 ChatMessage 全体内容（content + reasoning + tool_calls）估算 token 的函数
**以便** 后续预算监控能准确评估单条消息的 token 消耗

**验收标准**：

- **Given** 一条普通 ChatMessage，`content = "I will help you with the task"`，`reasoning_content = None`，`tool_calls = None`
  **When** 调用 `estimate_message_tokens(&msg)`
  **Then** 返回 `estimate_tokens("I will help you with the task")` 的结果

- **Given** 一条包含 reasoning 的 ChatMessage，`reasoning_content = Some("Let me think about this carefully...")`
  **When** 调用 `estimate_message_tokens(&msg)`
  **Then** 返回 content_tokens + reasoning_tokens 之和

- **Given** 一条包含 tool_calls 的 ChatMessage（如 JSON 序列化的 tool call 数组）
  **When** 调用 `estimate_message_tokens(&msg)`
  **Then** 返回 content_tokens + tool_calls 序列化后的 token 之和（每个 tool call 调用 `tc.to_string()` 后用 `estimate_tokens` 计算）

- **Given** 一条同时包含 content + reasoning + tool_calls 的 ChatMessage
  **When** 调用 `estimate_message_tokens(&msg)`
  **Then** 三者 token 之和等于分别计算后的总和

- **Given** 函数签名
  **When** 查看 `estimate_message_tokens`
  **Then** 为 `pub fn estimate_message_tokens(msg: &agent_diva_core::session::ChatMessage) -> usize`，与 `estimate_tokens` 同文件

---

### Story 1.3: 编写 token_estimate 单元测试

**作为** 质量保证工程师
**我想要** token_estimate 模块有完整的单元测试覆盖
**以便** 确保估算逻辑在边界情况和各种输入类型下行为正确

**验收标准**：

- **Given** 测试模块 `#[cfg(test)] mod tests`
  **When** 运行 `cargo test --package agent-diva-agent -- compaction::token_estimate`
  **Then** 所有测试通过，至少覆盖：空字符串、纯 ASCII、中文多字节、混合内容、长输入、带 reasoning 的消息、带 tool_calls 的消息

- **Given** 测试 `empty_string`
  **When** 运行测试
  **Then** `assert_eq!(estimate_tokens(""), 0)`

- **Given** 测试 `ascii_baseline`
  **When** 运行测试
  **Then** `estimate_tokens("hello world")` 返回正整数

- **Given** 测试 `chinese_multi_byte`
  **When** 运行测试
  **Then** `estimate_tokens("你好世界")` 返回正整数，无 panic

- **Given** 测试 `message_with_reasoning`
  **When** 构造含 reasoning_content 的 ChatMessage 并调用 `estimate_message_tokens`
  **Then** 返回值 ≥ content 单独估算值

---

## Epic 2: 上下文预算监控

**目标**：实现 `context_budget.rs` 模块，提供 BudgetConfig 配置、ContextBudgetMonitor 预算估算和检查能力。

**对应 ADR**：第 2.4 节 Context Budget 阈值，第 4.3 节 context_budget.rs

**依赖**：Epic 1（token_estimate 模块）

**产出**：
- `agent-diva-agent/src/compaction/context_budget.rs`

---

### Story 2.1: 定义 BudgetConfig 结构体及其默认值

**作为** agent 核心开发者
**我想要** 一个可配置的预算参数结构体 BudgetConfig
**以便** 上下文预算的各项阈值可通过配置注入，同时提供合理的默认值

**验收标准**：

- **Given** `BudgetConfig` struct 定义
  **When** 查看结构体字段
  **Then** 包含 `max_tokens: usize`、`system_budget_ratio: f64`、`compact_threshold_ratio: f64`、`keep_recent_count: usize`

- **Given** `BudgetConfig::default()`
  **When** 调用默认构造函数
  **Then** 返回 `BudgetConfig { max_tokens: 180_000, system_budget_ratio: 0.15, compact_threshold_ratio: 0.80, keep_recent_count: 10 }`

- **Given** `BudgetConfig` 实现 `Debug` 和 `Clone`
  **When** 使用 `{:?}` 打印 BudgetConfig 实例
  **Then** 输出包含所有字段名和值

- **Given** `BudgetConfig` 支持字段级覆盖
  **When** 构造 `BudgetConfig { max_tokens: 128_000, ..Default::default() }`
  **Then** `max_tokens` 为 128_000，其余字段保持默认值

---

### Story 2.2: 实现 ContextBudgetMonitor::estimate_total()

**作为** agent 核心开发者
**我想要** 一个能对组装后的 `Vec<Message>` 估算总 token 数的函数
**以便** Agent Loop 在实际发送请求前掌握上下文 token 使用量

**验收标准**：

- **Given** ContextBudgetMonitor 实例和一组 Message（含 system/user/assistant）
  **When** 调用 `monitor.estimate_total(&messages)`
  **Then** 返回所有消息的 content + reasoning_content token 之和

- **Given** Message 的 content 为 `None`（如某些 tool 消息）
  **When** 调用 `estimate_total`
  **Then** 该消息贡献 0 token（`unwrap_or_default()` 返回空字符串，`estimate_tokens("")` = 0）

- **Given** Message 包含 reasoning_content
  **When** 调用 `estimate_total`
  **Then** 该消息的 token 数 = content_tokens + reasoning_tokens

- **Given** 空消息列表
  **When** 调用 `monitor.estimate_total(&[])`
  **Then** 返回 0

- **Given** 函数签名
  **When** 查看 `estimate_total`
  **Then** 为 `pub fn estimate_total(&self, messages: &[Message]) -> usize`

---

### Story 2.3: 实现 ContextBudgetMonitor::check() 及 BudgetReport

**作为** agent 核心开发者
**我想要** 一个完整的预算检查函数，返回 BudgetReport（含 utilization_pct、needs_compaction 等字段）
**以便** Agent Loop 可以据此做出是否需要压缩的决策

**验收标准**：

- **Given** 一组消息（messages[0] 为 system prompt，其余为历史消息）
  **When** 调用 `monitor.check(&messages)`
  **Then** 返回 `BudgetReport` 包含 `total_estimated_tokens`、`history_tokens`、`system_tokens`、`history_budget`、`threshold`、`needs_compaction`、`utilization_pct`

- **Given** 预算分配逻辑
  **When** `max_tokens=180_000`, `system_budget_ratio=0.15`
  **Then** `history_budget = 180_000 * (1.0 - 0.15) = 153_000`，`threshold = 153_000 * 0.80 = 122_400`

- **Given** 历史消息 token 超过 threshold 且总 token 超过 `max_tokens / 2`
  **When** 调用 `check`
  **Then** `needs_compaction = true`

- **Given** 历史消息 token 超过 threshold 但总 token 未超过 `max_tokens / 2`（极小 session）
  **When** 调用 `check`
  **Then** `needs_compaction = false`（安全网：过小的 session 不压缩）

- **Given** 历史消息 token 未超过 threshold
  **When** 调用 `check`
  **Then** `needs_compaction = false`

- **Given** `utilization_pct` 计算公式
  **When** `history_budget > 0`
  **Then** `utilization_pct = history_tokens / history_budget * 100.0`

- **Given** `history_budget = 0`
  **When** 调用 check
  **Then** `utilization_pct = 0.0`（除零保护）

---

### Story 2.4: 编写 context_budget 单元测试

**作为** 质量保证工程师
**我想要** BudgetReport 的阈值计算和 needs_compaction 决策逻辑有单元测试
**以便** 确保预算检查在各种边界情况下决策正确

**验收标准**：

- **Given** 测试模块
  **When** 运行 `cargo test --package agent-diva-agent -- compaction::context_budget`
  **Then** 所有测试通过

- **Given** 测试 `threshold_calculation`
  **When** 使用默认 BudgetConfig 计算
  **Then** history_budget = 153_000, threshold = 122_400

- **Given** 测试 `needs_compaction_true`（构造超过 threshold 的消息）
  **When** 调用 check
  **Then** `budget_report.needs_compaction == true`

- **Given** 测试 `needs_compaction_false`（构造低于 threshold 的消息）
  **When** 调用 check
  **Then** `budget_report.needs_compaction == false`

- **Given** 测试 `utilization_percentage`
  **When** history_tokens = 76_500, history_budget = 153_000
  **Then** `budget_report.utilization_pct` 约等于 50.0

---

## Epic 3: Session compaction 数据模型

**目标**：在 Session 上新增类型安全的 compaction 字段，适配 SessionManager 序列化/反序列化，修改 get_history() 的起始索引逻辑。

**对应 ADR**：第 2.5 节 CompactSummary Schema，第 2.6 节 SessionManager 序列化适配，第 2.9 节 last_compacted 字段，第 3.1-3.2 节

**依赖**：无（纯数据结构，可独立于 Epic 1/2 先行开发）

**产出**：
- `agent-diva-core/src/session/store.rs`（修改）
- `agent-diva-core/src/session/manager.rs`（修改）

---

### Story 3.1: 定义 CompactSummary、CompactTrigger、CompactionRange 类型

**作为** agent 核心开发者
**我想要** 类型安全的 compaction 数据模型
**以便** 编译器可在编译期保证 compaction 数据的正确性，替代 `serde_json::Value` 方案

**验收标准**：

- **Given** `CompactSummary` struct
  **When** 查看字段定义
  **Then** 包含 `schema_version: u32`、`compact_id: String`、`created_at: String`、`trigger: CompactTrigger`、`source_range: CompactionRange`、`kept_recent_count: usize`、`pre_compact_message_count: usize`、`pre_compact_estimated_tokens: usize`、`summary: String`

- **Given** `CompactTrigger` enum
  **When** 查看变体
  **Then** 包含 `Auto`、`Manual`、`Reactive` 三个变体，全部标注 `#[serde(rename_all = "snake_case")]`

- **Given** `CompactionRange` struct
  **When** 查看字段
  **Then** 包含 `start_index: usize`、`end_index: usize`（exclusive）

- **Given** 所有类型实现 `Debug`、`Clone`、`Serialize`、`Deserialize`
  **When** 序列化再反序列化一个 CompactSummary
  **Then** 数据完全一致

- **Given** JSON 序列化输出
  **When** 序列化 `CompactTrigger::Auto`
  **Then** JSON 值为 `"auto"`（snake_case）

---

### Story 3.2: Session 新增 compaction 和 last_compacted 字段

**作为** agent 核心开发者
**我想要** Session struct 包含 `compaction: Option<CompactSummary>` 和 `last_compacted: usize` 字段
**以便** 压缩后的会话可持久化记录压缩状态，且不影响向后兼容

**验收标准**：

- **Given** Session struct 定义（`agent-diva-core/src/session/store.rs`）
  **When** 查看新增字段
  **Then** 包含 `compaction: Option<CompactSummary>`，标注 `#[serde(skip_serializing_if = "Option::is_none", default)]`
  **And** 包含 `last_compacted: usize`，标注 `#[serde(default)]`

- **Given** `Session::new()` 构造函数
  **When** 创建新 Session
  **Then** `compaction` 为 `None`，`last_compacted` 为 `0`

- **Given** 一个没有 `compaction` 和 `last_compacted` 字段的旧 JSONL session 文件
  **When** 反序列化 Session
  **Then** `compaction` 自动填充为 `None`，`last_compacted` 自动填充为 `0`（serde default），无错误

- **Given** 一个有 compaction 的新 Session
  **When** 序列化为 JSON
  **Then** compaction 字段出现在 metadata 行中（当 `is_some()` 时）

---

### Story 3.3: SessionManager save/load 序列化适配

**作为** agent 核心开发者
**我想要** SessionManager 的 `save()` 和 `load()` 方法能正确序列化和反序列化 compaction 字段
**以便** compaction 数据在会话持久化过程中不被丢失

**验收标准**：

- **Given** `SessionManager::save()`（`agent-diva-core/src/session/manager.rs`）
  **When** session 包含 `compaction: Some(...)` 和 `last_compacted: 120`
  **Then** JSONL metadata 行包含 `"compaction":{...}` 和 `"last_compacted":120`

- **Given** `SessionManager::load()`
  **When** 从包含 compaction 字段的 JSONL 文件加载
  **Then** 反序列化得到的 Session 包含正确的 CompactSummary 和 last_compacted 值

- **Given** 旧 JSONL 文件（无 compaction 字段）
  **When** `SessionManager::load()` 加载该文件
  **Then** 加载成功，`session.compaction` 为 `None`，`session.last_compacted` 为 `0`

- **Given** save/load 往返
  **When** 创建带 compaction 的 Session → save → load
  **Then** 加载的 Session 与原始 Session 的 compaction 数据完全一致

---

### Story 3.4: get_history() 适配 last_compacted 索引

**作为** agent 核心开发者
**我想要** `Session::get_history()` 以 `last_compacted` 作为起始索引获取消息
**以便** 已被压缩的消息不会重复出现在 context 中（它们已被 summary 替代）

**验收标准**：

- **Given** Session 的 `last_compacted = 0`（未压缩）
  **When** 调用 `session.get_history(50)`
  **Then** 从 `messages[0]` 开始取最近 50 条，行为与现有逻辑一致

- **Given** Session 的 `last_compacted = 120`，`messages.len() = 150`
  **When** 调用 `session.get_history(50)`
  **Then** 从 `messages[120]` 开始取消息，最多取 50 条（即 messages[120..150]，共 30 条）

- **Given** Session 的 `last_compacted > 0` 且剩余消息数 < 请求数
  **When** 调用 `session.get_history(50)`
  **Then** 返回所有未被压缩的消息，不 panic

- **Given** `last_compacted` vs `last_consolidated` 语义独立
  **When** `last_compacted = 120`，`last_consolidated = 80`
  **Then** `get_history` 仅使用 `last_compacted`，不关心 `last_consolidated`（两个指针独立演进）

- **Given** 现有回归测试
  **When** 运行现有 session 相关测试
  **Then** 全部通过（未压缩的 session 行为不变）

---

## Epic 4: 上下文压缩器

**目标**：实现 `compaction.rs` 模块，包含 compaction system prompt、ContextCompactor::compact() 核心逻辑、CompactionResult 返回类型。

**对应 ADR**：第 4.4 节 compaction.rs，第 2.5 节 CompactSummary，第 2.8 节 Agent Loop 集成

**依赖**：Epic 1（token_estimate）、Epic 2（context_budget）、Epic 3（Session 数据模型）

**产出**：
- `agent-diva-agent/src/compaction/compaction.rs`

---

### Story 4.1: 定义 COMPACTION_SYSTEM_PROMPT 常量

**作为** prompt 工程师
**我想要** 一个精心设计的 compaction system prompt
**以便** LLM 生成的摘要能保留所有可操作上下文（项目状态、决策、用户偏好、工具调用、文件路径等）

**验收标准**：

- **Given** `COMPACTION_SYSTEM_PROMPT` 常量
  **When** 查看内容
  **Then** 包含以下指令：
  - "compress the provided conversation into a dense, lossy summary"
  - 保留：project state, active tasks, decisions made
  - 保留：user preferences, identity, constraints
  - 保留：tool calls: what was done and why
  - 保留：file paths edited, commands executed, results
  - 保留：open questions, blockers, next steps
  - 输出格式：仅 summary，无 preamble/meta-commentary
  - 人称：第三人称过去时
  - 长度限制：Max 2000 characters

- **Given** Prompt 是 `&str` 常量
  **When** 编译 agent-diva-agent crate
  **Then** COMPACTION_SYSTEM_PROMPT 编译时嵌入，无运行时加载开销

---

### Story 4.2: 实现 ContextCompactor::compact() 核心逻辑

**作为** agent 核心开发者
**我想要** `ContextCompactor::compact()` 方法能：确定压缩范围 → 构建对话文本 → 调用 LLM → 生成 CompactSummary → 返回 CompactionResult
**以便** Agent Loop 可调用此方法执行一次完整的压缩

**验收标准**：

- **Given** Session 有 150 条消息，`last_compacted = 0`，`keep_recent_count = 10`
  **When** 调用 `ContextCompactor::compact(session, provider, model, config)`
  **Then** 压缩范围为 `messages[0..140]`（end = 150 - 10），保留最近 10 条

- **Given** Session 已压缩过一次（`last_compacted = 80`），当前共 200 条消息
  **When** 调用 `compact`
  **Then** 压缩范围为 `messages[80..190]`（start = last_compacted, end = 200 - 10）

- **Given** 压缩范围为空（`end <= start`）
  **When** 调用 `compact`
  **Then** 返回 skip_result，不调用 LLM，`new_compacted_index = start`（保持原值）

- **Given** LLM 返回 summary 文本
  **When** 构造 CompactSummary
  **Then** `schema_version = 1`，`trigger = CompactTrigger::Auto`，`compact_id` 格式为 `compact-YYYYMMDD-{hash}`，`created_at` 为 ISO8601/RFC3339 格式，`pre_compact_estimated_tokens` 由 `estimate_message_tokens` 累加计算

- **Given** compact_id 生成
  **When** 同一秒内两次压缩不同内容
  **Then** compact_id 的 hash 部分不同（时间戳前缀避免跨天碰撞）

- **Given** LLM 调用参数
  **When** compact 调用 `provider.chat()`
  **Then** `messages` 为 `[system(COMPACTION_SYSTEM_PROMPT), user(conversation_text)]`，无 tools，`max_tokens = 1024`，`temperature = 0.3`

---

### Story 4.3: 实现消息截断与对话文本构建

**作为** agent 核心开发者
**我想要** 在构建 compaction 的 user message 时对每条消息截断至 600 chars
**以便** 避免单条极长消息（如大文件内容、长 tool 输出）导致 compaction prompt 自身超过 LLM context

**验收标准**：

- **Given** `truncate(s, max_chars)` 辅助函数
  **When** 传入 `"hello world"`（11 chars < 600）
  **Then** 返回完整字符串 `"hello world"`

- **Given** `truncate(s, 600)`
  **When** 传入一个 1000 chars 的字符串
  **Then** 返回前 600 个 Unicode 字符 + `"..."`（共 603 chars）

- **Given** 对话文本构建
  **When** 遍历 `old` 消息列表
  **Then** 每条消息格式化为 `"[{role}]: {truncated_content}"`（只取 content 前 600 chars），用 `\n` 连接

- **Given** content 为空的消息
  **When** 构建对话文本
  **Then** 格式化为 `"[{role}]: "`（无 panic，`truncate("", 600)` 返回 `""`）

---

### Story 4.4: 实现 CompactionResult 和 skip 逻辑

**作为** agent 核心开发者
**我想要** `CompactionResult` 结构体包含 `summary: CompactSummary` 和 `new_compacted_index: usize`
**以便** 调用方可一次性获取压缩结果并更新 Session 指针

**验收标准**：

- **Given** `CompactionResult` struct
  **When** 查看字段
  **Then** 包含 `summary: CompactSummary` 和 `new_compacted_index: usize`

- **Given** 正常压缩完成
  **When** 返回 CompactionResult
  **Then** `new_compacted_index == end`（压缩范围的 end），`summary` 为完整 CompactSummary

- **Given** `skip_result(idx)` 辅助函数
  **When** 无需压缩（end <= start）
  **Then** 返回 CompactionResult，其中 `new_compacted_index == idx`（保持不变），`summary` 为默认值

- **Given** compact 函数的错误处理
  **When** LLM 调用失败（网络错误、provider 错误）
  **Then** 返回 `Err(...)`，不更新 Session 状态

---

### Story 4.5: 模块根文件 mod.rs 组装

**作为** agent 核心开发者
**我想要** `compaction/mod.rs` 正确声明三个子模块并 re-export 公共 API
**以便** 外部调用方可通过 `use agent_diva_agent::compaction::*` 访问所有 compaction 类型

**验收标准**：

- **Given** `compaction/mod.rs` 文件
  **When** 查看模块声明
  **Then** 包含 `pub mod token_estimate;`、`pub mod context_budget;`、`pub mod compaction;`

- **Given** public re-exports
  **When** 外部使用 `use agent_diva_agent::compaction::TokenEstimator`
  **Then** 可正确导入（通过 `pub use token_estimate::*` 或等效语句）

- **Given** 所有公共类型可被外部访问
  **When** 编译 `agent-diva-agent`
  **Then** `BudgetConfig`、`ContextBudgetMonitor`、`BudgetReport`、`ContextCompactor`、`CompactionResult` 均可见

---

## Epic 5: 上下文组装改造

**目标**：修改 `context.rs` 的 `build_messages()` 在 system prompt 后注入 compaction summary，修改 `loop_turn.rs` 在 turn 开头执行 budget check + compaction decision，修改 `agent_loop.rs` 初始化 BudgetConfig。

**对应 ADR**：第 2.7 节 Prompt Assembly 集成，第 2.8 节 Agent Loop 集成，第 3.3-3.5 节，第 5.1 节消息流转时序

**依赖**：Epic 1-4 全部完成

**产出**：
- `agent-diva-agent/src/context.rs`（修改）
- `agent-diva-agent/src/agent_loop/loop_turn.rs`（修改）
- `agent-diva-agent/src/agent_loop.rs`（修改）

---

### Story 5.1: ContextBuilder 新增 session 参数和 budget_config

**作为** agent 核心开发者
**我想要** ContextBuilder 接收 `session: &Session` 和 `budget_config: BudgetConfig`
**以便** build_messages 能访问 compaction summary 并用于后续预算决策

**验收标准**：

- **Given** `ContextBuilder` struct（`context.rs`）
  **When** 查看新增字段
  **Then** 包含 `budget_config: BudgetConfig`

- **Given** `build_messages()` 函数签名
  **When** 查看参数
  **Then** 新增 `session: &Session` 参数（除现有 `history` 和 `current_message` 外）

- **Given** 调用方适配
  **When** 所有现有 `build_messages` 调用点
  **Then** 已传入 `&session` 参数，编译通过

---

### Story 5.2: build_messages() 注入 compaction summary

**作为** agent 核心开发者
**我想要** 当 `session.compaction.is_some()` 时，在 system prompt 之后插入一条 `[Previous conversation summary]` 消息
**以便** LLM 可感知已被压缩的历史对话内容

**验收标准**：

- **Given** `session.compaction = None`（未压缩过）
  **When** 调用 `build_messages`
  **Then** 消息列表结构与现在完全一致（system prompt + history + current），无额外消息

- **Given** `session.compaction = Some(compact)` 其中 `compact.summary = "用户正在开发 Rust workspace agent-diva-pro..."`
  **When** 调用 `build_messages`
  **Then** 在 system prompt 之后、history 之前插入：
  ```
  [Previous conversation summary — generated at {compact.created_at}]
  {compact.summary}
  ```

- **Given** summary 注入后
  **When** 消息发送给 LLM
  **Then** LLM 看到的上下文为：system prompt → summary message → un-compacted history → current user message

- **Given** summary message 的 role
  **When** 查看插入的消息
  **Then** role 为 `system`（与 Claude Code 行为一致）

---

### Story 5.3: AgentLoop 初始化 BudgetConfig

**作为** agent 核心开发者
**我想要** `AgentLoop` struct 持有 `budget_config: BudgetConfig`
**以便** loop_turn 在执行时可访问预算配置进行决策

**验收标准**：

- **Given** `AgentLoop` struct（`agent_loop.rs`）
  **When** 查看字段
  **Then** 包含 `budget_config: BudgetConfig`

- **Given** `AgentLoop::new()` 构造函数
  **When** 创建 AgentLoop 实例
  **Then** 初始化 `budget_config` 为 `BudgetConfig::default()`

- **Given** `ToolConfig` 配置（`agent_loop.rs` 或 `config.rs`）
  **When** 查看字段
  **Then** 包含 `context_budget: BudgetConfig`，默认值为 `BudgetConfig::default()`

- **Given** 配置传递
  **When** 从 ToolConfig 构造 AgentLoop
  **Then** `budget_config` 正确传递，支持用户自定义阈值

---

### Story 5.4: loop_turn 集成 budget check + compaction decision

**作为** agent 核心开发者
**我想要** `process_inbound_message_inner` 在 turn 开头执行：get_history → build_messages → budget check → (if needed) compact → rebuild → agent loop
**以便** 在每次用户消息到达时自动检测并执行上下文压缩

**验收标准**：

- **Given** 新用户消息到达，`process_inbound_message_inner` 被调用
  **When** 执行流进入
  **Then** 顺序为：
  1. `session.get_history(50)`（内部由 `last_compacted` 过滤）
  2. `context.build_messages(history, msg, &session)`
  3. `ContextBudgetMonitor::new(&config.budget).check(&messages)`
  4. 若 `needs_compaction == false` → 直接进入 Agent Loop（正常流程）
  5. 若 `needs_compaction == true` → compact → 更新 session → rebuild → Agent Loop

- **Given** 触发 compaction
  **When** `needs_compaction == true`
  **Then** 执行 `ContextCompactor::compact(&session, &provider, &model, &budget)`，完成后：
  - `session.compaction = Some(compact_result.summary)`
  - `session.last_compacted = compact_result.new_compacted_index`
  - 调用 `SessionManager::save(session)` 持久化
  - 重新获取 `session.get_history(50)` + `build_messages`
  - 然后进入 Agent Loop

- **Given** compaction 失败（LLM 调用异常）
  **When** `compact()` 返回 `Err`
  **Then** 记录错误日志，使用原始 messages 继续（降级策略，不阻塞用户对话）

- **Given** compaction 在 turn 开头执行
  **When** 查看代码结构
  **Then** compaction 逻辑在 tool calling loop 之前，不在 tool call 中间触发（避免打断工具调用链）

---

### Story 5.5: ToolConfig 暴露 BudgetConfig 配置项

**作为** 产品配置管理员
**我想要** BudgetConfig 可通过 ToolConfig 配置
**以便** CLI/GUI 层可根据不同 provider 或用户偏好调整预算阈值

**验收标准**：

- **Given** `ToolConfig` struct
  **When** 查看字段
  **Then** 包含 `context_budget: BudgetConfig`

- **Given** `ToolConfig::default()`
  **When** 创建默认配置
  **Then** `context_budget` 为 `BudgetConfig::default()`（即 180K max, 0.15 system ratio, 0.80 threshold, keep 10）

- **Given** 用户自定义配置
  **When** 创建 `ToolConfig { context_budget: BudgetConfig { max_tokens: 128_000, ..Default::default() }, ..Default::default() }`
  **Then** 配置正确传递到 AgentLoop

- **Given** 禁用 compaction 的方式
  **When** 设置 `max_tokens: usize::MAX`
  **Then** threshold 永达不到，compaction 不会触发（无需 feature gate）

---

## Epic 6: Reactive Compact 安全网（P1）

**目标**：当 provider 返回 `context_length_exceeded` 错误时，catch 该错误并回退执行 compaction + 重试，作为 proactive budget check 的安全网。

**对应 ADR**：第 2.2 节 P1 Scope — Reactive compact

**依赖**：Epic 5（Agent Loop 已集成 compaction）

**产出**：
- `agent-diva-agent/src/agent_loop/loop_turn.rs`（修改，错误处理逻辑）

---

### Story 6.1: 识别并捕获 context_length_exceeded 错误

**作为** agent 核心开发者
**我想要** Agent Loop 在 provider 返回上下文溢出错误时能识别该错误码
**以便** 区分上下文溢出和其他类型的 API 错误

**验收标准**：

- **Given** Provider 返回错误（如 DeepSeek API 的 400 错误，body 中含 `context_length_exceeded` 或等效错误码）
  **When** Agent Loop 调用 `provider.chat()` 时
  **Then** 错误被解析并识别为上下文溢出错误（非通用 API 错误）

- **Given** 其他类型的 API 错误（如 401、429、500）
  **When** provider 返回
  **Then** 不被误判为上下文溢出，按原有错误处理流程

- **Given** 错误类型定义
  **When** 查看 provider 层的错误枚举
  **Then** 包含一个可区分的上下文溢出变体，或通过 `error.to_string().contains("context_length_exceeded")` 匹配

---

### Story 6.2: 溢出时触发 Compaction 并重试

**作为** agent 核心开发者
**我想要** 当捕获到上下文溢出错误时，自动执行一次 compaction 并重试原请求
**以便** 即使 proactive budget check 漏检，用户对话也不会因溢出错误而中断

**验收标准**：

- **Given** Provider 返回 `context_length_exceeded` 错误
  **When** Agent Loop 捕获该错误
  **Then** 执行一次 `ContextCompactor::compact()`（强制压缩，不依赖 needs_compaction 检查）

- **Given** 压缩完成后
  **When** `compact()` 成功返回
  **Then** 更新 `session.compaction` 和 `session.last_compacted`，重新 `build_messages`，重试 `provider.chat()`

- **Given** 重试后成功
  **When** provider 返回正常响应
  **Then** 用户无感知（除可能增加一次 LLM 调用时延），对话继续

- **Given** 压缩后仍然溢出（极端情况）
  **When** 第二次重试仍返回 context_length_exceeded
  **Then** 不再重试（避免死循环），返回明确错误给用户："上下文过长，请尝试 /compact 手动压缩或减少历史消息"

- **Given** compaction 本身失败
  **When** compact LLM 调用也返回错误
  **Then** 记录错误日志，将原始溢出错误返回给用户

---

### Story 6.3: Reactive compact 标记 CompactTrigger::Reactive

**作为** agent 核心开发者
**我想要** Reactive 触发的 compaction 使用 `CompactTrigger::Reactive` 标记
**以便** 后续审计时能区分计划内压缩（Auto）和应急压缩（Reactive）

**验收标准**：

- **Given** Reactive compaction 触发
  **When** 调用 `ContextCompactor::compact()`
  **Then** 传入参数或通过某种方式标识触发类型为 `CompactTrigger::Reactive`

- **Given** 生成的 CompactSummary
  **When** 查看 `trigger` 字段
  **Then** 值为 `CompactTrigger::Reactive`（不同于正常 Auto 触发的）

- **Given** JSON 序列化
  **When** 序列化 Reactive 标记的 CompactSummary
  **Then** `"trigger": "reactive"`

---

## Epic 7: 集成测试

**目标**：覆盖 token estimation 边界、budget 阈值计算、compaction 端到端流程、无 compaction 的 session 回归、长对话场景和多次 compaction 场景。

**对应 ADR**：第 7.5 节测试策略

**依赖**：Epic 1-6 全部完成

**产出**：
- 各模块内 `#[cfg(test)]` 单元测试（Epic 1/2 已部分覆盖，此处补齐集成测试）
- `agent-diva-agent/tests/` 新增集成测试文件

---

### Story 7.1: Mock LLM 集成测试 — compaction 端到端

**作为** 质量保证工程师
**我想要** 使用 Mock LLM provider 编写 compaction 端到端测试
**以便** 验证从 budget check 到 summary 生成到 message rebuild 的完整链路

**验收标准**：

- **Given** 一个 Mock LLM provider（可返回预设 summary 文本）
  **When** 构造一个超过 threshold 的 session（如 200 条消息，模拟高 token 消耗）
  **Then** 调用 `process_inbound_message_inner(user_message)` → 触发 budget check → needs_compaction == true → compact（使用 mock） → rebuild messages → agent loop 收到含 summary 的 messages

- **Given** Mock provider 返回预设 summary `"The user is building a Rust project..."`
  **When** compaction 完成
  **Then** `session.compaction.unwrap().summary` 等于预设值

- **Given** 集成测试
  **When** 运行 `cargo test --package agent-diva-agent --test integration_compaction`
  **Then** 测试通过

---

### Story 7.2: 向后兼容回归测试

**作为** 质量保证工程师
**我想要** 确保无 compaction 的 session 行为与引入 compaction 之前完全一致
**以便** 现有功能不被破坏

**验收标准**：

- **Given** 一个 `compaction: None, last_compacted: 0` 的 session
  **When** 用户发送消息
  **Then** `get_history(50)` 从 `messages[0]` 开始取，`build_messages` 不注入 summary，agent loop 正常执行

- **Given** 一个旧 JSONL session 文件（无 compaction 字段）
  **When** `SessionManager::load()` 加载
  **Then** session.compaction == None, session.last_compacted == 0

- **Given** 现有 session manager 测试
  **When** 运行 `cargo test --package agent-diva-core -- session`
  **Then** 全部通过

- **Given** 现有 agent loop 测试
  **When** 运行 `cargo test --package agent-diva-agent -- agent_loop`
  **Then** 全部通过（如有适配则重新通过）

---

### Story 7.3: 长对话场景测试 — 200 轮对话

**作为** 质量保证工程师
**我想要** 构造一个 200 轮的用户-助手对话并验证 compaction 自动触发
**以便** 确保真实长对话场景下 compaction 行为正确

**验收标准**：

- **Given** 构造 Session，手动填充 200 条消息（100 轮对话），消息内容足够长使 estimated tokens 超过 threshold
  **When** 发送新用户消息
  **Then** needs_compaction 检测为 true，compaction 执行，消息重建后 token 数显著降低

- **Given** 压缩后
  **When** 检查重组的 messages
  **Then** 包含 `[Previous conversation summary]` system message，历史消息仅包含 `last_compacted` 之后的未压缩消息

- **Given** 压缩后再次对话
  **When** 继续发送新消息
  **Then** Agent 能基于 summary 理解之前的对话上下文，正确回答问题

---

### Story 7.4: 多次 compaction 测试

**作为** 质量保证工程师
**我想要** 验证 session 可被多次压缩且每次压缩的 source_range 正确递增
**以便** 确保第二次、第三次压缩不会包含已被压缩的消息

**验收标准**：

- **Given** 第一次压缩后 `last_compacted = 120`
  **When** 新消息不断累积使 token 再次超过 threshold
  **Then** 第二次 compact 的 `source_range.start_index = 120`（从上次结束位置开始），`source_range.end_index = new_total - keep_recent_count`

- **Given** 第二次压缩后
  **When** 查看 session
  **Then** `session.compaction` 被更新为最新的 CompactSummary（旧 summary 被覆盖），`last_compacted` 递增

- **Given** 多次压缩
  **When** 查看 summary message
  **Then** build_messages 中只有一条 `[Previous conversation summary]`（最新），不包含历史 summary 堆叠

---

### Story 7.5: 边界情况与错误处理测试

**作为** 质量保证工程师
**我想要** 覆盖 compaction 的边界情况和错误处理路径
**以便** 确保系统在各种异常条件下不崩溃

**验收标准**：

- **Given** Session 只有 5 条消息（远未达 threshold）
  **When** budget check
  **Then** needs_compaction = false，不触发 compact

- **Given** LLM 返回空 summary
  **When** compact 完成
  **Then** `summary.summary` 为空字符串，系统不 panic，后续对话仍正常

- **Given** Session 消息数为 0（全新会话）
  **When** budget check 和 compact
  **Then** needs_compaction = false，compact 在 `end <= start` 时 skip

- **Given** compaction 调用中 LLM 超时
  **When** provider 返回超时错误
  **Then** `compact()` 返回 Err，Agent Loop 降级继续（使用原 messages），不丢失用户消息

- **Given** `last_compacted` > `messages.len()`（数据异常）
  **When** `get_history`
  **Then** 返回空列表，不 panic

---

## 实施顺序与依赖关系图

```
Epic 3 (Session 数据模型)     Epic 1 (Token 估算)
     │                              │
     │                              ▼
     │                        Epic 2 (预算监控)
     │                              │
     └──────────┬───────────────────┘
                ▼
          Epic 4 (上下文压缩器)
                │
                ▼
          Epic 5 (上下文组装改造)
                │
                ▼
          Epic 6 (Reactive 安全网)
                │
                ▼
          Epic 7 (集成测试)
```

- Epic 1 和 Epic 3 可并行开发（互不依赖）
- Epic 2 依赖 Epic 1
- Epic 4 依赖 Epic 1、2、3
- Epic 5 依赖 Epic 1-4
- Epic 6 依赖 Epic 5
- Epic 7 依赖全部

---

## 验收里程碑

| 里程碑 | 验收标准 |
|--------|---------|
| M1: 基础设施就绪 | Epic 1 + 2 完成，`cargo test` 单元测试全绿 |
| M2: 数据模型就绪 | Epic 3 完成，旧 session 文件可正常加载 |
| M3: 压缩能力就绪 | Epic 4 完成，Mock LLM 可生成 CompactSummary |
| M4: 集成完成 | Epic 5 完成，Agent Loop 可自动执行 budget check + compact |
| M5: 安全网就绪 | Epic 6 完成，provider 溢出错误被捕获并自动重试 |
| M6: 质量门 | Epic 7 完成，`just ci` 全绿，手动长对话验证通过 |
