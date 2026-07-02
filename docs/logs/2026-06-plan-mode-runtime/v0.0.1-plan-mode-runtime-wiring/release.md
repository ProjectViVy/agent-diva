# Plan Mode Runtime Wiring Release

## Deployment

No separate data migration is required. The planning database is opened under the workspace-local `.agent-diva/planning.db` path and initialized on demand.

## Notes

Operators should restart the CLI/Manager/GUI process after upgrading so the new ToolConfig wiring and Tauri commands are loaded.

