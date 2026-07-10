# 错误处理与边界条件

| 场景 | 处理 |
| --- | --- |
| 未完成调研即 `plan_submit` | 返回 `PlanIncomplete`，列出缺少的目标、文件影响、步骤、验证或未决问题处置。 |
| 无活跃计划写 TODO | 返回 `NoActivePlan`；现有工具也覆盖此错误，[planning/mod.rs](agent-diva-tools/src/planning/mod.rs:587)。 |
| 待批准期间收到 agent 模式消息 | 不升级权限，继续按待批准能力矩阵拒绝写/执行。 |
| 同时批准/撤销 | 使用 revision 比较更新；第二个请求得到 `ApprovalConflict`。 |
| 计划批准后发生范围变化 | 计划 revision 增加、批准失效，回到 `Drafting`，保留审计记录。 |
| TODO 全完成但验证失败 | 进入 `Verifying/Failed`；不可伪造 `Completed`。 |
| 工具元数据未知 | 默认 `Execute` 级，计划态拒绝。 |

错误枚举应携带 `plan_id`、当前状态、预期状态和 revision，但不得回显敏感路径或工具参数。关键转移、拒绝、批准、撤销用 `tracing` 记录结构化事件；不在生产代码使用 `unwrap`/`expect`。
