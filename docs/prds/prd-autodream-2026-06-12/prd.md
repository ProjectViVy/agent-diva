---
title: AutoDream — Agent-Diva 节律性反思蒸馏机制
status: final
created: 2026-06-12
updated: 2026-06-12
version: 1.1
revision_note: "v1.1 — 2026-06-12 scope-merge 边界分工修订（方案 C 批准）。新增与 Report System PRD 的边界声明，FR-12/13 标注为'生成层'。详见 docs/architecture/scope-merge-decision.md。"
---

# PRD: AutoDream — Agent-Diva 节律性反思蒸馏机制

## 0. Document Purpose

本文档面向 PM、架构师和下游 Epic/Story 拆分者。AutoDream 是 Agent-Diva 的跨会话记忆提炼基础设施，不是终端用户直接交互的功能。终端用户感知的是"记忆更连贯了"和"有待审查的记忆候选"。

本文档基于以下调研输入构建：
- `autodream-rhythm-distillation-design.md` — 核心设计规格
- `compression-research.md` — 前置压缩技术
- `autonomous-evolution-simplified-architecture-decision.md` — 架构决策
- `compression-taxonomy-decision.md` — 压缩分类决策
- `context-compaction-vs-autonomous-evolution-decision.md` — 边界决策

**边界声明 (v1.1 修订 — 2026-06-12)**:
> 依据 `docs/architecture/scope-merge-decision.md`, AutoDream 的定位是**跨会话反思蒸馏 (数据压缩层)**, 而**不是用户可见内容的直接管理方**。
> - **AutoDream 拥有**: 跨会话反思 (FR-1~9) + 记忆候选审查 (FR-10/11) + 日报/周报**生成** (FR-12/13)
> - **AutoDream 不拥有**: 月报 (P2 维持) + 报表固化 (SOP/Skill/Memory) + Session 智能搜索 + NotebookView 展示层
> - **消费方**: Report System PRD (`docs/prd-report-system/prd.md`) 负责展示、固化、搜索
> - **数据契约**: AutoDream 产出 markdown 文件至固定路径, Report System 扫描消费
> - **用户原话**: "autodream 只是一个压缩技术，而最终还是要回归到用户的可见内容中"

**已有 UI 基础：**
- `NotebookView.vue` — 报告列表+详情展示，支持日报/周报/月报切换，底部操作栏（固化为 SOP、固化为 Skill、更新记忆）
- `SelfEvolutionSettings.vue` — AutoDream 频率配置（daily/weekly/manual），触发阈值设置
- `SettingsView.vue` — 已集成 `self-evolution` 视图入口

AutoDream 的 UI 需求将复用并扩展以上组件，而非从零构建。

---

## 1. Vision

AutoDream 是 Agent-Diva 的节律性反思蒸馏机制。它在后台定期审视跨会话的交互证据，提取可审计的记忆候选、Journal 条目和节律报告，经用户确认后写入长期记忆。

与现有 consolidation 的区别：consolidation 是单会话内的消息压缩器，AutoDream 是跨会话的记忆提炼器。consolidation 直接覆盖 MEMORY.md，AutoDream 产出结构化候选供审查。

核心价值：让 Agent-Diva 的长期记忆从"被动累积"转向"主动反思"，从"直接覆盖"转向"候选审查"，从"单会话"转向"跨会话节律"。

---

## 2. Target User

### 2.1 Jobs To Be Done

- 作为 Agent-Diva 用户，我希望 AI 能记住跨会话的上下文，而不是每次对话都从零开始
- 作为 Agent-Diva 用户，我希望 AI 的记忆更新是经过我确认的，而不是悄无声息地改变
- 作为 Agent-Diva 用户，我希望收到关于我与 AI 交互的节律报告（日报/周报），了解我们的对话趋势
- 作为 Agent-Diva 开发者，我希望有一个可审计、可重跑、不阻塞主循环的后台反思机制

### 2.2 Non-Users (v1)

- 不面向多租户场景（v1 只支持单用户）
- 不面向实时性要求高的场景（AutoDream 是异步的，不保证实时性）

