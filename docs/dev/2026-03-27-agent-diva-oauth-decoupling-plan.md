# agent-diva OAuth 解耦实施计划

## 背景

截至 2026-03-27，`agent-diva` 已经完成 `openai-codex` 的真实 OAuth 登录闭环，包括：

- provider metadata 标记 `auth_mode=oauth`
- 独立 provider auth store
- CLI `login/status/use/refresh/logout`
- GUI provider 设置页的浏览器登录与 redirect 回填
- `openai-codex` runtime 直接消费 OAuth token

当前缺口不再是“有没有 OAuth”，而是“OAuth 抽象是否足够通用、可扩展、可复用”。

## 当前耦合点

### 1. core auth service 中仍包含 provider-specific 协议逻辑

`agent-diva-core/src/auth/service.rs` 目前直接持有：

- `OPENAI_OAUTH_CLIENT_ID`
- `OPENAI_OAUTH_TOKEN_URL`
- `refresh_openai_codex_tokens`
- `get_valid_openai_codex_access_token`

这意味着：

- `core` 知道具体 provider 协议细节
- 第二个 OAuth provider 接入时会继续向 `core` 堆特例
- CLI / GUI / runtime 只能围绕 `openai-codex` 做显式分支

### 2. provider login service 仍是单 provider 分发

`agent-diva-providers/src/provider_auth/mod.rs` 目前只有：

- `openai_codex` handler
- `match request.provider.as_str()` 的硬编码分发

这使得 metadata 虽然已经表达了 `login_supported`，但真正的 handler 绑定仍不是声明式的。

### 3. GUI auth 能力尚未下沉到统一 control plane

当前 GUI OAuth 命令在 `agent-diva-gui/src-tauri/src/commands.rs` 内直接编排：

- pending login
- browser callback wait
- redirect completion
- refresh / logout

而 `agent-diva-manager` 的 provider companion 还没有对应 auth API。

### 4. auth store 目前是独立文件，但不是 secret-store 语义

`agent-diva-core/src/auth/store.rs` 已经把 OAuth token 从 `config.json` 抽离到：

- `data/auth/profiles.json`

但当前设计仍缺：

- token 字段加密
- pending OAuth state 持久化与保护
- 明确的 schema migration 与 secret migration 语义

## zeroclaw 的解耦方式

`zeroclaw` 的实现可以拆成四层：

### 1. OAuth 公共协议层

文件：

- `.workspace/zeroclaw/src/auth/oauth_common.rs`

职责：

- PKCE state 生成
- URL encode/decode
- query params 解析

### 2. provider-specific OAuth adapter

文件：

- `.workspace/zeroclaw/src/auth/openai_oauth.rs`
- `.workspace/zeroclaw/src/auth/gemini_oauth.rs`

职责：

- authorize URL 构建
- code exchange
- device-code start/poll
- provider-specific JWT/account 解析

### 3. 通用 auth store 与 auth service

文件：

- `.workspace/zeroclaw/src/auth/profiles.rs`
- `.workspace/zeroclaw/src/auth/mod.rs`

职责：

- profile 持久化
- active profile 选择
- token refresh orchestration
- secret store 加密/迁移

### 4. CLI / runtime 只消费抽象

文件：

- `.workspace/zeroclaw/src/main.rs`
- `.workspace/zeroclaw/src/providers/openai_codex.rs`
- `.workspace/zeroclaw/src/providers/mod.rs`

职责：

- CLI 编排 pending login / paste-redirect / import
- runtime 通过 `auth_profile_override` 获取正确 profile
- fallback provider chain 传递 `provider:profile`

这套结构的关键不是“文件更多”，而是边界更清楚：

- 协议细节不放进 core store
- runtime 不直接写 OAuth 协议
- CLI/GUI 只做交互编排
- provider profile override 可以沿 provider chain 传播

## 对 agent-diva 的实施方案

### Phase 0：文档和产品口径对齐

目标：

- 去除“provider login 仍是 placeholder”的所有用户口径
- 明确当前支持范围和限制

改动：

- 更新 `docs/user-guide/commands.md`
- 更新 `docs/userguide.md`
- 补充 GUI 限制说明：仅 `openai-codex`、无 GUI device-code

验收：

- 用户文档不再声称 `provider login` 返回 `not_implemented`

### Phase 1：抽离通用 OAuth 核心能力

目标：

- 把 PKCE / redirect parsing / pending-state 数据模型从 `openai-codex` handler 中抽出来

