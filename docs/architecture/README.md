# 当前架构入口

> 更新日期：2026-08-13
>
> 本目录只保留当前可作为实施入口的架构摘要。完整决策记录、研究证据和迭代验证分别位于 `docs/research/` 与 `docs/logs/`；历史架构全文位于 `docs/dev/archive(old-docs-dont-read-me)/`，不作为当前依据。

## 首先阅读

1. [认知工作区边界](./current/cognitive-workspace-boundaries-2026-08.md)
2. [运行时审批边界](./current/runtime-approval-boundary-2026-08.md)
3. [上下文运行时边界](./current/context-runtime-boundary-2026-08.md)
4. [当前研究入口](../research/README.md)

## 当前权威决策

| 领域 | 当前依据 | 状态 |
| --- | --- | --- |
| 总体编排 | [`cognitive-workspace-reset-epic-2026-08`](../research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md) | R0–R4 研究包已交付，待 Research Gate；禁止提前定稿目标架构 |
| 当前实现盘点 | [`cognitive-r0-current-state-2026-08`](../research/cognitive-r0-current-state-2026-08/README.md) | 事实地图；不是目标架构 |
| STM / 上下文分层 | [`cognitive-r2-stm-context-2026-08`](../research/cognitive-r2-stm-context-2026-08/README.md) | R2 选项与约束；不是目标架构 |
| Persona 工作区技术 | [`cognitive-r3-persona-workspace-2026-08`](../research/cognitive-r3-persona-workspace-2026-08/README.md) | R3 事实与选项；不是目标架构 |
| Clean-break 安全 | [`cognitive-r4-clean-break-safety-2026-08`](../research/cognitive-r4-clean-break-safety-2026-08/README.md) | R4 影响与协议；不是删除切片 |
| Persona / WORLD | [`persona-markdown-clean-break-2026-08`](../research/persona-markdown-clean-break-2026-08/decision-record.md) | Approved Direction（七份权威，含 DARK；IDENTITY 含身体）；Implementation Pending |
| Memory / STM | [`stm-cross-session-clean-break-2026-08`](../research/stm-cross-session-clean-break-2026-08/decision-record.md) | Boundary Approved；存储与装配 Research Hold |
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
