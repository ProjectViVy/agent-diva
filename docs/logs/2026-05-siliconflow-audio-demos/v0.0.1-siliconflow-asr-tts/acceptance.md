# 验收步骤

## TTS 验收

1. 设置环境变量 `SILICONFLOW_API_KEY`
2. 运行：

```bash
cargo run -p agent-diva-providers --example siliconflow_tts -- "你好，这是 SiliconFlow TTS 测试。"
```

3. 观察输出：
   - 打印 `SiliconFlow TTS demo started`
   - 打印 `trace_id`
   - 打印 `audio_bytes`
   - 打印 `saved_audio`
4. 检查当前目录下是否生成 `siliconflow_tts_output.mp3`
5. 播放生成音频，确认内容与输入文本一致

## ASR 验收

1. 准备一个本地音频文件，例如 `sample.mp3`
2. 设置环境变量 `SILICONFLOW_API_KEY`
3. 运行：

```bash
cargo run -p agent-diva-providers --example siliconflow_asr -- sample.mp3
```

4. 观察输出：
   - 打印 `SiliconFlow ASR demo started`
   - 打印 `trace_id`
   - 打印 `transcription`
5. 确认转写文本与音频内容大致一致

## 验收标准

- TTS demo 能返回并保存可播放音频
- ASR demo 能返回非空转写结果
- 两个 demo 都能输出可用于排障的 trace id
