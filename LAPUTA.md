---
title: "Laputa — 架构索引 (基线 v0.0.6 final)"
date: 2026-06-12
status: approved (基线索引 — 设计态,非实现态)
owner: 大湿
applies_to: agent-diva-pro
source_of_truth:
  - docs/prds/prd-laputa-2026-06-12/prd.md          # PRD 主体 (387 行, v0.0.6 final)
  - docs/prds/prd-laputa-2026-06-12/.decision-log.md # 决策审计 (D-001~D-013, 13 条, 1 天走完)
related:
  - DECISION.md                                       # alife v1 (2026-06-18, 横向 dispose — 不背 laputa 设计)
  - DECISION-v2-next-phase.md                         # harness engineering v2 (2026-06-18)
  - docs/prds/prd-selfinprove-2026-06-12/D-010.r1     # 继承: Laputa = 稀薄文档层
  - docs/prds/prd-selfinprove-2026-06-12/D-011        # 继承: 一锅端 ship, stub 待实接
reconcile_docs:
  - docs/prds/prd-laputa-2026-06-12/reconcile-laputa-new-architecture.md
  - docs/prds/prd-laputa-2026-06-12/reconcile-mentle-laputa.md
  - docs/prds/prd-laputa-2026-06-12/reconcile-auto-evolution.md
review_docs:
  - docs/prds/prd-laputa-2026-06-12/review-rubric.md
  - docs/prds/prd-laputa-2026-06-12/review-adversarial.md
  - docs/prds/prd-laputa-2026-06-12/review-edge-case.md
follow_up_prds:
  - Laputa 架构后续 PRD     # OQ#12-14: 三轴主体性 / 进阶心跳 / MemoryProvider 4 lifecycle
  - AutoDream PRD           # OQ#15-16: 4 thin + 8 should-not / Heartbeat→RhythmPolicy 触发链
  - Mentle 集成 PRD         # OQ#17-18: 5 wing / 写白黑名单 / work_memory / Authority 4 级
supersedes: null
---

# LAPUTA — 架构索引 (基线 v0.0.6 final, 2026-06-12)

> **TL;DR**: Laputa = **1 稀薄文档层 + 14 content section + 4 接口 + 6 治理**。
> Subject-file substrate —— 用户拥有最终审阅权,agent 跨 session 累积用户授权的 identity / preference / relationship,**每天 5 分钟能审完变更**。
>
> ⚠️ **本文件是设计态基线索引**(v0.0.6 final, 2026-06-12),不是实现态。实现进度以代码 + `.decision-log.md` 后续条目为准;后续如有偏差,以最新 `.decision-log.md` + 代码现状为准,本文件仅作锚点。

---

## 0. 30 秒读懂

| 维度 | 答案 |
|---|---|
| **Laputa 是什么** | agent-diva-pro 的 subject-file substrate,1 份 `.laputa/state.json`,内含 14 个 content section |
| **不是** | DB / Vector store / Graph DB / Agent runtime / Scheduler / Auth / AutoDream / Mentle 集成 |
| **谁写** | selfinprove(proposal_inbox 审后调 Laputa 写入口)、Report System(走通用入口) |
| **谁读** | AutoDream(Orient 阶段)、Mentle(仅 recall)、UI |
| **核心规则** | Laputa 写、Mentle 读(authority 单向);Laputa 持久、AutoDream 触发(存储单源、调度在外);Laputa 接受所有写、selfinprove 是审稿人 |

---

## 1. 物理形态(D-007 锁)

**1 份主文件**:`.laputa/state.json`,顶层含 `schema_version: "1.0.0"` (semver)

**14 content section 分类**:

| 类别 | # | section | 写权限 | schema 谁定 |
|---|---|---|---|---|
| **Laputa-owned** | 1 | identity | agent_self | 本 PRD |
| | 2 | relationship | agent_self | 本 PRD |
| | 3 | commitment | **user_only** | 本 PRD |
| | 4 | preferences | agent_self | 本 PRD |
| | 5 | memory_md | agent_self | 本 PRD |
| | 6 | history_md | agent_self | 本 PRD |
| **Report-owned** | 7 | daily | agent_self | Report System PRD |
| | 8 | weekly | agent_self | Report System PRD |
| | 9 | monthly | agent_self | Report System PRD |
| **TBD** ⚠️ | 10 | journal-reflective | tbd | 等最终综合 PRD |
| | 11 | evolution proposal inbox | tbd | 等最终综合 PRD |
| | 12 | subject-file changelog | tbd | 等最终综合 PRD |
| | 13 | report indexes | tbd | 等最终综合 PRD |
| | 14 | AAAK summaries | tbd | 等最终综合 PRD |

