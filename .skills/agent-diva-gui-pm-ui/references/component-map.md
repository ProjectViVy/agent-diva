# Agent Diva GUI 组件映射

## 核心组件

| 组件 | 路径 | 职责 |
|------|------|------|
| App.vue | `src/App.vue` | 消息状态、Tauri 事件、配置/工具配置、会话缓存 |
| NormalMode.vue | `src/components/NormalMode.vue` | 主布局、Tab 切换（chat/settings）、侧边栏、主题、模型/历史下拉 |
| ChatView.vue | `src/components/ChatView.vue` | 消息列表、输入栏、Markdown 渲染、工具详情折叠 |
| SettingsView.vue | `src/components/SettingsView.vue` | 设置入口，路由到子设置页 |
| ConsoleView.vue | `src/components/ConsoleView.vue` | 网关日志、控制台输出 |
| CronTaskManagementView.vue | `src/components/CronTaskManagementView.vue` | Cron 任务 CRUD |

## 设置子组件

| 组件 | 路径 | 职责 |
|------|------|------|
| SettingsDashboard.vue | `src/components/settings/SettingsDashboard.vue` | 设置总览 |
| GeneralSettings.vue | `src/components/settings/GeneralSettings.vue` | 通用设置 |
| ProvidersSettings.vue | `src/components/settings/ProvidersSettings.vue` | LLM 提供商 |
| ChannelsSettings.vue | `src/components/settings/ChannelsSettings.vue` | 通道配置 |
| NetworkSettings.vue | `src/components/settings/NetworkSettings.vue` | 网络 |
| LanguageSettings.vue | `src/components/settings/LanguageSettings.vue` | 语言 |
| AboutSettings.vue | `src/components/settings/AboutSettings.vue` | 关于 |

## Tauri Commands（Rust 端）

- `send_message` / `stop_generation`：对话
- `get_sessions` / `get_session_history`：会话
- `update_config` / `update_tools_config`：配置
- `check_health` / `start_background_stream`：健康与流
- 服务管理、Cron 相关 commands 见 `src-tauri/src/commands.rs`

## 事件

- `agent-response-delta` / `agent-reasoning-delta` / `agent-response-complete`
- `agent-tool-start` / `agent-tool-end` / `agent-error`
- `external-message` / `agent-background-response`
