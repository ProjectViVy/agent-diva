# 实现 nanobot 水准 Provider 的 `zeroclaw` 抄作业地图

本文回答一个非常具体的问题：

> 如果 `agent-diva` 在 Provider 侧只需要先达到 `.workspace/nanobot` 的成熟度，而不是一步做到 `openclaw` 那种完整 provider platform，那么应该从 `.workspace/zeroclaw` 抄哪些部分，按什么顺序抄，分别落到 `agent-diva` 哪些 crate。

本文是实施地图，不是抽象讨论。结论以“够用、可落地、能支撑 P0/P1” 为第一优先级。

## 目标边界

这里的“nanobot 水准 Provider”只要求达到以下闭环：

1. `provider login` 真实可用，不再是 placeholder
2. OAuth / token 型 provider 的凭据不和主配置文件耦合
3. 登录完成后，`provider status`、`provider models`、实际 provider runtime 可以复用同一份认证状态
4. provider 至少具备基础能力声明，而不是把登录方式、模型发现方式、endpoint 规则散落在 CLI 分支里
5. 首批以 `openai-codex` 为标准样板，后续能扩展第二个 OAuth/device-flow provider

这里 **不要求** 一步做到：

- `openclaw` 那种细粒度 provider behavior compatibility matrix
- 完整插件化 provider catalog
- 所有 provider 都支持 live model discovery
- GUI 登录先行

因此，这次应采用：

- 主参考：`.workspace/zeroclaw`
- 补充参考：`.workspace/openclaw` 仅用于未来能力矩阵扩展，不进入首批照抄主体

## 一句话结论

如果目标只是追到 nanobot 水准，`zeroclaw` 最值得抄的不是“所有 provider 文件”，而是下面四层：

1. 统一认证命令面
2. 配置外凭据存储与 active profile 模型
3. OAuth provider 专用 runtime backend
4. 基础 model catalog / live discovery 流程

应避免直接照抄的部分有两类：

1. `zeroclaw` 的大而散 `main.rs` 命令分发
2. `onboard/wizard.rs` 里硬编码 provider 列表的 catalog 逻辑

正确做法是“抄结构，不抄堆积方式”。

## 总体抄写策略

建议按下面顺序迁移，而不是按文件顺序搬运：

1. 先抄 auth data model
2. 再抄 auth service
3. 再抄 `openai-codex` login flow
4. 再抄 `openai-codex` provider runtime
5. 最后抄 model discovery 的公共骨架

原因很简单：

- 没有 auth store，`provider login` 无法闭环
- 没有 runtime 消费 auth，登录完也只是“看起来成功”
- 没有 model/status 复用 auth，CLI 仍会割裂
- catalog 最后做，才能避免把硬编码表提前固化进架构

## 抄作业地图

### 地图 1：统一认证命令面

#### 应抄来源

- `.workspace/zeroclaw/src/main.rs:642`
- `.workspace/zeroclaw/src/main.rs:2098`

`zeroclaw` 已经把认证做成统一命令组：

- `auth login`
- `auth paste-redirect`
- `auth paste-token`
- `auth refresh`
- `auth logout`
- `auth use`
- `auth list`
- `auth status`

这套设计最值得抄的不是命令名本身，而是：

- CLI 只提供统一入口
- provider-specific 登录逻辑在分发后下沉
- status / refresh / use / logout 共用同一份 profile store

#### 对 `agent-diva` 的落点

- `agent-diva-cli/src/provider_commands.rs`
- 如有必要，新建 `agent-diva-cli/src/provider_auth_commands.rs`

#### 应该怎么抄

不要把 `zeroclaw` 的 `AuthCommands` 原样复制成 `main.rs` 巨型 match，而是提炼为：

- `provider login <provider>`
- `provider logout <provider>`
- `provider status [provider]`
- `provider use <provider> <profile>`
- `provider refresh <provider>`

然后把 CLI 层限制在三件事：

1. 参数解析
2. pretty/json 输出
3. 调用 `agent-diva-providers` 或 `agent-diva-core` 暴露的 auth service

#### 不要抄的部分

