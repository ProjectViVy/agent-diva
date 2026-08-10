# 上下文管理增强：C0 基线测量与架构决策

- **状态**：C0 调研完成；ADR-CTX-0..5 与 §6.6 施工补充决策冻结；C1-0 已完成
- **日期**：2026-08-10
- **范围**：Prompt Cache 稳定前缀、分层预算、工具结果引用化、按需工具/Recall
- **非范围**：不写代码；不重做 BML/Laputa 权威；不复活已关闭的 OpenHarness dry-run/ohmo 提案；不展开 Plan Mode 硬状态机 / Subagent Worktree（HARNESS-GAP 其余方向）
- **主线**：测量 → 稳定前缀 → 类型化预算 → 引用化工具结果 → 按需召回 → 压缩恢复验收
- **C1 施工图（Claude Code 专章）**：[`claude-code-prompt-cache-alignment.md`](./claude-code-prompt-cache-alignment.md) — 实施 Prompt Cache 时 **优先读该文件**

---

## 1. 问题定义（四层，非功能盘点）

Agent-Diva 已有 compaction、Working Memory checkpoint、L1 索引、prefetch 与 Memory CRUD，但上下文装配仍以 **字符串拼接 + 粗粒度 history 预算** 为主。结果是：

1. **稳定前缀被每轮易变内容打穿**，Prompt Cache 难以命中。
2. **预算只盯 history**，system / tools schema / skills / WM / recall 不进账。
3. **大工具输出** 要么 80k 硬截断，要么长期占满 transcript，缺少「原文引用 + 摘要进 prompt」。
4. **工具 schema 全量注入**，长尾工具与 MCP 挤占有效上下文。

本报告按四层定向调研，并给出可直接开 PR 的架构决策。

| 层 | 问题 | 成功判据 |
|----|------|----------|
| L1 稳定前缀与缓存 | 前缀字节/结构每轮漂移 | 无 mask/memory 变更时，连续两轮 stable prefix hash 不变；可解释 cache_break 原因 |
| L2 上下文预算与分层 | 简单时间截断 / 静默丢内容 | 分层预算 + 可观测裁剪报告 |
| L3 工具结果与长任务压缩 | 大输出永久进历史 | 原文侧车 + 摘要/指针；约束与 TODO 不丢 |
| L4 按需召回与工具挂载 | 每轮全量 tools | Core 常驻 + 长尾发现；召回失败可测 |

---

## 2. 方法与样本边界

### 2.1 首轮样本（固定）

| 研究目标 | 样本 | 源码/材料锚点 |
|----------|------|----------------|
| 长会话压缩与状态延续 | **Codex** | `.workspace/codex/codex-rs/core/src/compact.rs`、`client.rs`（compact / prompt_cache_key） |
| Prompt Cache、工具延迟挂载 | **Claude Code** + **OpenHarness** | `.workspace/claude-code/src/constants/systemPromptSections.ts`、`tools.ts`、`constants/tools.ts`（CORE_TOOLS）、`services/searchExtraTools/toolIndex.ts`、`services/compact/microCompact.ts`；`.workspace/OpenHarness/src/openharness/services/tool_outputs.py`、`tools/tool_search_tool.py`、`prompts/system_prompt.py` |
| Memory 与上下文分层 | **GenericAgent** | `.workspace/GenericAgent/ga.py`（working checkpoint / anchor）、`llmcore.py`、L0–L4 模板与 `memory/*_sop.md` |
| 落点 | **Agent-Diva** | `agent-diva-agent/src/agent_loop/turn/context.rs`、`context.rs`、`context_budget.rs`、`compaction/*`、`tool_assembly.rs`、`token_estimate.rs`、`agent-diva-tools/src/sanitize.rs` |

### 2.2 明确不做的调研方式

- 不写「十几个 agent 都有 memory/compact」对照表。
- 次级样本（OpenFang / ZeroClaw / Nanobot）仅用于交叉验证一句：轻量 agent 普遍 **控制 prompt 表面积**，而非扩展字符串拼接规则。
- 上级目录既有文 `morediva/claude-code-vs-agent-diva-harness-research.md`、`morediva/openharness-claude-code-diva-research.md` 作背景；**本专题正式产物以本目录为准**。

### 2.3 与既有冻结契约的关系

| 文档 | 关系 |
|------|------|
| `docs/architecture/memory-write-paths-contract.md` | **冻结**。C3 工具结果不得写成 BML 权威；WM / distill / consolidation 分工沿用该契约 |
| GA-MEM-PARITY Wave 0–6 | Memory 工具面、WM checkpoint、L1 索引、prefetch、GC 已完成；本专题 **接装配层**，不重做权威 |
| HARNESS-GAP-RESEARCH | 四个方向中 **Context 子集** 正式化为 C0–C5；Plan Mode / Worktree 仍 open 但不在本文实施 |
| OPENHARNESS dry-run / ohmo | **已关闭**；本文不复活 |

---

## 3. C0 基线：Agent-Diva 每轮上下文数据流

### 3.1 端到端路径

