# v0.9.9 Clippy compatibility

本迭代修复 Rust 1.98 Clippy 在 `agent-diva-core` 上将既有建议提升为错误的问题：

- 三处 `sort_by` 改为 `sort_by_key`，其中 Evolution 请求使用 `Reverse` 保持倒序。
- `SessionManager::get_or_load` 使用 `?` 保持原有 `None` 传播语义。

修复不改变业务排序方向或会话缓存行为。
