# Epic 1 评审修复验收

## 验收步骤

- apply 已批准提案，确认 authority section、rollback staging、changelog、audit、proposal state 在同一写边界下更新。
- 在 section/changelog 写入后注入 apply 失败，确认 section 内容以及 rollback/changelog/audit artifacts 被清理。
- 回滚已 apply 的 changelog，确认覆盖前校验当前内容、rollback changelog 含 unified diff，且原 changelog 标记为 reverted。
- 使用 `Last-Event-ID` 订阅 Laputa SSE，确认发生 replay；请求事件不可用时包含 `buffer_overflow`。
- 通过 HTTP error path 调用 Tauri Laputa 命令，确认结构化 JSON 错误字段被保留。
- 保存 session，确认 atomic replacement 后最终 JSONL 仍可读取。

## 状态

基于聚焦测试和 route 检查，Epic 1 backend closure 可接受。GUI 编译验证仍被无关 sandbox 错误阻断。
