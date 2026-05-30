# GenericAgent 调研文档总入口

## 本周期输出（2026-05）

### `记忆架构深层研究：GenericAgent L0-L4 × Mentle 可行性`
- [summary](../../logs/2026-05-memory-architecture-deep-dive/v0.0.1-architecture-analysis/summary.md)
- [verification](../../logs/2026-05-memory-architecture-deep-dive/v0.0.1-architecture-analysis/verification.md)
- [release](../../logs/2026-05-memory-architecture-deep-dive/v0.0.1-architecture-analysis/release.md)
- [acceptance](../../logs/2026-05-memory-architecture-deep-dive/v0.0.1-architecture-analysis/acceptance.md)

### `Laputa-work 架构调研：最小化接入可行性`
- [summary](../../logs/2026-05-laputa-architecture-audit/v0.0.1-laputa-integration-feasibility/summary.md)
- [verification](../../logs/2026-05-laputa-architecture-audit/v0.0.1-laputa-integration-feasibility/verification.md)
- [release](../../logs/2026-05-laputa-architecture-audit/v0.0.1-laputa-integration-feasibility/release.md)
- [acceptance](../../logs/2026-05-laputa-architecture-audit/v0.0.1-laputa-integration-feasibility/acceptance.md)

### `新 Laputa 架构设计：极简身份管理 + 三轴主体性 + 进阶心跳`
- [summary](../../logs/2026-05-laputa-new-architecture/v0.0.1-new-laputa-design/summary.md)
- [verification](../../logs/2026-05-laputa-new-architecture/v0.0.1-new-laputa-design/verification.md)
- [release](../../logs/2026-05-laputa-new-architecture/v0.0.1-new-laputa-design/release.md)
- [acceptance](../../logs/2026-05-laputa-new-architecture/v0.0.1-new-laputa-design/acceptance.md)

### `GenericAgent + mentle + 用户可控学习`
- [summary](../../logs/2026-05-genericagent-mentle-user-controlled-learning/v0.0.1-research-synthesis/summary.md)
- [verification](../../logs/2026-05-genericagent-mentle-user-controlled-learning/v0.0.1-research-synthesis/verification.md)
- [release](../../logs/2026-05-genericagent-mentle-user-controlled-learning/v0.0.1-research-synthesis/release.md)
- [acceptance](../../logs/2026-05-genericagent-mentle-user-controlled-learning/v0.0.1-research-synthesis/acceptance.md)

### `Codex + GenericAgent Plan Mode 对位`
- [summary](../../logs/2026-05-planmode-research/v0.0.1-codex-genericagent-diva/summary.md)
- [verification](../../logs/2026-05-planmode-research/v0.0.1-codex-genericagent-diva/verification.md)
- [release](../../logs/2026-05-planmode-research/v0.0.1-codex-genericagent-diva/release.md)
- [acceptance](../../logs/2026-05-planmode-research/v0.0.1-codex-genericagent-diva/acceptance.md)

### `DivaGeneric / NewEdge 设计固化`
- [architecture](newedge/architecture.md)：Generic Core、Laputa、Mentle、Plan Mode、节律触发与 autodream 的融合边界。
- [ui-design](newedge/ui-design.md)：Chat / Journal 双入口，以及 PlanCard、OptionCard、ReviewCard、JournalRefCard 的交互设计。

### `Claude Code AutoDream 迁移调研`
- [autodream-migration-research](autodream-migration-research.md)：确认 Claude Code AutoDream 实现事实，整理自动/手动触发、锁、后台子 agent、DreamTask UI、上下文压缩与 Agent-Diva 迁移方案。

### `Autodream 前置压缩技术调研`
- [compression-research](compression-research.md)：调研 Agent-Diva 接下来做 autodream/rhythm 前的压缩技术设计。覆盖 Source Capsule 数据模型、触发策略、checkpoint 设计、与 MemoryProvider/autodream/Journal/Mentle 的边界、MVP 实施建议。

## 历史核心调研（genericagent-upgrade-research）

- [summary](../../logs/genericagent-upgrade-research/v0.0.1-initial-research/summary.md)
- [notes](../../logs/genericagent-upgrade-research/v0.0.1-initial-research/notes.md)
- [verification](../../logs/genericagent-upgrade-research/v0.0.1-initial-research/verification.md)
- [release](../../logs/genericagent-upgrade-research/v0.0.1-initial-research/release.md)
- [acceptance](../../logs/genericagent-upgrade-research/v0.0.1-initial-research/acceptance.md)

## 建议阅读顺序

1. `genericagent-upgrade-research`（理解基础架构）
2. `2026-05-memory-architecture-deep-dive`（理解 L0-L4 本质与 mentle 定位）
3. `2026-05-laputa-architecture-audit`（理解 Laputa 人格层现状）
4. `2026-05-laputa-new-architecture`（理解新 Laputa 设计：7 文件 + 三轴 + 心跳）
5. `newedge/architecture.md`（理解 DivaGeneric 的当前融合设计稿）
6. `newedge/ui-design.md`（理解 Chat / Journal / Card 的产品承载方式）
7. `autodream-migration-research.md`（理解 Claude Code AutoDream 可迁移骨架与 Diva 语义差异）
8. `genericagent-mentle-user-controlled-learning`（理解用户可控学习闭环）
9. `planmode-research`（理解 Plan Orchestrator 与分阶段接入）
10. 对照各 `verification/acceptance` 作为实施前检查清单

## 说明

- 本目录当前采用“集中入口 + 原文链接”的整理方式，不改动原始日志目录结构，便于追溯与版本管理。
- `newedge/` 是当前 GenericAgent 方向的设计固化区，优先作为后续实现前的最新对照材料。
- 若后续需要“实体归档副本”，可在本目录新增镜像子目录按版本复制。
