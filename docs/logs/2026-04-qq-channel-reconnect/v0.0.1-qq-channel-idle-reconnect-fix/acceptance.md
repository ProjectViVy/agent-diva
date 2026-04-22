# Acceptance

- 长时间空闲后若 QQ 网关要求重连，客户端应自动重连，而不是停在离线状态。
- 服务端关闭连接后，下一次连接应优先发送 `Resume(op 6)`。
- 如果 `Resume` 被服务端以 `Invalid Session(op 9)` 拒绝，客户端应清空旧 session，并在下一次连接发送 `Identify(op 2)`。
- 服务端发送 WebSocket `Ping` 时，客户端应返回 `Pong`，连接不应因此掉线。
- QQ 新发起会话后，机器人应能重新收消息，不再出现“显示不在线且无法恢复”的状态。
