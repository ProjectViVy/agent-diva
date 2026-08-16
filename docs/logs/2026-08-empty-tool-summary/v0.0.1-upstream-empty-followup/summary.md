# 工具完成后空总结：用上游 finish/usage 分类再收尾

- 日期：2026-08-17
- 切片：长任务 `exec` 成功后出现「已完成 1 个工具调用……但模型未返回文字总结」
- 分支：`agent-diva-pro`（未 push）
- 版本目录：`docs/logs/2026-08-empty-tool-summary/v0.0.1-upstream-empty-followup/`

## 目标

工具已执行且 provider 调用 `ok`、但没有用户可见正文时，先读上游信号再决定 compact / summary-only，而不是立刻合成机械兜底。

## 原因

「获取上下文上限」是本地硬编码表 + 装配预算，只用于压缩门槛。`/models` 发现不解析 `context_length`。Reactive compact 只认 **失败** 的 `context_length_exceeded`。成功路径里 `finish_reason` 只特殊处理 `"error"`，`usage` 只进 ledger。因此 `200 + 空 content`（输出 4096 截断，或 prompt 顶满窗口仍返回 stop）会被当成「模型说完了」。

另外，summary-only bonus 原先只在撞上 `max_iterations` 时授予。一次工具 + 一次空 follow-up 永远进不了收尾采样。

## 改动

- `classify_empty_followup`：用 `finish_reason`、`usage`、已知 `context_window`、本轮装配报告分类为输入压力 / 输出截断 / 空 stop。
- 输入压力走既有 `rebuild_after_context_pressure`。
- 三类都最多授予一次 summary-only：禁工具、注入原有 nudge、该次 `max_tokens=8192`。
- 日常调用仍是 4096。思考过程不提升为用户回复。

## 未做

- 未从 `/models` 拉上游 `context_length`。
- 未全局提高所有轮次的输出预算。
- 未改 GUI 兜底文案（仍作最后一层）。
