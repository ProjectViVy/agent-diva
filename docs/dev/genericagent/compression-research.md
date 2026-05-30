# Autodream 前置压缩技术调研

> 状态：调研与方案文档。本文只固化结论，不实现代码。
> 调研对象：`agent-diva-agent/src/consolidation.rs`、`agent-diva-core/src/memory/provider.rs`、`agent-diva-core/src/session/`、`.workspace/claude-code/src/services/compact/`、`.workspace/claude-code/src/services/autoDream/`、`.workspace/memtle/src/`、`docs/dev/genericagent/newedge/architecture.md`、`docs/dev/genericagent/newedge/ui-design.md`、`docs/dev/genericagent/autodream-migration-research.md`。
> 核心原则：压缩是 autodream 的材料准备层，不是长期记忆写入层；它必须小、可追溯、可失败、可重跑。

## 1. 结论摘要

Agent-Diva 当前的 `consolidation.rs` 是"会话段总结器"，不是"可审计压缩器"。它在会话消息超过 100 条时触发，截取旧消息发送给 LLM，直接通过 `MemoryProvider::sync_turn()` 写入 MEMORY.md 和 HISTORY.md，没有结构化产物、没有证据引用、没有候选审查、没有重跑能力。

要支撑 autodream/rhythm，需要一个前置压缩层：将原始 session/history/evidence 压成小而可追溯的 **Source Capsule**，供 autodream 做节律审查、候选生成和 Journal 输出。

关键设计决策：

- **压缩产物 = Source Capsule**，存放于 `.agent-diva/compact/capsules/*.json`，事件流追加到 `.agent-diva/compact/events.jsonl`。
- **压缩不直接改写 L2/L3/SOUL/SOP**。压缩只产出事实摘要和候选引用，写入长期记忆需要经过 `LearningCandidate` → 用户/策略确认。
- **原始 evidence 保留不动**。压缩只是建立索引和摘要层，不删除原始 session。
- **最小 MVP 只做三件事**：session-segment compact → source capsule 写入 → 事件记录。后续再加 autodream 前置扫描和 capsule 合并。

## 2. 当前 Agent-Diva 压缩与 consolidation 现状

### 2.1 触发机制

`agent-diva-agent/src/consolidation.rs`:

```rust
pub const DEFAULT_MEMORY_WINDOW: usize = 100;

pub fn should_consolidate(session: &Session, memory_window: usize) -> bool {
    let consolidated = session.last_consolidated.min(session.messages.len());
    let unconsolidated = session.messages.len() - consolidated;
    unconsolidated >= memory_window
}
```

触发条件：未压缩消息数 ≥ `memory_window`（默认 100）。`last_consolidated` 是 `Session` 结构体上的 usize 指针，标记已压缩到哪条消息。

### 2.2 压缩逻辑

`consolidate()` 函数的流程：

1. 取未压缩消息的前半段（保留最近 `memory_window/2` 条作为上下文重叠）。
2. 截取每条消息前 500 个字符，拼成对话文本。
3. 通过 `MemoryProvider::system_prompt_block()` 读取现有 MEMORY.md。
4. 发送给 LLM，使用 `save_memory` 工具 schema，要求输出 `memory_update`（Markdown 长期记忆）和 `history_entry`（一行摘要）。
5. 通过 `MemoryProvider::sync_turn()` 直接写入 MEMORY.md 和 HISTORY.md。
6. 无论 LLM 是否返回预期工具调用，都推进 `last_consolidated` 指针。

### 2.3 调用位置

`agent-diva-agent/src/agent_loop/loop_turn.rs` 中 `process_inbound_message_inner()` 的末尾：

```text
save_turn(session, ...)
  -> if should_consolidate(session, memory_window)
     -> consolidate(session, ...) // 可能失败，只记录 error
  -> sessions.save(session)       // 始终保存
```

consolidation 在 turn 结束后同步运行，失败只记 error 不阻断。

### 2.4 差距分析

