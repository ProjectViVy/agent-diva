# Claude Code 导向：Prompt Cache 上下文管理对齐规格

- **状态**：C1a–C1d 已实现；P0-1..5 与 T1–T8 完成
- **日期**：2026-08-10
- **主样本**：`.workspace/claude-code`（唯一深读对象）
- **目标**：提高 Agent-Diva **Prompt Cache 命中率**；降低重复前缀计费与延迟
- **关联**：总论 [`c0-baseline-and-architecture-decisions.md`](./c0-baseline-and-architecture-decisions.md) ADR-CTX-1；本文件是 **C1 的详细施工图**
- **非范围**：Codex/GA 展开、Plan Mode/Worktree、完整 deferred TF-IDF、服务端 `cache_edits` 产品化

---

## 1. 问题一句话

Agent-Diva 在 provider 层 **已经会打 `cache_control`**，但装配层 **每轮改写 system 前缀与工具/插队消息**，服务端缓存键对不齐 → `cache_read` 接近 0，缓存形同虚设。

Claude Code 的核心不是「多一个 API 字段」，而是：

1. **前缀字节会话内尽量不变**（section 缓存 + 快照式 env）
2. **易变内容明确隔离**（`DANGEROUS_uncached` + reason，或挪到附件/后缀）
3. **工具 schema 排序与会话锁定**（built-in 连续前缀 + MCP 另段）
4. **可观测 break**（hash diff + cache_read 下降告警）
5. **中途不翻转** 会 bust cache 的 header/TTL（sticky latch）

---

## 2. Claude Code 机制深读

### 2.1 System Prompt Section 模型

**源码**

| 文件 | 作用 |
|------|------|
| `src/constants/systemPromptSections.ts` | section 定义：`cacheBreak` 标志 + 会话 cache Map |
| `src/constants/prompts.ts` `getSystemPrompt` | 组装 section 列表 |
| `src/bootstrap/state.ts` | `systemPromptSectionCache`；clear on compact/clear |
| `src/context.ts` | `getSystemContext` / `getUserContext` 整会话 `memoize` |

**规则**

```text
systemPromptSection(name, compute)
  → cacheBreak=false
  → 命中 cache 则不再 compute
  → 仅 /clear 或 /compact 清空

DANGEROUS_uncachedSystemPromptSection(name, compute, reason)
  → cacheBreak=true
  → 每轮 recompute
  → reason 强制：解释为何必须打破缓存
```

**设计含义**

- 默认可缓存；**打破缓存是显式、可审计的危险操作**。
- MCP 指令曾放在 uncached section（「MCP servers connect/disconnect between turns」）；后续用 **delta attachment** 避免整段 system 重算。

**Date / Git 快照**

- simple 路径：`Date: ${getSessionStartDate()}` — **会话开始日**，非每分钟时钟。
- git status 文案写明：*snapshot at start of conversation, will not update*。
- 对比 Diva：`chrono::Local::now().format("%Y-%m-%d %H:%M (%A)")` 写在 system **中段** → 每轮必变。

### 2.2 Tool 池与 Schema 缓存

**源码**

| 文件 | 作用 |
|------|------|
| `src/tools.ts` `assembleToolPool` | built-in 排序连续前缀 + MCP 另段排序 |
| `src/constants/tools.ts` `CORE_TOOLS` | 常驻工具集合 |
| `src/utils/toolSchemaCache.ts` | 会话内锁定 schema 渲染字节 |
| `src/services/searchExtraTools/toolIndex.ts` | 长尾 TF-IDF（缓存主线的下游） |

**关键注释（语义必须保留到 Diva）**

> Server `claude_code_system_cache_policy` places a global cache breakpoint after the last prefix-matched built-in tool; a flat sort would interleave MCP tools into built-ins and invalidate all downstream cache keys.

**TOOL_SCHEMA_CACHE 动机**

