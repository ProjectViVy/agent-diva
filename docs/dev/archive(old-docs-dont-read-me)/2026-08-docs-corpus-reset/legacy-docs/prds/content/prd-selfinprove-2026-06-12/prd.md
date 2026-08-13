---
title: "Self-Improve — Agent-Diva 自主进化用户可见层"
status: final
created: 2026-06-12
updated: 2026-06-12
version: 0.0.6
project: agent-diva-pro
author: John (PM)
related_prds:
  - docs/prds/prd-autodream-2026-06-12/prd.md  # 数据后端 (FR-12/13 日报/周报生成)
  - docs/prd-report-system/prd.md              # 消费方 (月报 / 固化展示 / 搜索 / NotebookView)
  - docs/prds/prd-laputa-2026-06-12/prd.md     # 写入目标 (4 接口实接, [LAPUTA-STUB] 拆)
decision_log: .decision-log.md
follow_up_prds:
  - Laputa 架构后续 PRD (D-008 OQ#12-14)
  - AutoDream PRD v2 (D-008 OQ#15-16)
  - Mentle 集成 PRD (D-008 OQ#17-18)
  - Laputa 架构后续 PRD 承载: POST /api/laputa/proposals/run-now 同步 trigger 端点 (FR-302 manual trigger 需要)
inherits:
  - selfinprove/D-009 (节律双模)
  - selfinprove/D-010 (Laputa 写权限依赖, 实质 B 一锅端, D-014 回写关闭)
  - selfinprove/D-011 (一锅端 ship, FR-5xx stub 待拆)
  - laputa/D-007 (1 稀薄层)
  - laputa/D-010.r1 (Laputa 现状 = 稀薄层)
  - laputa/D-012 (OQ#4 (a) 锁定, user_only / agent_self)
---

# PRD: Self-Improve — Agent-Diva 自主进化用户可见层

> **Memory boundary amendment (2026-07-30):** any Mentle recall integration
> described below is superseded. Self-improvement evidence and proposals target
> Embedded Laputa; Mentle/MenPalace has no target runtime role.

> **状态**: draft (Coaching path)
> **本文档正在 John (PM) 与大湿 的 Coaching 协作中生成，禁止直接落稿后即视为 final。**

## 0. Document Purpose

待 Discovery 完成后填写。

## 1. Vision

(锁定 v0.0.4 — D-010.r1 修正 + D-011 一锅端决策后冻结。)

**Back-link**: 产品愿景级纲领见 `docs/vision/04-进化路线图.md` L886-889: "agent-diva 会自主发起进化 / 不需要人类编写代码 / agent-diva 会自己改进自己 / 形成自主进化的 AI 网络"。

**PRD Vision (capability-first, 落地向)**:

> agent-diva-pro 让用户**看见并参与**自己 agent 的"人格"如何随时间生长——agent 在 **Laputa 稀薄文档层**持续积累 identity / preference / relationship 信号, 用户在 **Journal** 面板逐条审查哪些 preference 该被记住, 在 **Inbox** 跨日报/周报追踪人格漂移, 在 **Chat** 手动触发即时整理 (不必等节律, 不必在 session 内守候)。30 天后, 用户的 agent 在 Laputa 里有了只属于这位用户的身份定义和有机用户模型; 用户每天 ≤5 分钟, 干完这件事。

**节律双模** (D-009):
- 默认 **out-of-session**: AutoDream 后台跑, 用户回到 UI 审查 (主路径)
- 补充 **in-session manual**: 用户 Chat 内手动触发, 同步调用 (补充路径, 不依赖节律)

**Laputa 现状 (D-010.r1)**: 稀薄文档层, 取代原 Diva SOUL.MD 等, 统一写接口, 不在 v1 拆文件细节。
**Ship 策略 (D-011)**: 一锅端, 写权限走 stub, 待后续 Laputa PRD 接。

## 2. Target User

(锁定 v0.0.4 — D-001 + D-006 隐含约束后冻结。)

**Primary**: 尝鲜版 diva 用户 (D-001: 公开开源软件受众)。

具体刻画:
- **角色**: 开发者/技术尝鲜者, 已安装 agent-diva-pro 开源版, 跑至少 1 周
- **场景**: 日常用 Diva 协助编程/写作/调研, 偶尔触发深度 session
- **期望**: 看到自己的 agent 越来越"懂自己", 但**不愿被 agent 默默修改** (D-006: 全 user 审)
- **技术能力**: 能读 .md 文件, 能改 yaml 配置, 不怕看 JSON 输出
- **痛点**: 每天 agent 跑了什么 / 记住了什么 / 改了 Laputa 什么, 全是黑盒
- **价值主张**: 让用户**看见**这 3 件事, 每天 ≤5 分钟 (per Vision 段)

## 3. Concerns

已识别 (子 agent 调研 + D-006 触发 scope 放大器 + reconcile 3 份 + reviewer 5 critical):

| 编号 | Concern | 强度 | 来源 |
|------|---------|------|------|
| C-1 | Journal 批处理工作台交互质量 | **硬** | D-006 显式要求 |
| C-2 | 固化流程审计 (用户能撤销已应用变更) | **硬** | D-006 隐含, "撤回" 是必需 FR |
| C-3 | Always/Relevant/Archive selector 行为可预测 | 中 | D-007 契约 |
| C-4 | 4 表面 (Chat/Journal/Inbox/Settings) 命名与导航一致性 | 中 | 调研文档 §6 |
| C-5 | i18n 增量命名 (inbox.* / evolution.*) 不破坏已有 selfEvolution.* / notebook.* | 低 | 调研文档 i18n 现状 |
| C-6 | 后端 API 路由 (inbox/journal/evolution/memory.changelog) 当前 0 个, PRD 落"前端契约 + stub/mock" | **硬** | 子 agent P0 风险 |
| C-7 | ChatView 无 UiCard 抽象, 卡片化是结构性改动 | 中 | 子 agent P0 风险 |
| C-8 | Subscribe push vs pull (event bus 已就位?) | 中 | D-008 留 Open |
| C-9 | Laputa stub 拆后, selfinprove 侧契约同步成本 | 中 | reconcile-laputa G8 |
| C-10 | AutoDream 写 inbox/learning-candidates.jsonl vs selfinprove /api/laputa/apply 两条路径分工需澄清 | 中 | reconcile-autodream G6 |

## 3.5 Glossary (downstream 必须用这些术语, 引入同义词 = 纪律违规)

- **EvolutionProposal** — 跨会话反思蒸馏的产物, 8 种 proposal_type (memory_patch / journal_note / learning_note / identity_patch / relationship_update / commitment_set / sop_create / deprecation), 5 字段最少 (id / proposal_type / target_file / evidence_excerpt / proposed_patch), 8 状态 (pending_review / approved / rejected / edited / applied / reverted / needs_attention / run_failed)
- **UiCard** — Chat 消息的可点击卡片扩展, 11 字段 (kind / status / title / body / actions / evidence_preview / session_id / journal_ref / target_file / risk_level / created_at), 4 kind (evolution_proposal / evidence_peek / journal_ref / review)
- **Storage Mapping** — 9 路径契约 (selfinprove 写提案 → /api/laputa/proposals; 应用 → /api/laputa/proposals/{id}/apply; 读 → /api/laputa/snapshot + /api/laputa/section/{name}; 列表 → /api/laputa/changelog; 详情 → /api/laputa/changelog/{id}; 撤销 → /api/laputa/changelog/{id}/rollback; 事件 → /api/laputa/events/{proposals,changelog,errors}; 降级 → {workspace}/memory/pending.jsonl)
- **MemoryChangelog** — Laputa 写入审计的不可变记录 (id / action / target_section / before / after / diff / proposal_id / audit_event_id / created_at / applied_by), 30 天回滚窗口
- **4 表面 (4 Surfaces)** — Chat (执行) / Journal (归档+复醒, NotebookView) / Inbox (提案治理, 新顶级) / Settings (策略, SelfEvolutionSettings 已有)

## 4. Scope Boundaries (Working Draft)

依据 2026-06-12 Discovery 第一轮回答 + P0 决策 (D-002 + D-005~D-008):

| 能力 | 是否进 v1 | 备注 |
|------|----------|------|
| 记忆候选的展示/审查/通过/拒绝交互 | ✅ IN | 核心 (FR-1xx) |
| 固化流程 (SOP / Skill / Memory) | ✅ IN | 核心 (FR-2xx) |
| Journal 批处理工作台 (快捷键/批量/已读/撤回/搜索) | ✅ IN | **D-006 硬约束**, §6.3 |
| 节律配置 (daily/weekly/manual) | ✅ IN | D-009, 反向引用 AutoDream, 不重写 (FR-3xx) |
| Manual trigger 入口 (Chat 卡片 + 命令, 同步调用) | ✅ IN | D-009, 不依赖节律 (FR-3xx) |
| AutoDream 触发模式双轨 (cron 异步 + manual 同步) | ✅ IN | D-009 隐含, 需 AutoDream PRD 加 FR (留 Open) |
| 写 Laputa 稀薄文档层 (统一接口) 的 UI 入口 | ✅ IN | D-010.r1 + D-011, `[LAPUTA-STUB]`, 待 Laputa PRD 接 (FR-5xx) |
| Evolution run diagnostics 面板 | ✅ IN | 新增组件 EvolutionRunDiagnostics.vue (FR-3xx) |
| Memory changelog 顶级页 | ✅ IN | 提升自 settings/ (FR-3xx) |
| EvolutionProposal 审批路径: **全 user 审** | ✅ IN | D-006, 无 auto-merge |
| AutoDream 数据后端 (跨会话反思) | ↗ REF | docs/prds/prd-autodream-2026-06-12/ |
| Report System (月报 / 固化展示 / 搜索) | ↗ REF | docs/prd-report-system/ |
| MEMORY.md 渲染三层 (Always/Relevant/Archive) | ✅ IN | D-007 硬规则, §4.1 |
| MemoryProvider 最小契约 (evolve_session_end + subscribe_proposal_inbox) | ✅ IN | D-008 最小契约, §3 |
| 文件干预机制 (_intervene/_stop/_keyinfo) | ❌ OUT | D-005, stub 指向 Phase 3.4 |
| 知识图谱 / 技能生态 / Kanban / Plan mode | ❌ OUT | 后续 PRD 承载 |

## 5. Features & Functional Requirements

(Coaching 节 5, FR ID 稳定全局编号。每条 MUST 含: 标题 / 描述 / 验收。)

### 5.1 FR-1xx — 记忆候选交互 (Inbox 表面, 主战场)

**FR-101 — Inbox 列表展示 EvolutionProposal**
MUST 在 Inbox 表面展示所有 `status in (pending_review, needs_attention)` 的 EvolutionProposal。每条 MUST 显示: proposal_type, target_file path, evidence_excerpt 摘要 (≤120 字符), risk_level badge, created_at 时间。列表 MUST 支持按 status / risk_level / proposal_type 过滤, MUST 支持按 created_at 倒序默认排序。
验收: 启动后 Inbox 看到 ≥1 条 mock proposal, 过滤条件可改可重排。

**FR-102 — 单条 EvolutionProposal 详情查看**
MUST 能在 Inbox 点开 1 条 proposal, 看到完整 evidence_excerpt + 完整 proposed_patch (diff 视图) + target_file 路径 + 关联 session_id / journal_ref。详情页 MUST 显示 4 个动作按钮: 批准 / 拒绝 / 编辑 / 应用; MUST 显示 1 个跳转按钮: 跳转到 Chat 同主题。
验收: 点开 1 条 mock proposal, 看到 5 个按钮可点, 编辑动作可进入 diff 编辑器。

**FR-103 — 批准/拒绝/编辑/应用 单条 proposal**
MUST 能对单条 proposal 执行 4 个动作之一。批准 → status=approved (待 apply)。拒绝 → status=rejected (终态)。编辑 → 进 diff 编辑器改 proposed_patch, status=edited (待 apply)。应用 → 把 proposed_patch 写入 target_file, status=applied, 写入 changelog 1 条。
验收: 4 个动作在 stub 模式下都能跑通, 状态正确流转, 应用时 changelog +1。

**FR-104 — 批量 approve/reject** [D-006 硬约束]
MUST 能在 Inbox 列表多选 + 工具栏执行 "批量批准" 或 "批量拒绝"。批量操作 MUST 走同一 API, 但 MUST 逐条记录 changelog (1 批 1 changelog 条目, 含所有 proposal_id 引用)。
验收: 选中 3 条 mock proposal, 1 次 "批量拒绝" → 3 条 status=rejected, changelog +1 (含 3 个 id)。

**FR-105 — 标记已读** [D-006 硬约束]
MUST 能在 Inbox 列表标记单条或批量标记 proposal 为已读, 已读后 MUST 从默认 "未读" 过滤消失, 但 MUST 仍可经 "全部" 过滤看到。
验收: 1 条 proposal 标记已读, "未读" 过滤下消失, "全部" 过滤下仍在。

**FR-106 — 撤回已应用的 proposal** [D-006 硬约束]
MUST 能在 proposal 详情页对 status=applied 的 proposal 执行 "回滚"。回滚 MUST 调 changelog 反操作 (恢复 target_file 上一版本), status MUST → reverted, changelog MUST +1 (type=revert, 引用原 apply 的 changelog_id)。
验收: 1 条已应用 proposal 回滚后, target_file 内容回到原状, changelog +1 revert 条目。

**FR-107 — 跨表面跳转 (Inbox → Chat 同主题)**
MUST 能在 Inbox 详情页通过 1 个动作按钮跳到 Chat 表面的同主题对话 (经 proposal.evidence_excerpt 锚定的 session_id)。跳转 MUST 保留用户当前 Inbox 状态, 离开 Chat 后可一键返回。
验收: 从 Inbox 跳到 Chat, 看到对应 session 历史, 返回 Inbox 仍见原列表。

**FR-108 — 搜索/过滤** [D-006 硬约束]
MUST 能在 Inbox 顶部搜索框按 evidence_excerpt / target_file 关键词搜索, MUST 支持按 proposal_type / status / risk_level 多条件组合过滤。
验收: 搜索 "auth" 在 mock 数据中能命中, 多条件过滤组合正确。

**FR-109 — 键盘快捷键** [D-006 硬约束]
MUST 在 Inbox 支持至少 5 个键盘快捷键: J/K 上下移, A 批准当前, R 拒绝当前, E 编辑当前, / 聚焦搜索框。快捷键 MUST 在搜索框 focus 时禁用 (避免误触)。
验收: 按 J/A/R/E 各能触发对应动作, 搜索框 focus 时快捷键不响应。

### 5.2 FR-2xx — 固化流程 (Journal 表面, 核心, D-006 强约束)

**FR-201 — 从 Journal 报告固化为 SOP**
MUST 能在 Journal 报告详情页通过 "固化为 SOP" 按钮 (NotebookView 底部 3 动作之一已存在) 触发。固化 MUST 走 Laputa `POST /api/laputa/proposals/{id}/apply` (per FR-501, stub 拆后), proposal_type=`sop_create`, target=#1 identity 内 sop sub-section。状态: status=applied + changelog +1。验收: 1 条 mock Journal 报告固化后, Laputa 收到 sop_create proposal, changelog 1 条, identity section 含 SOP 文件引用。

**FR-202 — 从 Journal 报告固化为 Skill**
MUST 能在 Journal 报告详情页通过 "固化为技能" 按钮触发。固化走 Laputa 统一入口, proposal_type=`sop_create`, target=#1 identity 内 skill sub-section (跟 SOP 同 proposal_type, sub-section 区分)。验收: 同 FR-201, 但落到 skill sub-section。

**FR-203 — 从 Journal 报告更新长期记忆 (Laputa) — [LAPUTA-STUB] 拆**
MUST 能在 Journal 报告详情页通过 "更新长期记忆" 按钮触发。固化走 Laputa `POST /api/laputa/proposals/{id}/apply`, proposal_type 由 content type 决定 (`memory_patch` → #5 / `learning_note` → #4 / `identity_patch` → #1 / `relationship_update` → #2)。**降级契约**: Laputa API 不可用时 MUST 写 `{workspace}/memory/pending.jsonl`, 启动时 retry。验收: 1 条固化触发, Laputa 4 接口中至少 1 个被调, changelog +1; Laputa 挂时写 pending.jsonl, 启动后 retry 成功。

**FR-204 — 撤销已固化的**
MUST 能在 Journal 报告详情页对 status=applied 的固化操作执行 "回滚"。走 Laputa `POST /api/laputa/changelog/{id}/rollback` (per FR-403), 30 天窗口, 标 reverted=true。验收: 1 条已固化的回滚, Laputa 收到 rollback, changelog 标 reverted, target_section 回到 before。

**FR-205 — 批量固化**
MUST 能在 Journal 列表多选 + 工具栏 "批量固化" 一键, 1 批 1 changelog 条目, 含所有固化 proposal_id 引用。验收: 选 3 条 mock 报告, 1 次 "批量固化 SOP" → 3 条 proposal_type=sop_create, 1 个 changelog (含 3 id)。

### 5.3 FR-3xx — 节律/触发/诊断 (Settings + Chat manual + diagnostics)

**FR-301 — 配置 daily/weekly/manual 节律**
MUST EXTEND 现有 `SelfEvolutionSettings.vue`, 不重写。新增字段: enabled (bool) / frequency (daily | weekly | manual) / sessions_threshold (int) / messages_threshold (int)。Tauri 命令 `get_self_evolution_config` / `save_self_evolution_config` 已存在, 不另开。验收: 改 frequency=weekly 后, AutoDream 下次触发按 weekly 跑, 不按 daily。

**FR-302 — 手动 in-session 触发 (D-009 核心)**
MUST 暴露 Chat 命令 `/dream整理本周日报` / 卡片按钮 "立即整理", 同步调用 AutoDream 子流程 (D-009 manual 模式, 同步, 不依赖节律)。返回报告 markdown, 渲染到 Chat 消息或 Journal。验收: Chat 输入 `/dream整理本周日报`, 1s 内出日报, 写到 Journal。**Laputa 拆 stub 后**: 同步调用路径走 Laputa FR-301 fallback (`GET /api/laputa/proposals?since` 不够, 需新加 `POST /api/laputa/proposals/run-now` 同步 trigger, 留 Open)。

**FR-303 — Evolution run diagnostics 面板**
MUST 新增组件 `EvolutionRunDiagnostics.vue` (Settings 路由), 展示 5 字段: run_id / start_at / end_at / status (running/success/failed/needs_attention) / errors (array)。数据源: Laputa `GET /api/laputa/events/errors` (per FR-303) 60s 轮询。验收: 上次 AutoDream 失败的 run, 面板 1s 内显示 needs_attention + error 详情。

**FR-304 — Trust & safety 简化 (D-006 隐含)**
`require_confirmation_for` 配置从 5 复选 (identity/relationship/commitment/sop/deprecation) 简化为 1 总开关 `user_must_approve_all_writes: bool` (因 D-006 全 user 审)。`SelfEvolutionSettings.vue` UI 同步改。验收: 用户改 1 总开关, 5 类全审/全 auto 切换生效, FR-1xx 行为对。

**FR-305 — Laputa 订阅 fallback (D-008 P2)**
MUST 实现 60s 轮询 fallback `GET /api/laputa/proposals?since=ISO8601`, 客户端在 SSE 不可用时切轮询。功能等价 SSE。验收: SSE 断, 1 个新 proposal 产生, 客户端 60s 内拉到。

### 5.4 FR-4xx — Changelog 路由 (list/detail/rollback, Laputa 拆 stub)

**FR-401 — GET /api/selfinprove/changelog (list)**
MUST 返 paginated MemoryChangelog list (per Glossary), 5 字段 (id / action / target_section / proposal_id / created_at)。Query: `page` / `page_size` / `target_section` / `action` / `since` / `until`。底层走 Laputa `GET /api/laputa/changelog` (per Laputa FR-401) 代理。验收: mock 100 条, page=1 page_size=20 返 20 条 + has_more=true。

**FR-402 — GET /api/selfinprove/changelog/{id} (detail)**
MUST 返 MemoryChangelog 详情, 含 before/after diff (unified diff 格式) + 关联 proposal_id + audit_event_id。底层走 Laputa `GET /api/laputa/changelog/{id}` (per Laputa FR-402) 代理。验收: 1 条 changelog 查详情, diff 正确。

**FR-403 — POST /api/selfinprove/changelog/{id}/rollback — [LAPUTA-STUB] 拆**
MUST 接收 rollback 请求, 走 Laputa `POST /api/laputa/changelog/{id}/rollback` (per Laputa FR-403), 30 天窗口, 写新 changelog (action=rollback, 引用原 id)。底层走 Laputa 真实接口, 不再 mock。验收: 1 条 rollback 走通, target_section 回到 before, 旧 changelog 标 reverted; 31 天前返 Err::RollbackExpired。

### 5.5 FR-5xx — 14 文件 schema + 旧 8 模板迁移 (Laputa-owned, selfinprove 引用)

**FR-501 — 14 content section schema 引用 — [LAPUTA-STUB] 拆**
MUST 引用 Laputa PRD §4.1 14 section 分类 (per Laputa FR-501), selfinprove 侧不重新定义 section schema。8 种 proposal_type MUST 按 Laputa FR-102 路由 (memory_patch → #5 / learning_note → #4 / identity_patch → #1 / relationship_update → #2 / commitment_set → #3 / sop_create → #1 sub-section / journal_note → #10 ⚠️ TBD / deprecation → changelog)。验收: 8 种 proposal_type 各路由到正确 section, TBD section 走 TBD pool 模式。

**FR-502 — 旧 8 模板 → Laputa 1 稀薄层 映射 (引用 Laputa)**
MUST 引用 Laputa FR-502 8→14 映射表, selfinprove 不重写。0 写权限, 仅引用。

**FR-503 — 迁移策略 (引用 Laputa)**
MUST 引用 Laputa FR-503 迁移 (TBD pool + staging buffer + atomic swap)。selfinprove 0 写权限, 不实接。

**FR-504 — 8 模板废弃计划 (引用 Laputa)**
MUST 引用 Laputa FR-504 1 release 后删旧模板。selfinprove 0 写权限。

### 5.6 FR-6xx — Mentle 边界 (Laputa-owned, selfinprove 引用)

**FR-601 — Laputa 写 authority, Mentle 仅 recall (引用 Laputa FR-601)**
MUST 引用 Laputa FR-601 caller 检查。selfinprove 0 写权限, 仅消费 Laputa 4 接口。

**FR-602 — Mentle 召回 read 路径 (引用 Laputa FR-602)**
MUST 引用 Laputa FR-602 `LaputaReadAdapter`。selfinprove 0 写权限。

**FR-603 — Mentle 召回上下文注入规则 (引用 Laputa FR-603)**
MUST 引用 Laputa FR-603 `mentle_recall_policy: "off" | "on_demand"` (默认 on_demand, **无 "always"**, 跟 B §3 §8 原则一致)。selfinprove 0 写权限。

### 5.7 FR-7xx — Non-Functional Requirements

**FR-701 — 性能**
- Inbox 列表加载 MUST ≤500ms for 100 proposals (mock 数据)
- Laputa 4 接口代理调用 MUST ≤100ms each (Laputa FR-701 已锁)
- 60s 轮询 fallback MUST ≤5s 整链路 (per FR-305)
验收: 4 类操作均达指标, 用 `wrk` 或 `hey` benchmark。

**FR-702 — 审计完备**
所有 selfinprove 写 Laputa 操作 MUST 经 Laputa FR-702 审计通道, selfinprove 0 独立 audit 通道。`fs::write(.*\.laputa` 4 种 pattern 命中 0 处 (Laputa FR-702 已锁)。

**FR-703 — 撤销完备**
已应用的固化 MUST 可在 30 天内被回滚 (走 Laputa FR-403 4 边界: 毫秒精度 / UTC / 已撤销级联 / 中断恢复)。Laputa FR-703 已锁, selfinprove 引用。

**FR-704 — 并发安全**
flock 跨进程 + 死锁恢复 引用 Laputa FR-704。selfinprove 0 独立 flock 逻辑。

**FR-705 — 错误处理**
Laputa 7 种 Err 类型 (SchemaIncompatible / Unauthorized / LockTimeout / ConflictUnresolved / RollbackExpired / UnknownLayer / IoError, per Laputa FR-705) MUST 透传到 selfinprove UI, 不静默。MUST 新增 selfinprove 侧 3 种 Err: PendingJsonlFull (降级文件满) / ManualTriggerTimeout (FR-302 超 30s) / SubscriptionReconnectFailed (SSE 重连失败 3 次)。验收: 10 种错误类型各能触发, 返对应 Err。

**FR-706 — 可观测**
引用 Laputa FR-706 3 metrics (`laputa_writes_total` / `laputa_write_errors_total` / `laputa_rollbacks_total`)。selfinprove 0 独立 metrics, 仅消费 Laputa 暴露。


## Addendum

(空 — 材料汇总后由子 agent extract 落入。)

## 6. Open Questions (Coaching 留档)

(经 §5 FR 起草 + 3 reconcile + R1 rubric + Laputa PRD finalize 后, 11 条来自子 agent 调研 + 2 条新增 = 13 条。)

**Laputa PRD finalize 后状态 (per D-008 OQ 1-9 closure)**:
1. ✅ 写权限契约 — Laputa FR-101/501/601 已 full, selfinprove FR-501 stub 拆
2. ✅ Selfinprove 与 Laputa"自主 Level 1-2 + 进阶心跳"边界 — 转 Laputa 架构后续 PRD (D-008 OQ#12-14)
3. ✅ Mentle 与 selfinprove 写权限调和 — selfinprove FR-6xx 引用 Laputa FR-6xx
4. ✅ SOUL.md "AI 自写, 用户不可直接编辑" — Laputa D-012 OQ#4 (a) 锁定, user_only / agent_self 区分明确
5. ✅ AutoDream Orient 阶段读 Laputa 稳定路径 + schema 版本 — Laputa FR-506, selfinprove 引用
6. ✅ AutoDream FR-11 "写 MEMORY.md" 与 Non-Goals §5 矛盾 — Laputa FR-102/501 强制走 Laputa
7. 🟡 Report System FR-8 是 Laputa 第 2 写消费方 + pending.jsonl 降级 — selfinprove FR-203 降级契约已含, Report System 消费方背书 (D-008 OQ#7 半关)
8. ✅ memory.changelog 路由 — Laputa FR-4xx, selfinprove FR-4xx stub 拆
9. ✅ Subscribe push (SSE/WS) vs pull (60s 轮询) — Laputa FR-301, selfinprove FR-305

**新增 (10-13)**:
10. 🔴 开 (Polish 待补): FR-302 manual trigger 同步调用路径需新加 `POST /api/laputa/proposals/run-now` 端点 (Laputa 端), Laputa PRD v0.0.6 未列, **Laputa 架构后续 PRD 承载**
11. 🟡 半关 (R1 critical #1): FR-2xx~6xx outline → 已扩 full (D-013 Polish), 实际不关
12. ✅ 关 (R1 critical #2): §6 Open Questions 空白 → 已 populate (D-013)
13. ✅ 关 (R1 critical #3): §3 Glossary 缺失 → 已加 5 词条 (D-013)
14. ✅ 关 (R1 critical #4): §7 SM 缺失 → 已加 4 SM (D-013)
15. ✅ 关 (R1 high #5 + reconcile-laputa G8): FR-501 stub → 已拆, stub 引用 Laputa FR-501
16. ✅ 关 (audit): D-010 stale → 已回写关闭 (D-013)

## 7. Success Metrics

(launch 级, 4 SM + 2 counter-metrics。)

**Primary**:
- **SM-1 — 记忆候选每日审量**: 用户每日审查 proposal 数 ≥10 (从 Inbox 0 状态过滤后剩 pending_review)。Validates FR-1xx + D-006 全 user 审, **30 天后 ≥80% 用户日均 ≥10** (per Vision "用户每天 ≤5 分钟")
- **SM-2 — 固化采纳率**: 用户 approve 的 proposal / 用户总审 proposal ≥50% (含编辑后 apply)。Validates FR-2xx + FR-203 stub 拆, **30 天后稳定在 60-70%** (过低 = 候选质量差, 过高 = 噪音多)
- **SM-3 — Laputa stub 拆覆盖率**: 4 接口 (FR-501/502/203/403) stub 拆后真实调用率 ≥95%。Validates D-011 ship 策略

**Counter-metrics (不优化)**:
- **SM-C1 — auto-merge 比例**: 0 (D-006 全 user 审, auto-merge 永为 0)。若 >0 触发告警, 违反 D-006
- **SM-C2 — proposal 噪度**: 用户拒绝率 >40% 触发, 说明 candidate extraction 质量下降。Validates SM-2 平衡

## 8. Open Questions (Finalize 后)

(本节只留 Laputa final 后的硬阻塞, 多数已转 follow-up PRD。)

- 无新增 P0 (Laputa PRD 已 final, 9 OQ 1-9 已关; 10 转 Laputa 架构后续 PRD; 11 已扩 FRs; 12-16 已 polish)

## Revision Notes

- v0.0.1 — 2026-06-12: 起骨架, FR-1xx 写接口 9 条起草, scope 决策表初版。
- v0.0.2 — 2026-06-12: Vision / Target User / Scope 锁定, Decision log D-001~D-009 落档, 14 份 3 分类。
- v0.0.3 — 2026-06-12: D-009 节律双模 + D-010 Laputa 写权限依赖 + D-011 一锅端 ship 决策落档, FR-1xx 风格审通过。
- v0.0.4 — 2026-06-12: Finalize step 2 (input reconciliation) 完。3 子 agent 写 3 reconcile 文件 (laputa / autodream / report-system), 8+12+12 gap。IR-1 (Laputa v0.0.6) 我自己跑 (子 agent 失败 @ max_iterations)。
- v0.0.5 — 2026-06-12: Finalize step 3 (reviewer gate) 完。1 子 agent 写 review-rubric (21.7KB, verdict 不能 ship, 5 critical + 多 high); 2 子 agent (adversarial / edge case) 失败 (timeout / max_iterations), R1 5 critical + 32 reconcile gap 足够驱动 polish。
- v0.0.6 — 2026-06-12: **status: final**。Finalize step 4 (Triage) + 5 (Polish) + 7 (Close) 完。Polish 7 件: (1) §3.5 Glossary 5 词条新增; (2) FR-2xx 5 条全本扩; (3) FR-3xx 5 条全本扩; (4) FR-4xx 3 条全本扩 (Laputa stub 拆); (5) FR-5xx 4 条扩 (引用 Laputa); (6) FR-6xx 3 条扩 (引用 Laputa); (7) FR-7xx 6 条 NFR。+ §6 OQ populate 16 条; + §7 SM 4 条; + D-010 stale 关闭; + frontmatter `status: final` + `follow_up_prds` 标注 4 后续 PRD (Laputa 架构 / AutoDream v2 / Mentle 集成 + Laputa 架构承载 FR-302 同步 trigger 端点)。总 PRD 规模: 35 FRs (FR-1xx 9 + 2xx 5 + 3xx 5 + 4xx 3 + 5xx 4 + 6xx 3 + 7xx 6) + §3.5 Glossary 5 词条 + §6 OQ 16 + §7 SM 4。Step 6 External handoffs: skip。下一步: Sprint Change Proposal 拆 selfinprove stub (FR-501/203/403), Laputa 4 接口实接。
