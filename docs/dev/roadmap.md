# Agent Diva 长期 Roadmap

> 最后更新：2026-06-26
> 状态：草案，持续更新

## 1. 愿景

Agent Diva 的核心是**极简、自托管、可扩展**的个人 AI 助手框架。长期 roadmap 围绕三条主线推进：

1. **核心收敛**：Provider / Channel / Config 持续简化，保持最小可用内核。
2. **可靠性闭环**：retry、fallback、audit、observability、security 达到生产级。
3. **扩展性**：通过插件/兼容层接入更广泛的生态，但不把复杂度带入核心。

## 2. 已决策的长期方向

### 2.1 Provider 极简主义 + 生产级完整

- **保留**：Anthropic Messages API 原生驱动 + OpenAI-compatible 通用驱动。
- **质量要求**：所有保留的 provider 必须是"生产级完整"，而非最小可用。必须支持：
  - 正确的 model ID 路由（native endpoint 不发 LiteLLM prefix）
  - retry / fallback / rate-limit 识别
  - token usage 闭环
  - tool schema / function calling
  - streaming + non-streaming
  - 配置校验与明确的迁移错误
- **不做**：OAuth/网页登录/device flow、云平台 IAM、刷新 token provider、内置 gateway/聚合器。
- **扩展方式**：用户自部署 OpenAI-compatible 转接层（如 NewAPI）。

### 2.2 Channel 极简主义 + 生产级完整

- **一等公民**：Telegram、Discord、Slack、Email、QQ、Feishu/Lark、DingTalk、WeChat。
- **质量要求**：所有保留的一等公民 channel 必须是"生产级完整"，而非最小可用。必须支持：
  - 群聊 / 频道 / 私聊全场景
  - 文件/图片/媒体上传与下载（平台能力允许范围内）
  - 完整的入站/出站消息链路（接收、发送、thread 回复、@mention、去重）
  - QR/扫码/无 OAuth 的配置方式（如 WeChat iLink Bot）
  - 优雅关闭、断线重连、健康检查
  - 清晰的 ACL/allowlist 策略，无 fail-open 安全回退
- **保留但有限维护**：Matrix、Neuro-Link。
- **移除/插件化**：WhatsApp、Mattermost、Nextcloud Talk、IRC。
- **明确不做**：OAuth/网页登录/云平台 IAM channel、社交/内容平台。

### 2.3 ZeroClaw 兼容层（长期）

- **目标**：提供一个轻量 ZeroClaw-compatible adapter layer，让用户可以通过统一配置接入 ZeroClaw 生态中的其他 channel（如 WhatsApp Bridge、Mattermost、Nextcloud Talk、IRC 等）。
- **原则**：
  - 兼容层本身不内置具体 channel 实现。
  - 仅提供协议/配置桥接。
  - 用户需自部署对应 ZeroClaw channel 后端。
- **位置**：作为 Neuro-Link 的继任者或独立 plugin crate 实现。
- **状态**：本周不做；待 channel 简化稳定后再评估优先级。
- **待决策**：技术方案（WASM plugin / 独立进程 IPC / 直接复用 ZeroClaw channel crate）。

## 3. 近期排期（TODOLIST）

### agent-diva-pro

- **P1-8**: Provider architecture simplification
- **P1-9**: Channel architecture simplification

### agent-diva

- **P1-7**: Provider architecture simplification
- **P1-8**: Channel architecture simplification

## 4. 关键技术债

| 领域 | 问题 | 优先级 |
|------|------|--------|
| Channel | 无 feature flag，编译依赖臃肿 | 高 |
| Channel | Slack 缺失 thread backfill、文件上传、draft 流式 | 高 |
| Provider | `LiteLLMClient` 收敛为 `OpenAiCompatibleDriver` | 高 |
| Provider | 缺少 retry/fallback/rate-limit 分类 | 高 |
| Config | `ChannelsConfig` / `ProvidersConfig` schema 需要精简 | 高 |
| Observability | Debug bundle、provider raw HTTP tap、MCP RPC tap | 中 |
| Security | MCP 并发锁、重连、结果大小限制 | 中 |
| Skill | 注入检测 `instruction_hierarchy.rs` / `tool_result_filter.rs` 缺失 | 中 |

## 5. 不做清单（明确排除）

- 内置 LLM gateway / 聚合器
- OAuth / 网页登录 / 云平台 IAM provider 或 channel
- 原生支持所有主流 SaaS 平台（Teams、Google Chat、Webex、Salesforce 等）
- 多租户 / SaaS 托管服务
- 分布式 leader election / 高可用集群

## 6. 相关文档

- `docs/dev/provider-simplification-research-2026-06.md`
- `docs/dev/channel-simplification-decision-2026-06.md`
- `docs/prds/prd-harness-engineering-v1.1/prd.md`
- `TODOLIST.md`
