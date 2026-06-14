---
title: "Laputa — Agent-Diva 稀薄文档层"
status: final
created: 2026-06-12
updated: 2026-06-12
version: 0.0.6
project: agent-diva-pro
author: John (PM)
related_prds:
  - docs/prds/prd-selfinprove-2026-06-12/prd.md  # [LAPUTA-STUB] 持有方, 待拆 stub
  - docs/prds/prd-autodream-2026-06-12/prd.md     # 只读消费方 (FR-7 Orient)
  - docs/prd-report-system/prd.md                  # 隐式第 2 写消费方 (FR-8 memory provider)
decision_log: .decision-log.md
inherits:
  - selfinprove/D-010.r1 (Laputa = 稀薄文档层, 取代原 SOUL.MD 等)
  - selfinprove/D-011 (一锅端 ship, selfinprove FR-5xx 走 stub, 待本 PRD 实接)
follow_up_prds:
  - Laputa 架构后续 PRD (D-008 OQ#12-14: 三轴主体性 / 进阶心跳 / MemoryProvider 4 lifecycle)
  - AutoDream PRD (D-008 OQ#15-16: AutoDream 4 thin + 8 should-not / Heartbeat→RhythmPolicy 触发链)
  - Mentle 集成 PRD (D-008 OQ#17-18: Mentle 5 wing / 写白黑名单 / work_memory / Authority 4 级)
---

# PRD: Laputa — Agent-Diva 稀薄文档层

> **状态**: draft (Coaching path)
> **本文档正在 John (PM) 与大湿 的 Coaching 协作中生成, 禁止直接落稿后即视为 final。**
> **本 PRD 是从 0 定义 + 实接 Laputa: 0 真实现, 全字符串模板 + 注释 + 探测 stub。**

## 0. Document Purpose

待 Coaching 第 0 步完成填写。

(子 agent 调研输入: 4 份 sibling 决策文档 + 3 份现有 PRD + 完整代码扫描。结果已落 .decision-log.md 草稿区。)

## 1. Vision

(锁定 v0.0.5 — D-010 reviewer gate 4 critical 修 + 3 high 修后冻结。)

**Thesis (新增 per D-010 reviewer C-2)**: Laputa 是 agent-diva-pro 的 subject-file substrate — 让 agent 跨 session 累积用户授权的 identity/preference/relationship, 用户每天 5 分钟能审完变更。

**Laputa 现状 (D-002 + D-005 勘误)**: 0 真实现 (Laputa 自身) + Mentle 已实现 (3 source file + 集成文档 + CI 策略)。1 稀薄统一层, 14 个 content section (1-6 Laputa-owned / 7-9 report-system-owned / 10-14 TBD 等待最终综合 PRD), 统一写接口。

**Ship 策略 (D-006)**: 实接深度 = 全 (写+读+事件+changelog 4 接口), selfinprove FR-5xx stub 全消, 完成后单独发 Sprint Change Proposal 拆 stub。

**Laputa 跟现有系统的边界 (D-005)**:
- **selfinprove**: 第 1 写消费方 (经 proposal_inbox 审查后调 Laputa 写接口)
- **AutoDream**: 只读消费方 (FR-7 Orient 阶段读 Laputa 路径, 不直写)
- **Report System**: 第 2 写消费方 (FR-8 memory provider, 走 Laputa 写接口)
- **Mentle**: 已实现, 仅 recall, 不写 authority (跟 Laputa 边界走 FR-6xx)

## 2. Target User

待 Coaching 第二节填写。

候选受众 (待 5 问 Q2 拍):
- **实现者** (Rust + Vue 工程师, 给"如何写"指导)
- **架构师** (设计者, 给"为什么这样"决策记录)
- **文档读者** (用户/贡献者, 给"是什么"概念解释)
- **以上全要**

## 3. Concerns

待 Concern scan 完成后填写。已识别候选 (来自 3 stream 调研):
- 写权限契约: 写哪个文件 / 写格式 / 并发 / 冲突 (P0)
- Selfinprove 与 Laputa"自主 Level 1-2 + 进阶心跳"边界 (P0)
- Mentle 与 Laputa 写权限调和 (P0)
- SOUL.md"AI 自写, 用户不可直接编辑" — 用户怎么干预 (P0)
- AutoDream Orient 阶段读 Laputa 稳定路径 + schema 版本 (P0)
- AutoDream FR-11 写 MEMORY.md 跟 Non-Goals §5 矛盾 (P0)
- Report System FR-8 是 Laputa 第 2 写消费方, 消费方名单 (P0)
- memory.changelog 路由 Laputa 出还是 selfinprove mock (P0)
- Subscribe push vs pull 端点类型 (P0)
- 14 文件清单 vs 7 文件原型 vs 1 稀薄层, 现状归属 (P0)

## 4. Scope Boundaries (Working Draft)

(锁定 v0.0.2 — D-005 + D-007 后冻结。)

### 4.1 Laputa 物理形态 (D-007)
- **1 稀薄统一层**: `.laputa/` 目录 + 1 份主 file (e.g. `state.json` 或 `index.md` + 14 section)
- **14 content section** (在 1 份主 file 内, 1 稀薄层), 分配:

| 类别 | 项目 | 内容 schema 谁定 | Laputa PRD 写什么 |
|------|------|------------------|------------------|
| **Laputa-owned** | #1 identity / #2 relationship / #3 commitment / #4 preferences / #5 MEMORY.md / #6 HISTORY.md | 本 PRD 详写 | section schema + 写路由 FR-1xx |
| **Report-system owned** | #7 daily / #8 weekly / #9 monthly | report-system PRD | 仅出写接口 (FR-1xx 路由) |
| **TBD** ⚠️ | #10 journal-reflective / #11 evolution proposal inbox / #12 subject-file changelog / #13 report indexes / #14 AAAK summaries | 等待最终综合 PRD | 通用 section 写入接口 |

### 4.2 旧 8 模板迁移 (D-002 已发现)
- 旧 8 模板 (SOUL/MEMORY/HISTORY/IDENTITY/USER/BOOTSTRAP/PROFILE/TASK) 仍活在 `agent-diva-core/src/utils/mod.rs:37-66` + `sync_workspace_templates()` 注入
- **迁移策略** (待 FR-5xx 详写): 旧 8 模板 → 1 稀薄层 14 section, `sync_workspace_templates()` 改写

### 4.3 不在本 PRD 范围
- 旧 8 模板保留兼容 (不删, 仅迁移路径)
- Mentle 实现本身 (已实现, 本 PRD 不重写)
- AutoDream / Report System / Selfinprove 内部逻辑 (它们是消费方)

### 4.4 Non-Goals (Explicit, per D-010 reviewer H-3)

Laputa PRD **明确不做**:
- **不是 DB / Vector store / Graph DB**: P0 阶段 file-only, 不引入 sqlite / pg / qdrant / neo4j (per A §5 "P0 阶段 Laputa 只 file-only, 不要 db/graph/vector/resonance/scheduler")
- **不是 Agent runtime**: AgentLoop / ContextBuilder / Provider 仍在 GenericAgent, 不迁移到 Laputa
- **不是 Scheduler / Cron**: 触发链归 AutoDream, Laputa 仅被动接 proposal
- **不是 Auth / Identity Provider**: 写权限 = caller 信任, 不做 JWT / OAuth / API key
- **不是 AutoDream**: 4 thin responsibilities + 8 should-not-own 负向契约归 AutoDream PRD (per D-008 OQ#15)
- **不是 Mentle 集成**: 5 wing / 写白黑名单 / work_memory 归 Mentle 集成 PRD (per D-008 OQ#17)
- **不是 Laputa 架构**: 三轴主体性 / 进阶心跳 / MemoryProvider 4 lifecycle 归 Laputa 架构后续 PRD (per D-008 OQ#12-14)
- **不是知识图谱 / 技能生态 / Kanban / Plan mode**: 后续 PRD
- **不是月报 / 文件干预 (_intervene/_stop) / 模糊控制**: OUT of v1 (per selfinprove D-005 + D-011)

## 5. Features & Functional Requirements

(Coaching 节 5, FR ID 稳定全局编号。每条 MUST 含: 标题 / 描述 / 验收。⚠️ = TBD 等待最终综合 PRD。)

### 5.1 FR-1xx — Laputa 写接口 (4 治理: review/changelog/audit/rollback)

**FR-101 — Laputa 写入口统一契约**
Laputa MUST 暴露 1 个统一写入口 `LaputaWrite::apply_proposal(proposal: &EvolutionProposal) -> Result<ChangelogRecord, WriteError>`, 接受 selfinprove FR-501 stub 消除后的所有写请求。入口 MUST 走 review + changelog + audit + rollback 4 道治理。
验收: stub 接 stub, mock 返回 ChangelogRecord; 实接后 changelog +1。

**FR-102 — 写目标层判定 (Layer Routing)**
Laputa MUST 根据 EvolutionProposal.proposal_type 路由到目标 section:
- `memory_patch` → #5 MEMORY.md
- `journal_note` → #10 journal-reflective ⚠️ (TBD pool, 见 FR-503 备注)
- `learning_note` → #4 preferences
- `identity_patch` → #1 identity
- `relationship_update` → #2 relationship
- `commitment_set` → #3 commitment
- `sop_create` → 通用 sop section (合并入 #1 内的 sub-section)
- `deprecation` → 写 changelog (应用层), 不写正文
TBD sections (#10-14) MUST 接受写入 (per D-010 reviewer C-2, TBD pool 模式), content 存为 raw bytes, status=tbd, schema 待最终综合 PRD 决定。MUST 拒收 proposal_type 未知 (非 8 种之一) 的 proposal, 返回 Err::UnknownProposalType。
验收: 8 种 proposal_type 各路由到正确 section (含 #10 TBD pool), 未知 proposal_type 返 Err。

**FR-103 — 写权限治理 4 道 (review/changelog/audit/rollback)**
Laputa 写操作 MUST 走 4 道治理 (per stream A finding 2):
1. **review**: proposal 进入 inbox 等 user 审 (per selfinprove D-006 全 user 审)
2. **changelog**: 写入前 MUST 记录 ChangelogRecord (含 before/after diff)
3. **audit**: 写入后 MUST 发 AuditEvent (who/when/what/before/after 5 字段)
4. **rollback**: 用户 MUST 能在 30 天内回滚 (FR-403 复用)
4 道 MUST 串联, 任一失败 MUST 整体回滚 (atomic)。**atomic 边界 (per D-010 reviewer H-1 + edge case #1)**: changelog 落盘后 audit 失败时, MUST 把 changelog 标 `status=rolled_back` 并反向修复 target_section; target_section 已 commit 时 MUST 走反向 patch, 不得"假装回滚"留 target_section 在新状态。
验收: 模拟 1 次写, 4 道都触发; 模拟 audit 失败 (changelog 已落), 整体回滚, target_section 回到 before。

**FR-104 — 并发控制 (写锁)**
Laputa 写 MUST 走文件锁 (flock), 同一 target_section 并发写 MUST 串行化, 不同 target_section 并发写 MUST 可并行。
**跨进程 + 死锁恢复 (per D-010 edge case #3)**: flock MUST 跨进程工作 (用 OS flock 而非进程内 Mutex), MUST 加 deadlock detection (timeout 5s 必报), 死锁时 MUST 自动释放 + 写 AuditEvent 标 `status=needs_attention`, 不得无限等待。Windows / Linux / macOS 跨平台 MUST 行为一致 (POSIX flock 优先, Windows 用 `LockFileEx` fallback)。
验收: 2 个并发写同 target_section (跨进程), 后者等前者完成; 模拟死锁 (A 持锁等 B, B 持锁等 A), 5s 后自动释放 + 审计。

**FR-105 — 冲突解决 (三向合并)**
Laputa 写 MUST 检测冲突: 当 current_target 跟 expected_before 不一致时, MUST 走三向合并 (current + base + proposed)。冲突不可自动解时, MUST 写 AuditEvent 标 `status=needs_attention` 并通知 user。
验收: 模拟 1 次冲突写, 走三向合并或标 needs_attention, user 收到通知。

**FR-106 — Laputa 与 Mentle 写边界**
Laputa MUST 拥有 authority write; Mentle MUST 仅 recall (read), MUST NOT 写 Laputa。MUST 在 Laputa 入口处检查 caller, 非 Laputa-internal 写请求 MUST 走 LaputaWrite::apply_proposal, 不得绕过。
验收: 模拟 Mentle 尝试写 Laputa, 拒绝; 模拟 selfinprove 写, 走统一入口。

### 5.2 FR-2xx — Laputa 读接口 (snapshot + per-section)

**FR-201 — GET /api/laputa/snapshot**
MUST 返 1 个完整 snapshot 对象, 含 identity / memory / relationships 3 段 + 其他 section (按 §4 顺序)。TBD sections (#10-14) MUST 标 `status=tbd`, 不得返 null 或缺字段。
响应 schema (摘要): `{ sections: { identity: {...}, relationship: {...}, commitment: {...}, preferences: {...}, memory_md: {...}, history_md: {...}, daily: {...status: "owned"|"tbd"...}, ... }, updated_at: ISO8601 }`
验收: mock 状态 200, 14 section 全列出, TBD 标 status=tbd。

**FR-202 — GET /api/laputa/section/{name}**
MUST 按 section 名切片读, 返 1 个 section 对象 (含 content + metadata + last_modified + version)。name ∈ {identity, relationship, commitment, preferences, memory_md, history_md, daily, weekly, monthly, journal_reflective, proposal_inbox, changelog, report_indexes, aaak_summaries}。TBD sections (后 5 个) 返 status=tbd, 不得 404。
验收: 14 name 各能 GET, TBD 返 status=tbd, 未知 name 返 404。

**FR-203 — GET /api/laputa/snapshot?since=ISO8601**
MUST 返从 `since` 以来变更过的 section 列表 (增量读), 给 UI 实时刷新用。`since` 缺省时返全量。响应 schema: `{ changed_sections: [...], server_time: ISO8601 }`
验收: 写 1 个 section 后, 带 since=写前时间查询, 返该 section 1 个; since=写后时间查询, 返空。

**FR-204 — GET /api/laputa/changelog** (详 FR-401)
MUST 返 paginated changelog list, 详 FR-401。

**FR-205 — 读 TBD sections 时 MUST 返 status=tbd 标志**
MUST 在响应 metadata 显式标 `status: "tbd"`, 不得用 null/空字符串/缺字段 替代。selfinprove UI 据此隐藏"未定义"控件或显示"待定"占位。
验收: 5 个 TBD sections 各自返 status=tbd, 1-9 各自返 status=owned。

### 5.3 FR-3xx — Laputa 事件接口 (subscribe push/pull)

**FR-301 — Subscribe proposal_inbox**
MUST 暴露 SSE 端点 `GET /api/laputa/events/proposals`, 推 EvolutionProposal 变更事件 (created / approved / rejected / applied / reverted)。MUST 支持 fallback: 客户端 MUST 能用 `GET /api/laputa/proposals?since=ISO8601` 60s 轮询, 功能等价 SSE。**fallback 端点 MUST 落 FR-2xx 端点表 (per D-010 reviewer H-3)**: `GET /api/laputa/proposals` 返 proposal_inbox 全量 (filter: status, since)。
**SSE 断网 resume + ring buffer (per D-010 edge case #5)**:
- 断网: 客户端断网时, 服务端 MUST 继续推事件进 ring buffer (上限 1000 条, 超过丢弃最早, 标 `event: buffer_overflow`)
- 重连: 客户端重连时, 客户端 MUST 用 `Last-Event-ID` header 传最后收到的事件 ID, 服务端 MUST 从 ring buffer 重放
- ring buffer 满: 服务端 MUST 发 SSE 事件 `event: buffer_overflow` 通知客户端
事件 payload schema: `{ event_id, proposal_id, proposal_type, target_section, status, timestamp }`
验收: SSE 连上, 写 1 个 proposal, 客户端 1s 内收到事件; SSE 断, 切轮询, 60s 内拉到同一 proposal; fallback 端点直接 curl 返 200 + proposal 列表; 模拟断网 10s 重连, 100 事件全收到; 模拟 ring buffer 满, 客户端收到 buffer_overflow 事件。

**FR-302 — Subscribe changelog**
MUST 暴露 SSE 端点 `GET /api/laputa/events/changelog`, 推 ChangelogRecord 变更 (apply / revert / rollback)。给审计 UI 实时刷新用。
事件 payload schema: `{ event_id, changelog_id, action: "apply"|"revert"|"rollback", target_section, timestamp }`
验收: 写 1 个 changelog, 审计 UI 1s 内收到事件。

**FR-303 — Subscribe error (needs_attention)**
MUST 暴露 SSE 端点 `GET /api/laputa/events/errors`, 推 FR-105 冲突不可自动解时的 needs_attention 事件。
事件 payload schema: `{ event_id, error_type, target_section, attempted_proposal_id, conflict_reason, timestamp }`
验收: 模拟 1 次冲突不可解, 客户端收到 needs_attention 事件, 含 conflict_reason。

### 5.4 FR-4xx — Changelog 路由 (list/detail/rollback)

**FR-401 — GET /api/laputa/changelog** (FR-204 引用)
MUST 返 paginated changelog list。Query params: `page` (default 1), `page_size` (default 20, max 100), `since` (ISO8601), `until` (ISO8601), `target_section` (filter), `action` (filter: apply/revert/rollback), `proposal_id` (关联查询)。
响应 schema: `{ items: [...], total: N, page: 1, page_size: 20, has_more: bool }`
验收: mock 100 条 changelog, page=1 page_size=20 返 20 条, has_more=true; filter target_section=identity 返命中条数。

**FR-402 — GET /api/laputa/changelog/{id}**
MUST 返 1 条 changelog 详情, 含 before/after diff (unified diff 格式), 关联 proposal_id, 关联 audit_event_id。
响应 schema: `{ id, action, target_section, before, after, diff: "--- ...\n+++ ...\n@@ -1,3 +1,3 @@", proposal_id, audit_event_id, created_at, applied_by }`
验收: 1 条 changelog 能查详情, diff 正确显示行号和修改。

**FR-403 — POST /api/laputa/changelog/{id}/rollback**
MUST 接收 rollback 请求, 反向应用 before/after 修复 target_section, 写新 changelog (action=rollback, 引用原 changelog_id), 标原 changelog reverted=true。30 天窗口外 MUST 返 Err::RollbackExpired。
请求 schema: `{ reason: string, expected_current: string (optional, 用于 conflict detect) }`
**30 天撤销 4 边界 (per D-010 edge case #4)**:
- 毫秒精度: 30 天 MUST 按毫秒算, 30 天 + 1ms 后 MUST 仍可撤销 (而非 30 天 24h 整)
- 时区: 30 天 MUST 用 UTC, 客户端本地时间仅显示
- 已撤销级联: 1 条 changelog 被撤销后, 反向引用它的 changelog MUST 标 `stale=true`, 不可再被撤销 (避免死循环)
- 中断恢复: rollback 中途崩, MUST 留 staging 标记, 下次启动继续完成
验收: 1 条 changelog rollback, target_section 回到 before, 新 changelog 写入, 旧 changelog 标 reverted; 31 天前的 changelog 返 Err::RollbackExpired; 30 天 + 1ms 撤销成功; 模拟级联引用标 stale; 模拟中断恢复继续。

### 5.5 FR-5xx — 14 文件 schema + 旧 8 模板迁移

**FR-501 — 14 content section schema**
MUST 定义 14 section 的字段 schema。1-6 详写字段 (identity/relationship/commitment/preferences/memory_md/history_md), 7-9 引 report-system PRD (Laputa 仅存内容, schema 由 Report System 决定), 10-14 ⚠️ TBD 通用 section 写入接口 (允许任意 JSON / markdown / 其他格式, 不绑死 schema)。
MUST 给每个 section 定义:
- `name`: section 唯一标识
- `content_type`: "markdown" | "json" | "structured" | "tbd"
- `max_size_kb`: 建议上限 (memory_md 30 行 ≤2kb, identity 5kb, history_md 50kb, TBD 100kb)
- `render_layer`: "always" | "relevant" | "archive" | "tbd" (per A §22 3 值枚举, 注: 跟 §21 L0-L4 5 层不同, 用 3 值简化)
- `write_authority`: "agent_self" | "user_only" | "shared" | "tbd"

**14 section 写权限映射 (per D-010 reviewer finding C-1, 必填不得留空)**:
| # | section | write_authority | 备注 |
| -- | -- | -- | -- |
| 1 | identity | agent_self | AI 自写 (per D-005) |
| 2 | relationship | agent_self | agent 眼中用户, organic |
| 3 | commitment | user_only | 用户给 agent 的红线, agent 只读 |
| 4 | preferences | agent_self | 周度 organic extraction |
| 5 | memory_md | agent_self | 短记忆, 走 selfinprove D-006 全 user 审 |
| 6 | history_md | agent_self | 长 session 历史 |
| 7-9 | daily/weekly/monthly | agent_self | report-system FR-8 写 |
| 10-14 | TBD | tbd | 等最终综合 PRD |

**写入口归属 (per D-008 OQ#19, 跟 FR-102 8 种 proposal_type 一致)**:
- SOP 产出 → `sop_create` proposal → #1 identity 内 sop sub-section
- Skill 产出 → `sop_create` proposal → #1 identity 内 skill sub-section
- SOUL 演化 → `identity_patch` proposal → #1 identity
- MEMORY 压缩 → `memory_patch` proposal → #5 memory_md
- 上述 4 种产物 FR-102 8 种 proposal_type 已覆盖, 走同一 review queue, 不另开新入口
- `tombstone_strategy`: "deprecation" | "soft_delete" | "none"
验收: 14 section schema 文档完整, TBD 项显式标 content_type=tbd。

**FR-502 — 旧 → 新映射表** (8 → 14 section)
| 旧模板 (路径: `agent-diva-core/src/utils/mod.rs:37-66`) | 新 section | 备注 |
|----|----|----|
| SOUL.md | #1 identity | 合并 SOUL/IDENTITY 字段 |
| IDENTITY.md | #1 identity | 同上 |
| MEMORY.md | #5 memory_md | 字段重命名, 1:1 |
| HISTORY.md | #6 history_md | 字段重命名, 1:1 |
| USER.md | #2 relationship | USER 关注 user-as-seen, 进 relationship |
| BOOTSTRAP.md | 启动时初始化 (一次性) | 不入 Laputa 持久层, 启动后失效 |
| PROFILE.md | #2 relationship | profile-as-seen 进 relationship |
| TASK.md | #10 journal_reflective ⚠️ (TBD pool, content 存为 raw bytes, schema 待最终综合 PRD) | 等 TBD 综合 PRD 定 |
验收: 8 旧模板 → 14 section 1:1 映射 (除 BOOTSTRAP 一次性)。

**FR-503 — 迁移策略**
`sync_workspace_templates()` (路径: `agent-diva-core/src/utils/mod.rs:91-105`) MUST 改写, 不再注入 8 旧模板, 改为:
1. 检查 `.laputa/state.json` 是否存在
2. 不存在 → 创建 1 稀薄层空骨架 (14 section 全空, 标 status=tbd 或 owned)
3. 存在 → 增量迁移: 旧 8 文件如有内容, 按 FR-502 映射迁到 1 稀薄层, 旧文件备份到 `.laputa/legacy/{ISO8601}/`
4. 旧文件保留 1 release, 之后删 (FR-504)
5. **TBD pool 模式 (per D-010 reviewer C-2)**: #10-14 section MUST 接受写入, content 存为 raw bytes, status=tbd, schema 待最终综合 PRD 决定, 写入 MUST 不报 Err
6. **迁移原子性 (per D-010 edge case #2)**: 迁移过程 MUST 走 staging buffer (`.laputa/staging/`), 写完 14 section 后一次性 swap, 失败 MUST 整体回滚到 legacy 备份, 不得"半迁"
验收: 新装 Diva, 启动 1 次, `.laputa/state.json` 存在, 14 section 全列出; 旧装升级, 旧 8 文件内容迁到 1 稀薄层, 旧文件备份; 模拟中途崩, 状态回到 legacy 备份。

**FR-504 — 8 模板废弃计划**
MUST 在 release notes 标 8 旧模板为 deprecated, 1 release 后删 `sync_workspace_templates()` 中 8 字符串常量。Laputa 1 稀薄层成为唯一存储。
时间表: deprecate_now → remove_v0.7.0 (1 release 后)

**FR-505 — 启动时初始化 (BOOTSTRAP.md 一次性)**
BOOTSTRAP.md 是首次启动引导 (1 次性), MUST 不入 Laputa 持久层, 仅首次启动时使用, 之后失效 (mtime 检查)。
验收: 首次启动, BOOTSTRAP 内容注入, 第二次启动 BOOTSTRAP 失效。

**FR-506 — schema 版本标识 (给 AutoDream FR-7 Orient 用)**
MUST 在 `state.json` 顶层标 `schema_version: "1.0.0"` (semver)。Laputa 写 MUST 校验 schema 兼容性, 不兼容 MUST 走 schema 升级流程 (写新 changelog, 标 action=schema_upgrade, 触发 review)。
验收: state.json 含 schema_version, 写时校验, 不兼容返 Err::SchemaIncompatible。

### 5.6 FR-6xx — Laputa ↔ Mentle 边界

**FR-601 — Laputa 写 authority, Mentle 仅 recall**
MUST 在 Laputa 入口处 (FR-101) 检查 caller, 仅 `LaputaWrite::apply_proposal()` 入口可写。Mentle (`agent-diva-agent/src/mentle_runtime.rs`) MUST NOT 调用此入口, 仅可调 `LaputaRead::*` 召回。
Mentle 集成点: `agent-diva-agent/src/agent_loop.rs:570 pub fn mentle_active()` 是 recall 开关, Laputa 写 MUST 不依赖此函数返回值。
验收: Mentle 尝试调 apply_proposal 返 Err::Unauthorized; Mentle 调 snapshot/section 成功。

**FR-602 — Mentle 召回时 read 路径**
Mentle 召回 MUST 走 FR-2xx 读接口 (snapshot / section), 不得直接读 `.laputa/state.json` 物理文件, 避免绑死布局。MUST 在 Mentle 端加 1 层 `LaputaReadAdapter`, 包 FR-2xx HTTP client。
验收: Mentle 召回路径走 HTTP, 不直接 fs::read state.json。

**FR-603 — Mentle 召回上下文注入规则**
MUST 跟现有 `mentle_active()` 集成: Mentle recall 默认不注入 context, 仅显式调用 (e.g. 用户在 Chat 触发 "回忆我的设置") 才进。MUST 在 `.laputa/state.json` 顶层维护 1 个 `mentle_recall_policy: "off" | "on_demand"` 字段 (默认 on_demand, **不支持 "always" — 跟 B §3 §8 Mentle 不默认进 context 原则冲突, "always" 是 escape hatch 仅供调试用, 不进 enum**)。
验收: mentle_recall_policy=on_demand, 普通 session 不注入 Mentle recall; 用户手动触发时注入。

### 5.7 FR-7xx — Non-Functional Requirements

**FR-507 — Laputa 3 hook 实接 (per D-002 留 hook + D-010 reviewer finding C-3)**
MUST 引用 D-002 留的 3 个代码 hook 点, 不留孤儿:
- **Hook 1 写入口**: `agent-diva-manager/src/runtime.rs:335-365` (启用注释段 + `laputa_core::provider::LaputaMemoryProvider::new()`) — 由 FR-101 (Laputa 写入口) 调用
- **Hook 2 启动快照**: `agent-diva-core/src/memory/provider.rs:125-139` (`StartupContextSnapshot.laputa_state_root`) — 由 FR-201/202/203 (读) 引用
- **Hook 3 Mentle 召回边界**: `agent-diva-core/src/memory/provider.rs:247-275` (`laputa_wakeup` / `laputa_project_soul` / `laputa_recall_intent`) — 由 FR-603 (mentle recall) 引用
验收: 3 hook 各被至少 1 FR 引用, grep 验证 `laputa_state_root|laputa_wakeup|laputa_project_soul` 在 prd.md 出现 ≥3 处。

**FR-508 — End-to-end Laputa 闭环 (named-loop, per D-010 reviewer H-4)**
Laputa MUST 支持 1 个 named 端到端闭环: write (FR-101) → changelog (FR-103) → SSE notify (FR-301) → reader (FR-201) → human review (FR-403 if needed)。MUST 提供 1 个 e2e 验收脚本, 模拟 1 个完整 proposal 流程, 验证 4 接口全部触发, changelog +1, 事件推 1 次, 读 snapshot 含新内容, 撤销返 Err 之外流程通畅。
验收: 1 个 e2e test 跑通 (FR-101 → FR-103 → FR-301 → FR-201 → 撤销走 FR-403), 4 接口验证, 全链路 ≤500ms。

**FR-509 — 用户干预通道 (per D-012 OQ#4 锁定 a)**
用户 MUST 只能直接编辑 #3 commitment + #4 preferences, **不能**直接编辑 #1 identity / #2 relationship / #5 memory_md / #6 history_md (per D-005 #1 "AI 自写, 用户不可直接编辑" + 大湿 (a) 拍板)。
用户想改 #1 identity 走 selfinprove proposal flow (跟 agent 写 proposal 同一 review queue, per selfinprove D-006 全 user 审)。
验收: 用户调 `PUT /api/laputa/section/identity` 返 `Err::UserCannotEdit`; `PUT /api/laputa/section/commitment` 成功; `PUT /api/laputa/section/preferences` 成功。

**FR-701 — 性能**
- Laputa 写 MUST ≤50ms (mock 数据 100 行)
- Laputa 读 snapshot MUST ≤100ms
- Laputa 读 changelog list (100 条) MUST ≤200ms
- Laputa 读 single section MUST ≤20ms
验收: 4 类操作在 mock 数据下均达指标, 用 `wrk` 或 `hey` benchmark。

**FR-702 — 审计完备**
所有 Laputa 写 MUST 写 ChangelogRecord + AuditEvent, 不可绕过。Code review MUST 检查: 任何调 `fs::write(.*\.laputa|tokio::fs::write(.*\.laputa|File::create(.*\.laputa|serde_json::to_writer(.*\.laputa` 路径的代码必须走 `LaputaWrite::apply_proposal`。
验收: grep 整个 agent-diva-pro 仓库, 上 4 种 pattern 命中 0 处 (除 LaputaWrite 实现内) (per D-010 reviewer H-2)。

**FR-703 — 撤销完备**
已应用的 changelog MUST 可在 30 天内被回滚 (FR-403)。30 天后 MUST 走 "archive + 不可回滚" 模式 (旧数据保留但拒绝 rollback)。
验收: 应用 1 个 changelog, 30 天内回滚成功, 30 天 + 1s 回滚返 Err::RollbackExpired。

**FR-704 — 并发安全**
flock MUST 正确释放 (无死锁), MUST 加 deadlock detection (timeout 5s 必报)。同 target_section 写串行, 跨 target_section 写并行。
验收: 100 个并发写同 section, 全成功, 无死锁; 100 个并发写 14 不同 section, 几乎同时完成 (≤200ms 总)。

**FR-705 — 错误处理**
写失败 MUST 返 Err (不静默), AuditEvent 标 needs_attention。错误类型枚举: SchemaIncompatible / Unauthorized / LockTimeout / ConflictUnresolved / RollbackExpired / UnknownLayer / IoError。
验收: 7 种错误类型各能触发, 返对应 Err 枚举值。

**FR-706 — 可观测**
MUST 暴露 3 个 metrics (Prometheus 格式): `laputa_writes_total` (counter), `laputa_write_errors_total` (counter, by error_type), `laputa_rollbacks_total` (counter, by target_section)。
验收: 3 metrics 端点可拉, 数据随写/错/回滚实时增。

## 6. Open Questions (Coaching 留档)

(经 §5 FR 起草后, 9 条来自子 agent 调研 + 2 条新增 = 11 条。)

**仍需回答 (影响 Laputa PRD 最终)**
1. ~~写权限契约~~ (FR-1xx 6 条已详, **关**)
2. ~~Selfinprove 与 Laputa"自主 Level 1-2 + 进阶心跳"边界~~ (Laputa PRD 不背, 留 Laputa 架构后续 PRD, **关**)
3. ~~Mentle 与 Laputa 写权限调和~~ (FR-6xx 3 条已详, **关**)
4. SOUL.md "AI 自写, 用户不可直接编辑" — 用户怎么干预, 仅 expectations.md 旋钮够吗 (留 §3 commitment 设计, **开**)
5. AutoDream Orient 阶段读 Laputa 稳定路径 + schema 版本标识 (FR-506 已部分答, **半关**)
6. AutoDream FR-11 "写 MEMORY.md" 与 Non-Goals §5 矛盾 (FR-102 路由 + FR-501 写契约已强制走 Laputa, **关**)
7. Report System FR-8 是 Laputa 第 2 个写消费方, 消费方名单 + pending.jsonl 回灌契约要不要纳入 (FR-1xx 通用入口已支持, pending.jsonl 落降级契约待 §6 详, **半关**)
8. memory.changelog 路由 (list/detail/rollback) Laputa 出还是 selfinprove mock (FR-4xx 3 条已详, **关**)
9. Subscribe push (SSE/WS) vs pull (60s 轮询) (FR-301 已选 SSE + 60s fallback, **关**)
10. **新增**: 旧 8 模板 → 1 稀薄层 迁移测试策略 (FR-503 改 `sync_workspace_templates` 怎么验证回归, **开**)
11. **新增**: BOOTSTRAP.md 内容来源 (谁写首次启动引导? 仓库内置 / 远程拉? 落 FR-505 后, **半关**)

**经 reconcile 后新增 (12-15 转 follow-up PRD)**:
12. **转 Laputa 架构后续 PRD**: 三轴主体性 (SelfModel/SoulSignal 三分类、自反 5 问、自主 4 级阶梯) — 来自 A §1-3, 本 PRD §1 不外推, 留架构 PRD
13. **转 Laputa 架构后续 PRD**: 进阶心跳 (4h 周期 + LLM 评估 + 子代理委派) — 来自 A §3, 0 FR
14. **转 Laputa 架构后续 PRD**: MemoryProvider 4 lifecycle 钩子 (system_prompt_block / prefetch / sync_turn / on_session_end) — 来自 C §20.1, D-008 已锁最小契约, 4 钩子等架构 PRD
15. **转 AutoDream PRD**: AutoDream 4 thin responsibilities + 8 should-not-own 负向契约 — 来自 C §7, 本 PRD FR-102/501 路由已强制走 Laputa, "should-not-own" 归 AutoDream PRD 强制
16. **转 AutoDream PRD**: Heartbeat→RhythmPolicy→AutoDream 触发链 — 来自 C §6, 本 PRD 不背触发端
17. **转 Mentle 集成 PRD**: Mentle 5 wing (search/read/write/update/delete) + 写白黑名单 (9 该写 + 8 不该写) + work_memory 7 字段 + 4 阶段 retrieval — 来自 B §3-10, Mentle 集成后续 PRD
18. **转 Mentle 集成 PRD**: Authority 4 级 ordering + 5 MUST NOT + "casual→identity" 反向保护 — 来自 B §12-13, 本 PRD FR-6xx 边界已锁, 4 级 ordering 等 Mentle 集成 PRD
19. **本 PRD 补 (P1, 等 polish 阶段或后续小 PRD)**: autodream 4 项产物 (SOP/Skill 产出、SOUL 演化、MEMORY 压缩) 写入口归属; rhythm/expectations section 归属; 用户干预通道 (commitment 设计); failure downgrade 4 规则; configuration 5 开关
20. **本 PRD 补 (P1)**: P0 5 步闭环 named-loop FR + P0/P1/P2 phasing + 9 non-goals 段 — 来自 C §24-26, 端到端形状可加 polish 阶段

## Addendum

(空 — 材料汇总后由子 agent extract 落入。)

## Revision Notes

- v0.0.1 — 2026-06-12: 起骨架, 继承 selfinprove D-010.r1 + D-011, 3 子 agent 调研完成, 关键发现: Laputa 0 真实现。
- v0.0.2 — 2026-06-12: D-002 Mentle 勘误 + D-005 14 份分类 (Laputa-owned/Report-system owned/TBD) + D-006 Q2/Q3 落档 (全/全) + D-007 1 稀薄层 layout 锁定, §1 Vision 锁, §4 Scope 锁, §5 FR-1xx 写接口 6 条起草 + FR-2xx~7xx 大纲。下一步: FR-1xx 风格审。
- v0.0.3 — 2026-06-12: §5 FR-2xx 读接口 5 条全本 + FR-3xx 事件 3 条全本 + FR-4xx changelog 3 条全本 + FR-5xx 14schema+迁移 6 条全本 + FR-6xx Mentle 边界 3 条全本 + FR-7xx NFR 6 条全本 (合计 32 FRs); §6 Open Questions 落档 11 条 (5 关, 3 半关, 3 开)。下一步: 走 bmad-prd Finalize 阶段 (reviewer gate + polish + close)。
- v0.0.4 — 2026-06-12: Finalize step 2 (input reconciliation) 完。3 子 agent 写 3 reconcile 文件 (laputa-new-architecture / mentle-laputa / auto-evolution), 识别 2 真 phase-blocker + ~7 转 follow-up PRD + ~2 P1 本 PRD 补。2 真 phase-blocker 修: (a) FR-501 引用 C §21 L0-L4 改 A §22 3 值枚举; (b) FR-603 删 `always` enum (跟 B §3 原则冲突, 改为 escape hatch 文档化)。§6 Open Questions 扩 9 条 (12-20), 7 转 follow-up PRD, 2 P1 本 PRD 补。下一步: 派 reviewer gate。
- v0.0.5 — 2026-06-12: Finalize step 3 (reviewer gate) 完。3 子 agent 写 3 review 文件 (rubric / adversarial / edge case, 共 423 行)。修 4 真 critical + 3 小 high: (1) §1 Vision 加 thesis 句; (2) FR-501 加 14 section 写权限映射表; (3) FR-102/502/503 TBD pool 模式 + 迁移原子性; (4) FR-103 atomic 边界 (changelog 落盘后 audit 失败); (5) FR-507 新增 3 hook 实接; (6) FR-702 grep 扩 4 pattern; (7) FR-301 fallback 端点落 FR-2xx 表。7 高留 polish。下一步: polish + close。
- v0.0.6 — 2026-06-12: **status: final**。Finalize step 4 (Triage) 完: §6 OQ 1-20 标状态 (5 ✅ / 3 🟡 / 1 🔴 / 4 🔵 follow-up / 7 polish 后续)。Finalize step 5 (Polish) 完: (1) §4.4 Non-Goals 段新增; (2) FR-508 named-loop 端到端闭环; (3) FR-509 用户干预通道 (OQ#4 锁定 a); (4) FR-104/704 flock 跨进程 + 死锁恢复; (5) FR-403 30 天撤销 4 边界; (6) FR-301 SSE 断网 resume + ring buffer; (7) FR-5xx 写入口归属。Step 6 External handoffs: skip (无配置)。Step 7 Close: `status: final` + `version: 0.0.6` + follow_up_prds 标注 3 个后续 PRD。下一步: Sprint Change Proposal 拆 selfinprove FR-5xx stub (per D-011)。
