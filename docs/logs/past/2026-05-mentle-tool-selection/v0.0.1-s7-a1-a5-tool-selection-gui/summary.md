# Sprint 7 A1-A5 — Mentle Tool Selection and GUI Controls

## Summary

Implemented the Sprint 7 configuration, assembly, prompt-sync, persistence, and GUI control path for selective Mentle tool activation in Agent-Diva.

### Configuration model

- Added persisted `[mentle]` config on the root `Config` object:
  - `enabled: bool` (default `false`)
  - `mode: off | read_only | full | custom` (default `off`)
  - `allowed_tools: Vec<String>` (default empty)
- Runtime view: `MentleToolRuntimeConfig::from_config()` keeps legacy compatibility with `tools.builtin.mentle`.

### Assembly and prompt sync

- Dynamic `memtle_*` tools are filtered in `mentle_runtime` before registry assembly.
- Prompt routing remains registry-driven via `memtle_status` presence after filtering.
- Added `RuntimeControlCommand::UpdateMentle` and `AgentLoop::apply_mentle_config()` for runtime refresh after GUI/config updates.

### GUI

- Added `MentleSettingsCard` under General Settings with enable toggle, mode selector, custom checklist, save/reset.
- Extended tools API with `GET /api/tools/mentle/available` for dynamic tool discovery.

### Persistence

- GUI saves through existing `POST /api/tools` path; manager persists `config.mentle` and syncs `tools.builtin.mentle`.
- Saved settings trigger runtime Mentle refresh when the gateway control channel is active.
