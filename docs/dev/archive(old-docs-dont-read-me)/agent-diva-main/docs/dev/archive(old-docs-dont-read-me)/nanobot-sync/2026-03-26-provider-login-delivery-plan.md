# Provider Login 落地调研（OpenAI Codex 优先）

## 背景

当前 `agent-diva` 已公开暴露 `agent-diva provider login <provider>` 命令，但实现仍是占位：

- `agent-diva-cli/src/provider_commands.rs`
- `docs/user-guide/commands.md`
- `docs/userguide.md`
- `.workspace/agent-diva-docs/content/docs/cli/index.md`

这形成了明显的产品断层：文档宣称存在 Provider OAuth 登录能力，CLI 入口也存在，但用户执行后只能得到 `not_implemented`。

本调研只基于当前仓库与以下参考工程：

- `.workspace/nanobot`
- `.workspace/codex`

不包含代码实现，只给出落地方案、边界与迭代建议。

## 现状结论

### 1. `agent-diva` 当前状态

`run_provider_login()` 当前直接返回占位结果：

- `status = "not_implemented"`
- message 明确提示“implement OAuth/device flow per provider later”

同时文档已将该命令表述为正式 CLI 能力，且示例中已经出现 “OAuth 登录（如 openai-codex）”。因此当前优先级不是“继续补文档”，而是补齐最小真实闭环，并同步收敛文档措辞。

### 2. `nanobot` 的可参考做法

`nanobot` 已完成一个可工作的最小闭环，关键点有三层：

1. Provider Registry 层标记 `is_oauth`
2. CLI `provider login` 根据 registry 分发到 provider-specific handler
3. Provider 运行时不依赖 `api_key`，而是直接从外部 OAuth 凭据存储读取 token

对应证据：

- `.workspace/nanobot/nanobot/providers/registry.py`
- `.workspace/nanobot/nanobot/cli/commands.py`
- `.workspace/nanobot/nanobot/providers/openai_codex_provider.py`
- `.workspace/nanobot/README.md`

这个设计的优点是：

- CLI 层只负责统一入口，不在主命令里堆所有 provider 细节
- OAuth provider 与 API-key provider 能在 registry 层被显式区分
- 配置文件不必存放 OAuth access token
- provider runtime 可以独立完成 token 获取与刷新

### 3. `.workspace/codex` 的可参考点

`codex` 仓库里没有现成的 “provider login openai-codex” CLI 参考，但 Rust 侧已经沉淀了一套 OAuth 基础抽象，主要体现在 MCP OAuth 上：

- 先判断目标是否支持 OAuth
- 支持 discovery / callback / scopes / store mode
- 将“是否支持 OAuth”和“凭据存储方式”抽象为稳定配置

对应证据：

- `.workspace/codex/codex-rs/core/src/mcp/auth.rs`
- `.workspace/codex/docs/authentication.md`

它给 `agent-diva` 的启发不是直接复用某段现成 provider 登录代码，而是：

- Rust 侧应尽早把 “OAuth 支持能力”、“凭据存储”、“回调端口/URL”、“scope 来源” 设计成基础设施，而不是把登录逻辑硬编码在单个 provider 命令里
- 即使 Phase 1 只先做 `openai-codex`，结构上也要允许未来追加更多 OAuth / device-flow provider

## 优先级判断

### P0：`openai-codex`

这是最适合优先补齐的 Provider，原因如下：

1. 文档里已经点名它是 `provider login` 的代表例子
2. `nanobot` 已有真实闭环，迁移成本最低
3. 它天然是 OAuth provider，不适合继续伪装成普通 API-key provider
4. 补齐后能直接消除“表面有能力，实际没闭环”的最明显缺口

### P1：`github-copilot` 或同类 OAuth provider

如果 Phase 1 抽象到位，下一步最自然的是第二个 OAuth provider，而不是立刻扩展很多 API-key provider。这样可以验证通用机制是否成立。

### `qwen` 是否应进入本轮

结论：**当前不建议把 `qwen` 纳入本轮 `provider login` 首批实现。**

原因：

1. 当前 `agent-diva` 内部对 Qwen 的建模是 `dashscope` / OpenAI-compatible API key provider，而不是 OAuth provider
2. `.workspace/nanobot` 中 Qwen 也是 `dashscope`，走 API key，不走 OAuth
3. 本仓库当前没有任何 `qwen` OAuth 登录入口、registry 标记或凭据落盘结构
4. 若未来要支持“Qwen Portal / 订阅态 / OAuth”之类能力，更合理的做法是新增一个独立 provider 类型，而不是直接把现有 `dashscope` provider 混成双模式

