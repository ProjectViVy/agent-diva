# 可行性评估

可复用 `agent-diva-sandbox` 的 `ApprovalStore`、`ExecPolicyManager`、`ToolOrchestrator` 以及 Manager 已有 SSE 桥。主要风险是当前 shell 工具绕过该链路；实施必须先统一执行入口，避免双重策略。
