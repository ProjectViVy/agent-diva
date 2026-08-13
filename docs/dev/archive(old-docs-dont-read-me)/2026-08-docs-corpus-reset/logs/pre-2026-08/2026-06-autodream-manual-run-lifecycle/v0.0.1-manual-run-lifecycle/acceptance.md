# Story 3.1 Acceptance

## Product Checks

- Trigger a manual AutoDream run through `POST /api/autodream/runs` or `trigger_autodream`.
- Confirm a run ID is returned and `.agent-diva/autodream/lock` plus `runs/{run_id}/record.json` are created.
- Query status through `GET /api/autodream/runs/{id}` or `get_autodream_run_status`.
- Cancel through `POST /api/autodream/runs/{id}/cancel` or `cancel_autodream_run` and confirm the run reaches `cancelled`.
- Confirm list reads through `GET /api/autodream/runs` or `list_autodream_run_records`.
- Confirm checkpoint defaults keep `auto_mode_enabled=false` and `session_threshold_enabled=false`.

## Current Status

Implementation is ready for review. Manager route smoke passed; full GUI acceptance smoke remains blocked by pre-existing `agent-diva-sandbox` compile issues recorded in `TODOLIST.md`.
