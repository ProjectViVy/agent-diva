# Summary

- 将 `agent-diva` 的 OAuth 能力从 `openai-codex` 特例逻辑重构为通用 provider auth 框架。
- 在 `agent-diva-core::auth` 中新增公共 OAuth 工具与通用 pending login store。
- 将 `ProviderAuthService` 改为 provider-agnostic 的 OAuth token 存储、刷新和读取接口。
- 在 `agent-diva-providers::provider_auth` 中新增 OAuth backend trait、registry，以及 `openai-codex` backend 实现。
- CLI、GUI Tauri 命令和 Codex runtime 已改为通过统一 backend registry 消费 OAuth 能力。
