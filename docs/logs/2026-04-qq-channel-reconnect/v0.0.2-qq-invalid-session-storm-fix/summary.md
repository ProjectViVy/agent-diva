# Summary

- 重构 [agent-diva-channels/src/qq.rs](C:\Users\Administrator\Desktop\newspace\agent-diva\agent-diva-channels\src\qq.rs) 的 QQ WebSocket 重连流程，拆分为“单次连接执行 + 外层 supervisor”，由 `ExitReason` 驱动重连决策，不再对所有断线原因统一固定 5 秒死循环。
- 为 `Invalid Session (op 9)` 增加独立退避与风暴保护：连续无效会话按 `5s -> 15s -> 30s -> 60s` 递增退避，达到阈值后进入 cooldown，并在重连前清空本地 `session_id` / `last_sequence`，确保下一次使用 fresh `Identify`。
- 明确 `Resume` 条件：仅在同时存在 `session_id` 与 `last_sequence` 时才发送 `Resume`，`READY` / `RESUMED` 都会刷新 session 状态，任何携带 `s` 的事件都会持久化 sequence。
- 增加原因感知日志，覆盖握手模式、`Invalid Session` streak、下一次 backoff、cooldown 进入与恢复，便于线上区分“正常恢复”和“重连风暴”。
- 扩展 [agent-diva-channels/tests/qq_reconnect_integration.rs](C:\Users\Administrator\Desktop\newspace\agent-diva\agent-diva-channels\tests\qq_reconnect_integration.rs)：
  - `Reconnect(op 7) -> Resume`
  - `Resume -> Invalid Session -> fresh Identify`
  - 心跳 ACK 超时后优先 `Resume`
  - WebSocket `Ping/Pong`
  - 连续 `Invalid Session` 的递增退避