建议新增模块：

- `agent-diva-core/src/auth/oauth_common.rs`
- `agent-diva-core/src/auth/pending.rs`

建议职责：

- `oauth_common.rs`
  - PKCE state
  - URL/query parse
  - redirect code extraction
- `pending.rs`
  - pending login record
  - pending login store trait
  - 文件实现

验收：

- `openai_codex.rs` 只保留 OpenAI-specific authorize/token/device-code 协议

### Phase 2：把 provider-specific refresh/token 逻辑移出 core auth service

目标：

- 让 `ProviderAuthService` 只负责 profile store 和通用 orchestration

建议新增 trait：

- `ProviderOAuthBackend`

建议接口：

- `provider_name()`
- `start_browser_login()`
- `exchange_code()`
- `start_device_code()`
- `poll_device_code()`
- `refresh_tokens()`
- `extract_account_id()`

建议落点：

- `agent-diva-providers/src/provider_auth/backends/`

验收：

- `agent-diva-core/src/auth/service.rs` 不再出现 `OPENAI_OAUTH_TOKEN_URL`
- `refresh_openai_codex_tokens` 迁移为通用 refresh 入口加 backend dispatch

### Phase 3：把 login service 从硬编码 match 改成 registry/handler 注册

目标：

- `ProviderLoginService` 不再手写 `match "openai-codex"`

建议方案：

- 基于 provider metadata + backend registry 做查找
- backend registry 可先静态注册，再视情况改成声明式注册

建议接口：

- `get_oauth_backend(provider: &str) -> Option<&dyn ProviderOAuthBackend>`

验收：

- 登录分发逻辑不再依赖单一 provider 字符串常量

### Phase 4：统一 GUI / CLI / manager auth control plane

目标：

- GUI 不再独占 provider auth 编排逻辑

建议改动：

- 在 `agent-diva-manager` 增加 provider auth HTTP/command handlers
- Tauri 改为调用 manager auth API，而不是本地重复 orchestration

建议新增能力：

- `get_provider_auth_status`
- `login_provider`
- `get_provider_login_status`
- `complete_provider_login`
- `use_provider_profile`
- `refresh_provider_auth`
- `logout_provider`

验收：

- GUI 与未来 web/control plane 共用同一后端 auth 路径

### Phase 5：安全与持久化增强

目标：

- 提升 auth store 和 pending login 的安全性

建议改动：

- 为 `profiles.json` 内 token 字段增加 SecretStore 加密
- 给 pending login 持久化 `state` 和 `code_verifier`
- 对 pending login 文件设置最小权限
- 补 schema migration

验收：

- access token / refresh token 不再以明文形式落盘
- GUI 重启后仍可继续 paste-redirect 完成登录

### Phase 6：验证抽象是否成立

目标：

- 用第二个 OAuth provider 检验设计，不让抽象只服务于 `openai-codex`

建议候选：

- `gemini`

验收：

- 不新增第二套平行 auth 架构
- CLI / GUI / runtime 复用同一套 backend registry 与 auth store

## 推荐实施顺序

1. 文档同步
2. `oauth_common` + pending login store
3. backend trait + `openai-codex` backend 迁移
4. `ProviderLoginService` 注册化
5. manager auth API
6. GUI 改走 manager
7. secret store / pending persistence 安全增强
8. 第二个 OAuth provider 验证抽象

## 最小交付范围

如果本轮只做一版“可控、低风险”的解耦，建议止于：

- Phase 0
- Phase 1
- Phase 2
- Phase 3

这样能先把 `openai-codex` 的特殊协议从 core 中挪出去，且不必一次性改动 GUI/manager 全链路。

## 风险

- 若先做 GUI/manager，而不先抽 backend trait，最终只会把 `openai-codex` 特判从 Tauri 挪到 manager，结构收益有限。
- 若先做第二个 provider，而不先处理 core auth service 中的 provider-specific refresh 逻辑，会让历史包袱扩大。
- 若不处理 pending login 持久化，GUI 在浏览器跳转失败或进程重启后的恢复能力会一直偏弱。

## 验收口径

- 用户文档与真实能力一致
- `core` 不再直接硬编码 OpenAI OAuth refresh/token endpoint
- `provider login` dispatch 不再手写 provider `match`
- GUI/CLI 至少其中一端可通过统一 backend 接口工作
- token/pending-state 存储具备明确安全边界
