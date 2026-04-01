---
project_name: agent-diva-channels
date: 2026-03-30
module: agent-diva-channels
status: complete
parent_workspace: agent-diva
---

# agent-diva-channels

## 模块职责

为 agent-diva 提供**多聊天平台接入**：收消息（入站 `InboundMessage`）、发消息（出站 `OutboundMessage`），与 `agent-diva-core` 配置与总线对接。按平台分文件实现（Telegram、Discord、Slack、Matrix、Mattermost、IRC、Feishu、钉钉、QQ、Email、WhatsApp、Nextcloud Talk、Neuro Link 等），`manager` 统一装配与校验。

## 依赖与边界

- **上游**：`agent-diva-core`（配置 schema、消息类型）、`agent-diva-providers`（模型侧能力，与本 crate 并列使用）。
- **网络栈**：`reqwest`、`tokio-tungstenite`（含 native-tls）、各平台 SDK（如 `teloxide`、`slack-morphism`）。
- **邮件**：阻塞 IMAP（`imap`）+ `lettre` SMTP + 解析与 HTML 转义。
- **边界**：不实现推理/工具执行；只负责渠道生命周期、鉴权配置、发送方白名单/默认拒绝策略（见 `BaseChannel`）。

## 关键类型/入口

- `ChannelHandler`（`base.rs`）：`start` / `stop` / `send` / `set_inbound_sender` / `is_allowed`。
- `BaseChannel`：共享 `allow_from`、`deny_by_default`、运行态与入站 `mpsc`。
- `ChannelManager`：按配置启用渠道、`ChannelValidation` 校验必填字段、处理器映射。
- `ChannelError`：配置、连接、鉴权、发送等错误变体。
- `common::create_http_client`：**默认 30s** 超时的 `reqwest::Client`；`download_file` 等媒体下载走该客户端与用户目录下 `.nanobot/media`。

## 实现约定

- **网络与超时**：各 handler 在 `Client::builder().timeout(...)` 或长轮询 URL 参数上自行设定（如 Matrix sync、Nextcloud `timeout=30` + 请求 35s、QQ/Feishu 等常见 30s）；IRC/WhatsApp 等有读循环与 `tokio::time::timeout`/`sleep` 退避。新增渠道时应**明确 HTTP/WebSocket 超时、重连与背压**，避免无限挂起。
- **安全**：令牌与密码仅来自 core 配置；`is_allowed` 必须在处理入站前使用；媒体下载应对文件名做简单净化（如替换 `/`）。
- **异步**：`async-trait` + `tokio`；WebSocket/长连接与轮询并存，注意 `stop` 时任务取消与 grace（参考 Matrix 等现有模式）。

## 测试与检查

- `dev-dependencies`：`tokio-test`。以 `cargo test -p agent-diva-channels` 为准；集成测试需有效凭证时通常跳过或 mock。
- 变更后建议：`cargo clippy -p agent-diva-channels`、`cargo test -p agent-diva-channels`。

## 切勿遗漏

- 新渠道须在 `ChannelManager` 中注册并补齐 `channel_validation` 必填项。
- 默认 HTTP 客户端超时在 `common` 为 **30 秒**；长轮询类接口需与对端 `timeout` 参数对齐，避免过早断开或双重超时逻辑冲突。
- Email 使用阻塞 IMAP：勿在 async 运行时内长时间阻塞 `spawn_blocking` 未包裹的调用路径（若上层已包装则保持与现有 handler 一致）。
