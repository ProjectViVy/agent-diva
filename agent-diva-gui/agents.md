# agent-diva-gui

## OVERVIEW

Tauri + Vue 3 desktop application. The Rust backend lives under `src-tauri/` and is a workspace member at `agent-diva-gui/src-tauri`; the TypeScript frontend lives under `src/`.

## WHERE TO LOOK

| Concern | Location |
|---|---|
| Tauri backend entry | `src-tauri/src/lib.rs` (app builder, command registration, embedded gateway) |
| Tauri binary launcher | `src-tauri/src/main.rs` |
| Tauri manifest | `src-tauri/Cargo.toml` |
| Frontend pages / components | `src/components/`, `src/views/` or `src/pages/` |
| Stores / state | `src/stores/` |
| API clients | `src/api/` |
| Features | `src/features/` |
| Avatar runtime | `avatar-runtime-vrm/` |
| Frontend scripts | `package.json` |

## CONVENTIONS

- Add Tauri commands in `src-tauri/src/lib.rs` (or a submodule) and expose them via `#[tauri::command]`.
- Reuse `agent-diva-cli` library modules for config/runtime behavior; do not duplicate CLI logic.
- Embedded gateway starts via `agent-diva-manager::start_embedded_gateway_runtime` in release builds.
- The GUI listens on port `3000` and exposes `/api/hook/message` for external messages.

## ANTI-PATTERNS

- Do not call `agent-diva-cli/src/main.rs` private items; use the public `agent-diva-cli` library.
- Do not run Rust-only tests to validate GUI changes; use `just gui-automated-check` or a real workflow.
- Do not add long-running sync code in Tauri commands without spawning a task.

## NOTES

- `pnpm tauri dev` in `agent-diva-gui/` starts the dev server.
- `scripts/ci/prepare_gui_bundle.py` stages CLI/service binaries into `src-tauri/resources/bin/` before `tauri build`.