- 不要继续把 provider-specific OAuth 细节留在 CLI crate
- 不要照抄 `main.rs` 里巨型 `match provider.as_str()`

在 `agent-diva` 里，provider-specific handler 应放到 provider/auth 子模块，而不是 CLI 主流程。

### 地图 2：配置外凭据存储与 profile 模型

#### 应抄来源

- `.workspace/zeroclaw/src/auth/profiles.rs:19`
- `.workspace/zeroclaw/src/auth/mod.rs:28`

`zeroclaw` 在这块已经提供了 nanobot 水准之上的成熟度，尤其是：

- `AuthProfileKind = OAuth | Token`
- `TokenSet { access_token, refresh_token, expires_at, ... }`
- `AuthProfile`
- `active_profiles`
- `AuthProfilesStore`
- `AuthService`

这正好对应 `docs/dev` 里已经定下来的方向：

- provider metadata 要表达认证模式
- OAuth/token 不进主 `config.json`
- status/models/runtime 共用外部凭据存储

#### 对 `agent-diva` 的落点

- `agent-diva-core`
  - 新增 `auth/` 模块最合适
  - 或新建 `provider_auth.rs` / `credential_store.rs`

建议的最小拆分：

- `agent-diva-core/src/auth/profiles.rs`
- `agent-diva-core/src/auth/store.rs`
- `agent-diva-core/src/auth/service.rs`
- `agent-diva-core/src/auth/mod.rs`

#### 应该怎么抄

优先抄“数据模型 + 服务边界”，不要先抄文件格式细节。

第一阶段需要保留的结构：

- `AuthProfileKind`
- `TokenSet`
- `AuthProfile`
- `AuthProfilesData`
- `AuthProfilesStore`
- `AuthService`

第一阶段就应该具备的能力：

- upsert profile
- set active profile
- get active profile
- remove profile
- load profiles
- 根据 provider 获取 bearer token

#### 建议做的本地化调整

`agent-diva` 不需要照搬 `zeroclaw` 命名，可改成更贴合仓库结构的命名：

- `ProviderAuthMode`
- `ProviderAuthProfile`
- `ProviderAuthStore`
- `ProviderAuthService`

但字段语义应尽量保持一致，避免以后对照困难。

#### 不要抄的部分

- 不要先引入太多 `workspace_id`、多租户语义
- 不要先追求系统 keychain 集成

第一期只要做到：

- 与 `config.json` 解耦
- 目录权限收紧
- refresh token 可持久化

就已经满足 nanobot 对标所需。

### 地图 3：`openai-codex` 登录流

#### 应抄来源

- `.workspace/zeroclaw/src/auth/openai_oauth.rs`
- `.workspace/zeroclaw/src/main.rs:2201`
- `.workspace/zeroclaw/src/main.rs:1893`

最有价值的不是某个 HTTP 请求，而是完整闭环：

1. 生成 PKCE / state
2. 支持 browser 或 device-code flow
3. 暂存 pending login 状态
4. 交换 token
5. 落到 profile store
6. 设置 active profile

#### 对 `agent-diva` 的落点

- `agent-diva-providers/src/auth/openai_codex.rs`
- `agent-diva-core/src/auth/service.rs`
- `agent-diva-cli/src/provider_commands.rs`

建议边界：

- OAuth 协议细节放 `agent-diva-providers`
- profile store 放 `agent-diva-core`
- CLI 仅做命令路由

#### 应该怎么抄

推荐直接抄下面这条调用链的分层方式：

1. CLI 调 `login(provider, profile, mode)`
2. provider auth handler 启动授权
3. handler 拿到 `TokenSet`
4. `agent-diva-core` 的 auth service 存储 token/profile
5. CLI 输出结果

对 `openai-codex` 首批实现，建议只保留：

- browser + paste-redirect
- 可选 device-code
- import 旧凭据文件可后置

这样复杂度更稳。

#### 必须同步补的行为

如果只做 `provider login` 而不同时补这些接口，闭环还是不完整：

- `provider status`
- `provider models`
- provider runtime 读取 OAuth token

### 地图 4：OAuth provider 专用 runtime backend