- Tool schemas 在服务端位置靠前（约 position 2）；**任意字节变化** 会 bust 整个 tools 块及下游。
- 中途 GrowthBook / 动态 `tool.prompt()` 重渲染是常见 bust 源 → 首次渲染后锁死，直至显式 clear。

### 2.3 请求侧 `cache_control`

**源码**：`src/services/api/claude.ts`

- `userMessageToMessageParam` / `assistantMessageToMessageParam`：在 content 末 block 附加 `cache_control: getCacheControl(...)`。
- system blocks 同样带 control；**TTL/scope 翻转** 会单独被 break detector 捕获。

**Sticky latch（故意不中途翻转）**

`bootstrap/state.ts` 注释：

- `promptCache1hEligible` 会话内冻结，避免 overage 中途切换 1h↔5m TTL bust cache
- `cacheEditingHeaderLatched`：cached microcompact 一旦开启，header sticky-on
- AFK/fast-mode 相关 beta 同理

### 2.4 Prompt Cache Break Detection

**源码**：`src/services/api/promptCacheBreakDetection.ts`

每轮快照字段（Diva 应对齐 **子集**）：

| 字段 | 用途 |
|------|------|
| `systemHash` | strip `cache_control` 后的 system 内容 hash |
| `cacheControlHash` | control 本身（TTL/scope） |
| `toolsHash` | 全部 tool schema |
| `perToolHashes` | 定位哪个工具 description 变了 |
| `toolNames` + added/removed | 集合增减 |
| `model` / `betas` / `effort` / `extraBodyHash` | 请求元数据 |
| `prevCacheReadTokens` | 与本轮 `cache_read` 比较 |
| `cacheDeletionsPending` | microcompact 合法读下降 |

判定：

- 内容 hash 变 → `pendingChanges` 记 reason  
- 随后 `cache_read` 下降 ≥ `MIN_CACHE_MISS_TOKENS`（2000）且非预期 deletion → **真 break 告警**  
- 可写 diff 到 temp 便于诊断

### 2.5 Microcompact 与缓存（次优先）

| 路径 | 文件 | 与 cache 关系 |
|------|------|----------------|
| 客户端坍缩旧 tool result | `microCompact.ts` | 改 history 中部/尾部前内容；需注意是否破坏已缓存前缀（通常只清 tool 体） |
| 服务端 cache_edits | `cachedMicrocompact.ts` | `delete_tool_result`；保留前缀 cache 同时删旧结果 |
| 阈值 | trigger=10, keepRecent=5 | 工具结果条数驱动 |

Diva 无 Anthropic cache_edits 产品依赖前：**先做布局稳定 + 客户端摘要**；cached MC 标 C3。

### 2.6 CC 消息逻辑布局（缓存友好）

```text
[tools array: sorted CORE built-ins | sorted MCP]  ← schema 锁 + 稳定序
[system sections: mostly session-cached]           ← 无每轮时钟
[messages: history grows at end]
  ± cache_control on selected trailing blocks
[volatile: attachments / deltas / tool results]    ← 不插回 system 中段
```

---

## 3. Agent-Diva 现状：已有能力 vs 必 bust 点

### 3.1 已有（不要重复发明）

| 能力 | 位置 |
|------|------|
| `apply_cache_control`：system → text block + ephemeral；**last tool** 打 cache_control | `agent-diva-providers/.../openai_compatible.rs` |
| `supports_prompt_caching` 按 registry | 同上 |
| Anthropic usage：`cache_read_input_tokens` / `cache_creation_input_tokens` | `anthropic.rs` |
| Frozen Core **会话 capture** | 已是 SessionStable 好材料 |
| L1 索引预算、skills progressive | 可进 SessionStable |
| Compaction boundary | 预期 break，需固定顺序 |

### 3.2 必 bust 清单（P0 修复对象）

