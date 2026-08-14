# I1-S2 Persona Home — Summary

## Outcome

Implemented the config-rooted, machine-wide Persona Markdown authority described by D1.

- Added seven registered Persona kinds under `{config_dir}/persona` with five-file first-run initialization, incomplete repair, Unicode-aware caps, CAS heads, immutable snapshots/diffs, request states, and WORLD protections.
- Rebuilt Frozen Core from six Markdown projections and excluded WORLD. Removed prompt-based first-run onboarding and legacy JSON fallback.
- Added Manager `/api/persona` routes, Tauri mirrors, chat readiness admission, and config-root propagation.
- Added `world_read` as CORE and `persona_read`, `persona_request`, `persona_update` as DEFERRED; subagents receive none. Legacy `laputa_propose_section_write` is no longer registered.
- Replaced the product Persona page with a seven-file CodeMirror 6 workspace: current source/preview, pending review, history/diff, and direct CAS saves.
- Disconnected AutoDream's legacy persona-targeting proposal categories. New AutoDream Persona timing remains outside S2.

## Commits

- `a05e1d8c feat: add persona markdown authority kernel`
- `e8676eb8 feat: wire persona authority into runtime`
- `93f4b554 feat: add persona markdown workspace`
- `352cd57f fix: enforce bounded world proposal claims`
- `520de696 test: migrate context invariants to persona markdown`
- `1e46e067 test: align agent persona context coverage`
- `7345c140 test: initialize persona in chat e2e`
- `336861ba test: initialize persona in manager chat coverage`

## Remaining boundary

S5 still owns physical deletion of retired JSON/Laputa persona code and unused GUI components. Real Tauri desktop visual acceptance remains open in `TODOLIST.md`; automated GUI gates and local HTTP smoke passed.
