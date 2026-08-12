# Runtime correctness residuals

本切片收口三类运行时残留：无 `cache_control` provider 的 CORE hash、失败工具结果
进入 microcompact、以及 incomplete tool group 被 formatter 虚构为 completed。

- Agent 在调用边界按 `ToolDefinitionSet.core_count` 捕获显式 CORE hash；provider
  有 cache anchor 时继续信任 final-wire snapshot。
- 工具失败统一以 `Error:` 结果进入后续路径；错误与 materialization failure 不会被
  写成 `status=ok` artifact。
- checkpoint formatter 仅折叠 `group.complete=true` 的工具组。

接口 wire shape、provider model ID 和缓存策略没有改变。
