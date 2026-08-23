# Verification

- 源码检索 `bytes_stream().eventsource()`：仅 `send_message`、
  `start_background_stream`、`start_approval_stream` 三处。
- `continue_approved_plan_execution` 调用 `send_message`。
- 本 slice 只加 rustdoc，未改运行时；未跑 Tauri 桌面冒烟。
