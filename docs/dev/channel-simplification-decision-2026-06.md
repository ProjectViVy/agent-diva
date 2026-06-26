# Channel 架构简化决策文档

> 日期：2026-06-26
> 状态：已拍板，待实施
> 适用范围：agent-diva / agent-diva-pro

## 1. 背景

当前 `agent-diva-channels` 实现了 13 个 channel adapter（Telegram、Discord、Slack、Email、QQ、Feishu、DingTalk、WhatsApp、Matrix、IRC、Mattermost、Nextcloud Talk、Neuro-Link），全部硬编码且无 feature flag 切分。这与 Provider 层"极简主义"原则不一致，也带来了过重的编译依赖和维护负担。

本决策基于对 agent-diva、agent-diva-pro、zeroclaw、openfang 的 channel 架构调研，以及用户对 Provider 简化的同一套核心原则。

## 2. 核心原则

与 Provider 简化对齐：

- **不做**任何需要 OAuth / 网页登录 / 云平台 IAM / 刷新 token 的 channel。
- **不做**内置 gateway / 聚合器 / 中间件。
- **不做**部署复杂、需要外部桥接或商业审核的 channel。
- Agent Diva 的核心是**极简**；复杂的 channel 应通过用户自部署的转接层（如 ZeroClaw 兼容层、Neuro-Link、NewAPI 等）接入。

## 3. 决策结果

### 3.1 一等公民（原生维护）

这些 channel 是 Agent Diva 的核心使用场景，将继续原生维护：

| Channel | 说明 |
|---------|------|
| **Telegram** | 国际个人用户最常用，Bot API 简单稳定 |
| **Discord** | 社区/开发者场景 |
| **Slack** | 工作场景；保留并做链路核对和增强 |
| **Email (IMAP/SMTP)** | 异步工作流，不可替代 |
| **QQ** | 国内核心 IM 之一 |
| **Feishu / Lark** | 国内企业/办公场景 |
| **DingTalk** | 国内企业/办公场景 |
| **WeChat（新增）** | 国内最高频 IM，参考 ZeroClaw iLink Bot 方案新增 |

> **共 8 个一等公民 channel。**

### 3.2 保留但有限维护

| Channel | 说明 |
|---------|------|
| **Matrix** | 开源联邦协议，未来有战略价值；保留但不优先做 E2EE 等高级功能 |
| **Neuro-Link** | 暂时保留作为第三方自定义集成的标准入口，但未来会做重量级重构或替换 |

### 3.3 移除或插件化

以下 channel **从原生代码中移除**，未来考虑通过 ZeroClaw 兼容层或 Neuro-Link 插件化接入：

| Channel | 移除原因 |
|---------|----------|
| **WhatsApp** | 依赖外部 Node.js Bridge（Baileys），部署复杂，维护重 |
| **Mattermost** | 企业自托管，场景可被 Slack/Discord/Matrix 覆盖 |
| **Nextcloud Talk** | 小众，轮询实时性差 |
| **IRC** | 协议古老，用户群体极小 |

### 3.4 明确不做

- 任何带 OAuth / 网页登录 / 云平台 IAM 的 channel（Teams、Google Chat、Webex、Zoom 等）
- 社交/内容平台（Twitter/X、Bluesky、Reddit、Twitch、LinkedIn 等）

## 4. 与 Provider 简化的对称设计

| 层级 | 保留 | 移除/外置 |
|------|------|-----------|
| Provider | Anthropic 原生、OpenAI-compatible | 其他专属 provider 走自部署转接层 |
| Channel | Telegram、Discord、Slack、Email、QQ、Feishu、DingTalk、WeChat | WhatsApp、Mattermost、Nextcloud Talk、IRC 走插件/桥接 |
| 通用入口 | Neuro-Link（ interim ） | 复杂商业平台明确不做 |

## 5. 关键设计参考（ZeroClaw）

### 5.1 WeChat：iLink Bot QR 扫码方案

参考 ZeroClaw `wechat.rs` 实现：

- **接入类型**：微信个人号，通过 iLink Bot API（`ilinkai.weixin.qq.com`）。
- **协议**：HTTPS REST JSON + 长轮询 `getUpdates`，无 WebSocket，无第三方 SDK。
- **认证**：QR 码扫码登录，无 OAuth/网页登录；token 持久化到 `~/.agent-diva/wechat/`。
- **依赖**：`reqwest`（已有）+ `aes` / `ecb` / `md5` / `mime_guess` / `qrcode`（新增可选依赖）。
- **能力**：文本、图片、文件、视频、语音双向收发；语音可转文字。
- **限制/风险**：
  - iLink API 非公开文档，可能变更
  - 会话会过期（errcode -14），需重新扫码
  - 媒体传输使用 AES-128-ECB（协议强制），密码学较弱
  - 存在封号/合规风险，需用户知情

**决策**：采用 QR 扫码 + 长轮询方案，作为 WeChat 一等公民实现；企业微信/公众号/小程序不原生支持。

### 5.2 Slack：保留并增强

当前 Agent Diva Slack 实现为 P0 最小可用版本（Socket Mode + AppMention + DM + Thread Reply + 基础 Markdown 转换）。ZeroClaw 提供了更完整的链路实现，可作为增强参考。

**链路核对结论**：