**TBD pool 模式**(FR-503): #10-14 接受写入,content 存为 raw bytes,`status=tbd`,schema 等最终综合 PRD。

**旧 8 模板 → 14 section 映射**(FR-502):
  - `SOUL.md` + `IDENTITY.md` → #1 identity（已退役 → Frozen Core `sections/{identity,relationship,commitment,preferences}.json`，见 commit `c2ca8c1f`）
  - `MEMORY.md` → #5 memory_md
  - `HISTORY.md` → #6 history_md
  - `USER.md` + `PROFILE.md` → #2 relationship
  - `TASK.md` → #10 journal_reflective (TBD pool)
  - `BOOTSTRAP.md` → 已退役（commit `57044ba8` 移除注入，`36c68d8f` 移除种子）

**迁移策略**(FR-503): ~~`sync_workspace_templates()` 改写~~ → `sync_workspace_templates()` 已移除 4 件套（SOUL/IDENTITY/USER/BOOTSTRAP）种子，仅保留 PROFILE/HISTORY/TASK/MEMORY 共 6 模板（commit `36c68d8f`）。Frozen Core 4 节由 `LaputaStorage::open()` 自动种子为 JSON `null`。

---

## 2. 4 接口(写 / 读 / 事件 / Changelog)

### 2.1 写接口 FR-1xx(6 条)

**唯一入口**:`LaputaWrite::apply_proposal(proposal: &EvolutionProposal) -> Result<ChangelogRecord, WriteError>`

**8 种 proposal_type 路由**(FR-102):

| proposal_type | 落到 |
|---|---|
| memory_patch | #5 memory_md |
| journal_note | #10 journal_reflective (TBD pool) |
| learning_note | #4 preferences |
| identity_patch | #1 identity |
| relationship_update | #2 relationship |
| commitment_set | #3 commitment |
| sop_create | #1 identity 内 sop sub-section |
| deprecation | 仅写 changelog,不动正文 |

未知 proposal_type → 返 `Err::UnknownProposalType`。

### 2.2 读接口 FR-2xx(5 条)

| 端点 | 用途 |
|---|---|
| `GET /api/laputa/snapshot` | 全量 14 section 快照 |
| `GET /api/laputa/section/{name}` | 单 section 切片 |
| `GET /api/laputa/snapshot?since=ISO8601` | 增量读(给 UI 实时刷新) |
| `GET /api/laputa/proposals` | proposal_inbox 全量(SSE fallback) |
| (含 TBD status) | 5 TBD section 显式返 `status: "tbd"`,不得 null |

### 2.3 事件接口 FR-3xx(3 条,SSE)

| 端点 | 推什么 |
|---|---|
| `GET /api/laputa/events/proposals` | EvolutionProposal 变更(created/approved/rejected/applied/reverted) |
| `GET /api/laputa/events/changelog` | ChangelogRecord 变更(apply/revert/rollback) |
| `GET /api/laputa/events/errors` | needs_attention 事件(冲突不可解) |

**鲁棒性**(FR-301 polish):
  - 客户端 fallback: 60s 轮询 `GET /api/laputa/proposals?since=`
  - 断网: ring buffer(1000 条,超限标 `event: buffer_overflow`)+ `Last-Event-ID` resume

### 2.4 Changelog 接口 FR-4xx(3 条)

| 端点 | 用途 |
|---|---|
| `GET /api/laputa/changelog` | paginated list(`page`/`page_size`/`since`/`until`/`target_section`/`action`/`proposal_id`) |
| `GET /api/laputa/changelog/{id}` | 详情(unified diff + 关联 proposal/audit) |
| `POST /api/laputa/changelog/{id}/rollback` | 回滚,30 天窗口,毫秒精度,UTC |

**30 天撤销 4 边界**(FR-403 polish):
  - 毫秒精度(30 天 + 1ms 仍可滚)
  - UTC 时区
  - 已撤销级联: 反向引用标 `stale=true`,不可再滚
  - 中断恢复: staging 标记 + 启动续滚

---

## 3. 6 道治理(每次写必经)

```
proposal in
   ↓
[1. review]            user 审 (per selfinprove D-006 全 user 审)
   ↓
[2. changelog 落盘]    含 before/after diff
   ↓
[3. audit 发事件]       who/when/what 5 字段
   ↓
[4. flock 写锁]         跨进程 POSIX flock, 5s deadlock detection
   ↓
[5. 冲突解决]           三向合并(current + base + proposed), 不可解时标 needs_attention
   ↓
[6. rollback]           30 天内可回, atomic
```

