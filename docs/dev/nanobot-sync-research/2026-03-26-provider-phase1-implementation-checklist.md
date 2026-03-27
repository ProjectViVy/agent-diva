# Provider Phase 1 实施任务清单

本文是上一份 `zeroclaw` provider 抄作业地图的执行版清单。

目标不是继续讨论“应该做什么”，而是把 `Phase 1` 直接拆成：

- crate 级任务
- 模块级文件清单
- 测试项
- 验收口径
- 推荐实施顺序

本文默认目标是：

> 让 `agent-diva` 先达到 `.workspace/nanobot` 水准的 provider 闭环，首批只完成 `openai-codex`。

## Phase 1 范围

### 本阶段必须完成

1. `agent-diva provider login openai-codex` 不再是 placeholder
2. OAuth token/profile 与主配置解耦
3. `provider status` 能识别 `openai-codex` 登录状态
4. provider runtime 可以消费这份认证状态
5. provider metadata 至少能表达 `auth_mode/login_supported/credential_store`

### 本阶段明确不做

- GUI 登录流程
- 第二个 OAuth provider
- 完整 `provider models` live discovery
- 系统级 keychain 集成
- `openclaw` 式细粒度 capability matrix

## 总体实施顺序

推荐按下面 6 步推进：

1. 补 provider metadata 最小字段
2. 建 `agent-diva-core` auth store / profile 模型
3. 建 `agent-diva-providers` 的 `openai-codex` auth handler
4. 接通 `agent-diva-cli` 的 `provider login/status/logout/use/refresh`
5. 让 `openai-codex` runtime backend 消费 auth service
6. 补单元测试和最小 smoke test

理由是：

- metadata 不先补，后面仍会继续硬编码
- auth store 不先落，登录只是一次性动作
- runtime 不接 auth，闭环仍然是假闭环

## 任务清单

### A. `agent-diva-providers` 元数据最小补齐

#### 目标

让 provider registry 可以回答：

- 这个 provider 是否支持登录
- 登录方式是什么
- 凭据存在配置里还是外部 store
- runtime 应该走哪个 backend

#### 建议修改位置

- `agent-diva-providers/src/providers.yaml`
- `agent-diva-providers/src/registry.rs`
- `agent-diva-providers/src/discovery.rs`

#### 建议新增字段

- `auth_mode = api_key | oauth | token | device_flow`
- `login_supported = bool`
- `credential_store = config | external_secure_store`
- `runtime_backend = openai_compatible | openai_codex`

#### 本阶段最小要求

- `openai-codex`
  - `auth_mode = oauth`
  - `login_supported = true`
  - `credential_store = external_secure_store`
  - `runtime_backend = openai_codex`
- `dashscope` / `qwen`
  - `auth_mode = api_key`
  - `login_supported = false`

#### 完成定义

- CLI 不需要再用“字符串特判”判断 `openai-codex` 是否支持 login
- provider runtime 能通过 metadata 知道是否走专用 backend

### B. `agent-diva-core` 认证数据模型与存储

#### 目标

把 `zeroclaw` 的 `profiles.rs + auth service` 精简迁入 `agent-diva-core`。

#### 建议新增文件

- `agent-diva-core/src/auth/mod.rs`
- `agent-diva-core/src/auth/profiles.rs`
- `agent-diva-core/src/auth/store.rs`
- `agent-diva-core/src/auth/service.rs`

#### 建议新增类型

- `ProviderAuthKind`
- `ProviderTokenSet`
- `ProviderAuthProfile`
- `ProviderAuthProfilesData`
- `ProviderAuthStore`
- `ProviderAuthService`

#### `ProviderTokenSet` 最小字段

- `access_token`
- `refresh_token`
- `id_token`
- `expires_at`
- `token_type`
- `scope`

#### `ProviderAuthProfile` 最小字段

- `id`
- `provider`
- `profile_name`
- `kind`
- `account_id`
- `token_set`
- `token`
- `metadata`
- `created_at`
- `updated_at`

#### `ProviderAuthStore` 最小能力

- `load`
- `upsert_profile`
- `remove_profile`
- `set_active_profile`
- `clear_active_profile`
- `update_profile`

