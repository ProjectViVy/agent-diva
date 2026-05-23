# 验证记录

## 已执行

- 文档核对：
  - Demo 运行命令
  - 默认模型
  - 输出文件名
  - WebSocket 同步合成链路说明
- 编译验证：
  - `cargo check -p agent-diva-providers --example minimax_sync_tts`

## 观察点

- Example 是否成功编译。
- 是否按同步链路建立 WebSocket 连接并发送 `task_start` / `task_continue`。
- 是否能够逐块接收并解码 hex 音频数据。
- 是否能在 `is_final=true` 后落盘为本地音频文件。

## 结果

- `cargo check -p agent-diva-providers --example minimax_sync_tts` 已通过。
- 未执行真实 API 冒烟调用，原因是当前会话未提供可用的 MiniMax API Key。