| ID | 现象 | 源码锚点 | CC 对照 |
|----|------|----------|---------|
| **B1** | `## Current Time` 精确到分钟写进 system 中段 | `context.rs` `build_system_prompt_for_session` | session start date / 后缀 volatile |
| **B2** | WM `messages.insert(1, system)` | `turn/context.rs` `inject_working_memory` | 禁止插前缀后第二 system 当「伪前缀」 |
| **B3** | Prefetch 同样 `insert(1+wm)` | `turn/context.rs` | 同上，后缀区 |
| **B4** | plan_guard / approved_plan `insert(1)` | `PreparedTurnContext::prepare` | 贴当前任务，勿打断 stable |
| **B5** | channel/chat 拼进 **同一** system 字符串尾 | `build_messages_for_session` | 会话稳定可 SessionStable；否则 VolatileMeta |
| **B6** | L1 热刷新改 system 全文无 break 记录 | Wave 6 F4 | 允许变但 **reason 强制** |
| **B7** | tools definitions 顺序/集合无稳定契约 | `ToolAssembly` / `get_definitions` | 字典序 + built-in/MCP 分区 |
| **B8** | 无 system/tools hash 与 cache_read 关联观测 | — | `promptCacheBreakDetection` |
| **B9** | 多条 system 时 `apply_cache_control` 给 **每个** system 打 ephemeral | `apply_cache_control` 遍历所有 system | 仅稳定前缀 block 应作主 cache 锚；插队 system 会污染策略 |

**B9 细节**：当前实现对 `role=system` 的每条消息都加 `cache_control`。若 WM/prefetch 变成额外 system，则：

- 要么每轮重写多个 cached block（命中率差）
- 要么错误地把 volatile 标成可缓存前缀

C1 布局修复后应收敛为：**一条（或一组固定）stable system + 后缀 volatile 消息**；`apply_cache_control` 策略与布局契约对齐（见 P0-5）。

---

## 4. 目标布局（Diva C1 冻结）

### 4.1 消息顺序

```text
[0]  Message::system(STABLE_PREFIX)
       = Mask? + Identity + 静态工具叙述
       + Frozen Core
       + AGENTS.md + always skills + skills index
       + L0 policy + L1 index（会话缓存；热刷新记 break）
       // 禁止：Current Time、WM、prefetch、每轮 channel 抖动（若不稳定）

[1..] Compaction boundary + summaries（仅 compact 后变；预期 break）

[…]  History（user/assistant/tool）

[…]  Working Memory          // Turn/Session volatile — 独立消息，禁止 insert(1) 进前缀
[…]  Prefetch / Recall
[…]  VolatileMeta            // Current Time、channel、chat、scheduled note
[…]  Plan guard / approved plan（若需要）
[…]  Current user
```

上表是 **provider-neutral 逻辑顺序**，不是 wire role 的硬编码。WM、Recall、
VolatileMeta、Plan 默认不得直接假设为中途 `system`；serializer 必须按总论
DEC-CTX-A 选择 `MidConversationSystem` / `UserContextEnvelope` /
`NativeContextBlock`，未知 provider 使用安全的 user-context envelope。

### 4.2 Tools 数组顺序

```text
[ sorted built-in CORE... ][ sorted MCP / deferred-mounted... ]
         ↑ 连续前缀，会话内 schema 宜锁定
```

### 4.3 Section 语义（对齐 CC，最小类型）

```text
PromptSection {
  name: &'static str | String,
  stability: Stable | SessionStable | TurnVolatile,
  body: String,
  /// true ⇒ 本轮重算；变化必须带 reason
  recompute_every_turn: bool,
  break_reason: Option<String>,
}
```

