# 配置与依赖管理

本设计不引入新 Cargo 依赖。新配置采用兼容默认值：

```toml
[planning]
workflow_version = 2
todo_policy = "optional" # never | optional | always
require_plan_approval = true
```

| 配置 | 默认 | 含义 |
| --- | --- | --- |
| `todo_policy` | `optional` | 模型仅在任务复杂、可拆分、需跨轮追踪时提议 TODO；用户/计划可选择否。 |
| `require_plan_approval` | `true` | 复杂任务从待审计划进入执行必须显式批准。 |
| `workflow_version` | `2` | 允许读取旧存储并启动新语义。 |

配置不得成为绕过计划态写保护的开关；即使用户执行策略宽松，`Drafting/AwaitingApproval` 也固定拒绝 `WorkspaceWrite`、`Execute`、`External`。这与 oh-my-pi 将审批模式和工具声明分离的原则一致，[approval-mode.md](../../../.workspace/oh-my-pi/docs/approval-mode.md:38)。不新增环境变量或秘密。
