# 实现方案讲解

用 `ToolOrchestrator` 取代 `ExecTool` 的直接宿主执行。审批协调器以一次性 `approval_id` 挂起调用，事件携带命令、cwd、原因和可用决策；GUI 提交决策后恢复同一调用。全局规则写入 `~/.agent-diva/execpolicy.toml` 并更新内存策略。
