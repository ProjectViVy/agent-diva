# Release — GUI Provider 错误/重试可见性修复

- 版本：`v0.1.0-provider-error-retry-visibility`
- 日期：2026-08-06

## 发布方式

随常规开发分支 `agent-diva-pro` 提交（7 个 Conventional Commit，未 push）。
下次构建 GUI（`agent-diva-gui`）与 Manager 后自然生效；无需数据库迁移或
配置文件变更。

## 涉及二进制

- `agent-diva-manager`（SSE 转发逻辑、新事件映射）
- `agent-diva-gui`（Tauri 桥 + 前端渲染）——需重新构建桌面应用
- `agent-diva-cli`/服务端组件依赖 core/providers/agent 变更，随 workspace 构建

## 回滚

- 前端回滚：还原 `App.vue`/`ChatView.vue` 的徽标渲染与监听（未知事件会被忽略，
  前端对新事件名不敏感；`agent-error` 断流兜底移除后恢复旧行为）。
- 后端回滚：还原 `forward_chat_events` 为旧的 60s 静默断流循环；新 AgentEvent
  变体在旧 consumer 下进入 `_ =>`/忽略分支（已确认 e2e collector 穷尽匹配补齐）。
- 无需数据回滚。
