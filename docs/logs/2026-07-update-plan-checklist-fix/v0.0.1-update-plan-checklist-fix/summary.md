# 迭代总结

本迭代修复普通聊天调用 `update_plan` 后可能卡死的问题，并明确区分三种概念：运行时 `update_plan` 任务清单、Plan mode 的方案文档、仓库根目录的持久化 `TODOLIST.md`。

主要变更：

- Agent 成功执行 `update_plan` 后，先发送清单事件，再发送工具完成事件，避免客户端创建无法收口的空 assistant 行。
- GUI 使用当前 turn 的工具边界定位清单，并在最终响应到达时反向寻找实际 streaming assistant；即使事件顺序存在竞争也能结束 typing 状态。
- 工具状态规范化为 `pending`、`in_progress`、`completed`，同时兼容旧 PascalCase 输入。
- GUI/TUI 文案统一为 Task Checklist/任务清单，不再把执行清单称为 Plan。
- 系统提示词明确 Agent 拥有 `update_plan`，并说明它不是 Plan mode，也不会自动写入 `TODOLIST.md`。

影响范围：`agent-diva-core`、`agent-diva-tools`、`agent-diva-agent`、`agent-diva-manager`、`agent-diva-cli`、`agent-diva-gui`。
