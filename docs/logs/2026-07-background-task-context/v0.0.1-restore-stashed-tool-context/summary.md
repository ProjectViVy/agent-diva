# Summary

恢复 `BackgroundTaskContext` 与 `EnqueueBackgroundTaskTool::with_context`，修复 agent 调用方与 tools crate 接口不同步的问题。

变更范围仅限于背景任务工具导出/上下文传播，以及移除 agent loop 中未使用的 `MaskConfig` 导入。
