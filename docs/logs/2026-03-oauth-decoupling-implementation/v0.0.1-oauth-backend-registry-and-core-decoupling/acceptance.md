# Acceptance

1. `agent-diva-core` 不再直接持有 OpenAI OAuth token endpoint/client-id/refresh 实现。
2. `ProviderLoginService` 不再硬编码 `match "openai-codex"`。
3. CLI `provider login` / `provider refresh` 通过统一 backend registry 驱动。
4. GUI Tauri `login_provider` / `get_provider_login_status` / `refresh_provider_auth` 通过统一 backend registry 驱动，且前端 API shape 不变。
5. `agent-diva-providers/src/backends/openai_codex.rs` 改为使用通用 `ProviderAuthService` OAuth token 接口。
