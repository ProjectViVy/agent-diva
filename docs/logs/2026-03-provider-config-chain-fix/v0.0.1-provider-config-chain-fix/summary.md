# Provider Config Chain Fix Summary

- Fixed the provider configuration chain so GUI-visible providers can persist credentials even when they do not have a fixed schema slot.
- Normalized provider/model selection during config updates to prevent invalid cross-provider pairs such as `openai + deepseek-chat`.
- Unified startup/runtime config loading in the GUI so the settings page no longer falls back to stale DeepSeek state after provider changes.
- Added a Tauri `get_config` command and refreshed provider status after config saves to keep runtime state and status cards aligned.
- Preserved CLI JSON compatibility for `provider models --json` while keeping the newer catalog structure available.
