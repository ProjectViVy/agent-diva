# Summary

基础 BML CRUD 工具改为可靠的 CORE 工具面：`memory_add`、`memory_update`、
`memory_remove` 与既有查询工具不再依赖逐轮 `tool_search` 激活。

模型写入仍须遵守完整 MEMRULES。每个用户 turn 的第一次写调用现在只返回
`rules_required` 和规则全文，不执行写入；模型在下一次 provider call 重试后才落库。
`memory_list`、`memory_distill` 与 ACTMEM 管理工具继续保持 DEFERRED。
