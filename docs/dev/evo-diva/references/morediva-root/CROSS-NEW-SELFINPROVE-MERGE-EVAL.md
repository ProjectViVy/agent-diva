# agent-new / selfinprove 合并策略评估报告

> **日期**: 2026-06-06
> **来源任务**: t_b52af8a6 (kanban)
> **约束**: only-main/pro 归并口径；不 push，不改 main
> **治理依据**: `MOREDIVA-路由治理.md` — selfinprove=RESEARCH-ONLY, agent-new=PENDING-DEDUP

---

## 1. 仓库现状对比

| 维度 | agent-diva-agent-new | agent-diva-selfinprove |
|------|---------------------|----------------------|
| 分支 | `agent-diva-agent-new` | `autoresearch/...-20260531` |
| merge-base | `50f58c2` | `50f58c2` |
| commits ahead | 60 | 63 |
| committed md | 779 | 784 |
| 脏树文件 | 17 | 7 |
| 独有 committed 内容 | plan-mode-architecture + 2 log sets | AUTHORITATIVE-INDEX + self-evolution-ui-research + ui-mvp-decision logs |
| 角色（治理） | PENDING-DEDUP | RESEARCH-ONLY |

**核心发现**: 两分支共享同一个 merge-base，committed 核心研究文档完全相同（11 个 genericagent docs 内容一致）。差异仅在边缘：

- agent-new 多了 plan-mode 研究（已删除）
- selfinprove 多了 AUTHORITATIVE-INDEX + self-evolution-ui handoff

---

## 2. 脏树分析

### 共有脏文件（两仓库完全一致）

| 文件 | 状态 | 说明 |
|------|------|------|
| `autodream-rhythm-distillation-design.md` | M (已改) | 行尾/格式更新 |
| `context-compaction-research.md` | M (已改) | 行尾/格式更新 |
| `autoresearch.sh` | ?? (新) | 自动研究脚本 |
| `autoresearch_validate.py` | ?? (新) | 自动研究校验 |
| `candidate-evidence-journal-audit-design.md` | ?? (新) | 统一候选模型 spec |
| `shared-memory-rendering-research.md` | ?? (新) | MEMORY.md 渲染研究 |
| `docs/logs/2026-05-shared-memory-rendering/` | ?? (新) | 渲染研究日志 |

### agent-new 独有脏文件

| 文件 | 状态 | 说明 |
|------|------|------|
| `docs/dev/agent-plan/plan-mode-architecture.md` | D (删) | Plan Mode 架构文档，已清理 |
| `docs/logs/2026-05-planmode-research/` | D (删) | Plan Mode 调研日志，已清理 |
| `docs/logs/2026-06-plan-mode-architecture/` | D (删) | Plan Mode 设计日志，已清理 |
| `docs/dev/genericagent/README.md` | M (已改) | 无研究线警告头 |

### selfinprove 独有脏文件

| 文件 | 状态 | 说明 |
|------|------|------|
| `docs/dev/genericagent/README.md` | M (已改) | 添加了研究线警告头 + 指向 AUTHORITATIVE-INDEX |

---

## 3. 研究成果清单与决策

### 3.1 保留在 selfinprove（冻结研究母线）

selfinprove 已有 AUTHORITATIVE-INDEX.md 作为权威入口，以下 16 个研究文档 + 3 个 newedge 设计文档应保持原位：

| 文档 | 类型 | 决策 |
|------|------|------|
| AUTHORITATIVE-INDEX.md | 索引 | **保留** — 新人入口 |
| autonomous-evolution-simplified-architecture-decision.md | 核心决策 | **保留** — 架构方向 |
| context-compaction-vs-autonomous-evolution-decision.md | 核心决策 | **保留** — 边界划分 |
| compression-taxonomy-decision.md | 核心决策 | **保留** — 三机制分类 |
| mentle-laputa-memory-role-decision.md | 核心决策 | **保留** — Mentle/Laputa 边界 |
| candidate-evidence-journal-audit-design.md | 设计规格 | **保留（需 commit）** — 写路径 spec |
| autodream-rhythm-distillation-design.md | 设计规格 | **保留** — AutoDream spec |
| shared-memory-rendering-research.md | 设计规格 | **保留（需 commit）** — 渲染 spec |
| context-compaction-research.md | 研究 | **保留** — 已在 pro 实现 |
| compression-research.md | 研究 | **保留** — AutoDream 前置 |
| autodream-migration-research.md | 迁移研究 | **保留** — 可行性 |
| newedge/architecture.md | 架构冻结 | **保留** — DivaGeneric 总架构 |
| newedge/ui-design.md | UI 冻结 | **保留** — 交互设计 |
| newedge/agent-diva-pro-self-evolution-ui-research.md | 交接文档 | **保留** — pro UI 交接 |
| hermes-learning/ | 历史 | **保留（已标 superseded）** |
| genericagent-upgrade-research/ | 历史 | **保留（已标 historical）** |