| 维度 | 当前状态 | 可审计压缩要求 |
|---|---|---|
| 产物结构 | 无结构化产物，直接写 MEMORY.md | Source Capsule JSON，含 summary/key_facts/decisions/open_threads/evidence_refs |
| 证据引用 | 无。原始消息被截断到 500 字符后丢失上下文 | 保留 source_session_ids、turn indices、文件路径、excerpt_hash |
| 候选审查 | 直接写入长期记忆，无审查 | 生成候选，由 LearningCandidate/用户确认后再写 |
| 可重跑 | 不可重跑。指针推进后无法回溯 | capsule 生成后可被 autodream 多次消费，不影响原始 session |
| 失败恢复 | LLM 失败也推进指针 | 失败不写 capsule，不推进 checkpoint |
| 输入源 | 只看 session messages | 需要综合 session messages、tool results、plan evidence、history、rhythm records |
| token 管理 | 每条消息截断 500 字符，丢失上下文 | 按 token 预算分层：关键消息保留更多、低价值消息摘要压缩 |

### 2.5 Session 存储结构

`agent-diva-core/src/session/store.rs`:

```rust
pub struct Session {
    pub key: String,              // "channel:chat_id"
    pub messages: Vec<ChatMessage>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub metadata: serde_json::Value,
    pub last_consolidated: usize, // 压缩进度指针
}
```

`SessionManager` 按 key 管理多个 session，支持 `get_or_create`、`save`、`archive_and_reset`。Session 按 key 序列化为 JSON 文件存储在 sessions 目录。

**关键约束**：`last_consolidated` 是线性指针，不区分压缩粒度。未来如果引入 capsule 压缩，需要一个独立的 checkpoint 机制，不能复用这个指针。

## 3. Claude Code compact / autoDream 可借鉴点

### 3.1 Claude Code Compact 机制

`.workspace/claude-code/src/services/compact/` 实现了多层压缩：

- **base compact**：全量会话压缩，使用 `<analysis>` + `<summary>` 结构化 prompt。模型先写分析草稿（被 strip），再输出最终摘要。
- **partial compact**：只压缩选定消息之前的旧消息，保留最近消息。支持 `from` 和 `up_to` 两种方向。
- **micro compact**：更轻量的段级压缩。
- **snip compact**：对特定消息做定点裁剪。
- **auto compact**：token 预算触发的自动压缩。
- **session memory compact**：跨 session 的记忆压缩。
- **reactive compact**：响应式触发。

关键设计：

- **boundary marker**：压缩后插入边界标记，区分已压缩摘要和保留的原始消息。
- **post-compact restore**：压缩后恢复最近读取的文件、活跃 skill、plan 状态、异步 agent 信息。
- **structured prompt**：prompt 要求模型输出特定 section（Background、Accomplished、Relevant files、Open questions、Next steps 等），不是自由文本。

### 3.2 Claude Code autoDream 机制

`.workspace/claude-code/src/services/autoDream/autoDream.ts`:

- **触发**：每轮结束后 opportunistic 检查。Time gate（24h）+ session gate（5 sessions）+ lock gate。
- **锁**：`memory/.consolidate-lock`，文件内容是 PID，mtime 是 lastConsolidatedAt。跨进程、低成本、stale 可恢复。
- **执行**：forked subagent，`querySource = "auto_dream"`，`skipTranscript = true`，限制工具权限。
- **四阶段 prompt**：Orient → Gather recent signal → Consolidate → Prune and index。
- **手动入口**：`/dream` 在主循环运行，拥有完整工具权限。

### 3.3 可借鉴 vs 不照搬

| 可借鉴 | 不照搬 |
|---|---|
| `<analysis>` + `<summary>` 结构化 prompt 模式 | Claude Code 的 `stopHooks`、Ink REPL、task store 体系 |
| boundary marker 区分压缩段和保留段 | GrowthBook、KAIROS feature flag 机制 |
| lock 文件做跨进程并发控制 | `/dream` 的乐观更新时间戳（应成功后再更新 checkpoint）|
| forked subagent + 工具权限限制 | 具体工具名、Ink task UI |
| 四阶段 consolidation prompt | 单一 memory 目录语义（Diva 已有 L0-L4 分层）|
| auto/manual 触发分离 | 把 Plan Mode 运行状态写进 .laputa/ |