**atomic 边界**(FR-103 polish): 4 道串联,任一失败整体回滚。changelog 落盘后 audit 失败 → 反向 patch target_section,不得"假装回滚"留 target_section 在新状态。

**flock 跨进程 + 死锁恢复**(FR-104 polish): POSIX flock(Linux/macOS) + `LockFileEx` fallback(Windows),5s timeout,死锁时自动释放 + AuditEvent 标 `needs_attention`。

---

## 4. 用户干预规则(FR-509,D-012 你 "a" 拍板)

**用户可直接编辑**(走 PUT):
  - #3 commitment(给 agent 的红线)
  - #4 preferences(偏好)

**用户不可直接编辑**(返 `Err::UserCannotEdit`):
  - #1 identity / #2 relationship / #5 memory_md / #6 history_md

→ 改 #1 identity 等必须走 selfinprove proposal flow(跟 agent 写 proposal 同一 review queue,per selfinprove D-006 全 user 审)。

---

## 5. Laputa ↔ Mentle 边界(FR-6xx,3 条)

| | Laputa | Mentle |
|---|---|---|
| **写 authority** | ✅ 拥有 | ❌ 严禁 |
| **读 recall** | — | ✅ 仅走 FR-2xx HTTP 客户端 |
| **路径** | `agent-diva-manager/src/runtime.rs:335-365` (入口) | `agent-diva-agent/src/mentle_runtime.rs` (调用方) |
| **集成点** | — | `agent-diva-agent/src/agent_loop.rs:570 pub fn mentle_active()` (recall 开关) |

**Mentle 召回规则**(FR-603): 默认 `on_demand`,普通 session **不**注入 context,仅用户显式触发(Chat "回忆我的设置")才进。`always` 是 escape hatch 文档化但**不进 enum**(跟 B §3 §8 原则一致,day 1 移除以避免被默认开)。

Mentle 召回 MUST 走 FR-2xx HTTP 客户端,**不得**直接 `fs::read state.json`(避免绑死布局)。

---

## 6. 3 Hook 实接点(FR-507)

D-002 调研时已留 3 个 hook,Laputa PRD 必须接上,不留孤儿:

| Hook | 位置 | 由谁引用 |
|---|---|---|
| **1. 写入口** | `agent-diva-manager/src/runtime.rs:335-365` (启用注释段 + `laputa_core::provider::LaputaMemoryProvider::new()`) | FR-101 |
| **2. 启动快照** | `agent-diva-core/src/memory/provider.rs:125-139` (`StartupContextSnapshot.laputa_state_root`) | FR-201/202/203 |
| **3. Mentle 召回边界** | `agent-diva-core/src/memory/provider.rs:247-275` (`laputa_wakeup` / `laputa_project_soul` / `laputa_recall_intent`) | FR-603 |

---

## 7. NFR(Non-Functional Requirements,FR-7xx)

| FR | 指标 |
|---|---|
| **FR-701 性能** | 写 ≤50ms / 读 snapshot ≤100ms / changelog list(100 条) ≤200ms / section ≤20ms(mock 数据) |
| **FR-702 审计完备** | 4 种 grep pattern(`fs::write` / `tokio::fs::write` / `File::create` / `serde_json::to_writer`) 命中 0 处(除 LaputaWrite 实现内) |
| **FR-703 撤销完备** | 30 天内可滚,30 天 + 1s 返 `Err::RollbackExpired` |
| **FR-704 并发安全** | 100 并发写同 section 全成功无死锁 / 14 section 并发 ≤200ms 总 |
| **FR-705 错误处理** | 7 种错误类型枚举: SchemaIncompatible / Unauthorized / LockTimeout / ConflictUnresolved / RollbackExpired / UnknownLayer / IoError |
| **FR-706 可观测** | 3 Prometheus metrics: `laputa_writes_total` / `laputa_write_errors_total{error_type}` / `laputa_rollbacks_total{target_section}` |

---

## 8. 端到端闭环 FR-508(named-loop)

```
write (FR-101)
  → changelog (FR-103)
    → SSE notify (FR-301)
      → reader (FR-201)
        → human review (FR-403 if needed)
```

验收:e2e test 跑通,4 接口验证,全链路 ≤500ms。

---

