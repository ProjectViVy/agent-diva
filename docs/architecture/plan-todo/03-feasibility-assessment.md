# 可行性评估

## 结论

推荐渐进式改造，技术可行性高：核心实体已包含 phase、step、todo、revision，[model.rs](agent-diva-core/src/planning/model.rs:131)，运行时也已在批准后进入 `Execute`，[loop_runtime_control.rs](agent-diva-agent/src/agent_loop/loop_runtime_control.rs:352)。风险主要来自行为语义调整，而不是基础设施缺失。

| 影响面 | 风险 | 缓解 |
| --- | --- | --- |
| 现有 Plan 用户 | 计划期现有 `todo_write` 行为改变 | 先兼容读取旧列表；仅新会话使用新 policy。 |
| 工具/MCP | 白名单遗漏放行变异工具 | 默认拒绝未知能力；未知工具为 `External/Execute`。 |
| 并发批准 | 陈旧窗口批准错误版本 | `expected_revision` 比较更新，冲突要求重新审阅。 |
| GUI | 前端 `ExecMode` 与后端 phase 不一致 | 后端为真源，前端仅呈现。 |

备选方案是维持白名单并仅调整 prompt；不推荐，因为现有 prompt 已禁止外部动作，[loop_turn.rs](agent-diva-agent/src/agent_loop/loop_turn.rs:486)，仍无法表达 TODO 的阶段语义或可靠地覆盖新工具。

不新增第三方 crate；异步与序列化沿用现有 Tokio、Serde、store 抽象，最小化供应链风险。