```
AgentLoop::run_turn (loop_turn.rs)
  └─ prepare_runtime_context (agent_loop/turn/context.rs)
       ├─ 入站 media / SecurityContext 清洗
       ├─ execution_start 时可选 ExecutionContextPolicy::Compact
       │    （独立路径：对 boundary 前历史做 ContextCompactor）
       ├─ check_budget(session.get_history(50), BudgetConfig)
       │    └─ should_compact → ContextCompactor::compact (Auto)
       │         失败：warn，非阻塞，history 原样继续
       ├─ ContextBuilder::build_messages_for_session(...)
       │    ├─ system = build_system_prompt_for_session
       │    │    mask → identity → 工具叙述 → ## Current Time (每轮变)
       │    │    → Workspace → Frozen Core → AGENTS.md
       │    │    → always skills + skills summary
       │    │    → L0 Memory Management Policy
       │    │    → MemoryProvider::system_prompt_block (L1 索引等)
       │    │    → 尾部约定文案 / memory 工具指引
       │    │    → ## Current Session (channel/chat) 拼进 system 尾部
       │    ├─ compaction_history → boundary + summary system 消息
       │    ├─ history ChatMessage → provider Message
       │    └─ current user message
       ├─ PreparedTurnContext::prepare
       │    plan_guard / approved_plan insert(1)
       │    sanitize_messages_for_provider
       │    scheduled 系统注记
       │    覆盖 last = current_turn_message
       ├─ inject_working_memory → insert(1)  ## Working Memory
       └─ prefetch → insert(1 + wm)           ## Recall 块
  └─ provider 调用：messages + tools.get_definitions()（当前启用集全量）
```

### 3.2 组件与职责对照

| 组件 | 路径 | 现状职责 | C0 判定 |
|------|------|----------|---------|
| `prepare_runtime_context` | `turn/context.rs` | 一轮上下文总编排 | 编排中心；字符串/插队注入过多 |
| `ContextBuilder` | `context.rs` | system 拼装、history 转换、tool result 截断入口 | 无 Fragment 类型 |
| `context_budget::check_budget` | `context_budget.rs` | history 估算 vs `system_budget_ratio` 预留 | **只测 history**；system 用固定比例占位 |
| `token_estimate` | `token_estimate.rs` | chars/4×4/3 ≈ chars/3 | 可用；未按层聚合 |
| `ContextCompactor` | `compaction/compaction_exec.rs` | LLM 摘要 + quality gate + meta prior | 对话级 compact 已有；非 tool microcompact |
| `ToolAssembly` | `tool_assembly.rs` | 按 gate/mask 注册；`rebuild_tools_for_turn` | **全量 schema** 进 API |
| `truncate_tool_result` | `tools/sanitize.rs` | `MAX_TOOL_RESULT_CHARS = 80_000` 硬截断 | 无侧车、无摘要、无指针 |
| Working Memory | MemoryProvider + turn 注入 | 每轮 `## Working Memory` | 位置 insert(1) 破前缀 |
| Prefetch | MemoryProvider::prefetch | 意图检索注入 | 同上；失败 warn |
| Frozen Core | `capture_frozen_core_for_session` | 会话级冻结人格投影 | SessionStable，适合进前缀 |
| Memory 契约 | `memory-write-paths-contract.md` | WM / LTM / distill / AutoDream / consolidation | 与 C3 对齐，勿混淆 |

### 3.3 当前上下文组成（逻辑层，非 token 实测）

以下为 **逻辑层** 顺序（实现上部分被 merge 进同一 system 字符串）：

| 序号 | 逻辑块 | 稳定性 | 今日计量 |
|------|--------|--------|----------|
| 1 | Mask overlay | Session（mask 切换时变） | 计入 system 字符串，无独立计量 |
| 2 | Identity + 工具能力叙述 | 相对 Stable | 同上 |
| 3 | **Current Time** | **TurnVolatile** | 同上 — **必破 cache** |
| 4 | Workspace 路径 | SessionStable | 同上 |
| 5 | Frozen Core | SessionStable（capture） | 有 `DEFAULT_FROZEN_CORE_BUDGET` |
| 6 | AGENTS.md | SessionStable（文件变则变） | `WORKSPACE_MD_MAX_CHARS=4000` |
| 7 | Always skills + skills summary | SessionStable | progressive loading 已有 |
| 8 | L0 policy | Stable | 常量 |
| 9 | L1 index / startup memory | SessionStable；CRUD 后热刷新 | `l1_index_lines` |
| 10 | Compaction summaries | 会话累积 | meta-compact 有上限 |
| 11 | History（含 tool 全文/截断） | 增长 | `check_budget` + `get_history(50)` |
| 12 | Working Memory block | Turn/Session 变 | **未进 budget** |
| 13 | Prefetch/Recall | Turn 变 | **未进 budget** |
| 14 | Current user / media | Turn | — |
| 15 | Tool JSON schemas | 随 mask/plan/rebuild 变 | **未进 budget** |

> C0「测量」在本文的含义：建立 **组成、变化源、裁剪行为与盲区** 的工程基线。精确 token 占比需在 C1/C2 引入 `ContextAssemblyReport` 后由运行时日志补齐；禁止在无报告前用拍脑袋百分比当真理。

### 3.4 动态变化源（Prompt Cache 破坏清单）