### 3.2 升级为 main/pro backlog 卡

| 研究成果 | 目标 | 建议卡标题 | 优先级 |
|---------|------|-----------|--------|
| candidate-evidence-journal-audit-design.md | pro | "pro: 实现 unified candidate model + EvidenceRef + audit chain" | P1 — 写路径基础设施 |
| shared-memory-rendering-research.md | main | "main: Shared MEMORY.md 渲染 MVP (Always/Relevant/Archive)" | P1 — 被多处依赖 |
| newedge/agent-diva-pro-self-evolution-ui-research.md | pro | "pro: self-evolution UI surface 实现 (AutoDream/Review/Inbox)" | P2 — 产品 UI |
| autodream-rhythm-distillation-design.md | pro | "pro: AutoDream rhythm P0a 手动触发 + structured output" | P2 — 节律系统 |
| context-compaction-research.md | main/pro | 已有 t_465e9ad0 (feature/context-compaction) | 进行中 |
| compression-research.md | pro | "pro: AutoDream 前置材料压缩 (Source Capsule)" | P3 — AutoDream 依赖 |
| autodream-migration-research.md | pro | "pro: Claude Code AutoDream → Agent-Diva 迁移可行性验证" | P3 |

### 3.3 可归档项

| 项目 | 说明 |
|------|------|
| agent-new plan-mode-architecture | 已在 agent-new 脏树中删除，selfinprove 从未拥有。plan-mode 作为独立研究线已终止。 |
| agent-new planmode-research logs | 同上，2 个 log set 已删除。 |
| autoresearch.sh / autoresearch_validate.py | 自动研究工具脚本，非产品代码。保留在 selfinprove 即可，无需进 main/pro。 |
| hermes-learning/ | 已标 superseded，历史存档。 |
| genericagent-upgrade-research/ | 已标 historical，历史存档。 |

---

## 4. 最终建议

### agent-new 归宿

**结论: 完成去重后归档 (ARCHIVE)**

1. agent-new 的独有价值（plan-mode-architecture）已在脏树中主动删除，说明该研究线已终止。
2. 核心研究内容与 selfinprove 完全重叠（11 个 committed docs 内容一致）。
3. 治理规则已标记为 PENDING-DEDUP。
4. **动作**: 清理 agent-new 脏树（丢弃删除 + 丢弃未跟踪文件），然后标记为 ARCHIVED。不合并到任何分支。

### selfinprove 归宿

**结论: 保持 RESEARCH-ONLY，完成脏树 commit**

1. AUTHORITATIVE-INDEX.md 已建立权威入口。
2. 脏树中有 2 个重要未提交的研究文档（candidate-evidence-journal + shared-memory-rendering）需要 commit。
3. README 已添加研究线警告头。
4. **动作**: 在 selfinprove 内 commit 脏树变更（研究报告 + 格式更新），保持冻结状态。

### 工作树状态总结

| 仓库 | 分支状态 | 脏树 | 动作 |
|------|---------|------|------|
| agent-diva-agent-new | 60 commits ahead, PENDING-DEDUP | 17 files (含 10 个删除) | 清理脏树 → 归档 |
| agent-diva-selfinprove | 63 commits ahead, RESEARCH-ONLY | 7 files (含 2 个新研究) | commit 脏树 → 保持冻结 |

---

*报告结束。如需执行脏树清理或创建 backlog 卡，请确认。*
