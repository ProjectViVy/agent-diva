# Swarm 架构轮次归档（BMAD + 原始规划）

本目录为 **蜂群 / 大脑皮层** 本轮交付的 **只读归档**：便于审计、复盘与 PR 引用，**不**替代仓库内运行时文档（如 `agent-diva-swarm/docs/`、`docs/dev/architecture.md`）。

## 内容说明

| 路径 | 说明 |
|------|------|
| `original-swarm-planning/` | 独立仓库 `agent-diva-swarm` 侧 **最早一版** 研究与架构稿（ARCHITECTURE_DESIGN、研究笔记等）。 |
| `bmad-output/` | BMad 流程全量产出：PRD、epics、architecture、UX、sprint 变更提案、头脑风暴、**implementation-artifacts** 故事与回顾、`sprint-status.yaml`、评审用 `.diff` 等。 |

## 与主仓库的关系

- `_bmad-output` 在 monorepo 工作区根目录（`newspace/_bmad-output`）仍为创作源；此处为 **快照副本**。
- 实现代码与 crate 内 ADR/运维文档仍在各 crate 路径维护；合并本归档时不修改 `main` 分支行为，仅在功能分支上追加文档树。

归档更新日期：**2026-04-01**。