| ID | 变化源 | 位置 | 频率 | 影响 |
|----|--------|------|------|------|
| CB-1 | `## Current Time` 每轮刷新 | system 中段 | 每轮 | 中段前缀全部失效 |
| CB-2 | `inject_working_memory` `insert(1)` | system 之后第 1 条 | checkpoint 更新时内容变；有无块也变长度 | 打断「单 system 稳定前缀」 |
| CB-3 | prefetch `insert(1+wm)` | 同上后移 | 有意图且命中时 | 同上 |
| CB-4 | plan_guard / approved_plan `insert(1)` | prepare 阶段 | plan 模式 | 同上 |
| CB-5 | scheduled turn system 注记 | prepare | 定时任务 | 后缀型，相对可控 |
| CB-6 | startup_markdown 热刷新（Wave 6 F4） | system 内 L1 段 | memory CRUD 后 | Session 内前缀变；应记 cache_break |
| CB-7 | mask 切换 / rebuild_tools | system + tools 参数 | 策略变化 | 合法 break，需可观测 |
| CB-8 | 工具定义集合/顺序不稳定 | API `tools` 数组 | 注册顺序/MCP 上下线 | 即使 messages 稳定也可破 cache |
| CB-9 | compaction 注入 boundary/summary | system 消息序列 | compact 后 | 预期 break；应固定 post-compact 顺序 |

### 3.5 现状裁剪与失败语义

| 路径 | 触发 | 成功行为 | 失败/静默行为 |
|------|------|----------|----------------|
| Auto compact | `history_estimated > compact_threshold` | 写 `CompactSummary`，`last_compacted` 前进 | **Err → warn，继续用未压缩 history** |
| Manual/execution compact | plan 执行初始化 | quality≥0.6 才 Ready | 低于阈值 → execution Blocked（较好） |
| History 窗口 | `get_history(50)` | 条数上限 | **纯时间/条数截断**，非优先级 |
| Tool result | `> 80_000` chars | 截断 + 尾注 | **丢弃尾部，无原文引用** |
| Prefetch | 非空 intent | 注入 prompt_block | Failed → warn，消息列表不变（有测试） |
| Working memory | provider 返回 block | insert | Err → warn，跳过 |
| Consolidation | memory_window + 无 distill | 条目化 CRUD | 与对话 compact 独立；不替代 tool microcompact |

**C0 结论：** 已有「尽力压缩」路径，但缺少 **分层预算、强制报告、稳定前缀契约、工具结果引用**。继续在 `context.rs` 加拼接规则会放大 CB-1..CB-9。

---

## 4. 样本定向对照（机制 → 落点）

### 4.1 Codex：长会话压缩与状态延续

**机制要点**

1. **Compact 触发分相**：Auto / Manual；`CompactionPhase` 与 analytics 分离。
2. **`InitialContextInjection`**：
   - `DoNotInject`：pre-turn/manual compact 后清空 reference，下一轮完整 reinject initial context；
   - `BeforeLastUserMessage`：mid-turn compact 后模型被训练为「summary 在历史末」，故把 initial context 插在最后一条真实 user 之前。
3. **Context window exceeded 时从最旧删除**（`remove_first_item`），注释明确 **preserve cache (prefix-based)**。
4. **Compact 本身也有 retry / trim**，并通知 UI「trimmed N older items」。
5. **`prompt_cache_key`** 与 conversation 绑定（client 路径）。
6. 工具输出侧有独立 `TruncationPolicy` / `approx_token_count`（与对话 compact 分管道）。

**对 Diva 的可迁移决策（非抄代码）**

| 决策 | 说明 |
|------|------|
| D-CDX-1 | 区分 **pre-turn compact** 与 **mid-turn / reactive compact** 的 post-state：是否 reinject Frozen Core / L0 / L1 |
| D-CDX-2 | 硬超窗时默认 **head-trim 旧 history**，禁止从尾部砍当前任务 |
| D-CDX-3 | Compact 观测字段对齐：trigger / reason / tokens_before / tokens_after / truncated_count |
| D-CDX-4 | 工具结果 truncation **独立模块**，不与 `ContextCompactor` 混成一次 LLM 摘要 |

**Diva 已有对齐点：** `CompactTrigger::{Auto,Manual}`、quality gate、meta-compaction prior summaries、execution 初始化 compact。  
**缺口：** injection 策略未分相；超窗主要靠提前 compact；无 prefix-preserving trim 显式策略；无 compact analytics 结构化事件。

### 4.2 Claude Code + OpenHarness：Prompt Cache 与工具延迟挂载

**Claude Code 机制要点**

1. **System section 模型**（`systemPromptSections.ts`）：
   - `systemPromptSection`：计算一次，缓存至 clear/compact；
   - `DANGEROUS_uncachedSystemPromptSection(name, compute, reason)`：显式 volatile，**必须写 reason**。
2. **工具池装配排序**（`assembleToolPool`）：
   - built-in **按名排序** 且保持 **连续前缀**；
   - MCP **另段排序**；
   - 禁止 flat 混排 — 服务端 cache policy 在「最后一个 prefix-matched built-in」后下断点。
3. **CORE_TOOLS**：文件/Shell/检索/Agent/Task/Plan/Web/LSP/Skill/SearchExtraTools 等常驻；其余 deferred，经 TF-IDF `toolIndex` + SearchExtraTools 挂载。
4. **Microcompact**：对可压缩 tool result 做旧结果坍缩（cached microcompact / time-based 等路径）；与 full conversation compact 分层。
5. **Post-compact 固定顺序**：boundary → summary → kept messages → attachments（files/skills/plan/deferred tools）→ hooks。

**OpenHarness 机制要点**