#### `ProviderAuthService` 最小能力

- `store_openai_codex_tokens`
- `get_profile`
- `get_active_profile`
- `get_provider_bearer_token`
- `set_active_profile`
- `remove_profile`
- `load_profiles`

#### 本阶段要求

- token 不进入 `config.json`
- store 文件和主配置分离
- 至少支持文件锁或等价并发保护
- 至少支持 refresh token 落盘

### C. `agent-diva-providers` 的 `openai-codex` 登录处理器

#### 目标

把 `zeroclaw` 的 `openai_oauth` 方案落成 `agent-diva` 可复用 handler，而不是塞在 CLI 中。

#### 建议新增文件

- `agent-diva-providers/src/provider_auth/mod.rs`
- `agent-diva-providers/src/provider_auth/openai_codex.rs`

#### 建议核心接口

```rust
pub struct ProviderLoginRequest {
    pub provider: String,
    pub profile_name: String,
    pub mode: ProviderLoginMode,
}

pub enum ProviderLoginMode {
    Browser,
    DeviceCode,
    PasteRedirect { input: Option<String> },
}

pub struct ProviderLoginResult {
    pub provider: String,
    pub profile_name: String,
    pub account_id: Option<String>,
    pub status: String,
}
```

#### 本阶段最小能力

- 生成 PKCE/state
- 构造 authorize URL
- 支持回调/粘贴 code
- exchange token
- 交给 `agent-diva-core` auth service 持久化

#### 可以后置的内容

- import 旧 auth 文件
- 多 profile UX 优化
- GUI 配套登录入口

### D. `agent-diva-cli` 的 provider auth 命令闭环

#### 目标

让 CLI 只做统一入口与输出，不承载 OAuth 协议细节。

#### 建议修改位置

- `agent-diva-cli/src/provider_commands.rs`
- 如有必要，新建 `agent-diva-cli/src/provider_auth_commands.rs`

#### 本阶段命令面

- `agent-diva provider login openai-codex`
- `agent-diva provider status`
- `agent-diva provider status openai-codex`
- `agent-diva provider logout openai-codex`
- `agent-diva provider use openai-codex <profile>`
- `agent-diva provider refresh openai-codex`

#### CLI 层只负责

1. 参数解析
2. 调 provider auth service / login handler
3. pretty/json 输出

#### CLI 层不应负责

- 拼 OAuth URL
- 直接写 token 文件
- provider-specific HTTP 调用

#### 完成定义

- 当前的 placeholder 被移除
- 不支持 login 的 provider 会给出基于 metadata 的明确错误

### E. `openai-codex` runtime backend 接入认证

#### 目标

保证登录成功后，实际 provider runtime 真能用这份 token。

#### 建议修改位置

- `agent-diva-providers/src/backends/openai_codex.rs`
- 或 `agent-diva-providers/src/providers/openai_codex.rs`
- `agent-diva-providers/src/registry.rs`

#### 本阶段需要做到

- `openai-codex` 走独立 backend
- backend 在请求前通过 auth service 获取 bearer token
- 若 token 缺失，返回明确未登录错误
- 若 token 过期且存在 refresh token，支持最小自动 refresh 或至少返回明确刷新失败语义

#### 不要做的错误实现

- 不要把 `openai-codex` 塞进现有通用 OpenAI-compatible provider 路由
- 不要把 OAuth token 写回 provider config

### F. `provider status` 的状态面整理

#### 目标

让用户能看到：

- 哪个 provider 支持 login
- 当前 active profile 是什么
- 是否已登录
- token 是否可用

#### 建议修改位置

- `agent-diva-cli/src/provider_commands.rs`
- `agent-diva-core/src/auth/service.rs`
- `agent-diva-providers/src/registry.rs`

#### 状态面最小字段

- `provider`
- `auth_mode`
- `login_supported`
- `active_profile`
- `authenticated`
- `expires_at`

#### 完成定义

- 登录前 `provider status openai-codex` 明确显示未登录
- 登录后能显示 active profile 和认证可用状态

## 按 crate 的交付清单

### `agent-diva-core`

#### 交付项

