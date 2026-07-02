# Acceptance

## 验收步骤

1. 打开 Diva Pet 设置页，确认 ASR 默认处于开启状态。
2. 将 ASR provider 切换为 `SiliconFlow`，填写 `API Key`、`Base URL`、`Model`，确认页面出现自动保存状态提示。
3. 切换回 `Web Speech`，确认云端 ASR 输入框隐藏，环境不支持时显示明确提示。
4. 使用开发测试面板上传音频：
   - 当 ASR provider 为 `SiliconFlow` 时，可完成转写并触发 TTS 播报。
   - 当 ASR provider 为 `Web Speech` 时，面板应提示文件转写仅支持云端 ASR。
5. 切换 TTS provider 到 `MiniMax`，填写配置后保存，确认状态提示更新。
6. 在 MiniMax 配置下触发测试播报：
   - 成功时正常合成并播放。
   - 失败时日志中可区分保存成功但运行时连接失败，并包含更明确的连接错误信息。
7. 首次升级后关闭 ASR，重启 GUI，确认不会再次被强制打开。

## 通过标准

- ASR 默认开启与一次性迁移行为符合预期。
- ASR provider 可切换且配置可持久化。
- TTS 自动保存状态可见。
- MiniMax 切换后行为与错误反馈可观察、可区分。
