# Mimo Dedup And Cache Clear Summary

- Fixed provider listing so shadow/custom config entries no longer create duplicate rows when their ID matches a builtin provider such as `mimo`.
- Added a regression test to lock the dedup behavior in `ProviderCatalogService::list_provider_views`.
- Added a `General` page UI cache card with a one-click clear action for saved models, chat display preferences, and cached session history.
- Kept cache clearing scoped to GUI `localStorage`; it does not remove config files or provider credentials.
