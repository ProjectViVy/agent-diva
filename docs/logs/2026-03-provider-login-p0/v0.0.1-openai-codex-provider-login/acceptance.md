# Acceptance

建议按以下路径验收：

1. 执行 `agent-diva provider status openai-codex --json`，确认显示 `auth_mode=oauth`、`login_supported=true`。
2. 执行 `agent-diva provider login openai-codex`，确认不再输出 placeholder，而是进入真实 OAuth 流程。
3. 登录完成后执行 `agent-diva provider status openai-codex`，确认出现 `active_profile`、`authenticated=true`。
4. 执行 `agent-diva provider refresh openai-codex`，确认已登录 profile 可被 refresh。
5. 将默认模型切到 `gpt-5-codex`/`gpt-5.1-codex` 后运行最小 agent 请求，确认 runtime 会消费 auth store 中的 bearer token。