因此，本轮建议：

- `qwen` 继续保持 API key provider
- 在方案文档中预留“未来如需 Qwen OAuth，应作为新 provider spec 或新 auth mode 进入”的扩展口
- 不要为了“顺手一起做”而破坏 `dashscope` 当前清晰的 API key 语义

## 建议目标

### 产品目标

交付一个真实可用的最小闭环，使以下用户路径成立：

1. 用户执行 `agent-diva provider login openai-codex`
2. CLI 完成交互式 OAuth / device flow
3. OAuth 凭据保存在 config 之外的安全位置
4. 用户执行 `agent-diva provider set --provider openai-codex --model openai-codex/<model>`
5. `provider status` 能识别该 provider 已具备可用认证
6. `provider models` 与实际聊天路径能使用该认证

### 非目标

本期不建议同时做以下事项：

- 把所有 provider 都改造成统一 OAuth
- 在 `config.json` 中保存 access token / refresh token
- 把 `dashscope` 和未来可能存在的 Qwen OAuth 混为一个 provider
- 先做 GUI 登录而 CLI 仍不可用

## 建议架构

### 1. Registry 层增加认证模式

当前 provider registry 更偏向“模型路由 + API 元数据”。要支持真实登录，建议在 `agent-diva-providers` 的 provider metadata 中显式加入认证维度。

建议新增类似能力：

- `auth_mode = api_key | oauth | device_flow | local`
- `login_supported = true | false`
- `credential_store = config | external_secure_store`

对 `openai-codex`，建议定义为：

- `auth_mode = oauth`
- `login_supported = true`
- `credential_store = external_secure_store`

对 `dashscope`，保持：

- `auth_mode = api_key`
- `login_supported = false`

这样 CLI 才能基于 provider metadata 决定：

- 是否允许 `provider login`
- 错误提示是“不支持登录”还是“支持但未实现”

### 2. CLI 层改为“统一入口 + provider handler”

建议不要把 OAuth 逻辑全部写进 `run_provider_login()`。更合理的方式是：

- CLI 只做 provider 参数解析、JSON 输出、错误边界
- 真正的登录逻辑下沉到 `agent-diva-providers` 或 `agent-diva-core` 的 auth/login 子模块

建议结构：

- `agent-diva-cli`
  - 统一路由
  - 终端交互适配
  - JSON / pretty 输出
- `agent-diva-providers`
  - provider auth spec
  - provider login handler registry
  - runtime token access adapter
- `agent-diva-core`
  - 凭据存储抽象
  - 路径解析
  - token metadata / status model

这样后续再补第二个 OAuth provider 时，不需要重复改 CLI 主流程。

### 3. 凭据存储必须与配置文件解耦

参考 `nanobot`，建议 `openai-codex` 不在 `config.json` 中写入 token。

建议原则：

- `config.json` 只表达“默认 provider / 默认 model / 非敏感 provider 配置”
- OAuth token 存外部凭据存储
- `provider status` 只展示是否存在有效认证，不打印敏感值

Rust 侧可分阶段：

#### Phase 1

文件型凭据存储，落在 agent-diva runtime root 下的独立 auth 目录，至少做到：

- 与 `config.json` 分离
- 权限收紧
- 可记录 `provider`, `account_id`, `expires_at`, `refreshable`

#### Phase 2

按平台接系统级密钥存储：

- macOS Keychain
- Windows Credential Manager
- Linux Secret Service / fallback file store

如果当前迭代只求闭环，先做 Phase 1 即可，但接口必须可替换。

### 4. 运行时 Provider 调用不能再假设只有 API key

这是落地时最容易漏掉的一层。

当前 provider 体系大量逻辑默认建立在：

- `api_key`
- `api_base`
- LiteLLM prefix / OpenAI-compatible forwarding

但 `openai-codex` 不是这个模式。参考 `nanobot`，它需要的是：

- 专用 base URL
- OAuth access token
- 额外 header，例如 account id
- 请求体与常规 OpenAI-compatible 接口并不完全一致

因此建议不要把 `openai-codex` 硬塞进现有 LiteLLM / 通用 OpenAI-compatible provider 路径，而是：

- 为其增加独立 provider backend
- 允许 runtime 从 auth store 解析 token 与相关 metadata
- 把 model prefix 处理、headers、endpoint shape 明确收口在专用 backend 中