1. `tool_outputs.py`：inline / preview / microcompact 阈值 env 可配（默认 16k / 3k / 4k）。
2. `tool_search`：registry 子串搜索 — **MVP 级发现**，弱于 CC TF-IDF，但接口形态正确。
3. system prompt 仍偏静态拼接；OH 强项在 dry-run/hooks（本专题不实施 dry-run）。

**对 Diva 的可迁移决策**

| 决策 | 说明 |
|------|------|
| D-CC-1 | 引入 **Stable / SessionStable / TurnVolatile** section 标签；TurnVolatile **禁止** 插在 stable system 前缀中间 |
| D-CC-2 | Tool 定义：**内置字典序连续前缀 + MCP 后缀段**；重建时稳定排序 |
| D-CC-3 | Core tools 常驻 + `tool_search` / deferred mount；索引首轮关键词，二期可选 TF-IDF |
| D-CC-4 | Microcompact（tool result）与 Macrocompact（对话 summary）分管道，阈值可配置 |
| D-CC-5 | Post-compact 恢复清单写死顺序（含 discovered tools / skills 状态） |

**Diva 已有对齐点：** skills progressive loading；mask 重建工具；compaction boundary 文案。  
**缺口：** 无 section cache 语义；Current Time 在中段；WM/prefetch 插队；工具全量；仅 80k 截断无 microcompact。

### 4.3 GenericAgent：Memory 分层与工作记忆

**机制要点**

1. **L0–L4 纪律**：L0 更新 SOP；L1 索引；L2 长期事实；L3 经验 SOP；L4 raw sessions — 与 Diva BML/L1/Skills/distill 叙事同构。
2. **`update_working_checkpoint`**：key_info / related_sop；长任务每 7 轮 tip 强制刷新。
3. **Anchor prompt**：earlier_context fold + 最近 W 轮 history 交替注入 — **非单纯时间截断**。
4. **每轮 `<summary>` 一行** 写入 `history_info`，形成可 fold 的进度带。
5. **`start_long_term_update`**：显式晋升，禁止未验证信息 — 对齐 Diva distill + Action-Verified。

**对 Diva 的可迁移决策**

| 决策 | 说明 |
|------|------|
| D-GA-1 | **不重做** L0–L4 权威；装配层消费既有 L0/L1/WM/prefetch API |
| D-GA-2 | Working Memory 继续承载「决策/约束/未完成/证据 id」；**禁止** 塞入大工具原文 |
| D-GA-3 | 可借鉴「进度一行摘要带」作为 C3 microcompact 的结构化摘要字段，而非第二套 memory |
| D-GA-4 | C5 长任务验收用例可对齐 GA 节律：中途 checkpoint、可恢复、ask 用户确认 |

**Diva 已有对齐点：** Wave 2–6 已实现 checkpoint 注入、L1 预算、distill、GC、prefetch。  
**缺口：** 装配层未类型化；裁剪报告；工具结果引用；与 compact 的协同策略未产品化。

---

## 5. 四层差距映射

### 5.1 稳定前缀与缓存

| 项 | Diva 现状 | 样本证据 | 决策 |
|----|-----------|----------|------|
| 前缀布局 | 单 system 大字符串 + insert(1) 动态块 | CC section cache；Codex head-trim 保 prefix | ADR-CTX-1 |
| 时间戳 | system 中段每轮变 | CC 将 volatile 显式标出 | 移到 TurnVolatile 后缀 |
| 工具顺序 | 注册序 | CC built-in 前缀排序 | 稳定排序 + 分区 |
| 观测 | 几乎无 | Codex compaction analytics；CC cache break reason | prefix hash + break reasons |

### 5.2 上下文预算与分层

| 项 | Diva 现状 | 样本证据 | 决策 |
|----|-----------|----------|------|
| 预算维度 | history vs 固定 system_ratio | GA 分层；CC tool/micro 分层 | ADR-CTX-2 多层 BudgetPlan |
| 淘汰 | compact 或 get_history(50) | Codex head-trim + summary | 优先级淘汰 + 报告 |
| 静默失败 | compact/prefetch warn | 工业路径可观测 | 强制 AssemblyReport |

### 5.3 工具结果与长任务压缩

| 项 | Diva 现状 | 样本证据 | 决策 |
|----|-----------|----------|------|
| 大输出 | 80k 截断 | OH 阈值；CC microcompact | ADR-CTX-3 引用化 |
| 与 WM/BML | 契约已清，装配未接 ref | GA checkpoint + 显式晋升 | 严格分工表 |
| 对话 compact | LLM summary 已有 | Codex/CC 9 段/结构化 | 增强保留约束/TODO/证据 id |

### 5.4 按需召回与工具挂载

| 项 | Diva 现状 | 样本证据 | 决策 |
|----|-----------|----------|------|
| Tools | 全量 definitions | CORE_TOOLS + deferred | ADR-CTX-4 |
| Recall | prefetch 生产可用 | GA 读 L2/L3 纪律 | 保留 + 测错误/空/降级 |
| 检索算法 | 意图字符串 | CC TF-IDF；OH 子串 | 先确定性，后 TF-IDF |

---

## 6. 架构决策（冻结）

### ADR-CTX-0：类型化中间层 ContextFragment（强制）

**状态：** Accepted  
**理由：** 继续在 `context.rs` / `turn/context.rs` 叠加字符串规则无法表达稳定性、预算、引用与淘汰策略，且无法产生可测的装配报告。

**决策：** 引入装配中间层（名称可调整，语义不可砍）：

