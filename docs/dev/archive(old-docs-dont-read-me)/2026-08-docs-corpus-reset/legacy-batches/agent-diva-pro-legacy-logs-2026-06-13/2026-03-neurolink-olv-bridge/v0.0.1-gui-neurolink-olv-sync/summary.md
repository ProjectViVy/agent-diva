# Summary

本轮实现了 Diva GUI 对话到 OLV 数字人前端的 NeuroLink 桥接能力。

- `agent-diva` 侧：
  - 为 `neuro-link` 增加客户端注册帧和 `speak` 下行帧支持。
  - 将 `neuro-link` 纳入 runtime outbound 订阅列表。
  - 新增 GUI `FinalResponse -> neuro-link speak` 后台桥接，只转发 `channel=gui` 的最终回复。
  - 为桥接消息增加元数据：`source=diva`、`source_channel=gui`、`source_chat_id`、`mode=final`。
- `OPEN-LLM-VTuber` 侧：
  - 在 `system_config` 新增 `neuro_link` 配置段。
  - 新增后台 `NeuroLinkBridgeClient`，启动后自动连接 Diva 的 NeuroLink server 并注册为 `olv-avatar`。
  - 收到 `speak` 帧后，不再走 OLV 自己的 agent，而是直接复用现有 TTS/Live2D 播放链，向前端发送 `control` / `audio` / `force-new-message`。
- 额外收口：
  - 顺手修复了 `agent-diva-gui/src-tauri/src/process_utils.rs` 中阻塞 `just check` 的 clippy 机械问题。
  - 顺手收口了 `agent-diva-manager/src/manager/runtime_control.rs` 中阻塞 clippy 的循环写法。

影响范围：

- GUI 对话新增可选数字人同步输出能力。
- 不影响 CLI、普通 channel、OLV 自主 agent 对话的既有默认行为。
- OLV 只有在 `system_config.neuro_link.enabled=true` 时才会主动连接 Diva。