| 链路 | 当前 Agent Diva | 状态 |
|------|----------------|------|
| Socket Mode 连接 | 通过 `slack-morphism` 实现 | ✅ 等价 |
| 事件解析 | 强类型 Event 结构 | ✅ 更类型安全 |
| Bot 自循环过滤 | 已实现 | ✅ 等价 |
| @mention 检测 | 已实现 | ✅ 等价 |
| 用户 allowlist | 已实现 | ✅ 等价 |
| 线程上下文回填 | 缺失 | ⚠️ 待增强 |
| 附件处理 | 缺失 | ⚠️ 待增强 |
| Permalink 自动展开 | 缺失 | ⚠️ 待增强 |
| 草稿流式更新 | 缺失 | ⚠️ 待增强 |
| 文件上传 | 缺失 | ⚠️ 待增强 |
| Reaction | 缺失 | ⚠️ 待增强 |
| Block Kit | 缺失 | ⚠️ 待增强 |
| 用户显示名解析 | 缺失 | ⚠️ 待增强 |
| Polling 回退模式 | 仅 Socket Mode | ⚠️ 待增强 |

**可增强点（按 P0/P1/P2 优先级）**：

- **P0**：
  1. Polling 回退模式（无 `app_token` 时自动降级）
  2. Thread 上下文回填（首次进入线程自动拉取历史）
  3. 文件上传支持（`files.getUploadURLExternal` → upload → `files.completeUploadExternal`）
  4. Draft 流式更新（`send_draft` / `update_draft` / `finalize_draft` / `cancel_draft`）

- **P1**：
  5. Permalink 自动展开
  6. 附件处理（图片下载、文本预览、音频转录）
  7. Block Kit 支持
  8. Approval UI 增强（Block Kit 按钮）

- **P2**：
  9. 用户显示名解析
  10. Reaction 支持
  11. 健康检查（`auth.test` + Socket Mode 探测）
  12. Markdown → mrkdwn 转换保留并增强

## 6. 实施建议

1. **在 agent-diva-pro 分支先行实施**，与 Provider 简化（P1-8）同波推进。
2. 新增 **WeChat channel**：采用 ZeroClaw 同款 iLink Bot QR 扫码 + 长轮询方案。
3. 将 WhatsApp / Mattermost / Nextcloud Talk / IRC 标记为 `deprecated`，从 `ChannelsConfig` 和 `manager.rs` 中移除。
4. Slack 保留并增强：按 P0/P1/P2 优先级补齐链路缺口，直至生产级完整。
5. 对**所有保留的一等公民 channel**做全面增强，确保群聊、文件上传、完整入站/出站链路、无 OAuth 配置方式全部可用；不保留任何"最小可用"半成品。
6. Matrix 保留但不做 E2EE；Neuro-Link 保留但标记为"未来重构"。
7. 引入 Cargo feature flag，让每个 channel 可条件编译，减少二进制体积。
8. 更新 README、GUI channel 列表、用户文档、配置示例。

## 7. 长期 Roadmap：ZeroClaw 兼容层

- **目标**：未来引入一个轻量 ZeroClaw-compatible adapter layer，使用户可以通过统一配置接入 ZeroClaw 生态中的其他 channel（如 WhatsApp Bridge、Mattermost、Nextcloud Talk、IRC 等），而无需 Agent Diva 原生维护这些 adapter。
- **原则**：兼容层本身不内置具体 channel 实现，仅提供协议/配置桥接；用户自部署对应 ZeroClaw channel 后端。
- **位置**：作为 Neuro-Link 的继任者或独立 plugin crate 实现。
- **状态**：本周不做，纳入长期 roadmap，待 channel 简化稳定后再评估优先级。

## 8. 风险与待决策

- **WeChat iLink API 稳定性**：非公开 API，可能变更或不可用；需持续监控并准备 fallback 方案。
- **Slack 增强范围**：P0/P1/P2 增强点需要进一步排期，避免一次改造过大。
- **ZeroClaw 兼容层技术方案**：未来需明确是通过 WASM plugin、独立进程 IPC，还是直接复用 ZeroClaw channel crate。

## 9. 相关文件

- `agent-diva-pro/agent-diva-channels/src/*.rs`
- `agent-diva-pro/agent-diva-core/src/config/schema.rs`
- `agent-diva-pro/docs/prds/prd-harness-engineering-v1.1/prd.md`（Epic X 或新增 Epic）
- `agent-diva-pro/TODOLIST.md`（P1-9）
- `agent-diva/TODOLIST.md`
- `agent-diva/.workspace/zeroclaw/crates/zeroclaw-channels/src/wechat.rs`
- `agent-diva/.workspace/zeroclaw/crates/zeroclaw-channels/src/slack.rs`

## 10. 决策日志

- 2026-06-26：用户拍板 channel 简化范围。一等公民 8 个：Telegram、Discord、Slack、Email、QQ、Feishu、DingTalk、WeChat。保留 Matrix、Neuro-Link。移除 WhatsApp、Mattermost、Nextcloud Talk、IRC。ZeroClaw 兼容层纳入长期 roadmap。本周起不做代码实现。
- 2026-06-26（追加）：用户明确所有保留的 provider 和 channel 必须做到"生产级完整"，而非最小可用。所有一等公民 channel 必须支持群聊、文件上传、完整入站/出站链路、QR/扫码配置；所有保留 provider 必须支持 retry/fallback/rate-limit/token usage/tool schema 等完整能力。
