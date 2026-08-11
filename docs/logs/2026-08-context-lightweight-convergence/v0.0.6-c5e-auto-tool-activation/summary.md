# C5e Automatic Deferred Tool Activation

C5e 保留 CORE `tool_search`，但删除模型可见的 `mount_tool`。搜索结果在同一运行时中自动
替换 task-local `active_deferred_tools`，下一次 provider call 直接收到激活后的 deferred
schemas；激活集合硬上限为 8，新用户 turn 开始时回收旧集合。

Installed/Authorized 工具面仍由 builtin、MCP、mask、plan 与 approval policy 决定，模型只
能选择当前授权 catalog 中的 task-local active subset。旧的 discovered/mounted/revision
状态、session metadata 写入、`tool_not_discovered` 和独立 mount 协议均已删除，不做旧状态
迁移或兼容反序列化。

实现提交：`3ce9eba2`。
