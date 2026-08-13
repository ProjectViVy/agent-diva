# Release

- **Ship with:** rebuilt `agent-diva-gui` (Tauri) and gateway that includes updated `agent-diva-core`.
- **Requires restart:** yes for both GUI and gateway.
- **Migration:** none.
- **Rollback:** revert GUI markdown preservation + Tauri normalize-before-hash if approve unexpectedly accepts mutated bodies (unlikely; normalize is idempotent on stored plans).
