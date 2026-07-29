# RG-E8-S4a Mentle Clean-Break

> Repatriated evidence from `refactor/deep-governance` on 2026-07-30.
> This proves the design and branch implementation; current `agent-diva-pro`
> still requires a scoped port and must not be reported as already clean.

## Outcome

Removed the remaining Mentle/MenPalace product surface from Diva without adding a compatibility reader, migration path, feature flag, dependency, or MSRV change.

## Changes

- Deleted the orphan Mentle settings card.
- Removed Mentle DTOs, tool discovery API, capability binding, config shape/defaults, fixtures, and legacy Tauri command.
- Added deletion-proof assertions for Rust manifests and active GUI/Tauri product source.
- Activated the complete RG-E8-S4 implementation line in `TODOLIST.md`, with blocked dependencies stated explicitly.

## Impact

Garden may continue to use its own backend. Diva now has no active Mentle product integration; the next implementation line is the lightweight Embedded Laputa store.
