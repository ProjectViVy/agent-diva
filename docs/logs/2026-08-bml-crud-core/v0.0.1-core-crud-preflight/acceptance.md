# Acceptance

1. 在新会话首轮要求模型新增、更新或删除 BML 记录，不应出现 `tool_unavailable`，也不应先调用 `tool_search`。
2. 第一次写调用返回 `rules_required` 且 BML 未变化；模型读到完整 MEMRULES 后重试，结果才返回 `applied`。
3. 下一用户轮次可继续直接选择基础 CRUD，但必须重新完成当轮写前预检。
4. Plan、只读和 subagent 场景仍不暴露 BML 写工具；`memory_list`、`memory_distill` 与 ACTMEM 管理操作仍按需激活。
