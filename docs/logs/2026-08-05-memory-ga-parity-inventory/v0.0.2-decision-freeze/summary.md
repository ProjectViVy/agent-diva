# Summary — GA-MEM-PARITY 决策冻结

- 版本：`v0.0.2-decision-freeze`
- 日期：2026-08-06
- 类型：产品决策冻结（docs only，无代码变更）
- 前置：`v0.0.1-gap-inventory`（2026-08-05，残缺项盘点）

## 背景

GA-MEM-PARITY（sev-P0）盘点已完成，但 inventory §10 的 5 个 Open Questions
在 Wave 0/1 编码前必须冻结，否则「完全对齐」无法验收。本次由用户逐项拍板。

## 冻结结论（用户 2026-08-06 确认）

| # | 决策点 | 冻结结论 |
|---|--------|----------|
| 1 | 写入路径 | 混合分级：低风险即时 apply + 高风险 proposal 审批 |
| 2 | L3 经验载体 | Skills 为主（复用 2026-07-30 `skill-sop-unification` 决策，产品对象只有 Skill） |
| 3 | 工作记忆 | Session store，随会话生命周期，distill 显式晋升 |
| 4 | consolidation | 降级为兜底，distill + AutoDream 为主写路径 |
| 5 | authority_mode 默认 | 统一默认 Typed，Legacy 显式 opt-in/导入源 |

详细理由与 Wave 实施约束见 `inventory.md §10.1`。

## 变更文件

- `v0.0.1-gap-inventory/inventory.md`：§10.1 决策冻结表 + Wave 约束
- `v0.0.1-gap-inventory/acceptance.md`：元验收「产品决策冻结」勾选
- `TODOLIST.md`：GA-MEM-PARITY 条目更新为「决策已冻结，Wave 0/1 待排期」

## 下一步

- Wave 0（诚实与契约）：修 prompt 假承诺 A8、authority_mode 统一（F10）、
  sync_turn 诚实化设计、分工文档。
- Wave 1（Agent 记忆工具 P0）：memory 工具 add/list/search/update/remove/distill，
  按混合分级路由。
- 排期与验收入口：`TODOLIST.md` GA-MEM-PARITY + `acceptance.md`。
