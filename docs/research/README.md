# 研究资料入口

> 更新日期：2026-08-14
>
> 研究材料全部保留。当前研究包放在本目录；已经完成但未施工、较早或仅作参考的研究
> 放在 `historical/`。研究结论不能自动升级为已批准架构。
> **现行 Laputa 产品架构汇总**在 [`../architecture/laputa/`](../architecture/laputa/README.md)，
> 不要从 6 月根目录旧 `LAPUTA.md` 或本目录 historical 反推现行合同。

## 最新架构研究链

0. [Laputa 现行架构汇总](../architecture/laputa/architecture.md)（产品形状；不是本目录研究包）
1. [Cognitive Workspace Reset EPIC](./cognitive-workspace-reset-epic-2026-08/epic-orchestration.md)
1b. [D0 总体权威图（设计稿）](./cognitive-d0-domain-authority-2026-08/README.md)
2. [R0 当前系统盘点](./cognitive-r0-current-state-2026-08/README.md)
3. [R1 GenericAgent Evolution 研究包](./cognitive-r1-genericagent-evolution-2026-08/README.md)
4. [R2 STM 与上下文分层研究包](./cognitive-r2-stm-context-2026-08/README.md)
5. [R3 Persona 文档工作区研究包](./cognitive-r3-persona-workspace-2026-08/README.md)
6. [R4 Clean-break 数据安全研究包](./cognitive-r4-clean-break-safety-2026-08/README.md)
7. [Persona Markdown Clean Break](./persona-markdown-clean-break-2026-08/decision-record.md)
8. [STM 跨会话 Clean Break](./stm-cross-session-clean-break-2026-08/decision-record.md)
    （另有未批准分层提案 [`stm-layering-proposal.md`](./stm-cross-session-clean-break-2026-08/stm-layering-proposal.md)）
9. [Evolution / GenericAgent Reset](./evolution-genericagent-reset-2026-08/decision-record.md)
10. [BML Layer Extraction](./bml-layer-extraction-2026-08/bml-layer-extraction-research.md)
11. [Context C1–C5](./context-management-enhancement-2026-08/README.md)

## 当前研究包

| 包 | 状态 |
| --- | --- |
| `cognitive-workspace-reset-epic-2026-08/` | Research Gate 分域通过；可开 D0–D3 设计 |
| `cognitive-d0-domain-authority-2026-08/` | **D0 设计稿**；A/B/C 已拍（P22/S1/D7）；其余待点头 |
| `cognitive-r0-current-state-2026-08/` | **R0 完成物齐全**；待用户 Research Gate |
| `cognitive-r1-genericagent-evolution-2026-08/` | **R1 完成物齐全**；待用户 Research Gate；含 R0 Evolution 切片 |
| `cognitive-r2-stm-context-2026-08/` | **R2 完成物齐全**；待用户 Research Gate；不定物理权威 |
| `cognitive-r3-persona-workspace-2026-08/` | **R3 完成物齐全**；待用户 Research Gate；不定 revision store / 编辑器 |
| `cognitive-r4-clean-break-safety-2026-08/` | **R4 完成物齐全**；待用户 Research Gate；不创建保护分支 |
| `persona-markdown-clean-break-2026-08/` | Persona/WORLD 边界已批准，实施待研究/设计门禁 |
| `stm-cross-session-clean-break-2026-08/` | 边界已批准，存储与装配 Research Hold |
| `evolution-genericagent-reset-2026-08/` | Research Hold（产品边界）；执行研究见 R1 包 |
| `bml-layer-extraction-2026-08/` | BML 抽层研究，按研究结论推进 |
| `context-management-enhancement-2026-08/` | C1–C5 当前运行时施工与验证依据 |

## 历史调研（全文保留）

- [`historical/2026-06-harness-and-reference/`](./historical/2026-06-harness-and-reference/)：Alife、Hermes、Harness、Loop、Plan、workspace、后台任务。
- [`historical/2026-07-08-laputa-memory-history/`](./historical/2026-07-08-laputa-memory-history/)：早期 Laputa/Garden/Memory 候选、认知同步提案。
- [`historical/2026-08-approval-provider-history/`](./historical/2026-08-approval-provider-history/)：审批、HITL、Ask-User、Provider/DSML。
- [`papers/`](./papers/)：学术论文及索引。

历史调研是证据和参考，不是当前实施合同。与最新决策冲突的建议必须标记为 superseded，
不能通过“旧文档仍存在”重新激活。

## 关联入口

- 关键方向决策：[`../decisions/README.md`](../decisions/README.md)
- 当前架构：[`../architecture/README.md`](../architecture/README.md)
- 8 月验证日志：[`../logs/README.md`](../logs/README.md)