## 4. 压缩在 DivaGeneric / NewEdge 中的位置

### 4.1 在 L0-L4 中的位置

压缩产物（Source Capsule）位于 L4（证据层）和 L2（稳定事实层）之间的中间层：

```text
L0  学习公理 → 压缩遵守 L0 规则（不直接写 L2/L3）
L1  极小索引 → 压缩结果可被 index 引用
L2  稳定事实 → 压缩不直接写，只生成候选
L3  SOP/Skill → 压缩不直接写，只生成候选
L4  原始证据 → session/history/evidence 是压缩的输入
Capsule  压缩中间产物 → 供 autodream 消费，不注入日常 prompt
Inbox  候选层 → 压缩可生成 LearningCandidate 进入 inbox
```

### 4.2 在在线/离线路径中的位置

压缩属于离线路径，但可以有两个触发点：

- **在线路径末尾**（turn-end threshold）：轻量级 session-segment compact，生成小型 capsule。
- **离线路径入口**（session-end / autodream 前置）：跨 session capsule 合并、大型压缩、evidence 聚合。

```text
在线路径：
  Inbound message → LLM → tool calls → response
    → session save
    → [optional] turn-end compact → source capsule

离线路径：
  source capsules + session history + evidence
    → autodream worker
    → daily/weekly/monthly rhythm
    → LearningCandidate
    → user/policy decision
    → L2/L3/Laputa delta
```

### 4.3 在 provider composition 中的位置

压缩不能绕过 `MemoryProvider`。它应该：

- **读取**现有记忆：通过 `MemoryProvider::system_prompt_block()` 获取当前 MEMORY.md。
- **写入**压缩产物：写到 `.agent-diva/compact/` 目录（不在 MemoryProvider 边界内，因为这是中间产物，不是长期记忆）。
- **触发候选写入**：通过 `MemoryProvider::sync_turn()` 或未来的 `GenericCore::propose_learning_candidate()` 生成候选。

```text
CompressionWorker
  -> 读: session store, history, plan evidence, MemoryProvider::system_prompt_block()
  -> 压缩: LLM structured extraction
  -> 写: .agent-diva/compact/capsules/*.json
  -> 写: .agent-diva/compact/events.jsonl
  -> [可选] 生成 LearningCandidate -> .laputa/inbox/learning-candidates.jsonl
  -> 不写: MEMORY.md, HISTORY.md, SOUL.md, sop/*.md (除非经确认)
```

## 5. Source Capsule 数据模型建议

### 5.1 Capsule Schema 草案

```json
{
  "capsule_id": "cap-20260530-a1b2c3",
  "capsule_type": "session_segment",
  "created_at": "2026-05-30T10:30:00Z",
  "source_session_ids": ["slack:C12345", "discord:general"],
  "source_turn_range": {
    "start": 0,
    "end": 45
  },
  "summary": "用户要求调研 Provider 模型 ID 安全问题。发现了 LiteLLM prefix 自动重写的 bug，修复了 provider routing 逻辑，添加了测试。",
  "key_facts": [
    "DeepSeek 原生端点不应添加 LiteLLM prefix",
    "HybridMemoryProvider 已支持 Markdown 权威回退",
    "MentleToolRuntimeConfig 支持 off/read_only/full/custom 四种模式"
  ],
  "decisions": [
    {
      "content": "native endpoint 使用 raw model ID，gateway 使用 provider/model prefix",
      "evidence_refs": [
        {
          "source": "session",
          "session_id": "slack:C12345",
          "turn_index": 12,
          "excerpt_hash": "sha256:abc123"
        }
      ]
    }
  ],
  "open_threads": [
    "Mentle full profile 在后台 worker 中的工具隔离还未实现"
  ],
  "candidate_lessons": [
    {
      "content": "修改 provider routing 时必须检查 model ID 格式",
      "suggested_layer": "L3SopOrSkill",
      "evidence_refs": [...]
    }
  ],
  "evidence_refs": [
    {
      "source": "session",
      "session_id": "slack:C12345",
      "turn_range": [10, 25],
      "excerpt": "修复前的错误行为...",
      "excerpt_hash": "sha256:def456"
    },
    {
      "source": "plan_evidence",
      "plan_id": "plan-provider-fix",
      "step_id": "step-001",
      "path": ".agent-diva/plans/plan-provider-fix/evidence/step-001.md"
    },
    {
      "source": "file",
      "path": "agent-diva-providers/src/lib.rs",
      "line_range": [42, 68]
    }
  ],
  "compression_prompt_version": "v1",
  "token_budget_used": 2048,
  "raw_message_count": 45,
  "compressed_from_chars": 125000,
  "compressed_to_chars": 1800
}
```

