# SiliconFlow ASR / TTS Demo 总结

## 本次变更

- 新增 `agent-diva-providers/examples/siliconflow_tts.rs`
- 新增 `agent-diva-providers/examples/siliconflow_asr.rs`

## Demo 能力

- `siliconflow_tts.rs`
  - 调用 `POST /audio/speech`
  - 直接接收二进制音频
  - 保存为本地文件
- `siliconflow_asr.rs`
  - 调用 `POST /audio/transcriptions`
  - 通过 multipart/form-data 上传音频文件
  - 输出转写文本

## 默认参数

- TTS：
  - `baseUrl = https://api.siliconflow.cn/v1`
  - `model = fnlp/MOSS-TTSD-v0.5`
  - `voice = fnlp/MOSS-TTSD-v0.5:alex`
- ASR：
  - `baseUrl = https://api.siliconflow.cn/v1`
  - `model = FunAudioLLM/SenseVoiceSmall`

## 影响范围

- 仅新增独立 Rust example 与交付日志。
- 未修改现有 GUI、Tauri 或 provider 正式运行逻辑。
