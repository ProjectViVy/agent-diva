# Acceptance

## 用户/产品视角验收步骤

1. 默认配置下运行 agent，确认无行为变化（breaker 阈值 50 不触发）。
2. GUI/配置中（后续产品决策项）可设置：
   - `rejection_circuit_window_secs`（滑动窗口秒数，默认 60）
   - `rejection_circuit_threshold`（窗口内允许的拒绝次数上限，默认 50）
3. 触发场景：provider 在窗口内连续拒绝超过阈值时，循环拒绝发起下一次模型迭代，
   直到窗口滑过恢复。

## 验收通过标准

- 现有测试与 clippy 全绿（本 slice 已确认）。
- 代码评审确认新原语与审批拒绝熔断（`GuardianRejectionCircuitBreaker`）语义隔离。
- 明确：这是死循环熔断安全阀，不是预算管理；day/hour 限额部分已按用户决策推迟为
  待决策独立功能提案（见 TODOLIST.md GMH-41 条目）。