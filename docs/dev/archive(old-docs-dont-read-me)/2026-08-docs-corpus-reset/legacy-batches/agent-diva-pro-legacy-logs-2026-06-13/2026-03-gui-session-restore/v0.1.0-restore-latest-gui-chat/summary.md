# 迭代摘要

## 目标

Tauri GUI 启动后应自动展示**最近更新的 `gui:` 会话**历史，而不是每次生成新的 `chatId` 与空白欢迎态。

## 改动范围

- [`agent-diva-gui/src/App.vue`](../../../../agent-diva-gui/src/App.vue)
  - `refreshSessions()` 之后调用 `restoreLatestGuiChatOnStartup()`。
  - 仅考虑 `session_key` 以 `gui:` 开头的会话（避免误选 Telegram 等其它频道的「全局最新」会话）。
  - 按列表顺序（已按 `updated_at`/`created_at` 降序）依次尝试 `loadSession`，失败则试下一个 GUI 会话。
  - 若全部失败，回退为新的 `chatId` + 欢迎语（与无历史时一致）。
  - `loadSession` 改为返回 `Promise<boolean>`，便于启动恢复流程判断成功与否。

## 影响

- 有 GUI 聊天历史的用户再次打开应用时，主聊天区与当前会话应与上次最近使用的 GUI 线程一致（在时间排序意义下）。
- 无 `gui:` 会话时行为与改前相同（欢迎语 + 随机新 chat id）。