```text
ContextFragment {
  id: String,
  source: FragmentSource,   // 见枚举
  stability: Stable | SessionStable | TurnVolatile,
  priority: u8,             // 越大越不易淘汰
  token_estimate: usize,
  compressible: bool,
  content: Inline(String) | Ref { store: ArtifactStoreId, key: String, preview: String },
  eviction: Never | Compact | Drop | Summarize,
  cache_break_reason: Option<String>,  // 仅当本 fragment 导致前缀变化时
}

FragmentSource:
  Mask | Identity | FrozenCore | Rules | SkillsAlways | SkillsIndex |
  L0Policy | L1Index | WorkingMemory | PrefetchRecall |
  CompactionSummary | HistoryMessage | ToolResult | VolatileMeta |
  CurrentUser | PlanGuard | ScheduledNote

ContextBudgetPlan {
  total_max: usize,
  layers: Map<BudgetLayer, LayerBudget>,  // hard/soft limit + policy
}

ContextAssemblyReport {
  fragments_in: usize,
  selected: [...],
  dropped: [{ id, source, reason }],
  compacted: [{ id, strategy }],
  cache_break_reasons: [{ section, reason }],
  prefix_hash: String,
  totals_by_layer: Map<BudgetLayer, usize>,
}
```

**流水线（唯一推荐）：**

```text
collect_fragments(session, turn)
  → estimate_tokens
  → plan_budget(ContextBudgetPlan)
  → select_and_evict  → 产生 report.dropped/compacted
  → serialize_in_fixed_order  → Vec<Message> + ToolDefinitions
  → observe(report)  // tracing + 可选 audit
```

**非目标：** 一次 PR 重写全部 MemoryProvider；中间层先适配现有 provider 输出为 Fragment。

**落点建议：** 新模块 `agent-diva-agent/src/context_assembly/`（或 `context/fragment.rs` + `budget_plan.rs`），`prepare_runtime_context` 改为调用 assembler；`ContextBuilder` 退化为 fragment collectors。

---

### ADR-CTX-1（C1）：稳定前缀与缓存对齐

**状态：** Accepted  
**优先级：** 实施第一刀（风险低、降本直接）

**固定序列化顺序：**

```text
[STABLE / SESSION-STABLE PREFIX]
  1. Mask（若有）+ Identity + 静态规则叙述          stability: SessionStable
  2. Frozen Core 投影                               SessionStable
  3. AGENTS.md / always skills / skills index         SessionStable
  4. L0 policy + L1 index                             SessionStable（热刷新 → 记 cache_break）
  5. Tool definitions: sorted CORE 连续前缀
     + （可选）本会话已 mount 的 deferred（稳定排序）

[POST-PREFIX：允许每轮变]
  6. Compaction boundary + summaries（固定子序）
  7. History（含 ToolResult inline 或 preview）
  8. Working Memory block                            Turn/Session volatile — **禁止 insert(1) 进前缀**
  9. Prefetch / Recall block
 10. VolatileMeta：Current Time、channel/chat、scheduled 注记
 11. Plan guard / approved plan（若需要，紧贴当前任务，勿插入 stable 中段）
 12. Current user message
```

**规则：**

1. **TurnVolatile 不得改写 prefix 区字符串**；不得 `messages.insert(1, …)` 插到 system 前缀之后充当「第二 system 前缀」。
2. Working Memory / Prefetch 改为 **prefix 之后的独立 system/user 注记消息**，或并入 VolatileMeta 段。
3. Tool 名 **字典序**；built-in 与 MCP **分区拼接**（对齐 D-CC-2）。
4. 观测：每轮计算 `prefix_hash`（稳定区序列化字节）；变化时必须带 `cache_break_reasons`。
5. Provider 若支持 `prompt_cache_key`，绑定 `session_key`（对齐 Codex）。

**验收（C1）：**

- 单测：两轮装配仅 Current Time 变化 → prefix_hash 不变。
- 单测：WM 更新 → prefix_hash 不变，volatile 段变。
- 单测：tools 顺序跨 rebuild 稳定（同输入集合）。
- 回归：`PreparedTurnContext` 边界测试升级为「prefix 区不包含 WM/prefetch/time」。

---

### ADR-CTX-2（C2）：分层预算调度器

**状态：** Accepted  
**依赖：** ADR-CTX-0 骨架（可与 C1 同迭代引入类型，预算策略可第二 PR 收紧）

**默认 LayerBudget（相对 `max_tokens`，实施前用 AssemblyReport 校准）：**

| BudgetLayer | 建议 soft | hard 行为 |
|-------------|-----------|-----------|
| FrozenCoreAndRules | ~8–12% | Never drop Frozen；Rules 附件可截断 |
| ToolSchemasCore | ~10–15% | 超则拒绝再 mount deferred，不砍 core |
| L1Index | 配置 `l1_index_lines` | 缩行 / pointer-only |
| WorkingMemory | ~2–4% 或 max chars | Summarize key_info；禁原文 |
| PrefetchRecall | ~2–4% | Drop + report（可重试 search） |
| CompactionSummaries | ~5–10% | MetaCompactor（已有） |
| History | 剩余 | Compact 优先于条数截断；keep_recent |
| ToolResultInline | per-item + per-turn cap | 转 Ref（C3） |

**淘汰优先级（从先到后）：**

