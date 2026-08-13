# R0 当前状态图

- 状态：`Research Draft / Source-backed`
- 日期：2026-08-13
- 性质：当前实现地图；不定目标架构
- Evolution 细节见 [R1 切片](../cognitive-r1-genericagent-evolution-2026-08/r0-evolution-slice.md)

## 1. 一句话现状

今日 Diva 认知面是 **一套混域治理链 + 四条旁路**：Persona JSON、文件型 `memory_md`、BML sqlite、AutoDream 候选都汇入 `EvolutionProposal`；Skill 文件树、WORLD Markdown、危险工具审批、C1–C5 会话上下文各自独立。产品要求的四个工作区尚未在代码里分开。

## 2. 产品工作区 vs 代码对象

| 产品工作区 | 代码现状 | 标签 |
| --- | --- | --- |
| Persona | `PersonaMemoryView` 编辑 Frozen Core JSON + `memory_md` + changelog；保存创建 Proposal；右栏 `PersonaLifecyclePanel` 做 approve/apply/rollback | 源码事实 |
| Memory | `MemoryView` 读 BML；remove 仍建 Deprecation 提案并跳 Approval/Evolution | 源码事实 |
| Evolution | `EvolutionView` = 提案收件箱 + AutoDream runs + changelog + SelfEvolution 策略；**不是** Skill 管理 | 源码事实 |
| Chat Approval Center | `/api/approvals` 覆盖 command / plan / **memory**；`domain=memory` 深链回 Evolution | 源码事实 |

旁路（不经过 Evolution 主链）：

| 旁路 | 权威 | 标签 |
| --- | --- | --- |
| Skill runtime | `workspace/skills/<name>/SKILL.md` + `SkillsLoader` | 源码事实 |
| WORLD | `.laputa/cognitive/WORLD.MD` + `WorldGovernance` | 源码事实 |
| MEMRULES | `.laputa/cognitive/MEMRULES.MD`，人类文件；写路径只被 `governed_apply` 加载 | 源码事实 |
| 危险工具 M3 | `ApprovalCoordinator` + `/api/command-approvals` 旧轨 | 源码事实 |
| 会话上下文 | C1–C5 / `canonical_checkpoint_v1` / session `working_memory` | 源码事实 |

## 3. 模块地图

### 3.1 领域类型（`agent-diva-core`）