### 5.2 字段说明

| 字段 | 必需 | 说明 |
|---|---|---|
| `capsule_id` | 是 | 唯一标识，格式 `cap-YYYYMMDD-<short_hash>` |
| `capsule_type` | 是 | `session_segment` / `cross_session` / `plan_completion` / `manual` |
| `created_at` | 是 | ISO 8601 时间戳 |
| `source_session_ids` | 是 | 产生压缩输入的 session key 列表 |
| `source_turn_range` | 否 | 单 session 时的 turn 范围 |
| `summary` | 是 | 结构化摘要，2-5 句话 |
| `key_facts` | 是 | 本次会话中确立的事实列表 |
| `decisions` | 是 | 本次会话中做出的决策，每个含 evidence_refs |
| `open_threads` | 否 | 未完成的讨论或任务 |
| `candidate_lessons` | 否 | 可升级为 SOP/Skill 的候选经验 |
| `evidence_refs` | 是 | 指向原始 evidence 的引用列表 |
| `compression_prompt_version` | 是 | prompt 版本，用于重跑兼容 |
| `token_budget_used` | 否 | 压缩消耗的 token 数 |
| `raw_message_count` | 是 | 原始消息数 |
| `compressed_from_chars` | 否 | 原始字符数 |
| `compressed_to_chars` | 否 | 压缩后字符数 |

### 5.3 事件流 Schema

`.agent-diva/compact/events.jsonl` 记录所有压缩相关事件：

```json
{"event": "capsule_created", "capsule_id": "cap-20260530-a1b2c3", "session_id": "slack:C12345", "timestamp": "2026-05-30T10:30:00Z", "trigger": "turn_end_threshold"}
{"event": "capsule_consumed", "capsule_id": "cap-20260530-a1b2c3", "consumer": "autodream_daily", "timestamp": "2026-05-30T22:00:00Z"}
{"event": "candidate_generated", "capsule_id": "cap-20260530-a1b2c3", "candidate_id": "cand-xyz", "timestamp": "2026-05-30T22:01:00Z"}
{"event": "checkpoint_advanced", "session_id": "slack:C12345", "old_pointer": 0, "new_pointer": 45, "timestamp": "2026-05-30T10:30:00Z"}
```

### 5.4 压缩存储目录

```text
.agent-diva/
  compact/
    capsules/
      cap-20260530-a1b2c3.json
      cap-20260530-d4e5f6.json
      ...
    events.jsonl
    checkpoint.json
    lock
```

- `capsules/`：每个 capsule 一个 JSON 文件。
- `events.jsonl`：追加写入的事件流。
- `checkpoint.json`：每个 session 的压缩进度（替代 Session.last_consolidated 做更细粒度的跟踪）。
- `lock`：跨进程锁，借鉴 Claude Code 的 PID + mtime 模式。

### 5.5 Checkpoint 设计