## 9. Non-Goals(明确不做 — §4.4)

  - ❌ **不是 DB / Vector store / Graph DB**: file-only, P0 阶段不引 sqlite / pg / qdrant / neo4j
  - ❌ **不是 Agent runtime**: AgentLoop / ContextBuilder / Provider 还在 GenericAgent
  - ❌ **不是 Scheduler / Cron**: 触发链归 AutoDream
  - ❌ **不是 Auth / Identity Provider**: 写权限 = caller 信任,不做 JWT / OAuth
  - ❌ **不是 AutoDream**: 4 thin responsibilities + 8 should-not-own 负向契约归 AutoDream PRD
  - ❌ **不是 Mentle 集成**: 5 wing / 写白黑名单 / work_memory 归 Mentle 集成 PRD
  - ❌ **不是 Laputa 架构**: 三轴主体性 / 进阶心跳 / MemoryProvider 4 lifecycle 归 Laputa 架构后续 PRD
  - ❌ **不是知识图谱 / 技能生态 / Kanban / Plan mode / 月报 / 文件干预 / 模糊控制**

---

## 10. 关键边界(3 条,容易搞混)

1. **Laputa 写、Mentle 读**(authority 单向)
2. **Laputa 持久、AutoDream 触发**(存储单源、调度在外)
3. **Laputa 接受所有写、selfinprove 是审稿人**(写消费方统一入口)

---

## 10.1 报告系统逐级压缩索引关系(2026-06-21 新增)

### 核心原则

报告系统采用逐级压缩索引关系，避免重复召回：

```
月报生成 → 当月日记不再主动检索
周报生成 → 当周日报不再主动检索
日报生成 → 当日原始对话不再主动检索
```

### 索引层级与存储位置

**采用 mempalace 统一索引方案**：报告系统完全在 mempalace 内管理，Laputa 不存储报告内容。

| 层级 | 压缩程度 | 主动检索条件 | 存储位置 |
|------|----------|--------------|----------|
| **月报** | 高度压缩 | 始终可检索 | mempalace wings/monthly/ |
| **周报** | 中度压缩 | 月报未生成时可检索 | mempalace wings/weekly/ |
| **日报** | 轻度压缩 | 周报未生成时可检索 | mempalace wings/daily/ |
| **原始对话** | 无压缩 | 仅在需要详细回溯时查询 | mempalace wings/conversations/ |

### mempalace 统一索引结构

```
mempalace/
├── wings/
│   ├── daily/          ← 日报wing
│   │   ├── 2026-06-21/
│   │   └── 2026-06-20/
│   ├── weekly/         ← 周报wing
│   │   ├── week-25/
│   │   └── week-24/
│   ├── monthly/        ← 月报wing
│   │   ├── 2026-06/
│   │   └── 2026-05/
│   └── conversations/  ← 原始对话wing
│       └── ...
└── projects/           ← 项目详情wing
    └── ...
```

### 逐级检索流程

```
用户提问
    ↓
1. 判断时间范围
    ↓
2. 按优先级搜索：
   - 月报wing（如果时间跨度大）
   - 周报wing（如果时间跨度中）
   - 日报wing（如果时间跨度小）
   - 原始对话wing（如果需要细节）
    ↓
3. 返回结果
```

### 检索逻辑示例

```python
def recall_by_time_range(query: str, time_range: str) -> list:
    """根据时间范围选择检索层级"""
    
    if time_range == "month":
        # 先搜月报
        results = search_wing("monthly", query)
        if not results:
            # 降级到周报
            results = search_wing("weekly", query)
    
    elif time_range == "week":
        # 先搜周报
        results = search_wing("weekly", query)
        if not results:
            # 降级到日报
            results = search_wing("daily", query)
    
    elif time_range == "day":
        # 直接搜日报
        results = search_wing("daily", query)
    
    # 如果需要细节，搜原始对话
    if need_details:
        results += search_wing("conversations", query)
    
    return results
```

### 报告系统的双重目的

1. **用户视角**：让人类清楚了解AI的心路历程
2. **记忆视角**：成为memory的重要语料

### 节律性差异

| 系统 | 节律性 | 原因 |
|------|--------|------|
| **diva** | ✅ 需要 | 自主进化系统，需要周期性反思和压缩 |
| **hermes** | ❌ 不需要 | 有独立的进化逻辑，不需要报告系统 |

### 设计决策

- **D-014** (2026-06-21): 报告系统逐级压缩索引关系，避免重复召回
- **理由**：mempalace已经索引了所有原始内容，如果报告系统也索引相同内容，会导致重复召回
- **影响**：需要在召回逻辑中实现层级过滤，优先检索高层级报告

- **D-015** (2026-06-21): 新架构在hermes-laputa-python中验证，Rust版本维持原架构
- **理由**：新架构（报告走mempalace）需要验证可行性，但不影响现有Rust实现
- **影响**：
  - Rust版 (diva): 报告走Laputa section #7/#8/#9，维持原架构
  - Python版 (hermes-laputa): 报告走mempalace wings，验证新架构

