---
title: "Scope Merge Decision — Report System × AutoDream"
date: 2026-06-12
status: approved
owner: 大湿
approver: John (PM) on behalf of user
sprint_change_proposal: _bmad-output/planning-artifacts/sprint-change-proposal-2026-06-12.md
---

# Scope Merge Decision — Report System × AutoDream

> **TL;DR**: AutoDream = 数据压缩层（产生内容），Report System = 用户呈现层（消费内容）。两套子系统通过 markdown 文件路径契约解耦。

---

## 1. 决策

采用 **方案 C (边界分工)**。

### 1.1 用户原话

> "autodream 只是一个压缩技术，而最终还是要回归到用户的可见内容中"

### 1.2 一句话总结

- **AutoDream**: 跨会话反思 + 记忆候选 + Journal + **日/周报生成**
- **Report System**: **月报生成** + 报表固化 (SOP/Skill/Memory) + Session 智能搜索 + **NotebookView 展示层**

---

## 2. 边界协议

### 2.1 数据流契约

```
┌─────────────────────────────────────────────┐
│  AutoDream (跨会话反思)                       │
│  ├── 产出: memory_patch_candidates (JSON)    │
│  ├── 产出: journal_entries (JSON)            │
│  └── 产出: 日报/周报 markdown ──────────┐    │
└──────────────────────────────────────────│────┘
                                           │
                                           ▼
┌─────────────────────────────────────────────┐
│  Report System (用户视角)                    │
│  ├── 输入: rhythm_reports/* (AutoDream 写入)│
│  ├── 自有: monthly/{YYYY-MM}.md (独立生成)  │
│  ├── 展示: NotebookView (commit fcf768d)     │
│  ├── 固化: SOP/Skill/Memory (FR-6/7/8)      │
│  └── 搜索: Session 智能搜索 (FR-9/10)       │
└─────────────────────────────────────────────┘
```

### 2.2 调度边界

| 报表 | 调度器 | PRD owner | 触发时机 |
|------|--------|-----------|---------|
| 日报 | AutoDream 时间门 (24h) | AutoDream FR-12 | 蒸馏运行完成后 |
| 周报 | AutoDream 时间门 (24h) | AutoDream FR-13 | 蒸馏运行完成后 |
| 月报 | Report System cron | Report System FR-3 | 每月第一个周一 00:00 |
| 手动触发 | 任意时刻 | Report System (本 PRD) | 用户在 GUI 触发 |

### 2.3 写入路径契约 (AutoDream → Report System)

- **日报路径**: `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md`
- **周报路径**: `.agent-diva/autodream/reports/weekly/{YYYY-Www}.md`
- **写入方式**: 原子写入（write to .tmp → rename）
- **失败处理**: AutoDream 侧重试 3 次（退避 5min），仍失败则 events.jsonl + showAppToast

### 2.4 UI 共享

- **NotebookView owner**: Report System（commit `fcf768d` 是先例）
- **AutoDream 消费方式**: 扫描固定路径，不订阅 AutoDream 内部事件
- **SelfEvolutionSettings**: 归 AutoDream（触发频率配置）

---

## 3. 影响制品

| 制品 | 修订类型 | 负责方 | 截止日期 |
|------|---------|--------|---------|
| `docs/prd-report-system/prd.md` | FR-1/2/3 owner 标注 + 新增 §13 | PM (John) | 2026-06-13 |
| `docs/prd-report-system/validation-report-v3.md` | 新建 (P0-4 边界声明) | PM (John) | 2026-06-16 |
| `docs/prds/prd-autodream-2026-06-12/prd.md` | §0 边界 + FR-12/13 标注 | PM (John) | 2026-06-13 |
| `docs/architecture/autodream-architecture-2026-06-12.md` | ADR-007 更新 + ADR-008 新增 | Architect (bmm-architect) | 2026-06-14 |
| **本文件** `docs/architecture/scope-merge-decision.md` | **新建** | PM (John) | 2026-06-12 ✅ |
| `docs/epics/report-system-epics.md` | 新建 (P1 修复后拆分) | PM (John) | 2026-06-19 |
| `docs/epics/autodream-epics.md` | 新建 (验证现有 step 1 拆分) | PM (John) | 2026-06-19 |

