# 实现方案讲解

## 总体方案

1. 在 [loop_turn.rs](agent-diva-agent/src/agent_loop/loop_turn.rs:1230) 中先发 checklist update，再发 tool finished。
2. 在 [streamingMessages.ts](agent-diva-gui/src/utils/streamingMessages.ts:19) 中按消息角色反向定位流式 assistant，不依赖数组尾部。
3. 在 [App.vue](agent-diva-gui/src/App.vue:1990) 中原位更新本轮 `update_plan` 工具行，回退插入时保持 assistant 在尾部。
4. 将状态线规范为 `pending`、`in_progress`、`completed`，同时接受旧 PascalCase 输入。

## 接口设计

工具名、配置键和既有事件名称不变。`PlanItemStatus` 的序列化输出改为 snake_case，旧值仅作为反序列化别名。GUI 卡片内部类型改为 `checklist`，避免与正式计划卡混淆。

## 非回归保障

修改仅作用于普通聊天 `update_plan` 路径；Plan mode 和 execution TODO 的注册策略不变。不新增依赖、不引入 `unsafe`，生产路径不使用 `unwrap`/`expect`。