这比“先伪装成 openai 兼容 provider 再补丁式覆盖”更稳。

## 推荐实施阶段

### Phase 0：文档止血

在真正实现前，先把用户文档收敛为事实描述，避免继续扩大认知偏差。

建议动作：

- 把现有文档中的“已支持 OAuth 登录（如 openai-codex）”改成“接口已预留，`openai-codex` 为首批落地目标”
- 若短期内马上进入开发，可把文档改为“实验中 / 规划中”

如果实现会很快跟上，也可以不单独做此 phase，而是与 Phase 1 一起提交。

### Phase 1：只落 `openai-codex`

范围：

- provider registry 补认证模式
- CLI `provider login openai-codex`
- 独立 auth store
- `provider status` 识别 OAuth 就绪状态
- `provider set` 接受 `openai-codex`
- runtime provider 真正可调用

验收：

- 登录一次后可复用
- token 过期前可直接调用
- 用户无需编辑 `providers.openaiCodex` 配置块

### Phase 2：抽象为通用 OAuth provider 框架

范围：

- provider login handler registry
- auth store trait
- token status / expiry / refresh metadata
- JSON 输出统一 schema

验收：

- 新增第二个 OAuth provider 时无需再改 CLI 主路由

### Phase 3：评估下一批 provider

优先顺序建议：

1. `github-copilot`
2. 其他真正需要 OAuth / device-flow 的 provider
3. 如产品确认有明确需求，再单独评估 `qwen` OAuth 方案

## 建议命令契约

### `agent-diva provider login <provider>`

建议输出语义：

- 成功：`status=authenticated`
- 已存在有效认证：`status=already_authenticated`
- provider 支持登录但当前平台/依赖缺失：`status=blocked`
- provider 不支持登录：`status=unsupported`
- 登录失败：`status=failed`

建议 JSON 字段：

- `provider`
- `status`
- `message`
- `auth_mode`
- `account_id` 或 `account_label`（可选，脱敏）
- `expires_at`（可选）
- `reused_existing_session`

### `agent-diva provider status`

建议补充字段：

- `auth_mode`
- `configured`
- `authenticated`
- `credential_store`
- `expires_at`（可选）
- `missing_fields`

对于 OAuth provider：

- 不应把“没有 api_key”视为缺字段
- readiness 规则应改为“存在有效认证”而不是“存在 api_key”

## 风险与注意事项

### 1. 不要把 OAuth provider 伪装成普通 API-key provider

否则会出现：

- 状态检查误报
- model routing 勉强可配但运行时失败
- 文档和实现再次错位

### 2. 不要把 token 落进现有用户配置

否则会带来：

- secrets 泄漏风险
- 配置导出/打印时的脱敏负担
- 后续迁移到系统密钥存储时的兼容成本

### 3. `provider set` 与 `provider login` 必须解耦

推荐行为：

- `login` 只完成认证
- `set` 只完成默认 provider / model 切换

这样用户可以：

- 先登录后切换
- 先切换后补登录

### 4. `qwen` 不要提前混入 Phase 1

当前证据不足以支持“现有 Qwen provider 也应做 OAuth”。如果产品后来要支持，建议建模为：

- 新 provider spec
或
- 在 provider metadata 上新增 second auth profile，而不是直接修改 `dashscope` 现有语义

## 建议产出物

如果进入正式开发，建议下一迭代至少包含：

1. Provider OAuth 架构设计文档
2. `openai-codex` 登录流时序图
3. Auth store 目录与数据结构说明
4. CLI 契约更新
5. 测试矩阵

建议最小测试矩阵：

- `provider login openai-codex`
- `provider status`
- `provider set --provider openai-codex`
- runtime chat happy path
- 无凭据状态
- 凭据失效状态
- JSON 输出兼容性

## 最终建议

建议结论如下：

1. 立刻把 `openai-codex` 定义为首个真实落地的 OAuth provider
2. 实现时参考 `nanobot` 的最小闭环，但不要照搬 Python 依赖式做法，应结合 Rust 侧长期结构一次把 registry、auth store、runtime backend 分层理顺
3. 用 `.workspace/codex` 的 OAuth 基础设施思路约束抽象边界，避免只做一次性脚本式登录
4. `qwen` 当前不进入首批 `provider login`，维持 API key provider 语义；若未来要补，作为独立需求单独建模

如果只允许做一个方案方向，推荐方向是：

**先用最小成本补齐 `openai-codex` CLI OAuth 闭环，同时把 Rust 侧 provider auth 基础抽象一次立住。**
