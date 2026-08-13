# Acceptance

- Laputa Rust APIs expose proposal, apply, snapshot, section, changelog, rollback, and event behavior.
- Manager HTTP routes under `/api/laputa/*` proxy the same behavior.
- Tauri commands expose the same boundary for GUI consumers.
- Event delivery supports broadcast plus polling fallback.
- TBD sections return explicit `tbd` status.
