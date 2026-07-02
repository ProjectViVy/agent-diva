# 验收步骤

## 手动验收

1. 设置环境变量 `MINIMAX_API_KEY`。
2. 运行：

```bash
cargo run -p agent-diva-providers --example minimax_sync_tts -- "你好，这是同步语音合成测试。"
```

3. 观察控制台输出：
   - 打印 `MiniMax sync TTS demo started`
   - 成功建立连接后打印 `connected_success`
   - 至少打印 1 个 `chunk`
   - 结束时打印 `is_final`
   - 最终打印 `saved audio`
4. 检查当前目录是否生成默认音频文件：
   - `minimax_sync_tts_output.mp3`
5. 用外部播放器打开生成音频，确认内容与输入文本一致。

## 验收标准

- 能按 WebSocket 同步合成链路完成请求。
- 能逐块接收并解码音频数据。
- 能生成可播放的本地音频文件。
