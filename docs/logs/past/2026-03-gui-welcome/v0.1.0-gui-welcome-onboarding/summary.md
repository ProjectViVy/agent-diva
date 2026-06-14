# GUI 首次欢迎向导与外链修复

## 变更摘要

- 新增 `openExternalUrl`（`@tauri-apps/plugin-opener` 的 `openUrl`），用于在系统默认浏览器中打开 HTTP(S) 链接；浏览器开发模式下回退为 `window.open`。
- `NetworkSettings.vue` 中博查说明由 `<a target="_blank">` 改为按钮调用 `openExternalUrl`，避免 Tauri WebView 内嵌打开。
- 新增 `WelcomeWizard.vue`：首次启动（`localStorage` 键 `agent-diva-welcome-v1`）展示多步向导，引导 DeepSeek 与博查 API，可选保存密钥；完成页可跳转「模型与提供商」「网络/搜索」「控制台」或开始聊天。
- `App.vue` 集成向导与配置/工具保存；`NormalMode.vue` 通过 `defineExpose` 暴露 `openSettingsTab` / `openConsole`。
- 中英 `welcome.*` 文案补充。

## 影响范围

- 仅 `agent-diva-gui` 前端与文档日志；Rust 侧无行为变更。
