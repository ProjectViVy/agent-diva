# Release

## Method

1. Build and publish the updated `agent-diva-cli` binary.
2. Update user-facing documentation to recommend `--config` for explicit instance targeting.
3. Existing users can continue using `--config-dir`; no config migration is required.

## Deployment Notes

- No schema-breaking config changes were introduced.
- Runtime directories are now derived from the selected config file parent when `--config` is used.
- JSON consumers should switch to `status --json`, `channels status --json`, and `config doctor --json` as needed.
