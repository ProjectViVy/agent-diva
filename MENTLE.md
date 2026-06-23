---
title: "Mentle — 架构索引 (基线 v0.2, 2026-06-20)"
date: 2026-06-20
status: approved (基线索引 — 设计态补全 + Session-End 治理正式化)
version: 0.2
revision: 2
supersedes: null
owner: 大湿
applies_to: agent-diva-pro
related:
  - LAPUTA.md                                          # 镜像级基线索引 (2026-06-12)
  - DECISION.md                                        # alife v1 横向 dispose — 提 memtle = 0.1.2 已落 (2026-06-18)
  - DECISION-v2-next-phase.md                          # Harness Engineering — Mentle 集成边界 (2026-06-19)
  - docs/design-notes/autonomous-activity-thoughts.md  # 自主活动 4 层 + 3 态用户存在 (2026-06-18)
  - docs/research/harness-engineering-three-way-detailed-checklist.md  # 21 维度对比 (2026-06-19)
source_of_truth:
  - agent-diva-selfinprove/docs/dev/genericagent/mentle-laputa-memory-role-decision.md        # 295 行 accepted architecture decision (2026-05-31)
  - docs/prds/prd-laputa-2026-06-12/prd.md                                                    # PRD 主体 §5.6 FR-6xx (3 条)
  - docs/prds/prd-laputa-2026-06-12/reconcile-mentle-laputa.md                                # IR-2 决策调和 (51 行, 8 项 gap)
  - scripts/ci/verify_mentle_package_policy.py                                                # 策略护栏:memtle 0.1.2 来自 crates.io
  - agent-diva-pro/docs/prds/prd-laputa-2026-06-12/.decision-log.md                            # 345 行 D-001~D-013 (v0.2 增)
  - agent-diva-selfinprove/docs/dev/genericagent/autonomous-evolution-simplified-architecture-decision.md  # 749 行 accepted (Mentle 12-13 节 + 20.7 降级 + 20.8 配置)
  - agent-diva-selfinprove/docs/dev/genericagent/autodream-rhythm-distillation-design.md     # 893 行 (AutoDream ↔ Mentle 边界)
  - agent-diva-selfinprove/docs/dev/genericagent/candidate-evidence-journal-audit-design.md  # 425 行 (Mentle 不写 MEMORY.md, EvidenceRef Capsule type)
  - agent-diva-selfinprove/docs/dev/genericagent/context-compaction-research.md              # 766 行 (Compaction 永不调 Mentle)
  - agent-diva-selfinprove/docs/dev/genericagent/compression-research.md                      # 667 行 (Source Capsule ↔ Mentle Phase 4)
  - agent-diva-selfinprove/docs/dev/genericagent/shared-memory-rendering-research.md          # 683 行 (MEMORY.md 渲染 ↔ Mentle §12)
reconcile_docs:
  - docs/prds/prd-laputa-2026-06-12/reconcile-mentle-laputa.md   # IR-2 (本索引的"差距"主要来源)
review_docs:
  - docs/prds/prd-laputa-2026-06-12/review-adversarial.md        # R2 提 Mentle 召回边界 3 命名 (L42)
