# 兼容性与迁移分析

## 保留接口

`update_plan` 工具名、`BuiltInToolsConfig.update_plan`、`UpdatePlanArgs.plan`、`ChatPlanUpdate` 与 SSE `turn_plan_updated` 均保留，因此配置和客户端路由无需迁移。

## 状态格式

新事件序列化为 `pending`、`in_progress`、`completed`，对齐 Codex。Rust 在 [update_plan.rs](agent-diva-core/src/planning/update_plan.rs:19) 接受旧 `Pending`、`InProgress`、`Completed`；GUI 的 `ChecklistItem` 同时渲染两种拼写。

## UI 内部迁移

普通聊天卡片的内部 `kind` 从 `plan` 改为 `checklist`，仅用于本版本运行时生成的 JSON 卡片，不修改持久化 schema。正式计划审批卡不使用该类型。

## 破坏性变更

无配置、数据库或公开命令的破坏性变更；旧客户端仍能接收相同事件名，但若严格依赖 PascalCase 输出，应升级或使用兼容反序列化。
