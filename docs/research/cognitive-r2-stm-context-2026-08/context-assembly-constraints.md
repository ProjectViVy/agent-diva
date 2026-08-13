# R2 上下文装配约束

- 状态：`Research Draft / Source-backed`
- 日期：2026-08-13
- 性质：当前实现事实与对未来 STM 的**插入约束**；不定装配方案
- 依赖：R0 状态图 §6；C5 运行时合同；本包不重写 R0 全景

## 1. 当前对象图（源码事实）

```text
workspace
├── sessions/{safe_key}.jsonl          Session transcript + CanonicalCheckpoint
├── .laputa/memory.sqlite3             BML LTM + SessionCheckpoint (WorkingMemory)
├── .agent-diva/data/planning.sqlite3  durable Plan（workspace 单例 active_plan）
└── {config}/data/cron/jobs.json       cron 作业

进程内
├── ContextBuilder SessionStableCache  按 session_key 分桶的 prefix 快照
├── Frozen Core SESSION_SNAPSHOTS      按 (workspace, session) 捕获一次
├── EphemeralPlanRegistry              按 session_key；重启丢失
└── CronService.active_runs            内存；进程退出即丢
```

产品 STM 不在上图任何一层。实验观察：全仓 `struct Stm` / `enum Stm` / `fn stm_` /
跨会话 STM API 在 `.rs` `.vue` `.ts` 中 **零匹配**。

### 1.1 寿命对照

| 对象 | 键 | 持久化 | 谁写 | 谁清 | 进 Prompt 的位置 |
| --- | --- | --- | --- | --- | --- |
| CanonicalCheckpoint | `Session.key` | JSONL metadata 行 | `CheckpointCompactor` | `Session::clear` / reset / delete 文件 | 第二条 `system` |
| SessionCheckpoint | `working-checkpoint-{sha16(session_id)}` | BML 行，`session_id=Some` | `update_working_checkpoint` | `on_session_end` 物理 DELETE；Reset/Delete **不**清 | TurnVolatile `WorkingMemory` |
| BmlStartupIndex | workspace | BML `session_id IS NULL` 的 Applied 行投影 | `memory_add` / typed apply | 不因 session 结束删除 | SessionStable prefix §3 |
| Plan draft | `session_key` | 进程内 registry | Plan mode finalize | 重启丢失 | `PlanGuard` |
| Plan durable | workspace `active_plan` 单例 | `planning.sqlite3` | sync after create / approve | `delete_expired_plans`（30 天） | 批准后 `approved_plan` 动态段 |
| 产品 STM | — | **无** | — | — | — |

R0 已登记的未声明重叠第 3 条（session `WorkingMemory` vs 产品 STM vs
`canonical_checkpoint_v1`）在此拆开，不再混名。

## 2. C1–C5 固定装配

来源：`agent-diva-agent/src/context_assembly/mod.rs` `CONTEXT_SECTION_ORDER`。

```text
[SessionStable prefix — 一条 system，\n\n join]
  0 MaskAndIdentity
  1 FrozenCore                         进程内按 session 捕获；空则 first-run 块
  2 AgentRulesAndSkills
  3 MemoryPolicyAndIndex               L0 三句政策 + Typed L1 索引

[独立第二条 system，不是 PromptSection]
     CanonicalCheckpoint               ## Canonical Checkpoint v1 … [end]

[history 消息数组]
     user / assistant / tool           floor = max(last_consolidated, durable_message_index)

[TurnVolatile 动态信封，默认一条 user]
  6 WorkingMemory                      有块才注入
  7 PrefetchRecall                     有意图关键词才召回
  8 VolatileMeta                       channel/chat；cron 再加 scheduled_turn
  9 PlanGuard                          plan_mode / approved_plan / ask_mode
 10 CurrentUser                        实际是最后一条 Message，几乎不当 section
```

`SectionStability::Stable` 已声明，**没有任何生产 section 使用**（源码事实）。
`ContextSection::Compaction` / `History` / `CurrentUser` 在 enum 里是动态段，
生产 wire 并不把它们当 `PromptSection` 发送（源码事实）。

生产入口：`prepare_runtime_context`
（`agent-diva-agent/src/agent_loop/turn/context.rs`）。

### 2.1 三顶层预算区

`ContextBudgetRegion`（`context_assembly/budget.rs`）：

