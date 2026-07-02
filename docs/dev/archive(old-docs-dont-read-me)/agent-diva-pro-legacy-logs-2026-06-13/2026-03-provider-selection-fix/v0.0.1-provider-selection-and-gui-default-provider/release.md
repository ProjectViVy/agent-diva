# Release

- No special deployment procedure is required beyond the standard build/release path for this workspace.
- The change is backward compatible for existing configs without `agents.defaults.provider`.
  - When the field is missing, provider resolution still falls back to existing inference paths.
- Operators should expect newly saved configs to begin persisting `agents.defaults.provider`.
