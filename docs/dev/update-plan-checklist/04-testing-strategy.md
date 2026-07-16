# 测试策略

## 单元测试

- [update_plan.rs](agent-diva-core/src/planning/update_plan.rs:19)：规范状态输出、旧状态输入兼容。
- [update_plan.rs](agent-diva-tools/src/update_plan.rs:86)：空清单、超限、非法状态和多个 `in_progress`。
- [streamingMessages.test.ts](agent-diva-gui/src/utils/streamingMessages.test.ts:1)：卡片位于 assistant 之后仍能完成请求。

## 集成测试

- Agent loop 断言 `tool_started → checklist_updated → tool_finished → final_response`。
- Manager/CLI SSE 测试断言 snake_case 状态贯穿线协议。
- TodoCard 测试覆盖规范状态与旧 PascalCase 状态。

## 回归与 smoke

执行针对性 Cargo/Vitest、GUI build、`just fmt-check`、`just check`、`just test`。GUI smoke 观察调用 `update_plan` 后输入状态结束，且可立即发送下一条消息。
