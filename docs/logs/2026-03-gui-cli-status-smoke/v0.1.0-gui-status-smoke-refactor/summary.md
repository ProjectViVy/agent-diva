# Summary

- Added reusable `agent-diva-cli` library modules for runtime/status inspection, provider commands, and chat commands.
- Wired the GUI Tauri backend to a new `get_config_status` command that reuses CLI status logic instead of shelling out.
- Updated GUI settings surfaces to show doctor health, resolved paths, channel readiness, and provider readiness/current provider/current model.
- Added CLI smoke tests for direct `agent` invocation and lightweight `chat --help`.