- 新建 auth 模块
- 完成 auth store
- 完成 auth service
- 输出供 CLI/provider runtime 复用的稳定接口

#### 必做测试

- `upsert/load/remove profile`
- `set_active_profile`
- `OAuth profile` 与 `Token profile` 反序列化
- token 为空/损坏时的错误路径
- 并发写入或锁机制的最小覆盖

### `agent-diva-providers`

#### 交付项

- provider metadata 扩展
- `openai-codex` auth handler
- `openai-codex` runtime backend

#### 必做测试

- registry 能识别 `openai-codex.login_supported = true`
- login handler 对 code/token 交换成功路径
- login handler 对失败响应路径
- runtime 在无 token 时返回未登录错误
- runtime 在有 token 时能构造正确认证请求

### `agent-diva-cli`

#### 交付项

- `provider login`
- `provider status`
- `provider logout`
- `provider use`
- `provider refresh`

#### 必做测试

- 命令参数解析
- JSON 输出结构
- 对“不支持 login 的 provider”的错误提示
- 对“未登录但调用 refresh/status”的边界行为

## 最小 Smoke Test 清单

根据仓库规则，`provider login` 属于用户可见行为，必须补最小 smoke test。

建议最小 smoke 路径如下：

1. `agent-diva provider status openai-codex`
   - 预期：显示支持 OAuth，但当前未登录
2. `agent-diva provider login openai-codex`
   - 预期：进入真实授权流程，而不是 `not_implemented`
3. 完成登录后执行 `agent-diva provider status openai-codex`
   - 预期：显示已认证/active profile
4. 若 runtime 已接通，再执行最小 provider 调用或 `provider test`
   - 预期：能够使用 OAuth token 发起请求

如果自动化里无法跑真实 OAuth，至少要补：

- fake auth service + fake login handler 的 CLI 流程测试
- 文档中记录真实手工 smoke 路径

## 推荐测试清单

### 单元测试

- `agent-diva-core` auth model/store/service
- `agent-diva-providers` login handler
- `agent-diva-providers` runtime backend
- `agent-diva-cli` command routing

### 集成测试

- `provider login -> store profile -> provider status`
- `provider use -> runtime token resolution`

### 手工验证

- 本地真实 `openai-codex` 登录一次
- 重启 CLI 后状态仍可读
- active profile 仍可解析

## 风险与约束

### 风险 1：CLI 又长出 provider-specific 硬编码

如果图省事直接在 `provider_commands.rs` 里拼完 OAuth 流程，本阶段虽然能跑，但第二个 provider 一来就会继续膨胀。

### 风险 2：runtime 仍走 API key 假设

如果不单独拆 `openai-codex` backend，就会把 OAuth provider 错塞进 API-key 路径，后续很容易破坏 provider model-id safety。

### 风险 3：metadata 不先补，后面还是返工

如果不先把 `auth_mode/login_supported/credential_store/runtime_backend` 上提，`provider login`、`provider status`、`provider models` 最终还是会各写一套判断。

## 建议任务切片

如果要拆成 3 个实际开发 PR，建议这样切：

### PR 1

- `agent-diva-core` auth store/service
- `agent-diva-providers` metadata 最小扩展

### PR 2

- `agent-diva-providers` `openai-codex` login handler
- `agent-diva-cli` `provider login/status/logout/use/refresh`

### PR 3

- `openai-codex` runtime backend 接 auth
- 集成测试 + smoke test 记录

这样切的好处是：

- 每个 PR 都有明确边界
- 不会把 Phase 1 做成超大补丁
- 第一批评审就能先把 auth data model 定下来

## Phase 1 完成口径

当且仅当以下条件同时成立，Phase 1 才算完成：

1. `provider login openai-codex` 可真实执行
2. token/profile 不进入主配置文件
3. `provider status openai-codex` 可显示认证状态
4. `openai-codex` runtime 能消费该认证状态
5. CLI 不再使用 placeholder 文案
6. 至少有单元测试 + 最小 smoke test 记录

如果只完成了登录命令而 runtime 还不能消费 token，那么最多算“半闭环”，不应宣称已经达到 nanobot 水准。
