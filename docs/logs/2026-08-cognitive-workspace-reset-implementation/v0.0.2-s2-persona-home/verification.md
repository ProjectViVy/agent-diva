# I1-S2 Persona Home — Verification

## Focused gates

- `cargo test -p agent-diva-laputa` — passed after kernel implementation.
- `cargo test -p agent-diva-tools persona --lib` — 4 passed.
- Agent ToolAssembly Persona partition/subagent tests — passed.
- Manager Persona API initialize/read/CAS test — passed.
- `cargo test -p agent-diva-autodream` — passed after legacy persona proposal disconnection.
- WORLD protected-content and R6 entry-gate tests — passed.
- Strict Clippy for affected Laputa/Tools/Agent/AutoDream/Manager crates — passed.
- `pnpm test` — 63 files, 466 tests passed.
- `pnpm run build` — Vue type-check and Vite production build passed (existing large-chunk warnings only).
- `cargo check -p agent-diva-gui` — passed.
- Local Vite smoke at `http://127.0.0.1:4174/` — HTTP 200, 433-byte entry document.

## Workspace gates

- `just fmt-check` — passed.
- `just check` — passed (`cargo clippy --all -- -D warnings`).
- `just test` — passed after migrating the old Frozen Core/onboarding/chat fixtures to the Persona-ready contract.

The final successful `just test` traversed the complete workspace, including Agent 399 unit tests, Manager 120 tests (1 ignored), CLI integration tests, Laputa integration suites, GUI Rust targets, and all workspace doc tests. Existing non-fatal `authority_boundary_guard` dead-code warnings and `imap-proto` future-incompatibility notice remain unchanged.

## Deferred manual evidence

The environment exposed neither the standalone `agent-browser` executable nor the in-app browser control executor. No screenshot or real Tauri interaction is claimed. `PERSONA-S2-DESKTOP-SMOKE` remains open for visual and key-workflow acceptance.
