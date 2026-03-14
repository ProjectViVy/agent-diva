# Summary

- Added runtime provider model discovery in `agent-diva-providers` with a shared catalog DTO and OpenAI-compatible `/models` fetching logic.
- Added `agent-diva provider models --provider <name> [--static-fallback] [--json]` for explicit CLI access to runtime or fallback model catalogs.
- Added manager HTTP endpoint `GET /api/providers/:name/models` and Tauri command `get_provider_models` for GUI consumption.
- Updated the GUI Providers settings page to fetch live model catalogs on demand, surface fallback/error state, and prefer runtime results over bundled static metadata.
- Kept `providers.yaml` static `models` as the offline fallback catalog; runtime discovery does not rewrite config or bundled metadata.

# Impact

- Users can now inspect provider-specific model catalogs online instead of relying only on bundled metadata.
- Unsupported providers return structured `unsupported` or `static_fallback` results rather than silently failing.
- Existing provider selection behavior remains backward compatible.