| 区 | 默认硬限 | 装什么 |
| --- | --- | --- |
| `StablePrefix` | `system_budget_ratio` | mask + Frozen Core + skills + L1 + CORE tools |
| `CanonicalCheckpoint` | `0.10 * max_tokens` | 单一检查点体（另有 8000 字硬截断） |
| `ActiveTail` | 剩余 / 总上限 | history + 动态信封 + 当前 turn |

`BudgetLayer` 是计量桶，不是装配序。与 STM 相关的层：

| 层 | soft / hard | eviction |
| --- | --- | --- |
| `L1Index` | 4% / 6% | Compact（函数里 Compact **不真缩小**，只是不 Drop） |
| `WorkingMemory` | 3% / 4% | **Never** |
| `PrefetchRecall` | 3% / 4% | **Drop 优先** |
| `CompactionSummaries` | 8% / 10% | Compact |

压力下 `select_dynamic_sections` **先丢 Recall**。WM / PlanGuard / prefix 段 Never。

### 2.2 SessionStable cache

每个 `session_key` 一份四段 prefix 快照（`context.rs` + `section_cache.rs`）。
一个 `AgentLoop` 共用一个 `ContextBuilder`，按 session 分桶。

会打穿 cache：

| `CacheBreakReason` | 重建 |
| --- | --- |
| `MaskChanged` | MaskAndIdentity |
| `L1HotRefresh` | MemoryPolicyAndIndex（`system_prompt_revision` 变） |
| `AgentRulesReload` / `SkillsReload` | AgentRulesAndSkills |
| `SessionReset` | 四个 prefix |
| `SessionEnded` | 整桶删除 |

**不会**打穿：时间、channel、SessionCheckpoint、Recall、Plan/Ask/Scheduled、
history、CanonicalCheckpoint、current user。测试 `c1a_t1/t2/t3` 钉死（源码事实）。

Frozen Core 另有进程级 `SESSION_SNAPSHOTS`，与 section cache 独立。中途改人格文件，
两者都不变直到下次 capture / reset。

## 3. CanonicalCheckpoint

`agent-diva-core/src/session/store.rs`：

- schema `canonical_checkpoint_v1`；每 session 最多一份；体 ≤ 8000 Unicode scalar
- 触发 `Auto` / `Manual` / `Reactive` 只作遥测，不改压缩语义
- 更新规则：旧 body + 本次被裁剪消息 → 新检查点，**原子替换**，不追加
- 注入：`CANONICAL_CHECKPOINT_HINT` + `## Canonical Checkpoint v1\n{body}\n[canonical checkpoint end]`
- 压缩 prompt 固定节（`compaction/prompt.rs`）：目标与约束 / 已完成 / 关键决定 /
  当前状态 / 未解决问题 / 下一步 / 标识符与 artifact

入口唯一：`CheckpointCompactor::compact_snapshot`。

| 触发 | 输入快照 | 提交时机 |
| --- | --- | --- |
| Auto | 已持久 session（不含当前 turn）；history+checkpoint 约 80% 阈值 | 立刻写 session |
| Reactive | durable + 当前 turn 内存消息 | `pending_checkpoint_updates`，`finalize` 才耐久；新 turn 丢掉未提交 |
| Manual | Plan 执行 `ExecutionContextPolicy::Compact` 的 boundary 前历史 | 立刻；checkpoint_id = `execution:{id}` |

C5 文档写「先机械压缩，仍超硬预算才语义压缩」。代码上 Auto 由 80% 阈值**直接**调
LLM；机械折叠只发生在送给压缩模型的输入和 microcompact 工具结果。这是文档 vs
实现裂缝（推断，对照 `c5-lightweight-context-convergence.md` 与 `turn/context.rs`）。

`last_consolidated`（memory consolidation）与 `durable_message_index` 取 max 做
history floor。两者都不是 STM。

## 4. SessionCheckpoint（现 `working_memory`）

合同：`agent-diva-core/src/memory/working.rs` — 「volatile, session-scoped, and
never authoritative」。

写路径（源码事实）：

```text
update_working_checkpoint
  → CheckpointWriteRequest { session_id, key_info, related_sops, content }
  → TypedLaputaMemoryProvider::checkpoint_write
  → MemoryRecord {
        id: working-checkpoint-{sha16(session_id)},
        kind: WorkingMemory,
        trust: AppliedAuthority,                 // 与「非权威」注释矛盾
        provenance.source: LaputaAppliedSection, // 命名撒谎：不是 Laputa section
        scope.session_id: Some(session_id),
        evidence_refs: []
    }
  → TypedMemoryStore::put（立即生效，CAS）
```

