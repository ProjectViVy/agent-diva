# MiniMax TTS GUI 接入总结

## 本次改动

- 为 `agent-diva-gui` 的 Diva Pet 增加 `minimax` TTS 供应商。
- 在 GUI 设置页新增 MiniMax 入口，并暴露 `voiceId` 系统音色配置。
- 在 Tauri/Rust 侧新增 MiniMax WebSocket 合成命令，由后端负责带鉴权头连接 WebSocket。
- 在 `agent-diva-core`、GUI 前端、Tauri 命令层同步新增 `tts_voice_id` 配置链路。
- 为 MiniMax 场景明确禁用“参考音色/复刻音色”能力，避免与 SiliconFlow 混淆。

## 影响范围

- `agent-diva-core` 配置结构与校验。
- `agent-diva-gui` 的 Diva Pet 设置页、播放入口、TTS 服务与测试。
- `agent-diva-gui/src-tauri` 的命令注册、配置持久化与 MiniMax WebSocket 代理。
