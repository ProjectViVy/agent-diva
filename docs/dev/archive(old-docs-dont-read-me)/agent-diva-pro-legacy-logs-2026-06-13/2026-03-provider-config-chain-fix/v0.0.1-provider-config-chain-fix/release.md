# Release

- No special deployment or migration step is required beyond shipping the updated GUI, manager, CLI, and provider crates together.
- This change is backward compatible with existing config files because unsupported builtin provider credentials are stored through `providers.custom_providers` shadow entries instead of requiring a schema-breaking config rewrite.
