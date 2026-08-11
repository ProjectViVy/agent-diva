# Acceptance

1. 启动正常 Agent，确认 CORE `tool_search` 与 `mount_tool` 常驻；配置 MCP/custom 后确认其 DEFERRED schema 首轮隐藏。
2. 让模型执行 `tool_search` → `mount_tool`，确认同一 turn 的下一次 provider call 携带 mounted schema，并成功调用目标工具。
3. 重启并加载相同 session，确认 `tool_discovery_v1` 恢复 discovered/mounted；执行 compact 后只看到有界 mounted 工具重宣告。
4. 切换 mask、Assist、plan phase、builtin gate 或移除 MCP/custom source，确认旧 mount 不能绕过授权；源下线时直接调用返回 `tool_unavailable`。
5. reset/delete/shutdown 后确认内存状态清理，reset/delete 的 session metadata 不再保留 discovery 状态。
6. 输入无 Recall 意图、Recall 失败和 Recall 成功场景，确认稳定前缀、WM 与 current user 顺序不变；在层级/总预算压力下确认 Recall Drop report 可见且不污染 WM。
