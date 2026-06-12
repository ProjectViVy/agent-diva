# MiniMax 同步语音合成 Demo 总结

## 本次变更

- 新增 Rust example：`agent-diva-providers/examples/minimax_sync_tts.rs`。
- Demo 改为 MiniMax WebSocket 同步合成链路：
  - 连接 `wss://.../ws/v1/t2a_v2`
  - 等待 `connected_success`
  - 发送 `task_start`
  - 发送 `task_continue`
  - 循环接收分块 hex 音频直到 `is_final=true`
  - 发送 `task_finish`
- 默认模型为 `speech-2.8-hd`，默认输出文件为 `minimax_sync_tts_output.mp3`。

## 设计取舍

- 采用 Rust example，而不是浏览器页面。
- 原因：
  - 避免 CORS 干扰接口验证。
  - 不在前端页面暴露 API Key。
  - 更适合验证同步合成分块返回、hex 解码与本地落盘链路。
- 本版不内建播放器，只负责打印分块信息并保存完整音频文件。

## 运行方式

- 通过环境变量提供 `MINIMAX_API_KEY`。
- 文本可通过命令行第一个参数传入，或设置 `MINIMAX_TEXT`。
- 运行命令：

```bash
cargo run -p agent-diva-providers --example minimax_sync_tts -- "你好，这是同步语音合成测试。"
```

## 影响范围

- 影响 `agent-diva-providers` 的 MiniMax example。
- 不改变 MiniMax GUI 的协议，只统一默认模型与命名。
