# 验收标准

1. Plan 模式下写文件、编辑文件、shell、spawn、cron、MCP 等动作不可执行。
2. 计划转为 `AwaitingApproval` 后，本轮不再发起后续工具调用。
3. 待审批计划期间，即使后续请求使用 `agent` 模式，也不会重新暴露或执行 mutation 工具。
4. 用户显式批准并转为 `Execute` 后，执行回合仍可正常获得实现工具。