follow_up_prds:
  - Mentle 集成 PRD (Laputa PRD OQ#17-18 已明确推迟): 5 wing 写白黑名单 / work_memory 7 字段 / Authority 4 级 / 4 阶段 retrieval / 边界矩阵 (§4.5)
  - Mentle 综合治理 (用户拍板推迟): lap-write 行为 / Authority 4 级 / "casual→identity" 反向保护 / Obsidian 三向 split (§3.9)
  - Mentle Session-End 治理 (v0.2 §15 正式化): MVP P0/P1/P2 方案
mentle_governance_tests:
  - agent-diva-laputa/tests/mentle_governance.rs                 # Laputa 写操作不创建 palace.db / .mentle state
  - agent-diva-autodream/tests/mentle_governance.rs              # AutoDream 提案写不创建 palace.db
  - agent-diva-agent/tests/mentle_governance_boundaries.rs       # Mentle prompt 路由不暴露 raw memtle_* 名称
revision_notes:
  - v0.1 (2026-06-19) — 初版基线索引: 工具接入盘点 + 设计态契约 + 设计态 vs 实现态 18 项对比 + 暂存 §15
  - v0.2 (2026-06-20) — 设计态补全 + Session-End 治理正式化:
      (a) frontmatter 加 5 份 source_of_truth + 2 份 related + revision_notes 区段
      (b) §3 加 §3.9 Obsidian 三向 split (mentle-laputa §11)
      (c) §4 末尾加 §4.5 Mentle 边界矩阵 (14 行表: AutoDream/ContextCompaction/Capsule/MEMORY/Report/Heartbeat/Module/审计/Config/3 态用户/Candidate/Obsidian + 2 隐含)
      (d) §15 抬头的"暂存"标识去除, 正式化为设计态
      (e) §12 状态对比表加 7 项零覆盖的实现状态
      (f) §13 复盘触发器加 14 个新项 (对应 §4.5 边界矩阵每项 + Session-End 治理触发器)
---

# MENTLE — 架构索引 (基线 v0.1, 2026-06-19)

> **TL;DR**: Mentle = **1 个 crates.io 嵌入式长期记忆库 (memtle = 0.1.2) + 1 个 hybrid memory provider + 5 wing 工具面 + 3 hook 召回边界**。
> 现状是一个**工具接入** (mentle feature gate + HybridMemoryProvider + memtle_* dynamic tool registration),不是记忆综合治理系统;**综合治理已明确推迟**到 follow-up PRD (per Laputa PRD OQ#17-18 + 用户拍板 2026-06-19)。
> **Laputa 写、Mentle 仅 recall** (authority 单向,不可分割) — 写 authority 在 LaputaWrite::apply_proposal,记忆回望走 FR-2xx HTTP 客户端 + laputa_wakeup / laputa_project_soul / laputa_recall_intent 3 命名 hook。

> ⚠️ **本文件是设计态基线索引** (v0.1, 2026-06-19),**不是实现态**。实现进度以代码 + `scripts/ci/verify_mentle_package_policy.py` + 3 个 `mentle_governance*.rs` 测试为准;**Mentle 综合治理在 Laputa PRD OQ#17-18 已明确推迟到 follow-up PRD**, 本索引覆盖 ① 已实现工具接入 ② 设计态 5 wing / 写白黑名单 / work_memory / Authority 4 级契约 ③ 与 Laputa 边界。

---

## 0. 30 秒读懂

| 维度 | 答案 |
|---|---|
| **Mentle 是什么** | 1 个 crates.io 嵌入式长期记忆库 (`memtle = 0.1.2` published crate) + Agent-Diva 侧 4 个源文件的工具接入层 |
| **不是** | 记忆综合治理系统 / DB schema owner / 写 authority 来源 / 写触发器 / AutoDream 触发链 / 报告生成 / 长期身份记忆 |
| **当前阶段** | 工具接入已完成 (Sprint 1-7 RC),综合治理推迟到 follow-up PRD (per Laputa PRD OQ#17-18) |
| **核心物理形态** | `workspace/memory/palace.db` (SQLite) + 14 section 之外的 L2 嵌入式记忆层 (per HybridMemoryProvider) |
| **核心规则** | Laputa 写、Mentle 读 (authority 单向);Mentle 集成 = 工具接入 (不是治理);默认不注入 context (on_demand);`memtle_status` 是 prompt 路由激活锚 |
| **集成点** | `agent-diva-agent/src/agent_loop.rs:591 pub fn mentle_active()` (recall 开关) + `agent-diva-agent/src/mentle_runtime.rs` (组装) + `agent-diva-core/src/memory/hybrid.rs:286 HybridMemoryProvider` (记忆) |
| **feature gate** | `mentle = ["agent-diva-core/mentle", "dep:memtle"]` (optional, 默认关闭,Windows 需 `clang-cl.exe`) |
| **关键命名差异** | 设计文档: **Mentle** / 代码库: **memtle** (拼写差异是有意的; `MemtleToolkit` 是库名, `Mentle` 是功能模块名) |

---

## 1. 历史时间线(关键决策 5 条,2026-05-31 → 2026-06-19)

> 详细时间线见 §11。本节给"什么时候决定了什么"的快速索引。

| 日期 | 决策 | 来源 (path:line) | 关键产出 |
|---|---|---|---|
| **2026-05-31** | **Mentle / Laputa / AutoDream / Harness 边界决策** accepted | `agent-diva-selfinprove/docs/dev/genericagent/mentle-laputa-memory-role-decision.md:1` (status: accepted) | 5 wing / 9 该写 + 8 不该写 / work_memory 7 字段 / 4 阶段 retrieval / Authority 4 级 / 5 MUST NOT (核心契约基线) |
| **2026-06-12** | **Laputa PRD v0.0.5/6 final 锁 FR-6xx (3 条)** | `docs/prds/prd-laputa-2026-06-12/prd.md:288-301` | FR-601 (authority 单向) + FR-602 (recall 走 HTTP) + FR-603 (默认 on_demand, `always` 不进 enum) + D-005 14 份分类 + D-008 OQ#17-18 转 follow-up |
| **2026-06-12** | **IR-2 reconcile-mentle-laputa 锁定 8 项 gap** | `docs/prds/prd-laputa-2026-06-12/reconcile-mentle-laputa.md:9-18` | 主体方向一致, **PRD 完全漏掉**: 5 wing 工具面契约 / 9+8 写白黑名单 / work_memory 7 字段 / 4 阶段 retrieval / Authority 4 级 ordering |
| **2026-06-15** | **pro repo workspace 初始化** | `agent-diva-pro/AGENTS.md:36` | "Mentle integration is intentionally pinned to the published `memtle = 0.1.2` crate; do not replace it with a path/git override" |
| **2026-06-18** | **DECISION.md (alife v1) 标 memtle = 0.1.2 为"已完成的 Mentle runtime governance"** | `agent-diva-pro/DECISION.md:19, 83` | 4.1 长期记忆: "已完成 + 超越" / 4.2 自主活动: 基于已完成的 laputa 和 mentle, **待架构设计** |
| **2026-06-19** | **本 MENTLE.md 索引编写 + 用户拍板"现状仅工具接入,综合治理推迟"** | 本文件 | 本索引 + LAPUTA.md D 列表 baseline 校核 |

---

## 2. 现状盘点(实现态,代码引用)

> ⚠️ 本节只列**实际代码**。设计态契约见 §3。两者不一致时,在 MENTLE.md §3 §4 标 ⚠️。

### 2.1 Cargo 依赖(`memtle = 0.1.2` 来自 crates.io)

**Source of truth: `agent-diva-pro/Cargo.toml:87-89`** (workspace 级钉死)

```toml
# Mentle integration is frozen to the published crates.io package at 0.1.2.
# Do not replace this with a path/git override in the main workspace.
memtle = { version = "0.1.2", default-features = false }
```

**Feature gate**:
- `agent-diva-agent/Cargo.toml:14`: `mentle = ["agent-diva-core/mentle", "dep:memtle"]` (optional)
- `agent-diva-core/Cargo.toml:60`: `mentle = ["dep:memtle"]` (optional)

**反向治理测试**: `agent-diva-laputa/tests/mentle_governance.rs:89-94` 静态断言 `agent-diva-laputa/Cargo.toml` **不**含 `mentle` / `memtle` 字符串(同 4 行也出现在 `agent-diva-autodream/tests/mentle_governance.rs:120-126`)。

### 2.2 4 个源文件(Agent-Diva 侧接入层)

| 文件 | LOC | 职责 | 引用 |
|---|---|---|---|
| `agent-diva-core/src/memory/hybrid.rs` | 961 | `HybridMemoryProvider` (L0/L1 文件 + L2 palace), `PalaceStatusSnapshot` 3 状态 (Ready/Stale/Degraded), `MemoryProvider` trait 实现 | 本文件 §6 |
| `agent-diva-agent/src/mentle_runtime.rs` | 456 | `MentleRuntime::try_build` 组装 + `MentleToolkitTool` 适配器 (Tool trait) + 错误分类 (10 类别) + 降级 (3 类) | 本文件 §2.5 |
| `agent-diva-agent/src/mentle_discovery.rs` | 23 | `discover_mentle_tool_names` 条件编译包装 (cfg(mentle) 转发到 mentle_runtime) | 本文件 §7 |
| `agent-diva-agent/src/tool_config/mentle.rs` | 139 | `MentleToolRuntimeConfig` + `MentleToolMode` (Off/ReadOnly/Full/Custom) + `filter_mentle_tools` | 本文件 §8 |

> **支持文档** (Sprint 1-7 RC, 已固化): `docs/dev/past/legacy-docs/dev/mentle-integration/13-s3-a1-memtle-toolkit-tool-interface.md` (100 行,工具接口冻结) / `16-s3-a4-a6-mentle-runtime-assembly.md` (98 行,运行时组装) / `22-s4-a10-mentle-feature-build-env.md` (77 行,Windows clang-cl) / `25-s7-a1-mentle-tool-selection-and-gui-controls.md` (266 行,GUI 控件)。

### 2.3 核心集成点(per Laputa PRD FR-601 / FR-507)

| 集成点 | 位置 | 角色 | 引用 |
|---|---|---|---|
| **`pub fn mentle_active()` 召回开关** | `agent-diva-agent/src/agent_loop.rs:591` | bool 返回,影响 `ContextBuilder::with_mentle(mentle_active)` 是否向 prompt 注入 Mentle routing 描述 | Laputa PRD FR-601, FR-603 |
| **`MentleRuntime` 组装** | `agent-diva-agent/src/mentle_runtime.rs:23` | `try_build(workspace, tool_config)` 异步,启动失败返回 `None` 自动降级 | Laputa PRD FR-507 Hook 1 |
| **`HybridMemoryProvider` L0/L1+L2** | `agent-diva-core/src/memory/hybrid.rs:286` | 文件记忆 + palace 统一为 `MemoryProvider` trait | Laputa PRD FR-507 Hook 2 |
| **`MemoryProvider` trait 4 阶段** | `agent-diva-core/src/memory/provider.rs` (572 行) | `system_prompt_block` (startup) + `prefetch` (mid-turn intent) + `sync_turn` (post-turn) + `on_session_end` (shutdown) | Laputa PRD FR-201/202/203 + FR-603 |
| **`MemoryProvider` 3 命名 hook** | `agent-diva-core/src/memory/provider.rs:247-275` (doc comment) | `laputa_wakeup` / `laputa_project_soul` / `laputa_recall_intent` (D-010 reviewer R2 命名) | Laputa PRD FR-507 Hook 3, FR-603 |
| **Laputa 写入口** | `agent-diva-manager/src/handlers/laputa.rs:150-170` `apply_laputa_proposal_handler` | HTTP 入口调 `state.laputa.apply_proposal(id, actor, applied_at)`,返 `outcome {proposal, changelog, audit_event, rollback_request}` | Laputa PRD FR-101 |
| **build_agent_loop 传 Mentle config** | `agent-diva-manager/src/runtime.rs:355` | `mentle: MentleToolRuntimeConfig::from_config(config)` | Laputa PRD FR-507 Hook 1 启用 |

### 2.4 召回路径(per Laputa PRD FR-602 / FR-603)

```text
AgentLoop 启动
  ↓
MentleRuntime::try_build(workspace, tool_config)  ← mentle_runtime.rs:23
  ↓ (成功)
Arc<Mutex<MemtleToolkit>>  ← memtle::toolkit::MemtleToolkit::open(<workspace>/memory/palace.db)
  ↓
HybridMemoryProvider::new(file_manager, toolkit)   ← hybrid.rs:294
  ↓
AgentLoop.mentle_active = (custom_tools 含 memtle_status)  ← mentle_runtime.rs:106-108
  ↓
ContextBuilder::with_mentle(mentle_active)         ← agent_loop.rs:463
  ↓ (true 时)
系统 prompt 注入 "## Memory Palace Overview" markdown  ← hybrid.rs:336-369
  ↓
per-turn 调 MemoryProvider::prefetch(intent, room) ← hybrid.rs:371-414
  ↓ (调 memtle::tools::SearchArgs {query, limit: 5, wing, room, context})
注入 "## Palace Deep Recall" markdown
  ↓
per-turn 末 调 MemoryProvider::sync_turn(...)      ← hybrid.rs:416-492
  ├─ file_manager.save_memory / append_history  (L0/L1,authoritative)
  └─ toolkit.call_json("memtle_diary_write", ...) (L2,best-effort,失败降级不阻断)
```

### 2.5 Mentle 工具面 — 已实现(`memtle_*` 动态注册)

> ⚠️ **LAPUTA.md D 列表 baseline "Mentle 5 wing (search/read/write/update/delete_or_archive, 可选 link)" 在代码里**是动态注册** (`toolkit.tool_definitions()` 拉)**,**不是**硬编码 5 个 wing**。当前代码不依赖具体 wing 名称,只过滤 `name.starts_with("memtle_")` 前缀 (per `mentle_runtime.rs:441` + `tool_config/mentle.rs:51`)。

**实际工具面** (per `MemtleToolkit::tool_definitions()`,运行时动态):
- `memtle_status` — 只读,查询 palace 状态 (drawers/rooms/edges) — **`mentle_active` 激活锚** (per `mentle_runtime.rs:106-108` + `agent_loop.rs:533`)
- `memtle_search` — 只读,语义搜索记忆条目 (per `hybrid.rs:380` 调 `SearchArgs {query, limit: 5}`)
- `memtle_diary_write` — 写入,sync_turn 调,history_entry → palace (per `hybrid.rs:449`)
- `memtle_add_drawer` / `memtle_add_room` — 写入,具体 schema 由 `memtle 0.1.2` 决定
- **其他 memtle_* 工具** (per `memtle 0.1.2` crate 实际暴露,代码侧不知道也不依赖)

**代码侧对工具面契约的边界** (per `13-s3-a1-memtle-toolkit-tool-interface.md` 冻结):
- 只能读 `name` / `description` / `inputSchema` 3 字段
- 执行只能走 `MemtleToolkit::call_json(name, args).await`
- toolkit handle 固定为 `Arc<tokio::sync::Mutex<MemtleToolkit>>`
- 失败翻译为 `ToolError::ExecutionFailed`
- 缺 name/description/inputSchema 时 skip 该定义,warn 但不中断 (per `mentle_runtime.rs:373-382`)

### 2.6 Mentle 治理测试(反向 governance,3 个)

| 测试文件 | 关键断言 | 引用 |
|---|---|---|
| `agent-diva-laputa/tests/mentle_governance.rs` (94 行) | Laputa 写操作(proposal_apply_audit_and_rollback)**不**创建 `workspace/memory/palace.db` / `.mentle` state;`agent-diva-laputa/Cargo.toml` **不**含 mentle / memtle | lap-write 不污染 palace state |
| `agent-diva-autodream/tests/mentle_governance.rs` (126 行) | AutoDream proposal 写 + rhythm report 写 **不**创建 palace.db / .mentle;`agent-diva-autodream/Cargo.toml` 不含 mentle / memtle | autodream 走 Laputa,不经 palace |
| `agent-diva-agent/tests/mentle_governance_boundaries.rs` (137 行) | 启用 memtle_status + memtle_search 时,system prompt **不**含 `memtle_search` / `memtle_kg_query` / `memtle_*` / `mentle recall` / `palace memory` 字符串 | 召回文本走 MemoryProvider 拼接,非 raw tool name 暴露 |

### 2.7 GUI 控件(per S7-A1)

`agent-diva-gui/src/components/settings/MentleSettingsCard.vue` (247 行):
- 启用 toggle + mode 4 选 (`off` / `read_only` / `full` / `custom`)
- `custom` 模式 dynamic checklist of `memtle_*` tools (调 `listMentleTools` API)
- `fallback_tools = ['memtle_status', 'memtle_search']` (per line 28,read-only 预设)
- Save/Reset 行为,触发 `RuntimeControlCommand::UpdateMentle` (per S7-A1 §Implementation Status)

### 2.8 CI / Build(per `scripts/ci/verify_mentle_package_policy.py` 48 行 + `justfile:33-52`)

**`verify_mentle_package_policy.py`** 静态检查 (4 项):
1. root `Cargo.toml` 含 `memtle = { version = "0.1.2", default-features = false }` (line 12)
2. root `Cargo.toml` 不在 `[patch.crates-io]` 里 override memtle
3. 所有子 `Cargo.toml` 不能定义 `path` 或 `git` 依赖
4. `Cargo.lock` 必须从 `registry+https://github.com/rust-lang/crates.io-index` 解析 memtle 0.1.2

**justfile recipes**:
- `mentle-package-policy` — 调上面 CI 脚本
- `sprint5-default-check` — 9 个 default-lane 回归 (含 `test_with_toolset` / `subagent_does_not_receive_mentle_by_default` / `test_agent_loop_prefetch_failure_continues_without_recall_injection` 等)
- `mentle-check` — `cargo check -p agent-diva-agent --features mentle` + `cargo test -p agent-diva-core --features mentle memory` + `cargo test -p agent-diva-agent --features mentle mentle`
- `sprint5-check` = `fmt-check` + `sprint5-default-check` + `mentle-check`
- `epic6-release-gate` — 含 3 个 `mentle_governance*` 测试

**Windows 特性** (per `22-s4-a10-mentle-feature-build-env.md`): 需 `clang-cl.exe`,PATH 加 `C:\Program Files\LLVM\bin\`;Rust 1.88+ for mentle lane (default lane 1.80+)。

---

## 3. 设计态盘点(per `mentle-laputa-memory-role-decision.md`,295 行)

> 本节是 **2026-05-31 accepted architecture decision** 的索引,目前 Laputa PRD v0.0.6 final 锁了 §5.6 FR-6xx 3 条边界,其余契约**已明确推迟到 follow-up PRD** (per OQ#17-18)。

### 3.1 5 wing 工具面契约(`mentle-laputa-memory-role-decision.md:73-89`)

> ⚠️ **设计态 vs 实现态差异**:设计说 "5 wing = search/read/write/update/delete_or_archive, 可选 link" 是**契约约定**;实现态是**动态** `memtle_*` 工具(由 `memtle 0.1.2` crate 决定,代码侧不知道具体 wing 名称,只过滤前缀)。两者**不完全一致** — 实现的工具面是 `memtle 0.1.2` published crate 的实际暴露,5 wing 是设计层契约。

```text
设计态 (决策 §4):
  mentle.search(query, filters)
  mentle.read(id)
  mentle.write(note)
  mentle.update(id, patch)
  mentle.delete_or_archive(id)
  [可选] mentle.link(source_id, target_id, relation)
```

### 3.2 写白名单(9 该写,per 决策 §5)

1. user 显式说"记住/记录"
2. project research summaries
3. technical conclusions
4. reusable procedures
5. command notes / troubleshooting records
6. project decisions / constraints / open questions
7. daily/weekly/monthly report indexes
8. AAAK summaries of rhythm reports
9. AutoDream intermediate material (not yet authority)
10. major-task temporary work memory
11. source references that may help future recall

### 3.3 写黑名单(8 不该写,per 决策 §5)

1. one-off casual chat
2. unstable emotional fragments
3. inferred personality judgments without user confirmation
4. sensitive private information without explicit authorization
5. raw model speculation about the user
6. facts already reliably available in project files
7. content that belongs in Laputa authority files instead
8. anything that would silently become long-term identity memory without review

**关键硬规则** (per 决策 §5 末): "Mentle can store external notes and work material. **Laputa / MEMORY.md stores subject continuity and authority-bearing long-term memory.**"

### 3.4 work_memory schema(7 字段,per 决策 §10)

```text
work_memory/<project_or_task>/
  - current_goal
  - constraints
  - decisions
  - open_questions
  - evidence
  - next_actions
  - useful_links
```

**3 阶段生命周期** (per 决策 §10):
- task start → recall work_memory → load selected parts
- task progress → update when durable intermediate state appears
- task end → AutoDream / Laputa decides what becomes report / proposal / archive / long-term

**归属未定** (per IR-2 §3,reconcile-mentle-laputa.md:35): "PRD 无 FR 描述 work memory; 也未定 work memory 归属 (Mentle 暂存 vs Laputa 持久)"

### 3.5 4 阶段 staged retrieval(per 决策 §9)

```text
1. Search report indexes / AAAK summaries       ← (L2 palace indexes)
2. Select likely relevant daily/weekly/monthly  ← (L1 files)
3. Read the full report files                   ← (L0/laputa)
4. If needed, read original sessions or source   ← (L0/laputa)
```

**Mentle index 字段列表** (per 决策 §9):
- report titles
- dates and periods
- participants or projects
- AAAK summaries
- keywords
- links to full Laputa report files
- links to source sessions when available

> ⚠️ **Laputa PRD #13 report_indexes + #14 AAAK summaries TBD 段** (per `prd.md:88`) **未锁 4 阶段能力要求**;也未定义 Mentle index 字段列表(per IR-2 §5)。

### 3.6 Authority 4 级 ordering(per 决策 §12)

```text
1. Laputa authority files and MEMORY.md              ← 最高
2. Laputa changelog and proposal review records
3. Full rhythm reports and journal artifacts
4. Mentle indexes, notes, and temporary work-memory  ← 最低
```

### 3.7 5 条 MUST NOT(per 决策 §12 末)

1. Mentle must not silently rewrite `MEMORY.md`
2. Mentle must not silently rewrite identity files
3. Mentle must not become default prompt context
4. (隐含) Mentle must not authorize durable changes
5. (隐含) Mentle may not become authority source

### 3.8 "casual→identity" 反向保护(per 决策 §5+§8+§12 三处强调)

> ⚠️ **Laputa PRD 完全漏掉** (per IR-2 §3,reconcile-mentle-laputa.md:38):"PRD FR-103 review 流程仅间接拦截,无显式反向拒绝 FR"。decision 三处强调"不可静默变长期身份记忆",跟 D-005 §4 自治哲学一致。

### 3.9 Obsidian 三向 split（per 决策 §11,2026-05-31 accepted）

```
Laputa files       =  human-readable authority and rhythm files (权威 + 可读)
Obsidian           =  human-facing linked note workspace / knowledge garden (人类笔记)
Mentle             =  machine-facing semantic index and recall engine (机器索引)
```

**规则**：
- Mentle 可索引 Obsidian 和 Laputa 文件（recall handles）
- **Authority material 应保留为 readable / editable / versionable 文件**（不通过 Mentle 写入）
- Obsidian 集成走同样三向 split（不在 Laputa PRD 零覆盖）
- 未来 Obsidian 集成时机：先 Laputa 写入 → report index → work memory → AutoDream+Mentle 联通 → Obsidian 最后

> **本索引**：v0.2 新增此节 (per 大湿"补 mentle 接入设计" 拍板, 2026-06-20)。Laputa PRD §4.4 Non-Goals 完全未提 Obsidian, 走 deferred 占位, 未来起 Laputa 架构后续 PRD 时需补 §17 Obsidian 三向 split 能力级。

---

## 4. Laputa ↔ Mentle 边界(per `prd-laputa-2026-06-12/prd.md:288-301` §5.6 FR-6xx,3 条)

### 4.1 3 条 FR(原文)

| FR | 原文要点 | 实现态位置 | 状态 |
|---|---|---|---|
| **FR-601 — Laputa 写 authority, Mentle 仅 recall** | MUST 在 Laputa 入口处 (FR-101) 检查 caller, 仅 `LaputaWrite::apply_proposal()` 入口可写。Mentle MUST NOT 调用此入口, 仅可调 `LaputaRead::*` 召回。 | `agent-diva-manager/src/handlers/laputa.rs:150-170` `apply_laputa_proposal_handler` (Laputa 写) <br> `agent-diva-core/src/memory/hybrid.rs` `HybridMemoryProvider` (Mentle recall) | ✅ Laputa 写 / Mentle recall **解耦干净**,`mentle_governance.rs` 静态断言 lap crate 不含 memtle |
| **FR-602 — Mentle 召回时 read 路径** | Mentle 召回 MUST 走 FR-2xx 读接口 (snapshot / section), 不得直接读 `.laputa/state.json` 物理文件 | Laputa PRD 设计态路径 — **当前实现** `HybridMemoryProvider::prefetch` **不**走 LaputaReadAdapter,而是直接调 `memtle::tools::SearchArgs` (per `hybrid.rs:380-413`) | ⚠️ **设计态 vs 实现态 drift**:HybridMemoryProvider 当前走 **palace 内部 search**,**不**走 Laputa FR-2xx HTTP — 因为 palace 跟 Laputa 物理上是两个不同存储。需要 follow-up PRD 明确 "Mentle recall 是否需穿透 Laputa" |
| **FR-603 — Mentle 召回上下文注入规则** | `mentle_recall_policy: "off" \| "on_demand"` (默认 on_demand, **不支持 "always"**) | `agent-diva-agent/src/agent_loop.rs:591 pub fn mentle_active()` (boolean 开关,无 on_demand/always 枚举);Laputa PRD state.json `mentle_recall_policy` 字段**未实接** | ⚠️ **设计态 vs 实现态 drift**:state.json `mentle_recall_policy` 字段在代码里**未找到**(per grep `mentle_recall_policy` 仅在 prd.md 出现),实际开关是 `mentle_active()` boolean,行为近似 on_demand |

### 4.2 3 Hook 实接(per Laputa PRD FR-507,3 个)

| Hook | 位置(设计态 vs 实现态) | 由谁引用 |
|---|---|---|
| **1. 写入口** | `agent-diva-manager/src/runtime.rs:355` `mentle: MentleToolRuntimeConfig::from_config(config)` (实接) **+** `handlers/laputa.rs:150-170` `apply_laputa_proposal_handler` (Laputa 写入口) | FR-101 + MentleRuntime::try_build |
| **2. 启动快照** | `agent-diva-core/src/memory/provider.rs:125-139` `StartupContextSnapshot.laputa_state_root` (实接) | FR-201/202/203 + HybridMemoryProvider::system_prompt_block |
| **3. Mentle 召回边界** | `agent-diva-core/src/memory/provider.rs:247-275` (Laputa PRD 提的 3 命名 `laputa_wakeup` / `laputa_project_soul` / `laputa_recall_intent` **只在 doc comment 出现**,实际 trait 方法是 `system_prompt_block` / `prefetch` / `sync_turn` / `on_session_end` 4 个) | FR-603 |

> ⚠️ **`laputa_wakeup` / `laputa_project_soul` / `laputa_recall_intent` 3 命名**:per `agent-diva-core/src/memory/provider.rs:247-275` 是**doc 注释里出现的字符串**,表明该 module 按 D-010 设计的边界命名;实际 Rust trait 方法是 `system_prompt_block` / `prefetch` / `sync_turn` / `on_session_end` 4 个 — **命名差异**:设计态用 3 命名,实现态用 4 方法。这不是 bug,而是 Rust trait 抽象层把 3 命名映射到 4 生命周期事件。

### 4.5 Mentle 边界矩阵（v0.2 新增, 2026-06-20）

**Mentle 跟其他 12 个组件的边界**（v0.1 仅覆盖 Laputa↔Mentle, v0.2 扩展到全部相关组件）：

| 组件 | 边界 | 来源 | 当前覆盖 |
|------|------|------|---------|
| **Laputa** | Laputa 写 authority, Mentle 仅 recall (FR-6xx 3 条) | `prd-laputa-2026-06-12/prd.md:288-301` | ✅ §4.1 + §4.2 + §4.3 详 |
| **AutoDream** | AutoDream 可调 Mentle s/r/w, 但**不能授权 durable change**; worker 6 允许工具 / 8 禁用工具清单含 Mentle | `autonomous-evolution-simplified-architecture-decision.md:405-422` + `autodream-rhythm-distillation-design.md:658-669` | ⚠️ §3 partial, 工具白/黑名单未引 |
| **Context Compaction** | Compaction **永不**调 Mentle, 永不进 Mentle 索引 (MVP); P1+ 才考虑 Mentle 索引 compact summary | `context-compaction-research.md:719` + `§10.1` | ❌ 零覆盖 |
| **Source Capsule** | MVP capsule 不使用 Mentle, 存纯文件 `.agent-diva/compact/capsules/`; Phase 4 走 Mentle diary API 建全文索引 | `compression-research.md:514-518` + `§7.4-5` | ❌ 零覆盖 |
| **MEMORY.md 渲染** | palace status 只放轻量统计; heavy recall 只走 `prefetch()`, **不**注入 startup prompt | `shared-memory-rendering-research.md:419-438` (Section 12) | ⚠️ §6 partial |
| **Report System** | daily/weekly/monthly report 走 Laputa section #7/#8/#9; Mentle 可索引 AAAK summary 但 report 实体留 Laputa; 写失败时降级到 `pending.jsonl` | `prd-report-system/` + `reconcile-mentle-laputa.md:40` | ⚠️ §3 partial, `pending.jsonl` 未明示 |
| **Report System (新架构验证)** | D-015 (2026-06-21): 新架构在hermes-laputa-python中验证，Rust版维持原架构；Python版报告走mempalace wings (daily/weekly/monthly)，Rust版报告走Laputa section #7/#8/#9 | LAPUTA.md §10.1 D-015 | 🔄 验证中 |
| **Heartbeat / 失败降级** | Mentle 失败不阻塞 AgentLoop (4 降级规则之一); AuditEvent::ToolError 记录但不阻断 | `autonomous-evolution-simplified-architecture-decision.md:601-605` + DECISION-v2 §5 | ❌ 零覆盖 |
| **Harness Module 生命周期** | MentleToolRuntimeConfig 是 Module trait 实现之一 (`agent-diva-tooling/src/module.rs` + `inventory` 静态注册); start/stop 走统一 lifecycle | DECISION-v2-next-phase.md §5.1 | ❌ 零覆盖 |
| **Poke 8 事件链** | Mentle 工具调用 emit TokenUsed + ToolInvoked; 跟 5.5 行为审计配合 | DECISION-v2-next-phase.md §5.8 + §5.5 | ❌ 零覆盖 |
| **行为审计 (走 core/logging.rs)** | Mentle 工具调用 → `AuditEvent::ToolInvoked{tool, args_hash, timeout_ms}` / 失败 → `ToolDenied`; **不走** Laputa AuditEvent | DECISION-v2-next-phase.md §5.5 (用户拍板) | ❌ 零覆盖 |
| **Configuration 5 开关** | (1) Laputa 启用 / (2) rhythm 启用 / (3) AutoDream manual/heartbeat 启用 / (4) **Mentle tool-call 启用** / (5) **Mentle 默认 context 注入必须 false** | `autonomous-evolution-simplified-architecture-decision.md:607-612` | ⚠️ §3.7 partial (FR-603 仅 1 条) |
| **3 态用户存在** | Active 不触发 Mentle / Distracted 慢频 Mentle recall / Gone 自由活动可 Mentle recall | `autonomous-activity-thoughts.md:48-67` | ❌ 零覆盖 |
| **Candidate / Evidence / Audit 链** | EvidenceRef 含 `Capsule` source type; Mentle palace writes **永不**写 MEMORY.md; audit 事件 `capsule_consumed` 记录 Mentle 读取 | `candidate-evidence-journal-audit-design.md:104-110` + `§6.1-3` + `§8.2` | ❌ 零覆盖 |
| **Obsidian** | Laputa=authority / Obsidian=human-facing / Mentle=machine-facing 三向 split | 见 §3.9 (本索引 v0.2 新增) | ⚠️ §3.9 partial |

**总览**：14 项边界, v0.1 完整覆盖 1 项 + partial 5 项 + 零覆盖 8 项 = Laputa↔Mentle 8% 完整覆盖。

**v0.2 改进**：frontmatter 加 7 份新 source_of_truth + 2 份新 related, 把 14 项显式列出 (本节), 后续 follow-up PRD / implementation story 直接引用本节。

---

## 5. 调度记忆 CRUD 的规则(用户特别要求)

> ⚠️ **本节是设计态**,per `mentle-laputa-memory-role-decision.md` 决策,综合治理推迟到 follow-up PRD。代码里 sync_turn 已经**部分实现** write 行为(per `hybrid.rs:436-479` 调 `memtle_diary_write`),但治理(gating / review / 拒绝)未实接。

### 5.1 add(添加记忆)

| 谁触发 | 走哪条路径 | 写权限决策 | 与 Laputa authority 边界 |
|---|---|---|---|
| **agent loop sync_turn** | `MemoryProvider::sync_turn` → `HybridMemoryProvider::sync_turn` → **双写**: `file_manager.append_history` (L0/L1 authoritative) + `toolkit.call_json("memtle_diary_write", ...)` (L2 best-effort) (per `hybrid.rs:436-479`) | **当前实接**: 无 gating,同步直写 + 失败降级<br>**设计态**: 9 该写白名单 (per §3.2),需 harness / prompt policy 判别 | Laputa **不**参与(append_history 写 HISTORY.md,**不**走 LaputaWrite::apply_proposal) — 这意味着 HISTORY.md 写入**绕过 Laputa 治理** |
| **user 显式 "记住这个"** | agent loop 解析 user intent → harness 决策 → 9 该写白名单之一 → 走 `memtle.write(note)`(工具面) | 设计态: 走 `mentle.write` 工具面,经白名单校验后落 palace | 严禁走 LaputaWrite::apply_proposal (per FR-601 authority 单向) |
| **selfinprove proposal** | selfinprove FR-5xx proposal_inbox → Laputa 审 → LaputaWrite::apply_proposal → **写 Laputa, 不写 Mentle** (per `mentle_governance.rs:48-86` 静态断言 Laputa 写不创建 palace state) | Laputa 审稿人 6 道治理 (per LAPUTA.md §3) | **写 Laputa authority**(per FR-601 边界) — **严禁**走 Mentle |
| **AutoDream report 写** | AutoDream write_daily/weekly/monthly → 走 Laputa 写入口 (per `mentle_governance.rs:88-118` 断言 AutoDream 写不创建 palace state) | AutoDream 4 thin + 8 should-not-own 负向契约(转 AutoDream PRD) | 写 Laputa authority (daily/weekly/monthly report 走 Laputa section #7/#8/#9) — 严禁写 palace |

### 5.2 delete(删除记忆)

| 谁触发 | 走哪条路径 | 写权限决策 | 与 Laputa authority 边界 |
|---|---|---|---|
| **agent loop 直删** | 当前实接**未实现** delete 路径(per `hybrid.rs`,sync_turn 只追加 diary 不删) | 设计态: delete_or_archive(per 决策 §4 5 wing) | 严禁走 Laputa 删除(per FR-601,Laputa 走 30 天回滚,不是删) |
| **user 显式 "忘掉这个"** | 设计态: 走 `mentle.delete_or_archive(id)` 工具面 | 走 review queue(per Laputa 写治理 6 道) | Laputa 走 FR-403 rollback (30 天内) |
| **selfinprove proposal 撤销 Laputa** | selfinprove proposal_inbox → LaputaWrite::apply_proposal (deprecation 类型) → **仅写 changelog,不动正文** (per LAPUTA.md §2.1 8 种 proposal_type 路由) | Laputa 6 道治理 | Laputa 写 changelog,**不动 Laputa section 正文** (per `prd-laputa-2026-06-12/prd.md` 8 种路由) |
| **AutoDream / 报告归档** | AutoDream PRD 转,本节不展开 |  |  |

### 5.3 modify(修改记忆)

| 谁触发 | 走哪条路径 | 写权限决策 | 与 Laputa authority 边界 |
|---|---|---|---|
| **agent loop 直改** | 当前实接**未实现** modify 路径(per `hybrid.rs`,sync_turn 只追加不覆盖) | 设计态: `mentle.update(id, patch)` (per 决策 §4 5 wing) | 严禁直改 Laputa section(per FR-509,用户不可直改 #1/#2/#5/#6) |
| **user 显式 "改这个"** | 设计态: 走 `mentle.update` 工具面 | 走 review queue(per Laputa 写治理 6 道) | Laputa 走 selfinprove proposal flow(per FR-509) |
| **selfinprove proposal** | selfinprove proposal_inbox → Laputa 审 → LaputaWrite::apply_proposal → patch target_section (per LAPUTA.md §2.1 8 种 proposal_type 路由,memory_patch → #5 memory_md) | Laputa 6 道治理 | **写 Laputa authority** |

### 5.4 query(查询记忆)

| 谁触发 | 走哪条路径 | 写权限决策 | 与 Laputa authority 边界 |
|---|---|---|---|
| **AgentLoop startup (laputa_wakeup / laputa_project_soul)** | `MemoryProvider::system_prompt_block` → `HybridMemoryProvider::system_prompt_block` → 拼接 file_markdown + palace_markdown(per `hybrid.rs:336-369`) | 无 gating,启动时注入 | Laputa 读 + Mentle 读,**只读不写** |
| **per-turn intent (laputa_recall_intent)** | `MemoryProvider::prefetch(intent, room)` → `HybridMemoryProvider::prefetch` → `toolkit.search(SearchArgs {query, limit: 5, wing, room, context})` (per `hybrid.rs:380-413`) | 空白 intent → `SkippedNoIntent`(per `hybrid.rs:372-377`) | Laputa 读 + Mentle 读 |
| **用户显式 "回忆我的设置"** | design: `mentle.search(query, filters)` 工具面 (per §3.1) | per FR-603 默认 on_demand,用户显式触发才进 | 仅 recall,不写 |
| **4 阶段 staged retrieval** | 设计态 (per §3.5) | 设计态:index → AAAK → full report → source sessions | Mentle index + Laputa 报告,4 阶段能力级 |

### 5.5 写权限决策总览

| 类型 | 走 Laputa 写入口 (FR-101) | 走 Mentle write 工具面 | 双写 (Laputa + Mentle) | 走 review queue | 拒绝 |
|---|---|---|---|---|---|
| identity / relationship / commitment / preferences / memory_md / history_md (Laputa-owned 6 section) | ✅ 必须走 (per FR-601) | ❌ 严禁 | ❌ | ✅ Laputa 6 道治理 (per LAPUTA.md §3) | ❌ |
| daily/weekly/monthly report (Laputa #7/#8/#9) | ✅ Report System 走 | ❌ 严禁 | ❌ | ✅ AutoDream 4 thin | ❌ |
| journal/proposal_inbox/changelog/report_indexes/AAAK (Laputa TBD #10-14) | ✅ 通用入口 (per FR-503) | ❌ 严禁 | ❌ | ✅ 通用 section 写入接口 | ❌ |
| Mentle 9 该写白名单 (per §3.2) | ❌ 严禁 (per FR-601) | ✅ 走 `mentle.write` | ❌ | ❌ | ❌ |
| Mentle 8 不该写黑名单 (per §3.3) | ❌ 严禁 | ❌ 严禁 | ❌ | ❌ | ✅ 静默拒绝 (per 决策 §5 硬规则) |
| casual 聊天 (per §3.8) | ❌ 严禁 | ❌ 严禁 | ❌ | ❌ | ✅ 反向保护 (per "casual→identity" 规则) |
| work_memory 7 字段 (per §3.4) | ❌ 严禁(暂存,不走 authority) | ✅ `mentle.write` (设计态) | ❌ | ❌ (task end 时由 AutoDream 决定) | ❌ |

---

## 6. Mentle 内部模型(`memtle 0.1.2` published crate,运行时模型)

> 概念层级 (per `diva-dev-ultra/docs/core-modules/mentle.md:18-30`):

| 概念 | 含义 | 类比 |
|---|---|---|
| **Palace** | Mentle 数据库实例, 路径 `workspace/memory/palace.db` | 一个 SQLite 文件 |
| **Wing** | 记忆的顶级分类 (`history` / `project` / `personal` 等) | 顶层目录 |
| **Room** | Wing 下的子分类 (e.g. `history/diary` / `history/facts`) | 二级目录 |
| **Drawer** | 具体的记忆条目 (i64 计数, per `hybrid.rs:21`) | 单个文件 |
| **Graph** | Room 之间的关联关系 (edges_total 计数, per `hybrid.rs:24`) | 链接 |

**L0/L1/L2 三层架构** (per `diva-dev-ultra/docs/core-modules/mentle.md:84-101`):

```text
┌─────────────────────────────────────────────┐
│             HybridMemoryProvider            │
├──────────────┬──────────────────────────────┤
│  L0/L1 层    │         L2 层               │
│  文件记忆     │       Mentle 宫殿           │
│              │                              │
│  MEMORY.md   │   palace.db (SQLite)        │
│  HISTORY.md  │   ├─ Wing: history          │
│  .agent-diva │   │  ├─ Room: diary         │
│   /memory/   │   │  └─ Room: facts         │
│              │   └─ Wing: project          │
│              │      └─ Room: roadmap       │
├──────────────┴──────────────────────────────┤
│           MemoryProvider trait              │
│  system_prompt_block()  prefetch()          │
│  sync_turn()           on_session_end()     │
└─────────────────────────────────────────────┘
```

**3 状态 PalaceStatusSnapshot** (per `hybrid.rs:60-68`):

```text
Ready(snapshot)              ← 启动成功,数据新鲜
Stale {snapshot, error}      ← 有旧数据但刷新失败
Degraded {reason}            ← 从未成功获取
```

---

## 7. Mentle 工具面 — 实现态细节(per `mentle_runtime.rs` 456 行)

### 7.1 5 类错误阶段(`MentleErrorPhase`,per `mentle_runtime.rs:136-152`)

```text
StartupOpen          // 启动时打开 database
ToolDefinition       // 解析 tool_definitions
ToolCallTransport    // 工具调用 transport (call_json)
ToolCallPayload      // 工具调用返回值
```

### 7.2 10 类错误分类(`MentleErrorCategory`,per `mentle_runtime.rs:155-183`)

Io / Database / Json / Config / InvalidArguments / UnknownTool / NotFound / InvalidDefinition / ToolPayload / Internal

### 7.3 3 类降级(`MentleFallbackAction`,per `mentle_runtime.rs:186-200`)

```text
DisableMentle   ← 启动失败 → 整个 Mentle runtime = None, fallback to file-only memory
SkipTool        ← 单个 tool_definitions 解析失败 → 跳过该 tool,其他继续
ReturnToolError ← call_json 失败 → 返 ToolError::ExecutionFailed 给 agent
```

### 7.4 工具发现 + 注册(per `mentle_runtime.rs:422-456` + `mentle_discovery.rs:12-23`)

```text
discover_mentle_tool_names(workspace):
  1. 拼 db_path = workspace/memory/palace.db
  2. create_dir_all(parent)  // 失败返空 vec
  3. MemtleToolkit::open(&db_path)  // 失败返空 vec
  4. tool_definitions()  // Vec<serde_json::Value>
  5. 过滤 name.starts_with("memtle_"),去重,排序
  6. 返 Vec<String>
```

**关键不变量** (per `13-s3-a1-memtle-toolkit-tool-interface.md` 冻结):
- 工具数不固定,不依赖具体 wing 名称
- 工具定义只读 3 字段: `name` / `description` / `inputSchema`
- 执行只能走 `MemtleToolkit::call_json(name, args).await`
- toolkit handle 固定为 `Arc<tokio::sync::Mutex<MemtleToolkit>>`

---

## 8. Mentle 配置(`MentleToolRuntimeConfig`,per `tool_config/mentle.rs` 139 行)

### 8.1 4 类 mode(`MentleToolMode`,per `tool_config/mentle.rs:17-22`)

```rust
pub enum MentleToolMode {
    Off,       // 完全禁用
    ReadOnly,  // 只读: 仅 memtle_status + memtle_search
    Full,      // 全功能: 所有 memtle_*
    Custom,    // 自定义: 仅 allowed_tools
}
```

### 8.2 READ_ONLY 预设白名单(per `tool_config/mentle.rs:6`)

```rust
const READ_ONLY_TOOLS: &[&str] = &["memtle_status", "memtle_search"];
```

> ⚠️ **read_only 仅 2 个工具**,**不含** `memtle_diary_write` (sync_turn 内部直接调,不走工具面 gating)。这是 sync_turn 的**特殊通道** — 设计态 vs 实现态差异。

### 8.3 工具过滤逻辑(`allows_tool`,per `tool_config/mentle.rs:49-63`)

```rust
pub fn allows_tool(&self, name: &str) -> bool {
    if !name.starts_with("memtle_") { return false; }  // 只处理 memtle_* 前缀
    match self.mode {
        Off => false,
        ReadOnly => READ_ONLY_TOOLS.contains(&name),
        Full => true,
        Custom => self.allowed_tools.iter()
            .any(|allowed| allowed == name && allowed.starts_with("memtle_")),
    }
}
```

### 8.4 配置示例(per `25-s7-a1-mentle-tool-selection-and-gui-controls.md:35-39`)

```toml
[mentle]
enabled = true
mode = "read_only"
allowed_tools = ["memtle_status", "memtle_search", "memtle_read"]
```

### 8.5 兼容旧格式(per `tool_config/mentle.rs:26-33`)

```rust
pub fn from_config(config: &Config) -> Self {
    let mut runtime = Self::from_core(&config.mentle);
    if runtime.is_default_off() && config.tools.builtin.mentle {
        runtime.enabled = true;
        runtime.mode = MentleToolMode::Full;
    }
    runtime
}
```

> 兼容旧 `tools.builtin.mentle = true` 格式 → 默认转 `Full` mode。

### 8.6 subagent 默认不继承(per `sprint5-default-check` 7 个回归测试)

`test_subagent_does_not_receive_mentle_by_default` / `test_build_agent_tools_reuses_custom_tools_with_cron` / `test_register_default_tools_preserves_custom_tools_with_cron` / `test_build_subagent_prompt_omits_mentle_routing` — subagent 默认**不**接收 Mentle 工具。

---

## 9. CI / Build 跨平台(per `scripts/ci/verify_mentle_package_policy.py` + `justfile:33-52`)

### 9.1 静态护栏:4 项

| 检查 | 实现 | 失败行为 |
|---|---|---|
| root `Cargo.toml` 钉 `memtle = "0.1.2"` + `default-features = false` | `verify_mentle_package_policy.py:12-17` | `sys.exit` |
| root `Cargo.toml` 不在 `[patch.crates-io]` 段 override | `verify_mentle_package_policy.py:19-22` | `sys.exit` |
| 所有子 `Cargo.toml` 不含 `path` 或 `git` 依赖 | `verify_mentle_package_policy.py:24-33` | `sys.exit` |
| `Cargo.lock` 从 crates.io registry 解析 memtle 0.1.2 | `verify_mentle_package_policy.py:35-41` | `sys.exit` |

### 9.2 justfile 5 个 recipe

| Recipe | 命令 | 何时跑 |
|---|---|---|
| `mentle-package-policy` | `python scripts/ci/verify_mentle_package_policy.py` | CI gate |
| `sprint5-default-check` | 9 个 default-lane 回归测试 | CI gate |
| `mentle-check` | `cargo check -p agent-diva-agent --features mentle` + 2 个 `--features mentle` test | CI gate (mentle lane) |
| `sprint5-check` | `fmt-check` + `sprint5-default-check` + `mentle-check` | CI gate (Sprint 5) |
| `epic6-release-gate` | 含 3 个 `mentle_governance*` 测试 + service / migration / storage / apply / authority_boundaries | RC release gate |

### 9.3 跨平台(per `22-s4-a10-mentle-feature-build-env.md:11-20`)

| 平台 | Rust | Native toolchain | 备注 |
|---|---|---|---|
| Linux | 1.80+ (default lane) / 1.88+ (mentle lane) | gcc | CI 跑 default + mentle 双 lane |
| macOS | 1.80+ / 1.88+ | clang (Xcode CLT) | CI 跑双 lane |
| Windows | 1.88+ | **`clang-cl.exe`** 必需 | PATH 加 `C:\Program Files\LLVM\bin\`;否则 `cc-rs: failed to find tool "clang-cl.exe"` |

**当前 host** (per AGENTS.md): Rust `1.93.0 (254b59607 2026-01-19)`,Windows 编译器在 `C:\Program Files\LLVM\bin\clang-cl.exe`。

---

## 10. Mentle 治理测试(3 个反向 governance,per §2.6)

| 测试 | 验证内容 | 关键断言 |
|---|---|---|
| `agent-diva-laputa/tests/mentle_governance.rs` | Laputa 写 authority 时不创建 palace.db / .mentle state;laputa crate 不依赖 memtle | `assert_no_mentle_state` (line 43-46) + `laputa_governance_crate_does_not_depend_on_mentle` (line 89-94) |
| `agent-diva-autodream/tests/mentle_governance.rs` | AutoDream 写 proposal / rhythm report 不创建 palace.db / .mentle state;autodream crate 不依赖 memtle | `output_emission_and_proposal_creation_do_not_create_mentle_state` (line 48-86) + `rhythm_report_writes_do_not_sync_to_mentle_state` (line 88-118) + `autodream_crate_does_not_depend_on_mentle` (line 120-126) |
| `agent-diva-agent/tests/mentle_governance_boundaries.rs` | Mentle prompt 路由不暴露 raw `memtle_search` / `memtle_kg_query` / `memtle_*` / `mentle recall` / `palace memory` 字符串 | `enabled_mentle_runtime_does_not_expose_default_recall_routing_in_governance_prompt` (line 102-123) + `enabled_mentle_search_without_status_is_not_a_governance_runtime_surface` (line 125-137) |

**测试覆盖矩阵**:

| 谁 | 写 Laputa | 写 Mentle | 验证手段 |
|---|---|---|---|
| LaputaService::apply_proposal | ✅ | ❌ (mentle_governance.rs:48-86) | 反向 |
| AutoDreamOutputEmitter | ✅ (Laputa) | ❌ (mentle_governance.rs:48-86) | 反向 |
| AutoDreamReportWriter | ✅ (Report) | ❌ (mentle_governance.rs:88-118) | 反向 |
| HybridMemoryProvider::sync_turn | ❌ (L0/L1 file) | ✅ (memtle_diary_write) | 正向(设计态有 gating 但未实接) |
| HybridMemoryProvider::prefetch | ❌ | ✅ (memtle search) | 正向 |
| HybridMemoryProvider::system_prompt_block | ❌ (L0/L1 read) | ✅ (palace snapshot read) | 正向 |

---

## 11. 关键决策时间线(8 条,2026-05-31 → 2026-06-19)

| # | 日期 | 决策 | 谁拍 | 来源 (path:line) | 关键产出 |
|---|---|---|---|---|---|
| 1 | 2026-05-31 | **Mentle / Laputa / AutoDream / Harness 边界决策** accepted | (大湿 隐式) | `agent-diva-selfinprove/docs/dev/genericagent/mentle-laputa-memory-role-decision.md:3` (status: accepted) | 5 wing / 9 该写 + 8 不该写 / work_memory 7 字段 / 4 阶段 retrieval / Authority 4 级 / 5 MUST NOT (本索引 §3 全部来源) |
| 2 | 2026-06-12 | **Laputa PRD §5.6 FR-6xx 锁 3 条边界** | 大湿 (隐式) | `docs/prds/prd-laputa-2026-06-12/prd.md:288-301` | FR-601 authority 单向 / FR-602 recall 走 HTTP / FR-603 on_demand 默认 + `always` 不进 enum |
| 3 | 2026-06-12 | **IR-2 reconcile-mentle-laputa.md 锁定 8 项 gap** | 子 agent | `docs/prds/prd-laputa-2026-06-12/reconcile-mentle-laputa.md:9-18` | 主体方向一致, **PRD 完全漏掉**: 5 wing 工具面契约 / 9+8 写白黑名单 / work_memory 7 字段 / 4 阶段 retrieval / Authority 4 级 ordering / casual→identity 反向保护 / Obsidian deferred / phasing 段落 |
| 4 | 2026-06-12 | **Laputa PRD D-008 OQ#17-18 转 follow-up Mentle 集成 PRD** | 大湿 (隐式) | `docs/prds/prd-laputa-2026-06-12/prd.md:371-372` (OQ#17: 5 wing + 写白黑名单 + work_memory + 4 阶段 retrieval) + `prd.md:372` (OQ#18: Authority 4 级 + 5 MUST NOT + casual→identity) | 决策: **5 wing 写白黑名单 / work_memory 7 字段 / Authority 4 级 等留待 follow-up PRD** |
| 5 | 2026-06-12 | **Laputa PRD §4.3 Non-Goal 标 "Mentle 实现本身 (已实现, 本 PRD 不重写)"** | 大湿 (隐式) | `docs/prds/prd-laputa-2026-06-12/prd.md:95` | 工具接入 Sprint 1-7 RC 已完成,Laputa PRD 不背 Mentle 内部 |
| 6 | 2026-06-15 | **pro repo workspace 初始化 + AGENTS.md 钉 memtle 0.1.2 策略** | (morediva 团队) | `agent-diva-pro/AGENTS.md:36` | "Mentle integration is intentionally pinned to the published `memtle = 0.1.2` crate; do not replace it with a path/git override" |
| 7 | 2026-06-18 | **DECISION.md (alife v1) 标 memtle = 0.1.2 为"已完成 + 超越"** | 大湿 (隐式) | `agent-diva-pro/DECISION.md:19, 83` (2.4 长期记忆) + 83-85 (2.6 自主活动) | 4 长期记忆: laputa PRD 已落地 + 超越 alife 路径;2.6 自主活动: 基于已完成的 laputa 和 mentle, **待架构设计** |
| 8 | 2026-06-19 | **用户拍板 "Mentle 综合治理推迟 + 现状仅工具接入" + 本 MENTLE.md 索引编写** | 大湿 | 本文件 + 用户本轮 prompt | ① 综合治理推迟到 follow-up PRD (per OQ#17-18) ② 工具接入基线盘点 ③ 与 LAPUTA.md 同步索引风格 |

---

## 12. Mentle ↔ Laputa 状态对比(本索引核心,跟 LAPUTA.md D 列表 baseline 校核)

> ⚠️ **本节是验证 LAPUTA.md 抽出的 D 列表(用户 prompt D 部分)与代码 + IR-2 文档的一致性**。

| 项 | LAPUTA.md / IR-2 抽出的描述 | 代码 / CI / 测试 实际 | 一致? |
|---|---|---|---|
| **Mentle 5 wing** (search/read/write/update/delete) | OQ#17 转 follow-up;reconcile IR-2 标"PRD 完全漏掉" | 当前实接**不是** 5 wing 硬编码,而是 `memtle_*` 动态注册 (per `memtle 0.1.2` crate 决定);工具面契约冻结在 `13-s3-a1-memtle-toolkit-tool-interface.md` | ⚠️ **设计态 vs 实现态 drift** (设计=5 wing,实现=动态) — 不是错误,是抽象层把"5 wing 契约"降级为"memtle_* 前缀 + 3 字段接口" |
| **写白名单 9 该写 + 黑名单 8 不该写** | OQ#17 转 follow-up;reconcile IR-2 标"PRD 零覆盖, governance 缺位" | **代码实接**:`hybrid.rs:436-479` sync_turn 调 `memtle_diary_write` 直写 **无 gating** | ⚠️ **设计态 vs 实现态 drift** (设计=白黑名单 gating,实现=best-effort 直写,失败降级) — 是 follow-up PRD 待补 |
| **work_memory 7 字段** | OQ#17 转 follow-up;reconcile IR-2 标"PRD 无 FR 描述 work memory" | **代码实接**:**未实现** (sync_turn 只写 history_entry,无 7 字段结构) | ❌ **未实现** — 推迟到 follow-up PRD |
| **Authority 4 级 ordering** | OQ#18 转 follow-up;reconcile IR-2 标"partial" | **代码实接**:`hybrid.rs` 拼装时 **先** `file_manager` (L0/L1) **后** `palace_snapshot` (L2),**隐含** Authority L0>L2 ordering | ⚠️ **部分实接** (隐含 ordering),但 4 级清单 + 5 MUST NOT 否定规则**未显式落到代码** |
| **5 MUST NOT 否定规则** | OQ#18 转 follow-up;reconcile IR-2 标"无显式否定契约" | **代码实接**:`mentle_governance.rs:48-94` 静态断言 Laputa / AutoDream crate **不**依赖 memtle → **隐含** MUST NOT 1 (Laputa 不被 palace 污染);其他 4 条 MUST NOT **未实接** | ⚠️ **部分实接** (1/5) |
| **"casual→identity" 反向保护** | reconcile IR-2 标"missing in PRD (governance gap)" | **代码实接**:**未实接** (per `mentle_runtime.rs` / `hybrid.rs`,无 casual 检测) | ❌ **未实现** — 推迟到 follow-up PRD |
| **4 阶段 staged retrieval** | reconcile IR-2 标"missing in PRD" | **代码实接**:`hybrid.rs:336-413` 走 2 阶段 (system_prompt_block 拼 + prefetch palace search),**不**是 4 阶段 (index → AAAK → full report → source sessions) | ⚠️ **设计态 4 阶段,实现态 2 阶段** — 推迟到 follow-up PRD |
| **召回策略: 默认 on_demand, `always` 不进 enum** | FR-603 锁 | **代码实接**:`mentle_active()` 是 boolean,无 on_demand/always 枚举;Laputa PRD state.json `mentle_recall_policy` 字段**未实接** | ⚠️ **设计态有 enum,实现态是 boolean** — FR-603 部分实接 |
| **Mentle 召回 MUST 走 FR-2xx HTTP 客户端, 不得 fs::read** | FR-602 锁 | **代码实接**:`hybrid.rs::prefetch` **不**走 LaputaReadAdapter,而是直接调 `memtle::tools::SearchArgs` (palace 内部 search) | ❌ **设计态 vs 实现态 drift** — HybridMemoryProvider 走 palace 内部 search,不走 Laputa FR-2xx HTTP |
| **Laputa 写, Mentle 仅 recall (authority 单向)** | FR-601 锁 | **代码实接**:`mentle_governance.rs:48-94` 静态断言 lap crate 不含 memtle;`mentle_runtime.rs` 不调 `LaputaWrite::apply_proposal` | ✅ **完全实接** |
| **`mentle_active()` 召回开关** | FR-601, agent_loop.rs:570 (LAPUTA.md 写的) | **实际** agent_loop.rs:**591** (LAPUTA.md 写 570 是过时的,本索引用 591) | ⚠️ **LAPUTA.md 行号需更新** (570 → 591) |
| **3 hook 集成 (laputa_wakeup / project_soul / recall_intent)** | FR-507, provider.rs:247-275 | **实际** `provider.rs:247-275` 是 doc comment,3 命名 **不在 Rust 代码中**;实际 trait 方法是 `system_prompt_block` / `prefetch` / `sync_turn` / `on_session_end` 4 个 | ⚠️ **命名差异** (3 命名 vs 4 方法) — 不是 bug,设计态命名是 domain 抽象,实现态命名是 lifecycle 抽象 |
| **`memtle = 0.1.2` 来自 crates.io** | AGENTS.md 锁 | **实际** `Cargo.toml:87-89` 钉死 + `verify_mentle_package_policy.py:12-41` 4 项静态护栏 | ✅ **完全实接** |
| **3 个 mentle_governance 测试** | (本索引 §2.6) | `agent-diva-laputa/tests/mentle_governance.rs` (94 行) + `agent-diva-autodream/tests/mentle_governance.rs` (126 行) + `agent-diva-agent/tests/mentle_governance_boundaries.rs` (137 行) | ✅ **完全实接** |
| **Mentle 工具选择 GUI (S7-A1)** | S7-A1 §Implementation Status | `agent-diva-gui/src/components/settings/MentleSettingsCard.vue` (247 行) | ✅ **完全实接** |
| **Mentle 综合治理(authority 4 级 / 写白黑名单 / casual→identity)** | 用户拍板**推迟**到 follow-up PRD | 当前**未实接** | 🟡 **明确推迟** (per 用户拍板 2026-06-19) |
| **Mentle ↔ AutoDream 边界 (6 允许 / 8 禁用工具清单)** | autonomous-evolution §12 + autodream-rhythm-distillation §7.2 | `MentleToolRuntimeConfig` 4 mode 已有, 但 worker 6/8 清单未实接 | ⚠️ 部分实接 (本索引 §4.5 v0.2 增) |
| **Mentle ↔ Context Compaction 边界 (compaction 永不调 Mentle)** | context-compaction-research §10 | 实接 (compaction 不调 MemoryProvider, 自然不调 Mentle) | ✅ 隐含实接 (本索引 §4.5 v0.2 增) |
| **Mentle ↔ Source Capsule 边界 (MVP 不使用 Mentle)** | compression-research §7.4 | 实接 (capsule 存纯文件, Phase 4 才走 Mentle) | ✅ MVP 实接 (本索引 §4.5 v0.2 增) |
| **Mentle ↔ MEMORY.md 渲染边界 (palace status 轻量, recall 走 prefetch)** | shared-memory-rendering-research §12 | `HybridMemoryProvider` 拼接已部分实接, 但 "轻量统计 vs heavy recall" 分级未明示 | ⚠️ partial (本索引 §4.5 v0.2 增) |
| **Mentle ↔ Heartbeat 失败降级 (4 降级规则之一)** | autonomous-evolution §20.7 | `hybrid.rs:436-479` sync_turn 失败降级已实接, 但 heartbeat tick → Mentle 调用未实接 | ⚠️ partial (本索引 §4.5 v0.2 增) |
| **Mentle ↔ Harness Module 生命周期** | DECISION-v2 §5.1 (本 wave P0) | **未实接** (DECISION-v2 §5.1 待开干) | ❌ 零实接 (本索引 §4.5 v0.2 增) |
| **Mentle 行为审计 (AuditEvent::ToolInvoked)** | DECISION-v2 §5.5 (本 wave P0) | **未实接** (DECISION-v2 §5.5 待开干) | ❌ 零实接 (本索引 §4.5 v0.2 增) |
| **Mentle Configuration 5 开关** | autonomous-evolution §20.8 | 仅 FR-603 mentle_recall_policy 1 条实接; 其余 4 开关零实接 | ⚠️ 1/5 实接 (本索引 §4.5 v0.2 增) |
| **Mentle ↔ 3 态用户存在** | autonomous-activity-thoughts §2 | **未实接** (presence 层未起) | ❌ 零实接 (本索引 §4.5 v0.2 增) |
| **Mentle ↔ Candidate/Evidence/Audit 链** | candidate-evidence-journal-audit §3/§6/§8 | `EvidenceRef` 已有, 但 Capsule source type + Mentle 不写 MEMORY.md 治理测试未实接 | ⚠️ partial (本索引 §4.5 v0.2 增) |
| **Obsidian 三向 split 集成** | mentle-laputa §11 (本索引 §3.9 v0.2 增) | **未实接** (Obsidian 集成 deferred) | 🟡 明确 deferred (本索引 §3.9 v0.2 增) |
| **Mentle Session-End 治理 (§15 MVP P0/P1/P2)** | 本索引 §15 (v0.2 正式化) | **未实接** (P0 ~80 LOC 待起 follow-up PRD) | ❌ 零实接 (本索引 §15 v0.2 正式化) |

**结论**:
- **完全实接** (✅): 5 项 — authority 单向、memtle 0.1.2 钉死、3 个治理测试、GUI 工具选择、mentle_active 开关
- **部分实接** (⚠️): 8 项 — 5 wing 设计 vs 动态注册、写白黑名单隐含实接、Authority 隐含 ordering、MUST NOT 1/5 实接、2 阶段 vs 4 阶段 retrieval、on_demand boolean vs enum、3 命名 vs 4 方法、LAPUTA.md 行号漂移 (570→591)
- **明确未实接** (❌): 4 项 — work_memory 7 字段、casual→identity 反向保护、4 阶段 retrieval (实现态 2 阶段)、Mentle recall 走 Laputa FR-2xx HTTP (实际走 palace 内部)
- **明确推迟** (🟡): 1 项 — Mentle 综合治理 (用户拍板)

---

## 13. 复盘触发器(什么时候该 v2 这个索引)

- [ ] **Mentle 集成 PRD (OQ#17-18) 起稿** — 5 wing 写白黑名单 / work_memory 7 字段 / Authority 4 级 / 4 阶段 retrieval 任一落地,本索引 §3 6 项契约需逐条更新
- [ ] **`memtle 0.1.2` → 0.2+ 升级** — `verify_mentle_package_policy.py:12` 钉死 version 需改,`Cargo.toml:89` 同步,IR-2 工具面契约需复核
- [ ] **Laputa 架构后续 PRD (OQ#12-14) 落地** — MemoryProvider 4 lifecycle 改名,本索引 §4.2 3 命名 hook 需同步
- [ ] **AutoDream PRD 落地** — rhythm report 写走 Laputa 入口契约变化,本索引 §5.1 / §10 测试矩阵需更新
- [ ] **`mentle_recall_policy` 字段实接到 Laputa state.json** — 当前 `prd.md:300` 设计有,代码无;一旦实接,本索引 §4.1 FR-603 状态从 ⚠️ 改 ✅
- [ ] **HybridMemoryProvider::prefetch 改为走 LaputaReadAdapter HTTP** — 当前走 palace 内部 search,一旦按 FR-602 改造,本索引 §4.1 FR-602 状态从 ⚠️ 改 ✅
- [ ] **4 阶段 staged retrieval 落地** — 当前 2 阶段 (system_prompt + prefetch palace search),一旦扩到 4 阶段 (index → AAAK → full report → source sessions),本索引 §3.5 需更新
- [ ] **Laputa PRD state.json `mentle_recall_policy` 字段** 出现 — 同步本索引 §4.1 实施态
- [ ] **AGENTS.md `mentle = 0.1.2` 升级** — 同步本索引 §2.1 + §11 时间线
- [ ] **MentleSettingsCard.vue 增减控件** — 同步本索引 §2.7
- [ ] **`laputa_wakeup` / `laputa_project_soul` / `laputa_recall_intent` 3 命名** 显式落到 Rust trait(当前仅 doc comment) — 同步本索引 §4.2 ⚠️
- [ ] **新加 `mentle_governance*` 测试**(当前 3 个) — 同步本索引 §2.6 / §10
- [ ] **v0.2 新增边界实接** (对应 §4.5 矩阵每项):
    - [ ] **Mentle ↔ AutoDream 6 允许 / 8 禁用工具清单**实接到 `MentleToolRuntimeConfig`(per autonomous-evolution §12) — 同步本索引 §4.5 partial → complete
    - [ ] **Context Compaction 永不调 Mentle** 显式约定实接 (加 lint 规则 / code review check) — 当前隐含, 需显式保护
    - [ ] **Source Capsule ↔ Mentle Phase 4 集成** (Mentle diary API 建全文索引) — 当前 MVP 不使用, Phase 4 落地时同步
    - [ ] **HybridMemoryProvider 渲染分级** (轻量统计 vs heavy recall) 显式分层 — partial, 需明示策略
    - [ ] **Heartbeat 调用 Mentle 失败降级** 路径实接 (per DECISION-v2 §5.7 presence 触发) — 当前未起 presence 层
    - [ ] **Mentle 作为 Module trait 实现之一** (per DECISION-v2 §5.1) — inventory 静态注册 + lifecycle — 本 wave P0 待开干
    - [ ] **Poke 8 事件链 TokenUsed/ReasoningReceived** 在 Mentle 工具调用后 emit (per DECISION-v2 §5.8) — 本 wave P0 待开干
    - [ ] **AuditEvent::ToolInvoked/ToolDenied** 在 Mentle 工具调用路径 emit, 写 `core/logging.rs` (per DECISION-v2 §5.5) — 本 wave P0 待开干
    - [ ] **Configuration 5 开关**全实接: Laputa / rhythm / AutoDream mode / Mentle tool-call / Mentle 默认注入 false — 当前 1/5
    - [ ] **3 态用户存在 → Mentle recall 时机**: Active 不触发 / Distracted 慢频 / Gone 自由 — presence 层 v0.2 后落地
    - [ ] **EvidenceRef::Capsule source type** + Mentle 不写 MEMORY.md 治理测试 (3 个 governance test 已实接, 需补 Capsule 来源治理)
    - [ ] **Obsidian 三向 split 集成** (per §3.9) — deferred, Laputa 架构后续 PRD 时落地
- [ ] **v0.2 §15 Session-End 治理 MVP 落地** (per §15.6):
    - [ ] **P0 ~80 LOC**: 扩展 `SessionEndRequest` 传 session_key / unconsolidated messages + `on_session_end` 压缩+路由+去重+写入+回写
    - [ ] **P1 ~30 LOC**: `system_prompt_block` 改为输出 wing/room/工具清单, 告诉 LLM "你有 N 个 wing, 可以用 memtle_search/diary_read/kg_query 按需查"
    - [ ] **P2 ~20 LOC**: `DEFAULT_MEMORY_WINDOW` 100 → 20~30 + session-end 强制触发 consolidation
    - 前置依赖: P0 需 `SessionEndRequest` 扩展 (改 `agent-diva-core/src/memory/provider.rs`); P0 路由逻辑需决定 LLM 路由 vs 规则引擎; P2 强制触发需 session 内容在 `on_session_end` 时仍可访问
- [ ] **Mentle 集成 PRD 落地** (本索引 v0.2 follow_up_prds 第 1 项) — 起 `prd-mentle-2026-06-XX.md` 跟 LAPUTA.md 平级, 本索引 §3.1-3.9 + §4.5 + §15 拆为正式 FR

---

## 14. 一句话哲学

> Mentle 是 Agent-Diva 的**嵌入式长期记忆库** (memtle = 0.1.2 published crate),通过 4 个源文件接入 AgentLoop,**不是大脑,不是治理系统,是一份由 Laputa 拥有写 authority、Mentle 仅 recall 的"工具面"**。综合治理已明确推迟到 follow-up PRD。

---

## 15. Session-End 治理分析与最小可行方案（v0.2 正式化, 2026-06-19 → 2026-06-20）

> 本节 v0.2 **正式化**为 Mentle 架构设计的核心方案（v0.1 标"暂存", v0.2 经大湿拍板"补 mentle 接入设计"后升格为正式设计态）。核心论点: **Session 和 Mentle 是两层不同存储, Mentle 的正确用法是 session_end 压缩写入 + 按需召回, 而不是启动注入。** 未来 follow-up Mentle 集成 PRD 需把本节拆为正式 FR。

### 15.1 双层存储分离原则

```text
┌─────────────────────────────────────────────────────────────┐
│  Session JSONL (.sessions/)                                 │
│  = 原始对话流水,一字不改,简单粗暴存储                       │
│  = 可回溯的审计日志                                        │
├─────────────────────────────────────────────────────────────┤
│  Mentle palace.db (workspace/memory/)                       │
│  = 压缩后的结构化记忆,快速索引 + 关联                      │
│  = Wing/Room/Drawer 分类 + KG 三元组 + Tunnel 连接          │
│  = "图书馆":存所有内容,但都是压缩过的                      │
├─────────────────────────────────────────────────────────────┤
│  MEMORY.md (workspace/memory/)                              │
│  = 小而精、权威、干净的启动上下文                           │
│  = 每次启动都注入到 prompt (L0/L1 权威层)                   │
└─────────────────────────────────────────────────────────────┘
```

**关键区分**:
- **Session** ≠ **Mentle**: Session 是审计日志 (raw), Mentle 是压缩索引 (curated)
- **MEMORY.md** 是"核心记忆" (启动注入), **Palace** 是"图书馆" (按需查询)
- Palace 治理: 快速索引 + 关联, 按 wing/room 分类, 用 KG 建立关系

### 15.2 启动时只注入 MEMORY.md, 告诉 LLM 怎么读 Mentle

**用户拍板**: 启动时注入 palace 内容 = 污染上下文窗口。Palace 可能有几千条 drawer, 全塞进 prompt 既浪费 token 又模糊焦点。

正确做法:
```text
启动注入:
  1. MEMORY.md 内容 (已有, 干净权威)
  2. Palace 概况: N 个 wing, M 个 room, 最近活动时间
  3. 告诉 LLM 可用工具 + 每个工具能查什么:
     - memtle_search(query) — 语义搜索, 需要回忆时用
     - memtle_diary_read(limit) — 读最近日记
     - memtle_kg_query(subject) — 查关系
     - memtle_get_drawer(id) — 读具体记忆
     - memtle_list_drawers(wing, room) — 浏览某个分类
```

**LLM 自己决定何时查 palace** (on_demand 原则的延伸):
- 用户提到某个项目 → LLM 主动调 `memtle_search(query="项目名")`
- 需要回忆最近做了什么 → `memtle_diary_read(limit=5)`
- 需要查某个关系 → `memtle_kg_query(subject="用户")`

### 15.3 Session-End 是唯一的 Palace 写入时机

```text
Session-End 流程 (目标态):
  1. 压缩: LLM 把会话内容压缩为结构化摘要
  2. 路由: 决定每个摘要写入哪个 wing/room
  3. 去重: check_duplicate(content) → 已有则 update_drawer, 否则 add_drawer
  4. 写入: add_drawer(wing, room, compressed_content) + diary_write(session_summary)
  5. 可选: kg_add(subject, predicate, object) 提取关系
  6. 回写: MEMORY.md 同步更新 (如果有值得进核心记忆的内容)
```

**Session-End 只写不读**: 写入 palace 时不需要读 palace, 只需要读 session 内容。

### 15.4 Palace 去重三层机制 (memtle 0.1.2 内置, 当前全闲置)

| 层级 | 机制 | 工具 | 行为 | 当前使用 |
|---|---|---|---|---|
| **L1 精确去重** | 确定性 SHA256 ID (`wing+room+content` → `drawer_id[:24]`) | `add_drawer` 内置 | 相同内容 → 相同 ID → 返回 `already_exists`, 不重复创建 | ❌ 没接 |
| **L2 近似去重** | 关键词重叠搜索, `relevance > 3.0` 判为重复 | `memtle_check_duplicate` | 返回 `is_duplicate: bool` + 匹配列表 | ❌ 没接 |
| **L3 更新替代** | 修改已有 drawer 的 wing/room/content, ID 重算 | `memtle_update_drawer` | 内容变化 → 新 ID → 如与新 drawer 冲突则拒绝 | ❌ 没接 |

**正确的 session_end 去重流程**:
```
对每条压缩摘要:
  a. check_duplicate(content)
  b. if is_duplicate=true → update_drawer 追加/更新已有 drawer
  c. if is_duplicate=false → add_drawer 创建新 drawer
  d. add_drawer 幂等: 完全相同 wing+room+content → already_exists
```

### 15.5 Session 边界感知现状 (P0 缺口)

| 问题 | 现状 |
|---|---|
| **session 结束判定** | 无显式标签, 靠 agent loop 停止时隐式触发 `on_session_end` |
| **session_id** | 硬编码 `"agent-loop-shutdown"`, 所有 session 共享同一标签 |
| **幂等机制** | `Manager` 内 `HashSet<String>` 仅在进程生命周期有效, 重启后清空 |
| **SessionEndRequest 内容** | 只传 `workspace_root` + `session_id`, **不传 session 内容** |
| **后果** | HybridMemoryProvider 在 `on_session_end` 时根本不知道要压缩什么 |
| **crash 场景** | kill -9 / 断电 → `on_session_end` 根本不调用 → 本会话记忆全部丢失 |

### 15.6 最小可行实施方案 (暂存, 待 follow-up PRD)

**P0 — Session-End 路由写入** (~80 LOC)
```text
1. 扩展 SessionEndRequest 传入 session_key 或 unconsolidated messages
2. HybridMemoryProvider::on_session_end:
   a. 取 session 未合并消息
   b. 调 LLM 压缩为结构化摘要 (复用 consolidation.rs 的 prompt 模式)
   c. 路由: LLM 决定 wing/room (或规则引擎 fallback)
   d. 去重: check_duplicate → add_drawer / update_drawer
   e. diary_write(session_summary)
   f. 回写 MEMORY.md (如有重要更新)
```

**P1 — 启动 Prompt 描述 Palace 能力** (~30 LOC)
```text
修改 HybridMemoryProvider::system_prompt_block 的 render_markdown:
  - 当前: 只输出 active_drawers / graph_rooms / graph_tunnels 统计数字
  - 改为: 输出 wing 列表 + room 列表 + 可用工具描述
  - 告诉 LLM: "你有 N 个 wing, 可以用 memtle_search/diary_read/kg_query 按需查"
```

**P2 — 降低 consolidation 阈值 + session-end 强制触发** (~20 LOC)
```text
1. DEFAULT_MEMORY_WINDOW: 100 → 20~30 (短会话不再丢记忆)
2. agent_loop 停止时: 如果 unconsolidated > 0, 强制触发一次 consolidation
```

**前置依赖**:
- P0 需要 `SessionEndRequest` 扩展 (改 `agent-diva-core/src/memory/provider.rs`)
- P0 的路由逻辑需要决定: LLM 路由 vs 规则引擎 (LLM 更灵活但多一次调用)
- P2 的强制触发需要 session 内容在 `on_session_end` 时仍然可访问

### 15.7 与既有架构决策的一致性

| 既有决策 | 本方案是否兼容 | 说明 |
|---|---|---|
| FR-601 authority 单向 (Laputa 写, Mentle 读) | ✅ | Mentle 写入不走 LaputaWrite::apply_proposal |
| FR-603 on_demand 默认 | ✅ | 启动不注入 palace 内容, LLM 按需查 |
| 3.7 5 MUST NOT | ✅ | Mentle 不静默改 MEMORY.md (回写是显式操作) |
| 3.2 写白名单 9 该写 | ✅ | session_end 路由可遵循白名单 |
| 3.3 写黑名单 8 不该写 | ✅ | 路由逻辑可拒绝 casual 内容 |
| 3.8 casual→identity 反向保护 | ✅ | 路由层可拦截 casual → 拒绝写入 |

### 15.8 用户原始想法提炼与 Agent 分析

> 本节记录用户原话 + agent 代码事实分析,保留讨论脉络。

#### 用户原始想法 (按讨论顺序)

**R1 — 核心诉求**:
> "为了保障 diva 本轮更新之后有基础的记忆治理能力, 例如我一个会话结束了之后, diva 要怎么记住本轮会话的内容, 从而在之后的会话中记住这些事情。新的会话, 不丢记忆, 而且知道自己是谁"

**R2 — 存储分离 + Mentle 定位**:
> "我觉得还是得分开。首先得明确 session 的存储和 mentle 的存储不是一回事, 治理方面的逻辑: mentle: 快速索引+关联, session 仅做简单粗暴存储。所以说, mentle 事实上的作用: 治理每一个对话的结尾, 应该写入哪个房间, 哪个翼。日记也是要放入 mentle 的, 我对 mentle 的理解: 放入所有内容, 但是都是压缩过后的内容。最后再根据需要进行召回。我漏了什么吗?"

**R3 — 启动只读 MEMORY.md, 告诉 LLM 怎么读 Mentle**:
> "不, 读取只读 memory 确保最干净的上下文。正确的用法: 应该是告诉 llm 怎么去读 mentle, 有什么可能的内容。"

**R4 — 三个关键问题**:
> "session 结束的判定标签你知道是啥吗? 同一 session 可以做覆盖操作吗? 对反复提到的内容, 如何去重?"

#### Agent 代码事实分析

**对 R1 的分析 — 三个子问题对应三个缺口**:

| 用户诉求 | 代码缺口 | 严重度 |
|---|---|---|
| "会话结束后 diva 怎么记住" | `on_session_end` 是空操作 (`manager.rs:505-518` 只做幂等去重, 不写任何内容) | P0 |
| "新的会话不丢记忆" | `system_prompt_block` 只注入 MEMORY.md, palace snapshot 只渲染统计数字 (`hybrid.rs:125-148`) | P1 |
| "知道自己是谁" | `load_identity_header()` 返回硬编码字符串 (`context.rs:221-223`), 不读任何持久化身份文件 | P1 |

**对 R2 的分析 — 用户说对了什么、漏了什么**:

| 用户论点 | 代码事实 | 评价 |
|---|---|---|
| "session 和 mentle 不是一回事" | Session JSONL (`.sessions/`) vs palace.db (`memory/`), 物理分离 | ✅ 完全正确 |
| "session 仅做简单粗暴存储" | `Session` 结构 (`store.rs:66-93`): `Vec<ChatMessage>` + `last_consolidated` 指针 | ✅ 正确 |
| "mentle: 快速索引+关联" | palace 有 `search` (语义搜索) + `kg_query` (知识图谱) + `traverse` (图遍历) + `find_tunnels` (跨 room 连接) | ✅ 正确, 但 KG/tunnel 全闲置 |
| "治理每一个对话的结尾, 写入哪个房间/翼" | 当前实现硬编码 `wing="history", topic="history"` (`hybrid.rs:509-516`), **无路由决策层** | ✅ 正确指出目标, 但当前代码完全没做 |
| "日记也是要放入 mentle 的" | `sync_turn` 已调 `memtle_diary_write` (`hybrid.rs:449`), 但 `diary_read` **从未调用** | ✅ 写了, 但写了不读 |
| "放入所有内容, 压缩过的" | `consolidation.rs` 设计意图就是 LLM 压缩, 但阈值 100 条, 短会话不触发 | ✅ 方向对, 实现有缺陷 |
| (未提及) palace 去重 | `add_drawer` 幂等 + `check_duplicate` + `update_drawer` 三层全闲置 | 盲区 1 |
| (未提及) startup 读回 | palace snapshot 只输出统计数字, 不读内容 | 盲区 2 |
| (未提及) KG 三元组 | `kg_add/kg_query/kg_invalidate` 三个工具都没接 | 盲区 3 |

**对 R3 的分析 — 用户纠正了 agent 的错误方向**:

Agent 曾提议 "startup 时读 palace diary 最近条目 + 搜索最近 session", 用户明确否定:
- 启动时注入 palace 内容 = **污染上下文窗口**
- 正确做法: **告诉 LLM 怎么读 + 有什么可读**, 让 LLM 按需查
- 这与 FR-603 `on_demand` 原则完全一致 — Mentle 不主动注入

Agent 评价: 用户的方向更干净。MEMORY.md 作为唯一启动注入源保持最小上下文, palace 作为"图书馆"由 LLM 自主查阅。这比"启动时预加载 palace 内容"更节省 token, 也更符合 on_demand 原则。

**对 R4 的分析 — 三个问题的代码事实**:

**Q1: session 结束判定标签**
```rust
// agent_loop.rs:651-653 — 硬编码, 所有 session 共享
session_id: Some("agent-loop-shutdown".to_string())
// manager.rs — HashSet<String> 仅在进程生命周期有效
```
→ 无真正 session 边界感知, 仅进程关闭时的 best-effort hook

**Q2: 同一 session 覆盖操作**
```rust
// dispatch.rs:806 — 确定性 ID: SHA256(wing+room+content)[:24]
// dispatch.rs:626-641 — add_drawer 幂等: 相同内容返回 "already_exists"
// dispatch.rs:925-935 — update_drawer: 内容变 → ID 变 → 冲突拒绝
```
→ 可以覆盖, palace 三层去重机制完善但全闲置

**Q3: 反复提到内容的去重**
→ L1 精确 (SHA256 幂等) + L2 近似 (关键词重叠 > 3.0) + L3 更新 (update_drawer)
→ 正确流程: session_end 压缩后先 check_duplicate → 已有则 update → 否则 add

#### 讨论中形成的共识

1. **Session ≠ Mentle ≠ MEMORY.md** — 三层各有职责, 不能混淆
2. **Mentle = 图书馆, 不是大脑** — 存入压缩内容, LLM 按需查, 不主动注入
3. **Session-End 是唯一写入时机** — 只写不读, 压缩 + 路由 + 去重 + 写入
4. **MEMORY.md 是唯一启动注入** — 保持最干净的上下文, 告诉 LLM 有哪些工具可用
5. **去重机制已有但闲置** — palace 内置三层去重, session_end 路由时需接入
6. **SessionEndRequest 需扩展** — 当前不传 session 内容, 是 P0 前置阻塞项

---

## 附录:文件地图

```text
agent-diva-pro/
├── MENTLE.md                                              ← 本文件 (架构索引, 2026-06-19)
├── LAPUTA.md                                              ← 镜像级基线索引 (2026-06-12, 同样格式)
├── DECISION.md                                            ← alife v1 横向 dispose (2026-06-18, 提 memtle = 0.1.2)
├── DECISION-v2-next-phase.md                              ← Harness Engineering (2026-06-19)
├── AGENTS.md:36                                           ← "Mentle integration is intentionally pinned to the published memtle = 0.1.2 crate"
├── Cargo.toml:87-89                                       ← memtle = 0.1.2, default-features = false (钉死)
├── justfile:33-52                                         ← mentle-package-policy / sprint5-default-check / mentle-check / sprint5-check / epic6-release-gate
├── scripts/ci/
│   └── verify_mentle_package_policy.py                    ← CI 静态护栏:4 项 (memtle 必须从 crates.io 0.1.2 解析)
│
├── agent-diva-core/
│   ├── Cargo.toml:54-60                                   ← [dependencies.memtle] workspace = true, optional = true; features.mentle = ["dep:memtle"]
│   └── src/memory/
│       ├── hybrid.rs                                      ← 961 行,HybridMemoryProvider + PalaceStatusSnapshot + MemoryProvider 实现
│       └── provider.rs:247-275                            ← MemoryProvider trait 4 方法 + 3 命名 hook (laputa_wakeup/project_soul/recall_intent) doc comment
│
├── agent-diva-agent/
│   ├── Cargo.toml:14, 51-53                               ← features.mentle = ["agent-diva-core/mentle", "dep:memtle"]; [dependencies.memtle] workspace = true, optional = true
│   ├── src/
│   │   ├── mentle_runtime.rs                              ← 456 行,MentleRuntime::try_build + MentleToolkitTool + 错误分类/降级
│   │   ├── mentle_discovery.rs                            ← 23 行,discover_mentle_tool_names 条件编译包装
│   │   ├── tool_config/mentle.rs                          ← 139 行,MentleToolRuntimeConfig + MentleToolMode + filter_mentle_tools
│   │   └── agent_loop.rs:591                              ← pub fn mentle_active() (LAPUTA.md 写的 570 已过时)
│   └── tests/
│       └── mentle_governance_boundaries.rs                ← 137 行,验证 Mentle prompt 路由不暴露 raw memtle_* 名称
│
├── agent-diva-laputa/
│   ├── tests/
│   │   └── mentle_governance.rs                           ← 94 行,Laputa 写不创建 palace.db + laputa crate 不依赖 memtle
│   └── Cargo.toml                                         ← (无 memtle 依赖,mentle_governance.rs:89-94 静态断言)
│
├── agent-diva-autodream/
│   ├── tests/
│   │   └── mentle_governance.rs                           ← 126 行,AutoDream 写不创建 palace.db + autodream crate 不依赖 memtle
│   └── Cargo.toml                                         ← (无 memtle 依赖,mentle_governance.rs:120-126 静态断言)
│
├── agent-diva-gui/
│   └── src/components/settings/
│       └── MentleSettingsCard.vue                         ← 247 行,S7-A1 GUI 控件: enable/mode/custom checklist
│
├── docs/prds/prd-laputa-2026-06-12/
│   ├── prd.md:20, 95, 288-301, 371-372                     ← §5.6 FR-6xx 3 条 (Laputa ↔ Mentle 边界) + OQ#17-18 转 follow-up Mentle 集成 PRD
│   └── reconcile-mentle-laputa.md                         ← IR-2 51 行,锁定 8 项 gap (本索引 §3 + §12 主要来源)
│
├── docs/dev/past/legacy-docs/dev/mentle-integration/
│   ├── 13-s3-a1-memtle-toolkit-tool-interface.md          ← 100 行,Sprint 3 A1 工具接口冻结 (本索引 §2.5 + §7.4 来源)
│   ├── 16-s3-a4-a6-mentle-runtime-assembly.md             ← 98 行,Sprint 3 A4-A6 运行时组装 (本索引 §2.2 + §2.3 来源)
│   ├── 22-s4-a10-mentle-feature-build-env.md              ← 77 行,Sprint 4 A10 Windows clang-cl (本索引 §9.3 来源)
│   └── 25-s7-a1-mentle-tool-selection-and-gui-controls.md ← 266 行,Sprint 7 A1 GUI 控件 (本索引 §2.7 + §8.4 来源)
│
└── docs/dev/archive(old-docs-dont-read-me)/mentle-integration/  ← 同 4 份文件 archive 副本 (可忽略)

morediva/
├── papers/                                                ← 7 篇 paper,Reflexion 2023 跟 memtle 0.1.2 语义相关 (未直接引用)
│   └── 2303_11381.md                                      ← Reflexion 论文 (与"self-reflection → memory"机制相关)
├── .workspace/                                            ← 参考项目目录,目前不含 memtle/ 子目录 (AGENTS.md:35 提到)
└── diva-dev-ultra/docs/core-modules/
    └── mentle.md                                          ← 618 行,Python/JS 视角的 Mentle 集成详解 (本索引 §6 + §7 部分参考)
```
