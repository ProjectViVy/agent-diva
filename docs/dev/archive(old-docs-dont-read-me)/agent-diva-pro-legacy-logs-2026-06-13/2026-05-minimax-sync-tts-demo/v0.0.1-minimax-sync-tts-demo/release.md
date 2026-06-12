# 发布说明

## 交付内容

- Rust example：`agent-diva-providers/examples/minimax_sync_tts.rs`

## 使用前提

- 需要有效的 MiniMax API Key。
- 需要本地可访问 `https://api.minimaxi.com` 对应的 WebSocket 网关。
- 需要 Rust 工具链与项目依赖可正常编译。

## 执行方法

```bash
set MINIMAX_API_KEY=你的密钥
cargo run -p agent-diva-providers --example minimax_sync_tts -- "你好，这是同步语音合成测试。"
```

## 非发布项

- 本次不涉及正式二进制发布。
- 本次不新增播放器能力，音频播放由外部工具手动验证。
