# Summary

- Fixed GUI provider model refresh so it uses the provider currently selected in the UI instead of being polluted by stale provider config stored on disk.
- Extended the Tauri `get_provider_models` command to accept optional `api_base` and `api_key` overrides from the GUI.
- Updated the providers settings screen to pass the selected provider's default API base and current API key when refreshing online models.
- Impact: selecting `deepseek` in GUI no longer reuses an old Xiaomi Mimo API base during runtime model discovery.