### 2.3 Key User Journeys

- **UJ-1. 用户首次触发 AutoDream**
  - **Persona + context:** 大湿，使用 Agent-Diva 一段时间后，发现 AI 记忆有些零散
  - **Entry state:** 在 GUI 设置页看到"AutoDream"选项，或收到通知"您有 12 条记忆候选待审查"
  - **Path:** 点击"运行 AutoDream" → 系统后台启动蒸馏 → 完成后通知"发现 5 条候选，请审查"
  - **Climax:** 用户查看候选列表，选择接受/拒绝/修改
  - **Resolution:** 接受的候选写入长期记忆，AI 后续对话更连贯

- **UJ-2. 用户查看节律报告**
  - **Persona + context:** 大湿，想了解最近与 AI 的交互趋势
  - **Entry state:** GUI 中"节律报告"入口
  - **Path:** 点击"周报" → 查看本周主题、高频话题、情绪趋势、学习收获
  - **Climax:** 报告呈现结构化的交互摘要
  - **Resolution:** 用户可选择将报告中的发现转为记忆候选

---

## 3. Glossary

- **AutoDream** — Agent-Diva 的节律性反思蒸馏机制，跨会话提取记忆候选
- **Consolidation** — 单会话内的消息压缩机制，直接改写 MEMORY.md
- **Context Compaction** — 会话上下文压缩，解决 context window 压力
- **Source Capsule** — 压缩中间产物，存放于 `.agent-diva/compact/capsules/`
- **Memory Candidate** — AutoDream 产出的结构化记忆提案，需经审查后写入
- **Journal Entry** — AutoDream 产出的交互记录条目
- **Rhythm Report** — 日报/周报，呈现交互趋势和主题
- **Lock File** — 并发控制文件，防止多实例同时运行 AutoDream
- **Checkpoint** — 记录上次蒸馏时间戳，用于时间门判断
- **Forked Subagent** — 独立运行的子代理，不阻塞主循环

---

## 4. Features

### 4.1 触发与调度

**Description:** AutoDream 支持手动触发和时间门自动触发。MVP 只支持这两种，会话门放 P1。

**Functional Requirements:**

#### FR-1: 手动触发 AutoDream

用户可通过 **SelfEvolutionSettings.vue** 中的按钮或 CLI 命令手动触发 AutoDream。

**Consequences (testable):**
- SelfEvolutionSettings.vue 提供"立即运行 AutoDream"按钮
- 手动触发时，系统检查 lock 文件，若已有运行则返回"已有蒸馏进行中"
- 手动触发成功后，返回 task ID 供轮询
- 手动触发支持取消操作

#### FR-2: 时间门自动触发

系统根据 **SelfEvolutionSettings.vue** 中配置的频率（daily/weekly/manual）自动触发 AutoDream。

**Consequences (testable):**
- SelfEvolutionSettings.vue 提供频率选择：daily / weekly / manual
- 距上次蒸馏 ≥ 配置间隔且距上次检查期间有新会话时触发
- 自动触发不阻塞用户当前对话
- 自动触发失败时记录日志并通知用户

### 4.2 并发控制

**Description:** 通过 lock 文件和 checkpoint 机制确保 AutoDream 的单实例运行和失败可重跑。

**Functional Requirements:**

#### FR-3: Lock 文件机制

AutoDream 运行前获取 lock，运行后释放。

**Consequences (testable):**
- Lock 文件路径：`.agent-diva/autodream/lock`
- Lock 内容：PID + 启动时间戳
- 获取失败时返回已有运行的 PID 和启动时间
- Stale lock 检测：PID 不存在且启动时间 > 60min 时自动回收

#### FR-4: Checkpoint 机制

记录上次成功蒸馏的时间戳，用于时间门判断。

**Consequences (testable):**
- Checkpoint 文件路径：`.agent-diva/autodream/checkpoint`
- 成功运行后更新 checkpoint
- 失败时不更新 checkpoint，下次触发可重跑

### 4.3 输入收集