| name（建议） | stability | recompute |
|--------------|-----------|-----------|
| mask | SessionStable | mask 切换时 |
| identity_rules | Stable / SessionStable | 否 |
| frozen_core | SessionStable | capture 一次 |
| agents_md | SessionStable | 文件变更或 session 开始 |
| skills | SessionStable | 否（目录变可 invalidate） |
| l0_policy | Stable | 否 |
| l1_index | SessionStable | CRUD 后可刷新 + reason |
| volatile_meta | TurnVolatile | 是（time 等） |
| working_memory | TurnVolatile | 内容变时 |
| prefetch | TurnVolatile | 是 |

### 4.4 会话快照与失效语义（冻结）

采用「会话快照优先、显式事件失效」：AGENTS.md 与 Skills 在 session start/reload
capture；mask 切换重建 mask section + tool pool；L1 apply 只刷新 L1 section 并记录
`l1_hot_refresh`；compact 只改变 post-prefix boundary/summary，**不清空 stable section
snapshot**；session end/reset 才清除全量 session cache。完整矩阵与例外规则见总论
DEC-CTX-C。

---

## 5. P0 实施切片（可直接开 PR）

### P0-1 移出 TurnVolatile（最高 ROI）

**实现状态（2026-08-10）：已完成。** `ContextBuilder` 已按 C1-0 typed sections 构建
唯一 stable system；Current Time/session、WM、Recall、Plan/Ask/Scheduled 进入统一动态
envelope。`LLMProvider::dynamic_context_transport` 默认 fail-safe 到
`UserContextEnvelope`，现有 Anthropic/OpenAI-compatible/Ollama adapter 均显式采用该
策略；`NativeContextBlock` 在通用消息路径 fail closed。reactive compaction 复用同一
turn 的动态 section 快照。T1–T3 与 provider wire-shape/compaction 回归已落地。

**改动文件（预期）**

- `agent-diva-agent/src/context.rs`：去掉 system 中段 Current Time；channel 策略明确
- `agent-diva-agent/src/agent_loop/turn/context.rs`：删除 `insert(1)` 注入；改为 append 到 history 之后、user 之前

**强制施工约束：** C1-0 已完成 `PromptSection` / `SectionStability` / 固定逻辑顺序；
P0-1 必须通过该类型层收集和序列化，并通过 provider capability 选择 wire role，禁止
只移动字符串或统一写成中途 `system`。

**验收**：T1–T3

### P0-2 Session section 缓存

- 按 `session_key` 缓存 Stable/SessionStable section 字符串
- clear/invalidate 严格遵守 §4.4 / DEC-CTX-C；compact 不清 stable snapshot

**验收**：同会话两轮 `system_hash` 在无刷新时不变；L1 刷新仅 `break_reason=l1_hot_refresh`

**实现状态（2026-08-10）：已完成。** `ContextBuilder` 按 session 缓存四个稳定
`PromptSection`、渲染结果和单调 `prefix_version`；mask/L1 revision 只重建对应 section，
reset/delete/shutdown 遵守 DEC-CTX-C，compact 不清 stable snapshot。T5–T6 已落地；
生产 `system_hash` 仍按原计划归 P0-4。当前 L1 revision 覆盖同一运行时 provider 写入；
Manager 外部治理 apply 与 Skills 管理入口的 runtime invalidation 接线已进入待办。

### P0-3 Tool 稳定排序 +（可选）schema 锁

**实现状态（2026-08-10）：已完成。** `ToolRegistry` 通过公开
`ToolSchemaPartition::{Core, Deferred}` 保存装配角色，`register()` 保持兼容并默认
CORE，MCP/custom 注册路径显式进入 DEFERRED；`get_definitions()` 固定输出 CORE
字典序连续前缀与 DEFERRED 字典序后缀，并递归规范化 JSON object 键序、保留 array
顺序。

- `get_definitions()` 输出前稳定排序
- 禁止 JSON object 键序导致字节漂（`serde_json` 注意 BTreeMap / 排序序列化）
- 本阶段不引入可选 `ToolSchemaCache`：现有内置与 MCP schema 在工具实例内为静态值；
  显式重注册/集合变化继续作为合法 schema 变化，后续由 C1d 记录 break