#### 应抄来源

- `.workspace/zeroclaw/src/providers/openai_codex.rs`
- `.workspace/zeroclaw/src/auth/mod.rs:162`

这部分是 `zeroclaw` 最值得抄的地方之一，因为它证明了一件事：

> `openai-codex` 不应该被硬塞进普通 API-key OpenAI-compatible provider 路径。

`zeroclaw` 的做法是：

- `openai-codex` 有独立 provider backend
- runtime 从 auth service 取有效 access token
- token 快过期时自动 refresh
- 请求头和 endpoint shape 由专用 backend 处理

这和当前 `docs/dev` 中的 P0 结论完全一致。

#### 对 `agent-diva` 的落点

- `agent-diva-providers/src/backends/openai_codex.rs`
- 或 `agent-diva-providers/src/providers/openai_codex.rs`

#### 应该怎么抄

不要抄整份网络请求细节，先抄这三个接口层面的事实：

1. provider runtime 初始化时知道 auth service
2. provider call 前能够取到有效 OAuth token
3. refresh 行为由 auth/service 处理，而不是散落在聊天逻辑里

建议在 `agent-diva-providers` 里明确区分两类 backend：

- `OpenAiCompatibleProviderBackend`
- `OpenAiCodexProviderBackend`

这样后续不会破坏“原生 provider model id 不应被错误改写”的规则，也不会把 OAuth provider 和 API-key provider 混在同一路由。

### 地图 5：基础 model catalog / live discovery

#### 应抄来源

- `.workspace/zeroclaw/src/onboard/wizard.rs:1312`
- `.workspace/zeroclaw/src/main.rs:729`

`zeroclaw` 在 catalog 侧可以借鉴的是“流程骨架”，不是 provider 列表本身。

可复用的思路：

- 支持 `models refresh`
- 支持 `models list`
- 支持“provider 是否支持 live discovery”的判断
- 支持按 provider endpoint 拉取模型目录

#### 对 `agent-diva` 的落点

- `agent-diva-providers/src/discovery.rs`
- `agent-diva-providers/src/registry.rs`
- `agent-diva-cli/src/provider_commands.rs`

#### 应该怎么抄

只抄下面这套流程：

1. 先通过 provider metadata 判断是否支持 model discovery
2. 若支持，取 provider 专属或兼容 endpoint
3. 拉取并归一化 model IDs
4. 缓存结果
5. `provider models` / onboarding 复用缓存

#### 明确不要照抄的部分

`zeroclaw` 的下面两类逻辑不要原样搬：

- `supports_live_model_fetch()` 的硬编码 provider 名单
- `models_endpoint_for_provider()` 的大型 `match`

在 `agent-diva` 中，这两类信息应该上提到 provider registry metadata，比如：

- `model_discovery = static | live`
- `models_endpoint`
- `requires_auth_for_models`

也就是说，这里要抄的是控制流，不是数据存放位置。

## 建议增加的 provider metadata

为了让 `agent-diva` 达到 nanobot 水准且避免以后返工，建议现在就在 provider metadata 中补这组最小字段：

- `auth_mode = api_key | oauth | device_flow | token`
- `login_supported = true | false`
- `credential_store = config | external_secure_store`
- `model_discovery = static | live`
- `models_endpoint = optional`
- `runtime_backend = openai_compatible | openai_codex | anthropic | ...`

这些字段不需要一步到位全部被 GUI 消费，但必须先存在于 provider 层，否则：

- CLI 会继续堆硬编码
- provider runtime 会继续靠字符串判断
- onboarding 和 status 之后还要重做

## 按 crate 的具体落位建议

### `agent-diva-core`

负责：

- auth profile 数据模型
- auth store
- active profile 选择
- token refresh 协调

第一批建议新增：

- `src/auth/mod.rs`
- `src/auth/profiles.rs`
- `src/auth/store.rs`
- `src/auth/service.rs`

### `agent-diva-providers`

负责：

- provider metadata
- provider auth handlers
- provider runtime backend
- model discovery

第一批建议新增或改造：

