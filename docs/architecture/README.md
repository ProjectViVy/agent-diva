# 当前架构入口

> 更新日期：2026-08-14
>
> 本目录只保留当前可作为实施入口的架构摘要。**Laputa 现行架构汇总在 [`laputa/`](./laputa/README.md)。**
> 完整决策记录、研究证据和迭代验证分别位于 `docs/research/` 与 `docs/logs/`；历史架构全文位于 `docs/dev/archive(old-docs-dont-read-me)/`，不作为当前依据。根目录 `LAPUTA.md` 只做入口。

## 首先阅读

1. [Laputa 现行架构](./laputa/architecture.md)
2. [认知工作区边界](./current/cognitive-workspace-boundaries-2026-08.md)
3. [运行时审批边界](./current/runtime-approval-boundary-2026-08.md)
4. [上下文运行时边界](./current/context-runtime-boundary-2026-08.md)
5. [当前研究入口](../research/README.md)

## 当前权威决策

| 领域 | 当前依据 | 状态 |
| --- | --- | --- |
| 总体编排 | [`cognitive-workspace-reset-epic-2026-08`](../research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md) | Research Gate 分域通过；D0 设计稿待评审 |
| **D0 权威图** | [`cognitive-d0-domain-authority-2026-08`](../research/cognitive-d0-domain-authority-2026-08/domain-authority.md) | 设计稿；A/B/C 已拍；不是 Architecture Gate |
| **D1 Persona** | [`cognitive-d1-persona-workspace-2026-08`](../research/cognitive-d1-persona-workspace-2026-08/persona-architecture.md) | **已批准**；实施未授权 |
| **D2 Memory** | [`cognitive-d2-memory-stm-2026-08`](../research/cognitive-d2-memory-stm-2026-08/memory-architecture.md) | **已批准**；实施未授权 |
| **D3 Evolution** | [`cognitive-d3-evolution-skill-2026-08`](../research/cognitive-d3-evolution-skill-2026-08/evolution-architecture.md) | **已批准**；实施未授权 |
| **D4 交付** | [`cognitive-d4-clean-break-delivery-2026-08`](../research/cognitive-d4-clean-break-delivery-2026-08/delivery.md) | **已批准**；说切再切 |
| 当前实现盘点 | [`cognitive-r0-current-state-2026-08`](../research/cognitive-r0-current-state-2026-08/README.md) | 事实地图；不是目标架构 |
| STM / 上下文分层 | [`cognitive-r2-stm-context-2026-08`](../research/cognitive-r2-stm-context-2026-08/README.md) | R2 选项与约束；不是目标架构 |
| Persona 工作区技术 | [`cognitive-r3-persona-workspace-2026-08`](../research/cognitive-r3-persona-workspace-2026-08/README.md) | R3 事实与选项；不是目标架构 |
| Clean-break 安全 | [`cognitive-r4-clean-break-safety-2026-08`](../research/cognitive-r4-clean-break-safety-2026-08/README.md) | R4 影响与协议；不是删除切片 |
| **Laputa 汇总** | [`laputa/architecture.md`](./laputa/architecture.md) | 2026-08-14 产品架构汇总；实施未授权 |
| Persona / WORLD | [`persona-markdown-clean-break-2026-08`](../research/persona-markdown-clean-break-2026-08/decision-record.md) | 决策原文；七份权威、P20 车道、WORLD 走工具 |
| Memory / STM / ACTMEM / MEMRULES | [`stm-cross-session-clean-break-2026-08`](../research/stm-cross-session-clean-break-2026-08/decision-record.md) | 决策原文；`ACTMEM.MD` 全局一份走工具；**S9** 手册不进 Laputa |
| Evolution / Skill | [`evolution-genericagent-reset-2026-08`](../research/evolution-genericagent-reset-2026-08/decision-record.md) | Research Hold |
| BML | [`bml-layer-extraction-2026-08`](../research/bml-layer-extraction-2026-08/bml-layer-extraction-research.md) | 存储权威已冻结；抽层仍按研究结论实施 |
| Context C1–C5 | [`context-management-enhancement-2026-08`](../research/context-management-enhancement-2026-08/README.md) | 当前运行时施工与验证依据 |

## 不得再作为当前依据的内容

- 旧 Persona JSON、14-section 扁平模型、通用 Proposal/治理驱动的人格编辑；
- `memory_md` / `MemoryMd` 文件型长期记忆及其迁移、双读、双写或 fallback；
- 将 session `working_memory` checkpoint 直接称为跨会话 STM；
- 旧 AutoDream → Memory/Evolution Proposal → Governance 链路；
- 以“研究建议”冒充已经批准的目标架构。

这些内容仍保留用于历史追溯，但只能从归档清单进入，不能从当前入口进入。

## 证据与门禁

- 8 月 12–13 日的 summary、verification、acceptance、release 文档保持在 `docs/logs/` 原位置。
- Cognitive Workspace Reset 必须完成 R0–R4 Research Gate，再进入 D0–D4 Architecture Gate。
- 本入口不授权生产代码删除、保护性分支创建或兼容策略扩展。
