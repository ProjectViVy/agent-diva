# Acceptance

1. 配置 QQ 官方 bot，并启动 `agent-diva`。
2. 在首次连接后，确认日志会记录 `attempt_mode=identify`。
3. 模拟服务端关闭连接或下发 `op 7`，确认下一次握手记录 `attempt_mode=resume`，并且消息恢复投递。
4. 模拟 `Resume` 后服务端返回 `op 9`，确认本地日志显示 `streak` 与 `next_backoff`，且下一次握手改为 `attempt_mode=identify`。
5. 连续触发多次 `op 9`，确认重连间隔递增，并在达到阈值后出现 cooldown 日志，而不是每 5 秒持续风暴重连。
6. 模拟心跳 ACK 丢失，确认连接会在超时后退出并优先尝试 `Resume`。