1. 过期/可 microcompact 的 ToolResult inline  
2. Prefetch 块  
3. 过旧 History（head）→ 触发/复用 macro compact  
4. L1 行数压缩  
5. Skills index 细节  
6. **永不：** Frozen Core、当前 user、未完成 tool_call 配对、活跃 plan 约束、WM 中标记的 constraints  

**强制：** 任何 Drop/Summarize 写入 `ContextAssemblyReport`；debug/trace 默认打印；关键路径可 audit。  
**禁止：** 仅 `get_history(N)` 作为唯一策略长期保留（可作 fallback，但必须 report reason=`legacy_count_cap`）。

**与现有 `BudgetConfig` 关系：**

- 保留 `max_tokens` / `compact_threshold_ratio` / `keep_recent_count` 作为 History 层参数。
- `system_budget_ratio` 升级为 **多桶之和的上限校验**，不再假装 system 已计量。

---

### ADR-CTX-3（C3）：工具结果引用化

**状态：** Accepted  
**依赖：** Fragment `Ref`；session 侧车存储

**策略：**

```text
tool_execute → raw_output
  → if len <= inline_threshold: Inline(full or lightly trimmed)
  → else:
       store.put(session, tool_call_id, raw_output)
       prompt: ToolResultRef {
         preview: head/tail or structured summary,
         key: tool_call_id,
         stats: {chars, truncated: true},
         status/error: ...,
         reclaim_hint: "use read_tool_result / read_file on artifact path"
       }
  → optional microcompact: older Inline → Ref/preview when history pressure
```

**阈值建议（可配置，对齐 OpenHarness 量级）：**

| 参数 | 建议默认 |
|------|----------|
| `tool_output_inline_chars` | 8_000–16_000（替换「一刀 80k 永久进历史」） |
| `tool_output_preview_chars` | 2_000–4_000 |
| `microcompact_tool_result_chars` | 4_000 |
| 安全上限 | 保留 80k 或更高作为 **单次 store 写入上限**，不是 prompt 常驻上限 |

**与现有子系统分工（必须遵守 memory-write-paths-contract）：**

| 存储 | 职责 | 非职责 |
|------|------|--------|
| Tool artifact / session store | 原始输出、可重读 | BML 权威 |
| Prompt ToolResultRef | 摘要、状态、失败原因、指针 | 永久全文 |
| Working Memory checkpoint | 决策、约束、TODO、证据 id 列表 | 复制 dump |
| BML / Laputa | 用户要求记住的事实；governed 变更 | 自动吞工具输出 |
| memory_distill → Skills | 可复用步骤/坑点 | 单次日志 |
| ContextCompactor | 对话级宏观摘要 | 替代每条 tool microcompact |
| Consolidation | 无 distill 时兜底晋升 | 工具原文仓库 |

**压缩摘要必须保留的字段（macro + micro 合同）：**

- 决策与用户约束  
- 未完成事项 / TODO  
- 证据引用（tool_call_id / path / hash）  
- 失败原因与已尝试手段  
- 当前阻塞  

**非目标：** 把 tool dump 写入 Typed authority；上向量库做工具结果检索（可后期）。

---

### ADR-CTX-4（C4）：按需工具与 Recall

**状态：** Accepted  
**依赖：** 稳定 tool 排序（C1）；Fragment 报告（C0/C2）

**分阶段：**

| 阶段 | 内容 | 检索 |
|------|------|------|
| C4a | 划分 CORE vs DEFERRED；CORE 常驻 schema；注册 `tool_search` | 名/描述关键词 |
| C4b | deferred mount 写入 session 已发现集合；post-compact 再宣告 | 同 C4a |
| C4c | 可选 TF-IDF 索引（移植 CC `toolIndex` 思想） | 无 embedding 硬依赖 |
| C4d | Recall：现有 prefetch 纳入 BudgetLayer；扩展测试 | 确定性 intent → 日后可重排 |

**CORE 建议集合（实施时按 Diva 实际工具名映射，思想对齐 CC）：**

- 文件读写改、目录/内容搜索、shell  
- ask_user、message（若产品启用）  
- update_plan / plan 相关只读与执行工具（受 mask 约束）  
- memory_add/list/search/update/remove/distill  
- update_working_checkpoint  
- tool_search / mount_tool（或等价）  
- cron 最小集（若默认启用）  

**MCP：** 默认 deferred；挂载后进入「会话已发现」集合并稳定排序。

**测试矩阵（强制）：**

| 场景 | 期望 |
|------|------|
| 无召回 | 不注入假块；report.prefetch=empty |
| 召回失败 | 消息前缀不被破坏；reason 可读（Legacy 已有模式） |
| 错误召回 | 预算可 Drop；不污染 WM |
| 工具未挂载 | 模型可 tool_search 后 mount；未 mount 调用返回明确错误 |
| mask 拒绝 | deferred 与 core 均不可绕过 mask |

---

### ADR-CTX-5（C5）：压缩与恢复验收

**状态：** Accepted  
**依赖：** C1–C4 最小闭环

**验收用例（实施阶段自动化 + 必要手工）：**