---

## 11. 总规模盘点

  - 8 Concerns (C-1~C-8)
  - **36 条 FR**(FR-101~109, 201~205, 301~303, 401~403, 501~509, 601~603, 701~706)
  - 20 条 Open Q(8 ✅ 关 / 3 🟡 半关 / 1 🔴 开 / 7 🔵 转 follow-up PRD)
  - 6 版 revision(v0.0.1 → v0.0.6,**一天走完** 2026-06-12)

---

## 12. 关键决策时间线(13 条,1 天)

| # | 决策 | 谁拍 | 关键产出 |
|---|---|---|---|
| D-001 | 启动 Laputa PRD | 大湿(隐式) | 起目录 + Coaching 模式 + launch 级 stakes |
| D-002 | 调研: Laputa 0 真实现 | 子 agent | 4 个 hook 点 + 8 字符串模板来源 |
| D-003 | 14 vs 7 vs 1 文件之争 | 子 agent | 拍"稀薄层 = 哪几份"待答 |
| D-004 | 9 条 Open Q 跨 PRD 转入 | 子 agent | 写权限 / Level 边界 / Mentle 调和等 |
| **D-005** | **14 份分类 + Mentle 勘误 + 5 项 TBD** | **大湿 4 句话** | **Laputa-owned 6 / Report 3 / TBD 5** |
| **D-006** | **受众 + 实接深度"全"** | **大湿"2、全 3、全"** | **Q2 全 / Q3 全(写+读+事件+changelog 4 接口)** |
| D-007 | 1 稀薄层 + 14 content section | 大湿(隐式) | §4 Scope 锁,旧 8 模板 → 1 层 14 section |
| D-008 | Input Reconciliation | 子 agent 3 流 | 2 phase-blocker 修 / 7 转 follow-up / 2 P1 本 PRD 补 |
| D-009 | 修 2 phase-blocker | John 主动 | FR-501 引用 / FR-603 `always` 删 |
| D-010 | Reviewer Gate | 3 子 agent | 4 critical + 3 high 修 / 5 medium 留 polish |
| D-011 | 修 4 critical + 3 high | John 主动 | 7 处小修(Vision thesis / 写权限映射 / TBD pool / 3 hook / atomic / grep 扩 / fallback 端点) |
| **D-012** | **OQ#4 锁定 a** | **大湿"a"** | **用户只能直改 #3+#4,改 #1 走 proposal flow** |
| D-013 | Finalize 收口 | John close | status: final,Polish 7 件全修 |

**你 3 个真正的拍板点**:D-005(14 份分类)/ D-006(全/全)/ D-012(OQ#4 a)。

---

## 13. 复盘触发器(什么时候该 v2 这个索引)

- [ ] 7 条 follow-up PRD 中任一起稿并影响 Laputa 边界(尤其 Laputa 架构后续 PRD)
- [ ] selfinprove FR-5xx stub 实接后,8 种 proposal_type 路由有变更
- [ ] Mentle 集成 PRD 落地,Authority 4 级 ordering 引入
- [ ] 5 项 TBD(#10-14)综合 PRD 发布,14 section schema 完整化
- [ ] 现有 8 模板 → 14 section 迁移完成,FR-503/504 关闭
- [ ] AutoDream 触发链与 Laputa 写入契约出现耦合冲突

---

## 14. 一句话哲学

> Laputa 是 agent-diva-pro 的 subject-file substrate —— 不是大脑,不是 DB,是一份用户拥有最终审阅权的、稀薄的、长期累积的 subject file。

---

## 附录:文件地图

```
agent-diva-pro/
├── LAPUTA.md                                       ← 本文件(架构索引)
├── DECISION.md                                      ← alife v1 横向 dispose(2026-06-18)
├── DECISION-v2-next-phase.md                        ← harness engineering v2(2026-06-18)
└── docs/prds/prd-laputa-2026-06-12/
    ├── prd.md                                       ← PRD 主体 387 行(v0.0.6 final)
    ├── .decision-log.md                             ← 决策审计 D-001~D-013 345 行
    ├── reconcile-laputa-new-architecture.md         ← IR-1
    ├── reconcile-mentle-laputa.md                   ← IR-2
    ├── reconcile-auto-evolution.md                  ← IR-3
    ├── review-rubric.md                             ← R1
    ├── review-adversarial.md                        ← R2
    └── review-edge-case.md                          ← R3
```