**Description:** AutoDream 从多个来源收集输入，按优先级读取。

**Functional Requirements:**

#### FR-5: 输入源优先级

AutoDream 按以下优先级读取输入：
1. 近期会话（session store，最近 N 个）
2. HISTORY.md（已压缩的会话摘要）
3. MEMORY.md（当前长期记忆）
4. Source Capsules（如有）

**Consequences (testable):**
- 每个输入源都有明确的读取边界（时间范围、数量限制）
- 输入源缺失时不阻塞，继续读取下一个
- 总 token 数超过预算时，按优先级截断

### 4.4 蒸馏执行

**Description:** 通过 forked subagent 执行四阶段蒸馏，不阻塞主循环。

**Functional Requirements:**

#### FR-6: Forked Subagent 执行

AutoDream 在独立的 subagent 中运行，与主循环隔离。

**Consequences (testable):**
- Subagent 拥有独立的工具权限（受限集）
- Subagent 运行期间主循环不受影响
- Subagent 支持取消/中断操作
- Subagent 运行超时（默认 5min）自动终止

#### FR-7: 四阶段蒸馏 Prompt

Subagent 执行四阶段蒸馏：Orient → Gather → Consolidate → Propose。

**Consequences (testable):**
- Orient：读取 Laputa 文件结构，了解当前记忆状态
- Gather：提取关键信号、矛盾、遗漏
- Consolidate：生成记忆候选 + Journal 条目
- Propose：输出结构化产物

### 4.5 产物输出

**Description:** AutoDream 产出结构化文件，不直接写 MEMORY.md。

**Functional Requirements:**

#### FR-8: 结构化产物格式

产物为 JSON 格式，包含以下字段：
- `memory_patch_candidates`: 记忆候选列表（内容、置信度、证据引用）
- `journal_entries`: Journal 条目列表
- `learning_candidates`: 学习候选列表
- `evidence_refs`: 证据引用列表
- `confidence`: 整体置信度
- `review_required`: 是否需要审查（始终为 true）

**Consequences (testable):**
- 产物路径：`.agent-diva/autodream/runs/{timestamp}/autodream_run.json`
- 每个候选都有唯一的 evidence_refs
- 产物 schema 版本化

#### FR-9: 事件流记录

每次运行追加事件到事件流。

**Consequences (testable):**
- 事件流路径：`.agent-diva/autodream/events.jsonl`
- 每条记录包含：timestamp、trigger_type、input_summary、output_summary、status

### 4.6 审查与确认

**Description:** 用户通过 GUI 审查候选，选择接受/拒绝/修改。

**Functional Requirements:**

#### FR-10: 候选缓存展示

GUI 通过 **NotebookView.vue** 展示待审查的候选列表。

**Consequences (testable):**
- 复用 NotebookView 的左右分栏布局：左侧候选列表，右侧候选详情
- 按时间排序，最新优先
- 显示候选内容、置信度、证据引用
- 支持搜索和筛选
- `[ASSUMPTION: §4.6]` NotebookView 的 `NotebookReport` 接口可扩展为包含候选类型字段

#### FR-11: 用户决策操作

用户在 NotebookView 底部操作栏对候选执行接受/拒绝/修改。

**Consequences (testable):**
- 复用 NotebookView 底部操作栏，扩展为三态操作：接受 / 拒绝 / 修改
- 接受的候选写入 MEMORY.md
- 拒绝的候选标记为 rejected，保留记录但不写入
- 修改的候选经用户编辑后写入
- 决策记录到审计日志
- 当前 NotebookView 已有 `solidifyAsSop`、`solidifyAsSkill`、`updateLongTermMemory` 操作，需扩展为支持候选审查语义

### 4.7 节律报告

**Description:** AutoDream 产出日报和周报，呈现交互趋势。

**Functional Requirements:**

#### FR-12: 日报 (AutoDream 生成 → Report System 展示)

每日产出交互摘要，通过 **NotebookView.vue** 展示。