- `src/provider_auth/mod.rs`
- `src/provider_auth/openai_codex.rs`
- `src/backends/openai_codex.rs`
- `src/discovery.rs`
- `src/registry.rs`
- `src/providers.yaml`

### `agent-diva-cli`

负责：

- `provider login/logout/status/use/refresh/models`
- pretty/json 输出
- 不保留 provider-specific OAuth 协议实现

## 分阶段照抄顺序

### Phase 1：打通 `openai-codex` 登录闭环

照抄重点：

- `AuthProfile` / `TokenSet` / `AuthService`
- `openai_oauth`
- `provider login openai-codex`
- `provider status`

验收标准：

- `agent-diva provider login openai-codex` 能真实成功
- token 不进入主配置文件
- `provider status` 能显示已登录

### Phase 2：让 runtime 真正消费认证

照抄重点：

- `get_valid_openai_access_token`
- `openai_codex` 独立 backend

验收标准：

- 登录后真实聊天路径可用
- token 临近过期能自动 refresh 或至少有清晰失败语义

### Phase 3：补齐 `provider models`

照抄重点：

- model refresh/list 流程骨架
- provider metadata 驱动 discovery

验收标准：

- `provider models` 对已支持的 provider 可工作
- onboarding 也能消费同一份发现结果

### Phase 4：抽象第二个 OAuth / device-flow provider

照抄重点：

- Gemini 那条登录流的结构
- 复用已有 auth store/service，而不是再做特例

验收标准：

- 第二个 provider 接入时不需要改 CLI 主流程
- 只需要新增 provider auth handler + metadata

## 可以直接参考的文件清单

如果只看最关键文件，优先级如下：

1. `.workspace/zeroclaw/src/auth/profiles.rs`
2. `.workspace/zeroclaw/src/auth/mod.rs`
3. `.workspace/zeroclaw/src/auth/openai_oauth.rs`
4. `.workspace/zeroclaw/src/providers/openai_codex.rs`
5. `.workspace/zeroclaw/src/main.rs`
6. `.workspace/zeroclaw/src/onboard/wizard.rs`

推荐阅读顺序：

1. 先读 `profiles.rs`，确认数据模型
2. 再读 `auth/mod.rs`，确认 service 边界
3. 再读 `openai_oauth.rs`，确认协议动作
4. 再读 `providers/openai_codex.rs`，确认 runtime 如何消费认证
5. 最后读 `main.rs` 与 `wizard.rs`，只提取 CLI 和 catalog 骨架

## 不建议抄的内容

### 1. 不要抄 `zeroclaw` 的巨型 CLI 聚合方式

`main.rs` 对研究很有价值，但不适合作为 `agent-diva` 的代码风格模板。`agent-diva` 应把逻辑拆回各 crate，避免继续膨胀入口文件。

### 2. 不要抄硬编码 provider 列表作为长期方案

`supports_live_model_fetch()` 和大型 endpoint `match` 更适合临时可用，不适合 `agent-diva-providers` 的长期架构。

### 3. 不要把 Qwen OAuth、Minimax OAuth 等复杂变体一起引入

这会把本来清晰的 P0 扩张成 provider platform 重构。当前目标只是追到 nanobot 水准，首批只应围绕 `openai-codex` 做出样板。

## 最终建议

如果只允许做一条最短路线，建议严格按下面的抄写顺序推进：

1. 先把 `zeroclaw` 的 `AuthProfile` / `AuthService` 迁到 `agent-diva-core`
2. 再把 `openai_oauth` 迁到 `agent-diva-providers`
3. 再把 `provider login openai-codex` 接到 CLI
4. 再把 `openai_codex` 独立 runtime backend 接上
5. 最后再把 `provider models` 的 discovery 骨架接上

这条路线的优点是：

- 最符合当前 `docs/dev` 已经形成的 P0 共识
- 最接近 nanobot 的“真实可用”标准
- 对现有 crate 边界破坏最小
- 不会把问题提前升级成 plugin/provider platform 全面重构

如果后续需要继续上台阶，再从 `openclaw` 引入更细粒度的 provider capability matrix；但那应是 nanobot 对齐之后的下一阶段，而不是当前主线。