**验收**：T4

**原子边界：** P0-3/C1b 只修改排序/分区/schema 稳定性。必须先证明相同工具集合
连续重建的完整 schema 序列化字节一致；不得在同一提交修改 `apply_cache_control`。
本切片已通过 T4 证明该字节契约，且未修改 provider cache-control 代码。

### P0-4 Cache break 观测

**实现状态（2026-08-11）：已完成。** AgentLoop 真实 provider call 已接入 SHA-256
system/tools/per-tool hash、强类型 break reason、provider/model/policy/TTL 分桶和
cache read/create 后验样本；结构变化、策略变化、显式删除与连续 miss 分类遵守
DEC-CTX-G。窗口驻留进程内，不持久化 prompt/schema 正文。

新建（建议）：

`agent-diva-agent/src/context_assembly/cache_observe.rs`

```text
struct CacheObserveState {
  last_system_hash: u64,
  last_tools_hash: u64,
  last_cache_read: Option<u32>,
}

fn note_pre_call(system: &str, tools: &[Value], breaks: &[BreakReason])
fn note_post_call(usage: &TokenUsageMap)
```

- tracing：`prefix_hash`, `tools_hash`, per-tool hashes(debug), declared break reasons,
  provider/model/cache policy/TTL, `cache_read`, `cache_creation`, stable prefix tokens 与
  session+provider+model+policy 分桶的连续趋势
- declared reason 对应的结构变化记 `expected_break`，不告警；hash 无 reason 立即 warn
- hash/policy 稳定时，单次 `cache_read` 下降只记样本；达到缓存阈值且连续 2 次显著
  miss 才 warn `suspected_cache_miss`
- provider/model/policy/TTL 变化和显式 cache deletion 单独分类，不归因装配回归

**验收**：单元测试 hash 差分；集成测 usage 字段透传（T7）

### P0-5 与 `apply_cache_control` 契约对齐

**实现状态（2026-08-11）：已完成。** 采用 CORE 段末方案：`ToolDefinitionSet` 在过滤后
保留 CORE 边界，启用缓存时只标记 CORE 末项；第一条 stable system 的末文本块是唯一
system 锚点。OpenAI-compatible 与原生 Anthropic wire 均遵守该布局，普通 tools Vec
兼容地视为全 CORE，禁用 provider 不注入 control。T7–T8 与 JSONL ledger 兼容测试已落地。

**决策（冻结）**

P0-5/C1d 必须在 P0-3/C1b 独立提交及 T4 字节稳定证据之后实施，不得与工具排序
混为一个提交。

1. **主 cache 锚点**：仅 **第一条** stable system（或 stable 合并后的唯一 system）打 ephemeral；  
   或：多 system 时 **只有** `stability!=TurnVolatile` 的 system 打 control。
2. Tools：保持 last tool 打 control；排序稳定后 last 在 MCP 段末（可接受）或改为 **core 段末**（更贴 CC built-in 断点）— 实施时二选一写测试钉死。
3. 文档与单测：`apply_cache_control` 在「1 stable system + N volatile system」布局下行为符合预期。

**验收**：provider 单测扩展布局用例；agent 装配集成 sniffs 请求 JSON（wiremock 可选）

---

## 6. 测试清单（实施时必绿）

| ID | 场景 | 断言 |
|----|------|------|
| **T1** | 两轮装配，仅 wall-clock 前进 | stable system 字节 / `system_hash` **相同** |
| **T2** | 仅 WM checkpoint 更新 | stable hash 不变；volatile 段变 |
| **T3** | prefetch 空 → 非空 | stable hash 不变 |
| **T4** | 同工具集两次 `get_definitions` | 序列化字节一致；名序字典序 |
| **T5** | mask 切换 | system_hash 变；`break_reason` 含 mask |
| **T6** | L1 热刷新 | 仅 l1 section 变；reason=`l1_hot_refresh` |
| **T7** | Anthropic 兼容 mock 返回 cache_read | usage 透传到 ledger/日志 |
| **T8** | 多 system 布局 | 仅 stable system 带 cache_control（P0-5） |

