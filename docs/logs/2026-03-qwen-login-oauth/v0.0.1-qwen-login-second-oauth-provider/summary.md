# Summary

- 本次为“恢复后补线修复”：将仓库中已存在但未接线完成的 `qwen-login` OAuth provider 补成真正可用闭环，而不是继续沿用此前日志中“已完成”的不实状态。
- `qwen-login` 已正式加入 builtin provider registry，元数据声明为 `auth_mode = oauth`、`login_supported = true`、`runtime_backend = openai_compatible`，默认 `api_base` 回退到 `https://dashscope.aliyuncs.com/compatible-mode/v1`，并与 `dashscope` 的 API Key 路径保持严格分离。
- `agent-diva-core::auth` 已补出 `oauth_common` 与通用 OAuth 刷新抽象，`ProviderAuthService` 不再只会刷新 `openai-codex`，现在可按 provider + backend 通用刷新并回写 `token_set`、`account_id`、`metadata.api_base`。
- `agent-diva-providers::provider_auth` 已切到 `backends/*` 单一实现源，建立 backend registry，把 `openai-codex` 与 `qwen-login` 统一纳入登录模式判断、浏览器登录、paste redirect、device-code 拒绝和 refresh 分发。
- CLI `provider login/status/refresh/use` 与 GUI Tauri `login_provider/refresh_provider_auth/logout/use_provider_profile` 已去掉只认 `openai-codex` 的硬编码；`qwen-login` 现在可浏览器登录、paste redirect、refresh、logout、profile 切换，GUI device-code 会返回 provider-specific 的明确不支持提示。
- CLI runtime 与 manager runtime 的 provider 构建入口现已统一先走 `resolve_openai_compatible_oauth_access(...)`，因此把默认 provider 切到 `qwen-login` 后，实际 OpenAI-compatible 调用会优先使用 auth store 中的 bearer token 和 `metadata.api_base`，而不是回退到 `config.providers.qwen_login.api_key`。
- 已补测试覆盖：provider registry 可发现 `qwen-login`、`ProviderLoginService` 可分发到 `qwen-login`、`qwen-login` refresh 保留 `api_base` / refresh token、`resolve_openai_compatible_oauth_access("qwen-login")` 优先读取 auth store、CLI `provider status qwen-login --json` smoke、GUI `qwen-login` browser / device-code 命令级 smoke。