工具门：`memory && working_memory && !subagent_mode`（`tool_assembly.rs`）。
session_id 由装配绑定，模型不能改。

读：每 turn `working_memory_block`；失败非致命，跳过。空 / tombstone / session 不匹配
→ 无块。

清：

| 事件 | 是否物理 DELETE WM |
| --- | --- |
| AgentLoop shutdown 对 `list_sessions()` 逐个 `on_session_end` | 是 |
| `run_startup_gc` | **函数存在，全仓仅定义一处，无生产调用**（实验观察：`rg run_startup_gc`） |
| Reset (`archive_and_reset`) | **否**；同 key 下一轮仍注入旧 WM |
| Delete session JSONL | **否**；且之后 `list_sessions()` 看不到该 key，shutdown GC 也扫不到 → 孤儿 |
| Stop / cancel | 否 |

L1 / `memory_list` / `memory_search` 排除 `session_id IS NOT NULL`。
GUI `MemoryView.vue:46` 仍把 `working_memory` 当作 BML kind 筛选值（实验观察）。

## 5. Session 生命周期

键：`{channel}:{chat_id}`（`admission.rs`）。磁盘：
`{workspace}/sessions/{safe_key}.jsonl`，`:` `/` `\` → `_`。
格式：首行 metadata（含 `canonical_checkpoint`），其后每行一条 `ChatMessage`。
原子 temp + `sync_all` + rename。**无文件锁**。

| 面 | channel | chat_id | session_key |
| --- | --- | --- | --- |
| GUI | `gui` | 生成的 chat id | `gui:{chatId}` |
| Manager API | `api` | `default` | `api:default` |
| CLI | `cli` | `direct` / TUI id | `cli:direct` |
| Slack / Discord / Telegram | 渠道名 | 渠道/聊天 id | `{name}:{id}` |
| GUI cron | 重映射 `api` | `cron:{to}` | `api:cron:…`（**不**进 GUI 聊天 session） |
| CLI cron 默认 | `cli` | `direct` | **与交互 CLI 同一 session** |

多 channel = 多份 transcript + 多份 CanonicalCheckpoint + 多份 SessionCheckpoint。
共享的只有 BML LTM（`session_id IS NULL`）和 Frozen Core / WORLD 文件。

Reset 归档名为 `{safe}.reset.{millis}.jsonl`，扩展名仍是 jsonl，
`list_sessions()` 可能把它当成活 session（源码事实）。

## 6. Plan / background / cron / subagent

### 6.1 Plan

两层存储：

1. `EphemeralPlanRegistry`：进程内、按 `session_key`；注释写明重启丢失
   （`tool_config/mod.rs`）
2. `planning.sqlite3`：workspace 级；`active_plan` **单例**（`singleton=1`）；
   `plan_execution_contexts` 按 plan+revision+`session_key`

`PlanningConfig::open_workspace` 启动时删除过期 `planning.db*`，防止旧库被误恢复。

`Plan` 字段含 `goal` / `open_questions` / steps / todos，语义像 STM 切片，但：

- 执行走 `Capability::PlanExecute` 审批
- draft 不跨进程
- durable `active_plan` 是 workspace 单例，与「每 session 一份 draft」冲突面已存在
- 注入在 `PlanGuard`（TurnVolatile），不进 prefix
- `PlanContextBuilder`（`## Active Plan` ≤800 字）**生产 agent loop 未调用**

Plan 不是 STM，也不能当 STM 宿主（建议：选项文把「复用 Plan store」标为高冲突）。

### 6.2 Subagent / spawn / enqueue

| 路径 | C1–C5？ | 父 history | SessionCheckpoint 工具 | 看到的 Memory |
| --- | --- | --- | --- | --- |
| `spawn` | 否；`build_subagent_prompt` | 明确禁止 | 不注册 | Typed L1 markdown |
| `run_supervised` | 否 | 否 | 否 | 同上 |
| `spawn_batch` | 更瘦，无 L1 | 否 | 否 | 无 |
| `enqueue_background_task` | 否 | 否 | 否 | 元数据只拷 `session_key` |

子代理完成后把公告打回父 `origin_channel:origin_chat_id`，**下一父 turn** 才加载父
SessionCheckpoint / CanonicalCheckpoint。子上下文从不继承它们。

### 6.3 Cron / Heartbeat