---

## 7. 与 ADR-CTX-0..5 的关系

| ADR | 本专章 |
|-----|--------|
| ADR-CTX-0 Fragment | C1 可用 **PromptSection** 作最小实现；完整 Fragment 可同 PR 骨架 |
| **ADR-CTX-1** | **本文件 = 详细规格** |
| ADR-CTX-2 预算 | 淘汰不得重写 stable 前缀；先做 hash 稳定再分层 |
| ADR-CTX-3 tool ref | 减少 history 体积，间接利于 cache；非 C1 阻塞 |
| ADR-CTX-4 deferred tools | 减少 tools 块抖动；C1 后做 |
| ADR-CTX-5 验收 | T1–T8 并入 V5 cache 用例 |

---

## 8. 非目标

- 抄写完整 `promptCacheBreakDetection.ts`（字段子集即可）
- 依赖 Anthropic `cache_edits` API 才能交付 C1
- 把 Current Time 完全去掉且永不告诉模型时间（**可以**在 VolatileMeta 提供）
- 修改 Model-ID Safety / 非 Anthropic 缓存语义（DeepSeek 等 `supports_prompt_caching=false` 仍受益于 **稳定前缀省重算**，但不强求 cache_read）

---

## 9. 建议 PR 拆分

```text
PR-C1a  P0-1 typed layout + provider capability + T1–T3
PR-C1b  P0-3 tool 排序 + schema 字节稳定 T4       ← 不改 cache_control
PR-C1c  P0-2 section 缓存 + T5–T6
PR-C1d  P0-4 分类观测 + P0-5 apply_cache_control + T7–T8
```

PR-C1a 不依赖完整 C2 Fragment 预算框架，但必须消费 C1-0 最小类型骨架；它仍是
风险最低、缓存收益最大的生产迁移。

---

## 10. 源码索引

### Claude Code（`.workspace/claude-code`）

- `src/constants/systemPromptSections.ts`
- `src/constants/prompts.ts`（`getSystemPrompt`）
- `src/context.ts`（memoize system/user context；git 快照）
- `src/tools.ts`（`assembleToolPool`）
- `src/constants/tools.ts`（`CORE_TOOLS`）
- `src/utils/toolSchemaCache.ts`
- `src/services/api/claude.ts`（cache_control 断点）
- `src/services/api/promptCacheBreakDetection.ts`
- `src/services/compact/microCompact.ts`
- `src/services/compact/cachedMicrocompact.ts`
- `src/bootstrap/state.ts`（section cache + sticky latches）

### Agent-Diva

- `agent-diva-agent/src/context.rs` — system 拼装 / Current Time
- `agent-diva-agent/src/agent_loop/turn/context.rs` — `prepare_runtime_context` / insert(1)
- `agent-diva-agent/src/tool_assembly.rs` — 工具注册
- `agent-diva-providers/src/openai_compatible.rs` — `apply_cache_control`
- `agent-diva-providers/src/anthropic.rs` — cache usage 字段

---

## 11. 结论

1. **Claude Code 的缓存优势 = 纪律化前缀 + 显式 volatile + 工具序 + 观测**，不是单独某个 API。  
2. Diva **差在装配**，不差在「会不会写 cache_control」。  
3. **最先做 P0-1**：移走 Current Time 与 WM/prefetch 插队，通常即可让 system 前缀在稳态轮次保持字节级稳定。  
4. 随后 tool 排序、section 缓存、break 观测、与 `apply_cache_control` 契约对齐，形成可回归的缓存工程能力。  
5. 本文件即 **CTX-C1 的实施规格**；总论 C0 中的 ADR-CTX-1 以此为准细化。
