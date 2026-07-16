# 架构设计探索

## 参考实现

Codex 将 `update_plan` 定义为 TODO/checklist 工具，并在 Plan mode 中明确拒绝调用；状态线采用 snake_case。参见 [plan.rs](../../../../.workspace/codex/codex-rs/core/src/tools/handlers/plan.rs:80) 与 [plan_tool.rs](../../../../.workspace/codex/codex-rs/protocol/src/plan_tool.rs:6)。

## 当前架构映射

- 工具契约位于 [update_plan.rs](agent-diva-tools/src/update_plan.rs:25)，仅校验并返回确认。
- Agent loop 在 [loop_turn.rs](agent-diva-agent/src/agent_loop/loop_turn.rs:1230) 把工具输入投影为事件。
- GUI 在 [App.vue](agent-diva-gui/src/App.vue:1823) 协调工具卡、流式 assistant 与最终事件。

## 数据流与故障根因

调用链为 provider tool call → tool started → checklist update → tool finished → provider follow-up → final。原实现先发送 tool finished，再追加 checklist 卡，导致卡片位于流式 assistant 占位符之后；完成处理只检查最后一项，因此无法结束请求。

## 适配原则

保留兼容名称 `update_plan`、`ChatPlanUpdate` 与 `turn_plan_updated`，但将产品语义限定为普通聊天任务清单。正式 Plan mode、执行期持久化 TODO 和仓库 `TODOLIST.md` 不共享状态。
