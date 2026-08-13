# GMH-24C verification

Passed:

- `cargo check --workspace`
- `cargo test -p agent-diva-laputa`
- `cargo test -p agent-diva-manager --lib` — 71 passed
- `cargo test -p agent-diva-agent --lib` — 360 passed
- governed typed apply/replay/receipt/rollback focused tests
- `python scripts/ci/check_laputa_clean_break.py`
- dependency-tree query confirms the removed package is absent
- Migration `memory --help` smoke
- GUI Vitest — 432 passed
- GUI production build and Tauri `cargo check`

The CLI executable smoke could not replace `target/debug/agent-diva.exe`
because the user's running desktop process holds the file. The process was not
terminated. The deferred real-desktop G2D run must restart onto this build.

The dedicated Rust 1.80 probe no longer reaches the removed runtime dependency,
but the broader workspace lock still contains unrelated current dependencies
whose declared MSRV is newer than 1.80. This remains an explicit release
blocker in `TODOLIST.md`; the default toolchain workspace check passes.

Final gates:

- `just fmt-check` passed.
- `just check` passed with warnings denied.
- `just test` compiled the complete workspace and reached the known Windows
  executable replacement block: the running desktop process denied removal of
  `target/debug/agent-diva.exe`. No test assertion failed before that point;
  affected Laputa, Manager, Agent and GUI suites passed independently.
