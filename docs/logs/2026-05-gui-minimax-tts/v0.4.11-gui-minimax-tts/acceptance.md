# 验收步骤

## 用户视角

1. 打开 `agent-diva-gui` 设置页的 `Diva Pet` 面板。
2. 在 `TTS 语音播报` 中选择 `MiniMax`。
3. 填写 `API Key`，确认默认 `Base URL`、`模型`、`音色 ID` 可见。
4. 确认“参考音色”区域显示 MiniMax 首版不支持提示。
5. 保存配置并重启 GUI，确认 `MiniMax` 与 `voiceId` 仍被正确回填。
6. 触发一次 AI 回复自动播报或开发测试播报，确认能听到返回音频。
7. 在 MiniMax 出错时，确认仍可回退到浏览器播报。

## 通过标准

- 无需手改 JSON 配置即可启用 MiniMax TTS。
- 配置不会在保存后退回 `browser`。
- 系统音色选择可用，参考音色能力边界清晰。
