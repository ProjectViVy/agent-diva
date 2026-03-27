# Acceptance

建议按以下步骤验收：

1. 执行 `agent-diva provider status qwen-login --json`，确认 provider 列表中存在 `qwen-login`，且其 `auth_mode` 为 `oauth`。
2. 执行 `agent-diva provider login qwen-login` 或 `agent-diva provider login qwen-login --paste-code '<redirect-url>'`，确认登录后 auth store 中生成 `qwen-login:<profile>` profile。
3. 执行 `agent-diva provider refresh qwen-login --profile <profile>`，确认 refresh 路径可工作且不会要求 API key。
4. 将默认 provider 显式切到 `qwen-login` 后，使用其默认模型进行一次 OpenAI-compatible 调用，确认 bearer token 与 `api_base` 来自 auth store metadata，而不是 `config.providers.qwen_login.api_key`。
5. 执行 `agent-diva provider status dashscope --json`，确认 `dashscope` 仍保持 API Key provider 语义，未被 `qwen-login` 的 OAuth 路径覆盖。
6. 在 GUI provider settings 页面查看 `qwen-login`，确认已出现与 `openai-codex` 同级的 OAuth 状态、浏览器登录、paste redirect、refresh、logout、profile 切换入口；GUI device-code 仍应明确不可用。