```json
{
  "sessions": {
    "slack:C12345": {
      "last_compacted_turn": 45,
      "last_compacted_at": "2026-05-30T10:30:00Z",
      "last_capsule_id": "cap-20260530-a1b2c3"
    },
    "discord:general": {
      "last_compacted_turn": 12,
      "last_compacted_at": "2026-05-30T09:15:00Z",
      "last_capsule_id": "cap-20260530-d4e5f6"
    }
  }
}
```

## 6. 触发策略与 checkpoint 设计

### 6.1 四种触发点

| 触发点 | 时机 | 压缩粒度 | 是否阻塞 |
|---|---|---|---|
| turn-end threshold | 未压缩消息 ≥ 阈值（默认 50） | session segment | 不阻塞（async 或 fire-and-forget） |
| session-end | `MemoryProvider::on_session_end()` | session tail | 不阻塞主 session |
| manual `/compact` | 用户手动触发 | 当前 session 全量 | 阻塞等待结果 |
| autodream 前置 | autodream worker 启动前 | 跨 session 聚合 | 阻塞 autodream，不阻塞主对话 |

### 6.2 触发阈值

```toml
[compression]
enabled = true
turn_end_threshold = 50          # 未压缩消息达到 50 条时触发
token_budget_per_capsule = 4096  # 单次压缩最大 token
max_chars_per_message = 1000     # 每条消息截取上限（当前 consolidation 是 500）
keep_recent = 25                 # 保留最近 N 条作为上下文重叠
auto_trigger = true              # turn-end 自动触发
session_end_trigger = true       # session-end 自动触发
```

### 6.3 Checkpoint 规则

- checkpoint 在 capsule 写入成功后才更新。**不在 LLM 调用前推进**。
- checkpoint 独立于 `Session.last_consolidated`。两者可以并存，直到 consolidation.rs 被完全替换。
- checkpoint 失败不影响 session save。
- 重跑安全：如果 capsule 文件已存在且 checkpoint 已推进，跳过重复压缩。

### 6.4 Lock 机制

借鉴 Claude Code 的 `.consolidate-lock`：

```text
.agent-diva/compact/lock
  内容: PID
  mtime: 上次压缩完成时间
```

- 获取锁：检查 PID 是否存活。存活则拒绝。不存活则覆盖。
- 释放锁：删除文件或更新 mtime。
- stale 恢复：PID 不存在 + mtime 超过 30 分钟 → 自动恢复。

## 7. 与 MemoryProvider / autodream / Journal / Mentle 的边界

### 7.1 压缩与 MemoryProvider 的边界

```text
允许:
  - 读: MemoryProvider::system_prompt_block() 获取现有记忆上下文
  - 写: .agent-diva/compact/ 目录（capsule、events、checkpoint）
  - 生成: LearningCandidate 提交到 .laputa/inbox/

禁止:
  - 直接改写 MEMORY.md、HISTORY.md
  - 直接改写 .laputa/SOUL.md、relationships.md、sop/*.md
  - 绕过 MemoryProvider 调用 Mentle 写入
  - 在压缩过程中执行 tool calls（压缩只做文本提取，不做代码执行）
```

### 7.2 压缩与 autodream 的边界

| 职责 | 压缩 | autodream |
|---|---|---|
| 输入 | session messages, tool results | source capsules, session history, rhythm records |
| 输出 | Source Capsule JSON | daily/weekly/monthly rhythm, LearningCandidate, Journal entry |
| 写入目标 | `.agent-diva/compact/` | `.laputa/rhythm/`, `.laputa/inbox/`, `.laputa/index.md` |
| 失败影响 | 只影响后续 autodream 的输入质量 | 影响 Journal 和长期记忆候选 |
| 触发方式 | turn-end threshold / session-end / manual | rhythm schedule / user request / Journal re-wakeup |
| 工具权限 | 无（纯 LLM 文本提取） | 可用 full Mentle 工具集 |

**关系**：压缩是 autodream 的材料准备层。autodream 消费 capsule 而不是原始 session，因为：
1. capsule 比原始 session 小得多，降低 autodream 的 token 成本。
2. capsule 包含结构化的 key_facts/decisions/evidence_refs，autodream 可以直接使用。
3. capsule 生成是可失败、可重跑的，不影响 autodream 的幂等性。

