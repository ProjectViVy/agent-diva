# 测试策略

## 状态机单元测试

- 所有合法转移：`Exploring→Drafting→AwaitingApproval→Executing→Verifying→Closed`。
- 所有非法转移：未提交不能批准；未批准不能执行；终态不能写入。
- 对每个状态和每个 `ToolCapability` 做表驱动测试；未知工具必须拒绝。
- 批准使用过期 revision 必须返回冲突并保持原计划/TODO 不变。

## TODO 测试

- 无 TODO 需求的计划，批准后不创建清单。
- `TodoPolicy::Generate` 批准后从冻结的 `PlanStep` 一次性生成，恰有一个 `InProgress`。
- patch 批次任一项失败时整批不提交；该原则锚定 oh-my-pi 的实现，[todo.ts](../../../.workspace/oh-my-pi/packages/coding-agent/src/tools/todo.ts:631)。
- 更新同一 TODO 不重复创建，参考 OpenHarness 的 upsert/no-op 测试，[test_core_tools.py](../../../.workspace/OpenHarness/tests/test_tools/test_core_tools.py:256)。

## 集成/E2E

1. 请求计划，读取目录、文件成功，写文件/shell/MCP/子代理均被拒绝。
2. 提交完整计划后，普通 agent 消息仍无法绕过 `AwaitingApproval`。
3. 批准后仅允许批准版本的步骤执行；重新编辑计划使批准失效。
4. GUI 依次显示探索、待审、执行、验证，并在待审显示计划而非执行进度。

将 OpenHarness 的“模式变更刷新 prompt”作为回归断言，[test_runtime_plan_mode.py](../../../.workspace/OpenHarness/tests/test_ui/test_runtime_plan_mode.py:36)，但将能力拦截作为更高优先级的断言。