| ID | 场景 | 断言 |
|----|------|------|
| V1 | 长任务多轮 + 自动 compact | 用户约束句仍在 summary 或 WM |
| V2 | 含 TODO / plan | 未完成项不丢 |
| V3 | 大工具输出 | prompt 无全文；artifact 可重读 |
| V4 | 进程重启 / session reload | Frozen Core + 历史 + ref 可恢复；WM GC 语义符合契约 |
| V5 | 两轮无策略变更 | prefix_hash 稳定 |
| V6 | 超预算 | AssemblyReport.dropped 或 compacted 非空 |
| V7 | mid-turn vs pre-turn compact | reinject 策略符合 ADR-CTX-1/Codex 分相 |

**与 GA 节律对齐的可选 soak：** 每 N 轮强制 checkpoint；长任务中途抽检约束句。

### 6.6 实施前施工补充决策（冻结）

以下决策补足 ADR-CTX-0..5 的跨阶段空白；后续实现不得以 provider 假设、临时字符串
搬运或“实现时再决定”为由绕过。

#### DEC-CTX-A：动态块的 provider-aware 序列化

- `ContextSection` 的顺序是 **provider-neutral 逻辑顺序**，不是所有 provider 共用的
  wire role。
- WM、Recall、VolatileMeta、Plan 位于 history 后、current user 前的 post-prefix 区；
  **默认不得直接序列化为对话中段 `system`**。
- serializer 必须根据 provider capability 在以下策略中显式选择：
  `MidConversationSystem`、`UserContextEnvelope`、`NativeContextBlock`。未知 provider
  fail-safe 到带稳定边界标记的 `UserContextEnvelope`，不得静默猜测。
- 各 provider adapter 必须有 characterization test，证明角色序列、tool-call 配对和
  current user 位置合法；逻辑 fragment 测试与 wire-shape 测试分层。

#### DEC-CTX-B：C1 必须消费 C1-0 类型骨架

- C1-0 已落地 `ContextSection`、`SectionStability`、`PromptSection`、固定逻辑顺序和
  `cache_break_reason`；C1a 必须从现有类型收集/序列化，禁止再做一轮字符串位置搬运。
- `prefix_hash` 尚未生产化，归 P0-4 与 cache observer 同步接入。
- 完整 `ContextFragment` 预算字段仍归 C2；这不允许 C1 绕过最小类型层。

#### DEC-CTX-C：会话快照与显式失效矩阵

Stable/SessionStable 采用「会话快照优先、显式事件失效」，禁止每轮文件系统轮询：

| Section/事件 | 会话内语义 | 显式失效动作 | break reason |
|--------------|------------|--------------|--------------|
| Frozen Core | 会话首次 capture 后冻结 | 新 session / session reset | `session_reset` |
| AGENTS.md | session start/reload 快照 | 显式 context reload 或新 session | `agent_rules_reload` |
| Always Skills / Skills index | session start/reload 快照 | 显式 skills reload 或新 session | `skills_reload` |
| Mask | 当前 mask 快照 | mask 切换时只重建 mask section 与 tool pool | `mask_changed` |
| L1 index | session-stable version | Memory apply 后只刷新 L1 section 并递增 prefix version | `l1_hot_refresh` |
| Compact | stable snapshot **不清空** | 只更新 post-prefix boundary/summary | `compaction_changed`（非 stable section break） |
| Session end | 清除全部 session cache | 下次会话重新 capture | `session_ended` |

除非 provider 明确要求重建 stable blocks，compact 不得借鉴样本实现而无条件清空 Diva
section cache；任何例外必须写 capability、reason 和回归测试。

#### DEC-CTX-D：Tool 排序与 cache-control 原子边界

- C1b 只做 tool definitions 的 built-in/MCP 分区、稳定排序与 schema 字节测试。
- 在相同 tool 集合连续重建的完整序列化字节一致之前，不得修改 provider
  `apply_cache_control` 锚点。
- C1d 才修改 cache-control；两个提交分别记录 cache 指标，便于定位回归来源。

#### DEC-CTX-E：C3 artifact 安全契约

C3 写入任何原始工具输出前必须同时满足：

1. workspace + session 双重隔离，读取必须验证当前 security context；
2. key 由运行时生成并绑定 tool_call_id，模型不得提交任意路径或伪造 key；
3. per-item、per-session、per-workspace 容量上限与确定性拒绝/淘汰行为；
4. retention、session end、workspace 删除和孤儿 GC 语义；
5. 写入、preview、日志和错误的敏感信息清洗；不得把原始输出写入 tracing/audit；
6. 重启后可读性必须显式选择并测试；若选择易失，恢复时返回稳定 `artifact_missing`
   而不是伪造摘要；
7. artifact 缺失、过期、越界、损坏均 fail closed，并保留可读错误码；
8. microcompact 不得破坏 assistant tool_call ↔ tool result 配对；Ref 仍保留状态、
   tool_call_id、大小/hash 和失败原因。

安全契约未具备时，C3 只能保留现有截断行为，不得提前上线半成品 artifact store。

#### DEC-CTX-F：C4 动态挂载生命周期

- `tool_search` / `mount_tool` 成功后，工具在 **同一 turn 的下一次 provider call**
  生效，不等待下一条用户消息。
- discovered/mounted 集合绑定 session；同名重复 mount 幂等，顺序确定。
- 每次 registry rebuild 都重新经过 mask、plan phase、approval 与 builtin gate；已发现
  不等于获授权，禁止以 session 状态绕过策略。
- compact 后重宣告 mounted tools；session end/reset 清除集合；工具源下线时返回明确
  unavailable，不保留幽灵 schema。
