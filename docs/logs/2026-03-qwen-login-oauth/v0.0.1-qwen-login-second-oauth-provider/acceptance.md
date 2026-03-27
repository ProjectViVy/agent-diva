# Acceptance

建议按以下步骤验收：

1. 执行 `agent-diva provider status qwen-login --json`，确认 provider 列表中存在 `qwen-login`，且其 `auth_mode = oauth`、`login_supported = true`、`runtime_backend = openai_compatible`。
2. 执行 `agent-diva provider login qwen-login` 或 `agent-diva provider login qwen-login --paste-code '<redirect-url>'`，确认登录后 auth store 中出现 `qwen-login:<profile>` profile，且 metadata 中带有 `api_base`。
3. 执行 `agent-diva provider refresh qwen-login --profile <profile>`，确认 refresh 路径可工作，不会要求 API key，且刷新后 `metadata.api_base` 与 refresh token 仍被保留/更新。
4. 执行 `agent-diva provider use qwen-login --profile <profile>`，随后以 `qwen-login` 默认模型发起一次真实 OpenAI-compatible 调用，确认实际请求使用的是 auth store 中的 bearer token 与 `metadata.api_base`，而不是 `config.providers.qwen_login.api_key`。
5. 执行 `agent-diva provider status dashscope --json`，确认 `dashscope` 仍保持 API Key provider 语义，未被 `qwen-login` 的 OAuth 路径覆盖。
6. 在 GUI provider settings 页面查看 `qwen-login`，确认已出现与 `openai-codex` 同级的 OAuth 状态、浏览器登录、paste redirect、refresh、logout、profile 切换入口。
7. 在 GUI 中对 `qwen-login` 尝试 device-code 登录，确认收到明确的不支持提示，而不是错误落入 OpenAI 分支。
