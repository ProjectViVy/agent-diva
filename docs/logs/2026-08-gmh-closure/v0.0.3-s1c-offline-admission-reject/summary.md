# Summary

GMH-41 自治预算与熔断的第三步（S1c）：离线高风险 admission 的"拒绝"分支。

## 变更

- `agent-diva-agent/src/agent_loop.rs`：`AgentLoop` 新增 `turn_rate_limiter`
  （`ActionTracker`，窗口 3600s）与 `max_actions_per_hour`（来自
  `SecurityConfig.max_actions_per_hour`，默认 100）。三个构造点接线；新增测试访问器。
- `agent-diva-agent/src/agent_loop/turn/admission.rs`：`admit_turn` 在分类后调用
  `enforce_turn_admission`：
  - 拒绝熔断已触发 → 拒绝入场（`ADMISSION_CIRCUIT_TRIPPED`）。
  - `max_actions_per_hour` 用尽（`try_record` 返回 false）→ 拒绝入场
    （`ADMISSION_TURN_RATE_EXCEEDED`）并告警。

## 关于"排队"分支

计划中 S1c 的 day/hour 预算耗尽→入队 supervised 队列，已随用户决策将 day/hour
限额推迟为待决策独立功能提案。因此本 slice 仅实现"拒绝"分支（`离线高风险排队或拒绝`
中的"或拒绝"），排队分支待 day/hour 方案定案后独立实现。

## Impact

- 首次真正执行 `max_actions_per_hour`（此前仅为配置字面量，从未强制）。
- 熔断触发时拒绝新 turn 入场，快速失败，避免占用 session 后在迭代入口才失败。