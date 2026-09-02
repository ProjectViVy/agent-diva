# Acceptance

1. 启动 Manager 后连接 `/api/neuro-link/v1/ws`，完成 `protocol/hello` 与 `session/open`。
2. 对已打开的 `session_key` 执行 `turn/start`；响应中的 admission 仍只出现一次。
3. AgentLoop 产生 assistant/tool/planning/final 事件时，当前 socket 只收到同一会话的
   typed notifications，通知 envelope 携带相同的 session/request/trace correlation。
4. 检查 profile/config root 下的 `super-channel-events.db`：terminal/final/planning 等
   durable 事件存在，assistant delta 等 transient 事件不被 `state/resume` replay。
5. 客户端断线后以最近 durable `CursorV1` 调用 `state/resume`，可恢复 durable 事件；若实时
   broadcast lagged，按协议继续使用 `state/resume`，不会把 transient gap 当作权威状态。
6. 不应看到新的 host/port/auth 配置项，也不应出现其他 session 的通知。