Cron 作业：`{config_dir}/data/cron/jobs.json`。`active_runs` 内存。
默认 payload `agent_turn` → 发布一条 inbound。admission 标 `scheduled=true`，
注入 `scheduled_turn()` 文案。复用算出来的 `session_key` 的 transcript / WM /
checkpoint。GUI cron 被隔离到 `api:cron:…`；CLI 默认 **污染** `cli:direct`。

`HeartbeatService` 只在 `agent-diva-core` 单测里 `new`；CLI / Manager 未接线。
`GET /api/heartbeat` 是健康检查，不是节律循环。`HEARTBEAT.md` 当前不是 STM。

### 6.4 Garden

`agent-diva-gui` 无 Garden 符号。AGENTS.md 与 BML 抽层研究：Garden facade
**未在 agent-diva 实现**。现有 Memory 页是 BML 列表，不是 Garden。

## 7. GenericAgent 对照（引用 R1，不重写）

基线 `ee5a474`。GA `update_working_checkpoint` 写的是**进程内** `self.working`
dict（`key_info` / `related_sop`），不写盘，不是 L0–L4。Diva Wave 2 把它做成 BML
行并在 session end 删除——抄了工具名，改了寿命。

GA L1 `global_mem_insight.txt` ≤30 行是存在性索引，对齐 Diva **BmlStartupIndex**，
不是 S3 STM。GA L2 对齐 BML LongTerm。GA L4 对齐归档 / transcript。
`put_task` 可继承上一任务 `key_info`；Diva 无此继承，反而 GC。

历史 Garden ADR-0002 把 `05 MEMORY.MD` 定义为 STM 权威检查点。S3 只恢复产品语义
（当前目标、开放回路、下一步、证据指针）；S2 删除 `memory_md` 物理路径。
不得把历史文件名当落点。

## 8. 给 D2 的插入约束（不选位置）

这些是约束，不是方案。D2 必须逐条回答，不能默认。

1. **禁止把 SessionCheckpoint、CanonicalCheckpoint、BmlStartupIndex 改名为 STM。**
   三者寿命、写入者、清空条件和 Prompt 位置都不同。S4 明文禁止一个 `working_memory`
   enum 表达两种寿命。
2. **若 STM 投影进 SessionStable prefix：** 每次自动更新都会 `L1HotRefresh` 或新的
   break reason，打断 prompt cache。现有测试要求 WM 变化不得改 prefix。
3. **若 STM 投影进 TurnVolatile（并入 WM 槽或新 section）：** 新 session 第一轮能读到
   （动态区每 turn 重读权威），但必须与 session end GC 解耦，否则会被现有
   `gc_session_scoped` 误删。
4. **禁止以 CanonicalCheckpoint body 为 STM 权威。** Compact 是有损替换；Auto/Reactive
   失败保留旧 checkpoint，不是活动工作集；session reset 清空它。
5. **禁止并入 Plan store。** 审批、ephemeral registry、workspace 单例 `active_plan`、
   启动删旧库，都与 S3/S5 冲突。
6. **禁止沿用 `memory_md` / `MemoryMd`。** S2 删除面。
7. **有界投影：** 不得整份历史、整库 BML、完整 WORLD、完整 transcript、MEMRULES、
   报告注入。现 `context_plane_invariants` 已禁 MEMRULES / 完整 WORLD / 报告 /
   退役人格；STM 必须继承这条负向不变量。
8. **compact 前后必须重读 STM 权威**，不得把 checkpoint 里的「下一步」当作唯一副本。
9. **优先级（若多块同时声明同一事实）：** Frozen Core（会话冻结人格）>
   用户刚说的 CurrentUser > STM 活动集 > PlanGuard > CanonicalCheckpoint 摘要 >
   BmlStartupIndex > PrefetchRecall。这是建议级排序，供 D2 取舍，不是已批准 ADR。
10. **subagent / cron 写权限必须显式：** 子代理今天无 WM 工具；cron 复用目标 session。
    STM 若 workspace 级，cron 与多 channel 都是并发写者。
11. **BML 完整性失败走 `DegradedMemoryProvider`。** STM 若与 LTM 同库，会一起降级。
12. **裸 `ContextBuilder::new` 默认 `MemoryManager`（`memory/MEMORY.md`）；**
    生产 Manager 才注入 Typed。任何 STM 接线必须测生产路径，不能只测裸 builder。

## 9. 本文件未展开

- Hold 选项比较 → `stm-options-and-experiments.md`
- 场景失败矩阵 → `stm-failure-and-concurrency-matrix.md`
- Persona / WORLD 投影重做 → R3
- 删除 `memory_md` 的零残留证明 → R4