### 7.3 压缩与 Journal 的边界

压缩不直接产生 Journal entry。但压缩产物是 Journal 的间接来源：

```text
compression → capsule
  → autodream 消费 capsule
    → daily rhythm
      → .laputa/rhythm/daily/YYYY-MM-DD.md
        → JournalView 展示
```

Journal 约束（来自 ui-design.md）：
- Journal entry 不应被静默改写。
- 修正应形成 follow-up 或 amendment record。
- 压缩产物需要能在 Journal 中审计（通过 evidence_refs 追溯到原始 session）。

### 7.4 压缩与 Mentle 的边界

Mentle 是可选的深层事实/证据/索引后端。压缩与 Mentle 的关系：

- **MVP 阶段**：压缩不使用 Mentle。capsule 纯文件态存储。
- **后续阶段**：capsule 可以通过 Mentle diary API 写入 evidence drawer，建立全文索引。
- **不做**：压缩过程中调用 Mentle search 做跨 session 去重（这是 autodream 的职责）。

### 7.5 压缩与 LearningCandidate 的边界

压缩可以生成 `candidate_lessons`，但这些不是自动写入的：

```text
compression
  -> candidate_lessons 字段
    -> [可选] 转为正式 LearningCandidate
      -> .laputa/inbox/learning-candidates.jsonl
        -> 用户确认 / policy 自动决策
          -> 写入 L2/L3/Laputa
```

哪些写入必须经过确认：

| 内容 | 确认要求 |
|---|---|
| L2 稳定事实 | 必须确认（用户关系、项目状态） |
| L3 SOP/Skill | 必须确认（已验证行为才能升级） |
| Laputa persona delta | 必须确认 |
| L4 证据归档 | 不需要确认（自动归档） |
| L1 索引更新 | 不需要确认（自动更新指针） |
| HISTORY.md 摘要行 | 不需要确认（自动追加） |

## 8. MVP 实施建议

### 8.1 MVP：先做什么

**Phase 1：Source Capsule 生成**（最小可行版本）

1. **新增 `agent-diva-agent/src/compression/` 模块**：
   - `mod.rs`：模块入口。
   - `capsule.rs`：`SourceCapsule` 数据结构定义。
   - `compressor.rs`：压缩逻辑（LLM 调用 + 结构化输出解析）。
   - `checkpoint.rs`：checkpoint 读写。
   - `lock.rs`：跨进程锁。

2. **定义 `SourceCapsule` 结构体**：
   - 包含 §5.1 中所有必需字段。
   - `Serialize + Deserialize`。
   - 独立测试序列化/反序列化。

3. **实现 session-segment compact**：
   - 从 session messages 中提取未压缩段。
   - 截取每条消息到 `max_chars_per_message`（默认 1000）。
   - 发送给 LLM，使用结构化 prompt（借鉴 Claude Code 的 section-based 输出）。
   - 解析 LLM 输出为 `SourceCapsule`。
   - 写入 `.agent-diva/compact/capsules/`。
   - 更新 checkpoint。

4. **接入 turn-end lifecycle**：
   - 在 `loop_turn.rs` 中，turn 结束后检查压缩阈值。
   - 与现有 `consolidation::should_consolidate` 并存（可配置使用哪个）。
   - 失败只记 warn，不阻断。

5. **写入 events.jsonl**：
   - 每次 capsule 创建、消费、checkpoint 更新都追加事件。

**Phase 1 验收标准**：
- session 达到 50 条消息时自动触发压缩。
- 生成一个结构化 JSON capsule，包含 summary、key_facts、decisions、evidence_refs。
- capsule 写入成功后 checkpoint 更新。
- 压缩失败不影响 session save 和主对话。
- 有单元测试覆盖 capsule 序列化、checkpoint 读写、lock 竞争。

