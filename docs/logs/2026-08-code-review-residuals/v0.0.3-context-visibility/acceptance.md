# Acceptance

1. 大工具结果折叠行只显示 preview，详情可见规模、artifact ID 和 read hint，复制动作
   写入真实 `artifact://` 引用。
2. auto/reactive compact 在当前 session 显示 started、completed 或 failed 单行状态；
   failed 明确原上下文仍保留。
3. 其他 session 的事件不会污染当前 GUI，会话切换清除旧状态。
