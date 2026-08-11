# Acceptance

## 用户/产品视角验收步骤

1. 默认配置下运行 agent，确认正常 turn 不受影响（`max_actions_per_hour` 默认 100）。
2. 配置 `max_actions_per_hour`（如 1）后连续发起 2 个 turn，第 2 个应被拒绝并告警
   "max_actions_per_hour exhausted; refusing new turn admission"。
3. 熔断触发场景（provider 连续拒绝超过阈值）下，新 turn 入场即被拒绝
   "refusing new turn admission"，快速失败。

## 验收通过标准

- 测试与 clippy 全绿（本 slice 已确认）。
- 明确：day/hour 限额的"排队到 supervised 队列"分支未被实现，已随用户决策推迟为
  待决策独立功能提案（见 TODOLIST.md GMH-41 条目）。