### 8.2 Phase 2：Autodream 前置消费

1. Autodream worker 启动时扫描 `.agent-diva/compact/capsules/` 中未消费的 capsule。
2. 合并多个 capsule 为跨 session 的压缩输入。
3. 生成 daily rhythm 时使用 capsule 的 summary/key_facts/decisions。
4. 消费后标记 capsule（在 events.jsonl 中记录 `capsule_consumed`）。

### 8.3 Phase 3：压缩与 consolidation 替换

1. 将 `consolidation.rs` 的 `sync_turn()` 直写逻辑替换为 capsule 生成。
2. 保留 `last_consolidated` 指针作为兼容层，实际使用新的 checkpoint。
3. 引入 `GenericCore::propose_learning_candidate()` 接口。

### 8.4 Phase 4：跨 session 合并与 Mentle 索引

1. session-end 时合并当前 session 的所有 capsule。
2. 可选地通过 Mentle diary API 建立全文索引。
3. 实现 capsule 过期清理策略（保留 30 天，已消费的可压缩归档）。

### 8.5 先不要做的事

| 不做 | 原因 |
|---|---|
| 直接改写 MEMORY.md / SOUL.md | 违反"L4 证据层不直接改 L2"原则 |
| 压缩中调用 Mentle search | 是 autodream 的职责，不是压缩的 |
| 跨 session 去重 | 需要 autodream 全局视角，不是前置压缩的 |
| 压缩 plan evidence | plan 有自己的 evidence 目录，不需要复制 |
| GUI 可视化 | 先做 CLI/日志可观测，GUI 后续 |
| capsule 合并/压缩 | 先一个 session 一个 capsule，后续再优化 |
| 自动升级 SOP/Skill | 必须经过 LearningCandidate 确认 |

### 8.6 压缩 Prompt 设计建议

借鉴 Claude Code 的结构化输出模式，但适配 Diva 的 capsule schema：

```text
You are a session compressor for Agent-Diva. Analyze the conversation segment below
and produce a structured compression.

You MUST respond with a JSON object matching this schema:
{
  "summary": "2-5 sentence overview of what happened",
  "key_facts": ["fact 1", "fact 2", ...],
  "decisions": [
    {"content": "what was decided", "context": "why"}
  ],
  "open_threads": ["unfinished item 1", ...],
  "candidate_lessons": [
    {"content": "reusable lesson", "suggested_layer": "L3SopOrSkill|L2Fact"}
  ],
  "evidence_refs": [
    {"type": "turn", "turn_index": N, "role": "user|assistant", "excerpt": "key quote"}
  ]
}

Guidelines:
- summary should be factual, not evaluative
- key_facts are things established as true during this conversation
- decisions must include the reasoning, not just the outcome
- candidate_lessons are patterns that could be reused, not one-off facts
- evidence_refs point to specific turns that support the extracted content
- Do NOT hallucinate content that isn't in the conversation
```

### 8.7 风险与测试

| 风险 | 缓解 |
|---|---|
| LLM 输出非 JSON | 解析失败时 fallback 到纯文本 summary capsule，标记 `parse_failed: true` |
| 压缩 LLM 调用太慢 | 设置 timeout（默认 30s），超时跳过本轮 |
| capsule 文件膨胀 | 定期清理已消费且超过 30 天的 capsule |
| checkpoint 和 last_consolidated 不一致 | Phase 1 并存，Phase 3 统一 |
| lock stale 导致无法压缩 | PID 检查 + 30 分钟超时自动恢复 |
| 压缩消耗过多 token | `token_budget_per_capsule` 限制 + 消息截断 |

**最小测试集**：
- `SourceCapsule` JSON 序列化/反序列化往返测试。
- `Checkpoint` 读写 + 并发安全测试。
- `Lock` 获取/释放/stale 恢复测试。
- 压缩 prompt 解析：正常 JSON、非 JSON fallback、空会话、超长会话。
- turn-end 阈值触发：低于阈值不触发、达到阈值触发、失败不影响 session save。