---

## 4. Action Items (按时间线)

### 4.1 已完成 ✅

- [x] 2026-06-12: Sprint Change Proposal 起草并获大湿批准
- [x] 2026-06-12: scope-merge-decision.md 落地 (本文件)

### 4.2 立即 (2026-06-13)

- [ ] 修订 `prd-report-system/prd.md`：FR-1/2/3 owner 标注 + §13 协作接口
- [ ] 修订 `prd-autodream-2026-06-12/prd.md`：§0 边界说明 + FR-12/13 owner 标注
- [ ] Auto-dream Decision Log 增加 D6: "日报/周报归属 — AutoDream 生成, Report System 展示"

### 4.3 短期 (2026-06-14 ~ 06-16)

- [ ] Architect 修订 `autodream-architecture-2026-06-12.md`：ADR-007 NotebookView 标注 + 新增 ADR-008 写入路径契约
- [ ] PM 补 Report System P1 (acceptance criteria + LLM 成本 guardrail)
- [ ] 新建 `prd-report-system/validation-report-v3.md`，评分目标 Fair+ → Good

### 4.4 中期 (2026-06-17 ~ 06-19)

- [ ] 双线 epic 拆分：`docs/epics/report-system-epics.md` + `docs/epics/autodream-epics.md`
- [ ] 更新 `_bmad-output/implementation-artifacts/sprint-status.yaml` 反映新 epic 结构
- [ ] bmad-check-implementation-readiness 跑一次整体规划成熟度检查

---

## 5. 决策理由 (Why C over A/B)

| 维度 | A (吸收) | B (退让) | C (边界分工) - 选这个 |
|------|---------|---------|-------------------|
| 概念清晰度 | ✗ AutoDream 被"知识管理"概念污染 | ✓ | ✓ |
| AutoDream 改动 | 0 | 大 (删 2 FR) | 微 (加 ADR-008) |
| Report System 改动 | 0 (退场) | 大 (推 final) | 中 (推 final + 缩 scope) |
| 架构返工 | 无 | 有 (删 reports 模块) | 极小 |
| 工作量 | Low | Medium | Medium |
| 长期可演进 | Low | High | High |
| 月报/固化/搜索 独立空间 | ✗ | ✓ | ✓ |

C 是 3 个方案中**唯一能同时保持** "AutoDream 不必学固化" 和 "Report System 不必学蒸馏" 的方案。

---

## 6. 风险与缓解

| 风险 | 等级 | 缓解措施 |
|------|------|---------|
| 数据流契约定义不清 | Medium | ADR-008 明确写入路径 + 失败处理 |
| NotebookView 边界争执 | Low | owner = Report System, 已在 §2.4 明确 |
| 月报被 AutoDream 节奏延迟 | Low | 月报不跟 AutoDream, 独立 cron |
| Report System P1 修复拖延 | Medium | 列为 owner 接受的先决条件, 06-16 deadline |
| AutoDream 架构 step 1 返工 | Low | 仅 ADR-007 表格 +1 行 + 新增 ADR-008 |

---

## 7. 参考

- Sprint Change Proposal: `_bmad-output/planning-artifacts/sprint-change-proposal-2026-06-12.md`
- Report System PRD: `docs/prd-report-system/prd.md` (v1.1 修订后)
- AutoDream PRD: `docs/prds/prd-autodream-2026-06-12/prd.md` (v1.1 修订后)
- AutoDream Architecture: `docs/architecture/autodream-architecture-2026-06-12.md` (v1.1 修订后)
- Project Context: `docs/project-context.md`
