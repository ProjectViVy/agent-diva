# Summary

- 修复 `agent-diva-channels/src/qq.rs` 的 QQ 长连接状态机，恢复会话级 `session_id` / `sequence` 持久化，支持断线后优先 `Resume`。
- 修复服务端返回 `Invalid Session (op 9)` 时的错误行为：现在会清空旧 session 并继续重连，下一次走 fresh `Identify`，不再直接退出整个 WebSocket 循环。
- 心跳改为使用服务端 `HELLO.d.heartbeat_interval`，并增加 ACK 丢失容忍；连续 3 次未收到 ACK 会主动断开并重连，避免空闲后连接僵死。
- 增加对服务端 WebSocket `Ping` 的 `Pong` 响应，降低长时间空闲后被网关判定离线的概率。
- 保留并接通 QQ 集成测试需要的 `QQ_ACCESS_TOKEN_OVERRIDE` / `QQ_GATEWAY_URL_OVERRIDE` / `QQ_API_BASE_OVERRIDE` 钩子，使 QQ 重连集成测试可稳定运行。
- 扩展 `agent-diva-channels/tests/qq_reconnect_integration.rs`，覆盖：
  - 断线后 `Resume`
  - `Resume -> Invalid Session -> fresh Identify`
  - `Ping/Pong` 保活
