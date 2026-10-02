# Release notes — DN-2 v0.1.0

Branch: `DIVA-NEXT-P0` (local, unpushed).

## User-facing changes (when run against the new Tauri shell)

- Chat send/stop/history/session-switch now execute real VIVY
  session/turn/run methods through the sealed shared library —
  no mock responses, no duplicate local orchestration.
- Approval and question cards bind backend review ids; backend
  rejections and unknown-outcome states render in the drawer.
- Plan approval flows through `session/work` + `plan/decide`
  (execute once / revise with feedback); goal resume/pause map onto
  `goal/resume` / `goal/pause`.
- The settings "compact now" button calls `context/compact`.
- Session rename/deletion go through `session/rename` /
  `session/delete`.

## Known gaps (recorded, not faked)

- Attachments are dropped on send (VIVY `turn/start` is text-only).
- Permission modes on send are not yet wired.
- Regenerate resends the last user turn (no rewind yet).
- Session titles come from the backend only; GUI auto-generation
  removed.
- `plan/decide start_goal` has no UI trigger yet.
- Config/persona/settings invokes (`load_config`, `getRuntimeConfig`,
  `get_tools_config`, `save_config`, `update_*`, `check_health`,
  `reset_session`, `open_desktop_pet`) remain dead seams for DN-3/4.