- 必测：search→mount→same-turn call、mask deny、plan phase change、compact、reset、
  provider retry 不重复 mount。

#### DEC-CTX-G：缓存观测与告警分类

每次 eligible provider call 至少记录：`system_hash`、`tools_hash`、逐工具 hash（debug）、
declared break reasons、provider、model、cache policy/TTL、cache read/create tokens、stable
prefix token estimate，以及最近窗口的命中趋势。

分类规则：

- hash 变化且有匹配 reason：`expected_break`，info/debug，不告警；
- provider/model/policy/TTL 改变：`policy_break`，独立计数，不归因装配回归；
- hash 变化但无 reason：`undeclared_structural_break`，立即 warn；
- hashes 与 policy 均稳定，但达到缓存阈值的请求连续 2 次显著 read miss：
  `suspected_cache_miss`，warn；单次下降只记样本，不告警；
- cache deletion/microcompact 明确声明时：`expected_deletion`，不误报。

观测窗口按 session + provider + model + cache policy 分桶；短于服务端最小缓存阈值的
前缀不进入 miss 告警分母。

---

## 7. 实施切片（PR 级，供后续开发）

```text
C0  本文档（完成）
C1  稳定前缀布局 + tool 稳定排序 + prefix_hash 观测
    └─ 建议同 PR 引入 ContextFragment 骨架（ADR-CTX-0 最小集）
C2  ContextBudgetPlan + 分层淘汰 + AssemblyReport 强制
C3  Tool artifact store + Ref 注入 + microcompact
C4  CORE/DEFERRED + tool_search + same-turn mount + prefetch 预算/测试矩阵
C5  验收用例集 + 长任务 soak 清单
```

**依赖图：**

```text
ADR-CTX-0 ─┬─► C1 ─► C2 ─► C3 ─► C5
           └────────► C4 (可与 C3 并行，需 C1 排序)
```

**建议代码落点（实施时，非本交付）：**

| 模块 | 动作 |
|------|------|
| `agent-diva-agent/src/context_assembly/` | 新建 Fragment/Plan/Report/serialize |
| `turn/context.rs` | 改为调用 assembler |
| `context.rs` | 拆 collect_* |
| `context_budget.rs` | 升级或包装为多层 |
| `tool_assembly.rs` | stable sort + core/deferred |
| `agent-diva-tools` / core session | tool artifact 读写；先满足 DEC-CTX-E 安全契约 |
| `compaction/*` | 保留 macro；新增 micro 或独立模块 |
| tests | 上表 V1–V7 与 cache_break 单测 |

---

## 8. 非目标与延期

| 项 | 处理 |
|----|------|
| Plan Mode 物理写拦截 | HARNESS-GAP 另向；不阻塞 C1–C3 |
| Subagent Git Worktree | 另向 |
| EventBus Trait Hooks | 已独立延期 |
| OpenHarness dry-run / ohmo | 已关闭 |
| Embedding 召回 | C4 以后可选 |
| LSP 工具 | 非上下文主线 |
| 重写 BML / 回退文件权威 | 禁止 |
| 精确计费 UI | 可消费 AssemblyReport，非阻塞 |

---

## 9. 源码与参考锚点

### Agent-Diva

- `agent-diva-agent/src/agent_loop/turn/context.rs` — `prepare_runtime_context`、WM/prefetch 注入  
- `agent-diva-agent/src/context.rs` — system 拼装、Current Time、history  
- `agent-diva-agent/src/context_budget.rs` — 粗粒度预算  
- `agent-diva-agent/src/compaction/` — macro compact  
- `agent-diva-agent/src/tool_assembly.rs` — 工具注册  
- `agent-diva-tools/src/sanitize.rs` — `MAX_TOOL_RESULT_CHARS`  
- `docs/architecture/memory-write-paths-contract.md` — 写路径冻结契约  

### 样本

- Codex: `.workspace/codex/codex-rs/core/src/compact.rs`  
- Claude Code: `.workspace/claude-code/src/constants/systemPromptSections.ts`、`tools.ts`、`constants/tools.ts`、`services/searchExtraTools/toolIndex.ts`  
- OpenHarness: `.workspace/OpenHarness/src/openharness/services/tool_outputs.py`、`tools/tool_search_tool.py`  
- GenericAgent: `.workspace/GenericAgent/ga.py`（working checkpoint / anchor / long-term update）  
- 背景（仓库外 morediva 根）: `claude-code-vs-agent-diva-harness-research.md`  
- 压缩模式分析: `.workspace/analysis/claude-code-compact-patterns.md`  

---

## 10. 结论

1. **C0 基线已足够开工：** 瓶颈不是「有没有 compact」，而是 **无类型化装配、无稳定前缀契约、无分层预算报告、无工具结果引用**。  
2. **最重要架构约束：** 引入 `ContextFragment` / `ContextBudgetPlan` / `ContextAssemblyReport`，停止在 context 路径继续堆字符串规则。  
3. **实施顺序冻结为** `C1 → C2 → C3 → C4 → C5`，其中 C1 与 Fragment 骨架同船，C4 可与 C3 并行。  
4. **Memory 主线已完成产品化的部分（GA-MEM）不再重复建设**；上下文增强只通过明确接口消费 BML/WM/prefetch，并遵守 write-path 契约。  
5. 本报告即为 **HARNESS-GAP-RESEARCH 在 Context 方向的正式实施规格**；其余 Harness 方向分轨推进。
