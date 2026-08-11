# CTX-C4：Deferred tool discovery and Recall verification

本迭代完成 CORE/DEFERRED 工具发现闭环：

- `ToolRegistry` 持有当前授权 catalog，并以共享 session/task 状态保存 discovered、mounted 和单调 revision。
- MCP/custom DEFERRED 默认不进入 provider schema；确定性关键词 `tool_search` 返回名称、描述、score、mounted 状态并记录 discovered。
- `mount_tool` 只允许挂载当前 session 已发现且仍在授权 catalog 中的工具；下一次同回合 provider call 立即得到新 schema。
- session metadata 使用版本化 `tool_discovery_v1` 恢复；重建重新经过 mask、Assist、plan phase、builtin gate，reset/delete/shutdown 清理内存与持久状态。
- Recall 保持原 MemoryProvider 与排序契约，补齐空、失败、成功、层级预算和总预算 Drop 的验证矩阵。

未改变 provider cache 算法、MemoryProvider 或 Recall 排序算法；未 push。
