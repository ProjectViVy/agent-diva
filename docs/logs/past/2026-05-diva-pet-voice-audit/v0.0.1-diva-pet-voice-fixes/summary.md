# Summary

## 本次迭代

- 修复 Diva Pet ASR 默认未开启的问题：
  - GUI 默认值改为开启。
  - 增加一次性迁移逻辑，升级用户首次加载时自动开启 ASR，后续仍允许手动关闭。
- 扩展桌宠 ASR 配置：
  - 新增 `web_speech` / `siliconflow` 两种 ASR provider。
  - 新增 ASR 的 `apiKey`、`baseUrl`、`model` 配置映射与默认值解析。
- 改造桌宠语音输入：
  - `web_speech` 继续沿用浏览器识别。
  - `siliconflow` 新增分段录音 + 云端转写流程。
- 改进桌宠设置页：
  - 新增 ASR provider 配置 UI。
  - TTS/ASR 配置继续自动保存，但增加“保存中 / 已保存 / 保存失败”状态提示。
  - 开发测试面板改为使用 ASR 凭证进行转写，不再错误复用 TTS 凭证。
- 修复 MiniMax TTS 连接链路：
  - GUI Tauri crate 为 `tokio-tungstenite` 启用 `native-tls`。
  - WebSocket 建连改为 `IntoClientRequest`，并增强连接错误信息，便于排查握手问题。

## 影响范围

- `agent-diva-core`
- `agent-diva-gui`
- `agent-diva-gui/src-tauri`
