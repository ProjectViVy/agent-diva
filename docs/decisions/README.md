# 关键决策集合

> 更新日期：2026-08-13
>
> 本目录只保留仍具有方向性、范围约束或安全价值的真实决策。它不是新的架构设计；
> 8 月 12–13 日的认知工作区记录仍是当前架构锚点，本文档集合中的旧决策若发生冲突，
> 以后者为准。

## 当前最高优先级

1. [`../architecture/README.md`](../architecture/README.md)：当前架构入口。
2. [`../research/README.md`](../research/README.md)：当前研究与 Research Hold 入口。
3. [`../research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md`](../research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md)：8/13 总 EPIC。

## 保留的历史/方向性决策

| 决策 | 日期 | 保留理由 | 当前状态 |
| --- | --- | --- | --- |
| [Alife 能力取舍](./alife-feature-disposition-2026-06-18.md) | 2026-06-18 | 明确自主活动、自我升级、插件、工作台、多开和 Live2D 的产品方向 | 方向性决策；自主活动尚待新架构设计 |
| [Harness Engineering 下一阶段](./harness-engineering-next-phase-2026-06-19.md) | 2026-06-19 | 定义 Agent runtime 的长期工程方向和 decide→act→observe 原则 | 历史方向；已完成部分由当前代码/日志证明 |
| [产品范围与工作台边界](./product-scope-and-workbench-2026-06-18.md) | 2026-06-18 | 保留视觉、浏览、插件和重工作流能力的归属决策 | 方向性决策；不得覆盖 8/13 工作区边界 |
| [安全与威胁模型基线](./security-and-threat-model-2026-07-29.md) | 2026-07-29 | 保留安全边界、风险模型和禁止默认信任的原则 | 安全基线；运行时审批以当前实现为准 |
| [AutoDream/Report System 边界](./autodream-report-boundary-2026-06-12.md) | 2026-06-12 | 保留手动优先、用户审查、失败可见和生成/呈现分层 | 历史边界；旧 Evolution 链路已被 8 月研究取代 |
| [Laputa/Memory 治理历史](./laputa-memory-governance-history-2026-06.md) | 2026-06–07 | 保留治理写入、审查、审计、迁移与恢复原则 | 历史基础；当前以 BML/Persona/STM 为准 |
| [Self-Improve 边界](./self-improve-boundaries-2026-06-12.md) | 2026-06-12 | 保留用户审查、Notebook 工作台和人格写入分层原则 | 历史边界；当前以 Evolution Reset 为准 |

## 不在本集合中的内容

- 已被 8/12–8/13 决策明确取代的 Persona、Memory、STM、Evolution 旧架构；
- 纯实施记录、一次性 Sprint 计划、重复 PRD review、UI 像素调整和单次 bug 修复；
- 未经用户确认的想法草稿；它们仍可在归档 ZIP 中追溯，但不作为决策。
