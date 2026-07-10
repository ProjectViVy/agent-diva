# 兼容性与迁移

## 兼容原则

现有序列化的 `PlanPhase` 不删除；新增状态优先映射，旧记录以确定规则迁移：`Explore/Plan→Drafting`，`AwaitingApproval→AwaitingApproval`，`Execute→Executing`，`Verify→Verifying`，终态→`Closed`。保留原枚举直到一个完整发布周期后再评估弃用。

| 旧行为 | 新行为 | 迁移 |
| --- | --- | --- |
| 计划期 `todo_write` 直接写清单 | 只允许候选工作项，执行 TODO 在批准后可选生成 | 旧 TodoList 以 `legacy=true` 只读展示，不自动改写。 |
| 前端 `mode=plan` | 后端返回 `plan_mode_state` 与 `capabilities` | UI 兼容读取旧 `ExecMode`，新字段优先。 |
| 无版本审批 | `expected_revision` 审批 | 缺 revision 的旧记录首次提交时初始化为 1。 |

存储迁移必须幂等、可重跑，并在启动/读取时惰性完成；不得静默删除旧 TODO。对外 API 新增字段使用 optional/默认值，避免旧 GUI 解码失败。
