# Summary

- Added a config-centric CLI path for `agent-diva` around explicit instance selection and runtime path resolution.
- Introduced global `--config` plus runtime `--workspace` override while preserving `--config-dir`.
- Added `config path|refresh|validate|doctor|show|init` and enhanced `onboard`, `status`, `channels status`, `cron`, and WhatsApp bridge path routing.
- Added redacted config output and JSON-safe structured output for automation.

# Impact

- Users can now manage multiple local instances by targeting a specific `config.json`.
- Status and doctor output are now suitable for scripts and GUI integration.
- Config refresh/onboard preserves existing values and syncs workspace templates instead of forcing destructive overwrite.
