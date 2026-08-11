# Summary

将 DEFERRED 工具自动管理加入 CTX-C5 排期。计划保留 `tool_search`，搜索结果自动成为
下一次 provider call 的有界 active tool set，并 clean break 删除模型可见 `mount_tool`、
持久 discovered 状态和独立 mount revision。本次只修改技术计划与 TODOLIST。
