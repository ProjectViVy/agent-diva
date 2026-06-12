# Release

- No migration is required.
- The fix is backward compatible: unsupported providers still fall back to static catalogs, but OpenAI-compatible providers with usable `api_base` now try runtime discovery first.
