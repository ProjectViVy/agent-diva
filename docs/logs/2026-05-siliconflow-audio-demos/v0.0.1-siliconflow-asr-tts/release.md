# 发布说明

## 交付内容

- Rust TTS demo：`agent-diva-providers/examples/siliconflow_tts.rs`
- Rust ASR demo：`agent-diva-providers/examples/siliconflow_asr.rs`

## 使用前提

- 需要有效的 `SILICONFLOW_API_KEY`
- 需要本地可访问 `https://api.siliconflow.cn/v1`
- ASR demo 需要一份本地音频文件

## 执行方式

```bash
set SILICONFLOW_API_KEY=你的密钥
cargo run -p agent-diva-providers --example siliconflow_tts -- "你好，这是 SiliconFlow TTS 测试。"
```

```bash
set SILICONFLOW_API_KEY=你的密钥
cargo run -p agent-diva-providers --example siliconflow_asr -- path/to/audio.mp3
```

## 非发布项

- 本次不涉及正式二进制发布。
- 本次不改动产品默认入口，仅提供开发验证 demo。
