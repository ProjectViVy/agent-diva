# Acceptance

Date: 2026-06-17

1. Run a legacy migration against a workspace containing existing `.laputa/state.json` fields and confirm those fields remain present after migration.
2. Place legacy `MEMORY.md` and `HISTORY.md` at the workspace root and confirm migration backs them up and maps them into Laputa sections.
3. Confirm `BOOTSTRAP.md` is no longer injected into the runtime system prompt after migration.
4. Confirm Laputa snapshot and section reads report the migrated schema version from `.laputa/state.json`.