| 符号 | 路径 | 角色 |
| --- | --- | --- |
| `LaputaSectionName` | `src/evolution/types.rs:220` | 9 个活 section：Identity / Relationship / Commitment / Preferences / **MemoryMd** / Daily / Weekly / Monthly / Changelog |
| `ProposalType` | `types.rs:73` | MemoryPatch, LearningNote, IdentityPatch, RelationshipUpdate, CommitmentSet, **SopCreate**, Deprecation |
| `ProposalType::target_section()` | `types.rs:85-93` | MemoryPatch→MemoryMd；SopCreate→**Identity**；LearningNote→Preferences |
| `EvolutionProposal` | `types.rs:234` | `proposed_patch: String`（apply 要求合法 JSON） |
| `AgentEvent` | `src/bus/events.rs:61` | 仅对话/工具/Plan；无 Persona/Memory/Evolution/AutoDream 变体 |
| `ApprovalLedgerError::NotFound` | `src/governance/ledger.rs:193` | 错误串 `approval request not found` |
| `MemoryRecordKind` | `src/memory/record.rs` | 含 Identity/Relationship/Commitment/Preference/LongTerm/**WorkingMemory** |
| `CanonicalCheckpoint` | `src/session/store.rs` | 对话压缩检查点 `canonical_checkpoint_v1` |

### 3.2 Laputa

| 模块 | 职责 |
| --- | --- |
| `layout.rs` `LaputaPaths` | `.laputa/` 全部路径 |
| `service.rs` `LaputaSection` | `content: serde_json::Value`；null→Tbd，非 null→Owned |
| `frozen_core.rs` | 会话启动捕获四段 JSON 为紧凑字符串；预算 4000 |
| `memory_provider.rs` | 文件权威适配器：Identity+Relationship+Commitment+Preferences+**MemoryMd** |
| `typed_store.rs` / `bml/` | BML sqlite 权威 |
| `typed_provider.rs` | 生产 MemoryProvider；CRUD + proposal sink |
| `governed_apply.rs` | `MemoryGovernanceCoordinator`；错误包装 `governance ledger failed: {0}` |
| `proposals.rs` | 文件提案 CRUD / legacy apply 写 section JSON |
| `cognitive/` | WORLD / MEMRULES / WorldGovernance |
| `persona_retire.rs` | SOUL/IDENTITY/USER.md → 提案；MEMORY.md 只归档 |

### 3.3 Agent / Tools / AutoDream

| 模块 | 职责 |
| --- | --- |
| `agent-diva-agent/src/context.rs` | C1 稳定前缀：Mask → FrozenCore → Skills → Memory L1 |
| `memory_boundary.rs` | `ContextBuilder::new` 默认 `MemoryManager`；生产由 Manager 注入 Typed |
| `tools` memory_* / `update_working_checkpoint` / `laputa_propose_section_write` | 见 inventory 写入者表 |
| `agent-diva-autodream` | Queued→…→Completed；默认读 MemoryMd+Identity；候选偏 MemoryPatch |

### 3.4 Manager HTTP（`server.rs`）

| 组 | 路径摘要 |
| --- | --- |
| AutoDream | `/api/autodream/runs[/:id[/live-text\|events\|cancel]]` |
| Laputa | `/api/laputa/{proposals,snapshot,persona-workspace,section/:name[/write],cognitive/:kind,changelog,recall-feedback,events}` |
| BML | `/api/bml/memories`, `/api/bml/memories/:id`, `POST .../remove` |
| Approvals | `/api/approvals/**` |
| Command HITL | `/api/command-approvals/**`（旧轨仍在） |
| Skills | `/api/skills`, `/api/skills/:name` |
| SelfEvolution | `/api/config/self-evolution` |

无 first-run initialize 路由。无 WORLD 写路由。

### 3.5 Tauri / GUI / CLI

- Tauri：`laputa_*`、`bml_list/get/remove_memory`、chat SSE→`agent-*`、approvals SSE→`approval-event`
- GUI 侧栏：`persona-memory` / `evolution` / `memory` / chat + Approval drawer / settings skills
- CLI：`persona-retire plan|propose|archive`；无独立 memory CRUD；approval domain 含 `memory`

## 4. 关键读写时序

### 4.1 Persona「保存」（源码事实）

```text
SectionEditor.handleSave
  → writeLaputaSection(section, JSON string, reason)
  → POST /api/laputa/section/:name/write
  → create_user_edit_proposal   （不写 section 文件）
  → MemoryGovernanceCoordinator.submit
       → governance.db  ApprovalRequest (capability=MemoryApply)
       → governance.sqlite3  proposal↔request 映射
  → GUI 右栏 / Evolution / Approval Center 可见
  → 用户 decide + apply
       Typed（默认）: adapt_governed_proposal → BML put_governed
                     finalize_typed_proposal write_authority=false
                     ↛ identity.json
       Legacy: apply_proposal → atomic_write_json(section_file)
```

推断：默认 Typed 下，GUI 再刷新仍读 JSON 种子 `null`；Frozen Core Prompt 本会话也不变。

### 4.2 BML Memory CRUD（源码事实）

```text
memory_add     → TypedMemoryStore.put (LongTerm, session_id=None)     立即生效
memory_list    → 过滤 tombstone / supersede / session-scoped
memory_search  → FTS5 search_visible
memory_update  → EvolutionProposal MemoryPatch → MemoryMd             不写 BML
memory_remove  → EvolutionProposal Deprecation → Changelog            不写 BML
GUI remove     → POST /api/bml/memories/:id/remove → 同样建提案
```

### 4.3 Frozen Core / first-run（源码事实）

```text
LaputaStorage::open
  → 缺文件则写四份 sections/*.json = null
  → seed WORLD.MD / MEMRULES.MD（不覆盖已有）

ContextBuilder 首次装配
  → capture_frozen_core_for_session（进程内按 workspace+session 缓存）
  → 四段皆空 ⇒ 注入 FIRST_RUN_ONBOARDING_BLOCK
       要求 ask_user + laputa_propose_section_write（再走提案链）
  → 之后本会话不再重捕；session reset 才 release
```

没有五份权威原子直写 API。决策记录里的 initialize/status 端点未实现。

### 4.4 AutoDream（引用 R1 切片）

```text
POST /api/autodream/runs 或 GUI triggerAutoDream('manual')
  → .agent-diva/autodream/**
  → 读 MemoryMd + Identity JSON
  → CandidateGate（拒绝 SopCreate）
  → .laputa/proposals/** + submit 审批
  → Evolution inbox
```

SelfEvolution 频率/阈值**只有配置页**；cron/heartbeat 无引用。Notebook 月报 cron 不是 AutoDream。

## 5. 事件四轨

```text
对话 / 工具 / Plan
  AgentEvent → /api/chat SSE → Tauri agent-* → ChatView

危险工具 (M3)
  CommandApprovalCoordinator → /api/command-approvals
                            ↘ ApprovalCoordinator → /api/approvals → ApprovalCenterDrawer

Persona / Memory / AutoDream 提案
  create_proposal → submit
    → governance.db + governance.sqlite3
    → LaputaEvent (proposal/changelog/error) poll/SSE
    → EvolutionView + PersonaLifecyclePanel
    → Approval Center domain=memory → openEvolutionProposal

AutoDream 运行
  .agent-diva/autodream/events.jsonl → /api/autodream/runs/:id/events
  不进 AgentEvent
```

`useEventStream.ts` 指向 `localhost:9100/api/sse/events`，**未接入 App.vue**（源码事实：遗留）。

WORLD 有独立 `world-ledger.jsonl`，不是 `LaputaEvent`。

## 6. Prompt / Context 耦合

稳定前缀（C1，`context_assembly` 固定顺序）：

1. `MaskAndIdentity` — mask 正文 + 硬编码 identity header；**不读 Frozen Core / WORLD**
2. `FrozenCore` — JSON 快照渲染 `## Frozen Core — {name}` + 紧凑 JSON；空则 first-run 块
3. `AgentRulesAndSkills` — `AGENTS.md` + SkillsLoader
4. `MemoryPolicyAndIndex` — L0 政策 + provider `system_prompt_block`

生产 Typed L1：`## Embedded Laputa Typed Memory`，只收 AppliedAuthority、非 tombstone、非 session-scoped。

非默认 `LaputaMemoryProvider`：五段 JSON（含 MemoryMd）→ `## Applied Laputa Authority`。

Turn 动态：`WorkingMemory` + 有意图才 `PrefetchRecall`。  
C5 区：`canonical_checkpoint_v1`（会话 store，≤8000 字）≠ working checkpoint ≠ 跨会话 STM。

禁止注入（`context_plane_invariants`）：MEMRULES、完整 WORLD、报告、退役人格文件。

`WorldStore::project` 存在，**无生产 Prompt 调用者**。

`ContextBuilder::new` 默认 `MemoryManager`（`memory/MEMORY.md`）。生产 Manager 用 `memory_provider_for_mode_with_governance`。裸 builder 测试不覆盖生产 L1。

## 7. GUI 信息架构

```text
NormalMode
  chat ──────── ChatView + ApprovalCenterDrawer (command/plan/memory)
  persona-memory  PersonaMemoryView
                    左：frozen_core 4 + long_term(memory_md) + changelog + memrules/world
                    中：JSON SectionEditor 或只读 pre(WORLD/MEMRULES)
                    右：PersonaLifecyclePanel（治理生命周期）
  evolution ────── EvolutionView  inbox | runs | audit | policy
  memory ───────── MemoryView     BML list/search/kind 含 working_memory
  notebook ─────── 可创建 MemoryPatch 并跳 Evolution
  settings ─────── SkillsSettings（Skill 安装）/ SelfEvolutionSettings（未接 cron）
```

深链：

- Approval Center `domain===memory` → `openEvolutionProposal(resource_id)`
- Memory 页 remove → `open-approval` → Evolution
- `ChatGovernanceCard` → inbox / runs

文案把 personality / memory / SOP / skill 捆在同一审批叙事（`locales/zh.ts` Evolution 空态、SelfEvolution）。

## 8. 权威分裂总图

```text
用户编辑 Identity
        │
        ├─ GUI/工具 ── proposal ── Typed apply ──► BML Identity 行 ──► L1 Prompt
        │                                      ↛ sections/identity.json
        │
        └─ Frozen Core / GUI 编辑器 / AutoDream 输入
                 只读 sections/identity.json（常为 null）

memory_add ──────────────────────────────► BML LongTerm ──► L1
memory_update / Notebook / AutoDream ────► MemoryPatch ──► memory_md.json
Typed apply 再把 patch 适配进 BML，不回写 json

update_working_checkpoint ──► BML WorkingMemory(session_id)
session end DELETE
GUI Memory 页仍可按 working_memory 过滤
canonical_checkpoint_v1 在 session store（对话压缩）
产品 STM（跨会话活动上下文）── 不存在
```

## 9. 启动恢复

| 步骤 | 位置 | 行为 |
| --- | --- | --- |
| 打开 `.laputa/` | `LaputaStorage::open` | 建目录、state.json、认知文件、四份 null section |
| 打开 BML | `TypedMemoryStore::open_canonical` | 缺库创建；完整性失败 → `DegradedMemoryProvider` |
| 打开审批账本 | Manager `bootstrap.rs` | `{workspace}/.laputa/governance.db` |
| 撤销未完成 command | bootstrap | |
| 恢复 Plan approvals | `task_runtime.rs` | |
| 恢复 Memory approvals | `recover_memory_approvals` | 私有 pending 迁入共享账本；不导入旧 allow receipt；缺 receipt 的已批准 → `needs_attention` |
| 恢复 AutoDream | `autodream.resumable_runs()` | |
| Laputa SSE | `Last-Event-ID` | replay |
| 孤儿 session WM | `run_startup_gc` | **无生产调用点** |

## 10. 本图未展开（见 inventory / R1 / R2–R4）

- Evolution 类型、AutoDream 阶段、Skill 脱钩细表 → R1 切片
- STM 方案与 Layer 1 实验 → R2
- Markdown revision / Diff / 编辑器 → R3
- 用户数据删除影响 → R4
