# 架构设计探索

现有 `ExecTool` 直接启动宿主 shell；`ToolOrchestrator` 已具备沙箱、会话批准和失败升级骨架，但未接入工具链。目标是在智能体工具执行路径中以审批协调器连接沙箱、`AgentEvent`、Manager SSE、Tauri 和 GUI；计划模式继续不注册 `exec`。
