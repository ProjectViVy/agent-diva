# v0.4.11 — DN-P-C closure candidate (repack + inspect + stage)

Repacked the sealed DIVA shared library from the accepted closure sources
and staged the inspected Generation into the Tauri shell.

- agent-vivy `db7f0c55` (+`2f38c2e5`): repacked `recipes/diva.vivy.yml`
  → `vivy-shared.so` (107,489,640 B, sha256 `d0155e26…`), generation
  `331bb89d…`, 21 modules including `vivy/diva-cognitive` with all 20
  `diva.cognitive.*` control actions.
- Two prerequisite repairs were required inside agent-vivy before the pack
  gate would pass: repinned module source digests after real `go mod tidy`
  drift (discord/qq/tui, both `vivy-module.yaml` and the embedded Go
  descriptor pins) and restored root `go.mod` requires that generated
  assemblies import (tui/headless/lsp/governance).
- Staged via `just shell-stage-runtime`; committed payload is
  `generation.json` + `vivy-shared.h` + `vivy_abi.h` (binary stays ignored
  per existing policy).
- `resources/vivy.default.yaml` gained allow rules for the nine
  `diva.cognitive.*` write actions — they are `EffectWrite`, which yields
  `PolicyPrompt` → `ErrApprovalRequired` on the embedded control pipe that
  has no approval continuation; same composition as the existing mask
  rules.
- DN-0P build inventory refreshed with a `dn_pc_refresh` block recording
  the new pins, artifact hashes and repair list; baseline captures kept.

## Evidence

- Pack/inspect: `go run ./sdk pack … --target shared` + `inspect-artifact`
  (in-repo commands; output manifest verified in-session).
- `just ci`: gui-test (67 files / 551 tests) + gui-build + shell-bridge-test
  all green, including `real_artifact_init_call_shutdown` and
  `real_artifact_dn1_rpc_smoke` against the newly staged `.so`.
- `node scripts/ci/check_legacy_frontend_calls.mjs --selftest`: 8/8.
- Five C exports verified on the sealed library: VivyInit, VivyCall,
  VivyPollEvents, VivyShutdown, VivyFree; `VIVY_ABI_VERSION 1` unchanged.

## Deferred

- Whole-workspace `cargo check`/clippy and Tauri candidate packaging on this
  VM (no pkg-config/glib/webkit2gtk) — owner/Windows runner.
- GUI-level candidate exercise (initialize→session→safe turn→approval→
  cancel→hide/reopen→Quit) — needs a running WebView2 host.
- Windows x64 DLL candidate — no mingw-w64 cross toolchain (unchanged DN-0P
  blocker).

Owner acceptance: pending.
