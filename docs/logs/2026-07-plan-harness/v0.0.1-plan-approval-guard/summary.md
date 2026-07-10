# 计划模式审批闸门修复

本迭代修复 Plan 模式生成计划后仍继续尝试文件写入的问题。

- 将待审批计划状态纳入每个回合的运行时安全判定，不再仅依赖请求中的 `exec_mode`。
- 待审批状态下重新组装工具时继续隐藏写文件、编辑、shell、spawn、cron、MCP 等外部动作。
- `plan_transition` 进入 `AwaitingApproval` 后立即结束当前 LLM 回合，避免模型收到拒绝结果后继续重试写操作。
- 参考 `.workspace/codex` 的只读沙箱/审批边界与 `.workspace/openakita` 的 plan/ask mutation deny 策略完成适配。
