# Context and tool-result visibility

新增严格 `ToolResultRef v1` 展示模型：折叠行使用 preview，详情展示大小、artifact ID、
read hint，并提供 `artifact://<id>` 复制动作。普通文本、损坏 JSON 和错误结果维持原文
显示。

新增 `AgentEvent::ContextCompaction`，由 Agent auto/reactive 路径发出 started/completed/
failed；Manager SSE、CLI、Tauri 和 GUI 使用同一事件。当前会话显示单一非旋转状态行，
失败摘要说明原上下文保留；手动 `/compact` 不重复发状态。
