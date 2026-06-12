# GUI Provider Dedupe Bridge Summary

- Stopped the Tauri `get_providers` command from proxying the running manager `/api/providers` response.
- Switched the desktop bridge to build provider views directly from the local config and `ProviderCatalogService`, avoiding stale duplicate builtin/custom entries from an older running backend.
- Added a frontend defensive dedupe step in the providers settings page keyed by `provider.name`, preferring builtin entries when duplicates still appear upstream.
- This specifically addresses repeated entries such as multiple `Mimo` rows even when the local config only contains one builtin provider plus one legacy shadow config entry.