**Consequences (testable):**
- 复用 NotebookView 的 daily/weekly/monthly 切换标签
- 日报内容：今日主题、高频话题、情绪趋势
- 产物路径：`.agent-diva/autodream/reports/daily/{date}.md`
- NotebookView 的 `get_notebook_reports` API 需支持按 `period: 'daily'` 查询

**v1.1 修订**: AutoDream 拥有"生成", Report System 拥有"展示"。AutoDream 产出 markdown 后写入固定路径 (`.agent-diva/autodream/reports/daily/{date}.md`), NotebookView 通过文件系统读取。**不直接修改 NotebookView 代码**。详见 `docs/architecture/scope-merge-decision.md` §2.3。

#### FR-13: 周报 (AutoDream 生成 → Report System 展示)

每周产出交互趋势报告，通过 **NotebookView.vue** 展示。

**Consequences (testable):**
- 复用 NotebookView 的 weekly 标签
- 周报内容：本周主题、学习收获、待办回顾、人格漂移检测
- 产物路径：`.agent-diva/autodream/reports/weekly/{week}.md`
- NotebookView 的 `get_notebook_reports` API 需支持按 `period: 'weekly'` 查询

**v1.1 修订**: 同 FR-12, AutoDream 拥有"生成", Report System 拥有"展示"。写入路径契约见 `docs/architecture/scope-merge-decision.md` §2.3。

### 4.8 失败处理与通知

**Description:** AutoDream 失败时通知用户，支持重试。

**Functional Requirements:**

#### FR-14: 失败通知

AutoDream 运行失败时通过 GUI 通知用户。

**Consequences (testable):**
- 通知方式：GUI toast 通知（复用 `showAppToast`）+ 日志
- 通知内容：失败原因、重试建议
- 自动重试最多 3 次
- 失败通知在 NotebookView 中显示为"运行失败"状态的报告项

---

## 5. Non-Goals (Explicit)

- AutoDream 不替代 consolidation（单会话压缩）
- AutoDream 不直接写 MEMORY.md（必须经过审查）
- AutoDream 不阻塞主循环（异步执行）
- AutoDream v1 不支持多租户
- AutoDream v1 不支持 Mentle 召回（P1）
- AutoDream v1 不支持 mask 隔离（统一蒸馏）
- AutoDream v1 不支持全自动接受（必须用户确认）

---

## 6. MVP Scope

### 6.1 In Scope

- 手动触发 + 时间门自动触发
- Lock/checkpoint 机制
- Forked subagent 执行
- 结构化产物输出（JSON）
- 用户审查界面（GUI）
- 日报 + 周报
- 失败通知

### 6.2 Out of Scope for MVP

- 会话门触发（P1）
- Mentle 召回（P1）
- Mask 隔离蒸馏（P1）
- 自动接受高置信度候选（P2）
- 月报（P2）
- 多租户（P2）

---

## 7. Success Metrics

**Primary**
- **SM-1**: 用户审查接受率 ≥ 70%。验证 FR-11。
- **SM-2**: AutoDream 运行成功率 ≥ 95%。验证 FR-14。

**Secondary**
- **SM-3**: 用户每周至少审查一次候选。验证 FR-10。
- **SM-4**: 节律报告用户打开率 ≥ 50%。验证 FR-12、FR-13。

**Counter-metrics (do not optimize)**
- **SM-C1**: AutoDream 运行时的 token 消耗。不应为了成功率而无限增加 token 预算。

---

## 8. Open Questions

1. 时间门 24h 是否可配置？默认是多少？
2. 用户未审查的候选如何处理？过期自动清理还是保留？
3. 节律报告的展示形式？纯文本还是富文本？
4. AutoDream 与 Plan Mode 的关系？Plan Mode 运行期间是否暂停 AutoDream？

---

## 9. Assumptions Index

- `[ASSUMPTION: §4.1]` 时间门 24h 是合理的默认间隔
- `[ASSUMPTION: §4.3]` 输入源按优先级截断的策略不会丢失关键信息
- `[ASSUMPTION: §4.6]` 用户愿意定期审查候选
- `[ASSUMPTION: §4.7]` 日报/周报对用户有价值
