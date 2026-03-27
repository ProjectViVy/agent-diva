# Summary

- 新增 `qwen-login` 作为第二个真实 OAuth provider，用于验证 `agent-diva` 的 provider-auth 框架不是 `openai-codex` 特例。
- `qwen-login` provider metadata 已接入 builtin provider catalog，采用独立 canonical id，与 `dashscope` 的 API Key 路径分离。
- 在 `agent-diva-providers::provider_auth::backends` 中新增 `QwenLoginOAuthBackend`，支持浏览器 PKCE 与 paste redirect，明确拒绝 device-code。
- 通用 auth profile 继续沿用现有 schema，但在运行期通过 profile metadata 持有 `resource_url` / `api_base`，供 Qwen OpenAI-compatible runtime 注入 bearer 与动态 base URL。
- CLI、manager、GUI Tauri 命令、provider model discovery 已统一走 `resolve_openai_compatible_oauth_access(...)`，使 `qwen-login` 可通过 auth store 直接驱动 OpenAI-compatible runtime，无需配置 API key。
- 用户文档已补充 `qwen-login` 的登录、刷新、状态查询示例，并明确 `dashscope = API Key`、`qwen-login = Qwen Code OAuth` 的边界。
