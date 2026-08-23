# Summary — GUI-TAURI-PLAN-STREAM-DISCONNECT

- 版本：`v0.4.0-plan-sse-disconnect`
- 日期：2026-08-23
- 类型：核对后关闭（无独立 plan SSE 循环）

## 背景

2026-08-06 记录：`send_message` 已有 `saw_terminal` 断流兜底，另有两条 plan 相关
SSE 循环在无终止事件时静默返回。

## 核对结果

`bytes_stream().eventsource()` 现存三处：

1. `send_message`：有 `saw_terminal`，无 final/error 时 emit `agent-error`。
2. `start_background_stream`：cron 后台流，断线后重连，不是单次 plan 请求。
3. `start_approval_stream`：审批 SSE，断线后重连，不是 plan 执行流。

`continue_approved_plan_execution` / `approve_active_plan_execution` 走
`send_message`，继承同一兜底。原先两条独立 plan 循环已不存在。

## 做了什么

- 在 `continue_approved_plan_execution` 上补 rustdoc，标明断流语义继承自
  `send_message`。
- **未改** background / approval 重连循环。
- 未改 Vue。

## 影响范围

- `agent-diva-gui/src-tauri/src/commands.rs` 文档注释
- `TODOLIST.md`
- 本日志
