# Verification — v0.4.11 DN-P-C

Environment: Linux x86_64 build VM, go1.26.4, node v24.19.0, pnpm 11.21.0,
`GOSUMDB=off` (pinned sibling replaces).

## agent-vivy (feat/dn-closure-wave1 @ db7f0c55)

- `go test ./internal/app ./internal/runtime ./internal/embedded ./cmd/vivy-shared ./sdk/internal/... -count=1`
  → all ok (sdk/internal 225.7s incl. pack/inspect matrix, assembly,
  conformance reproduction 54.7s). Earlier failures — missing
  `ui/node_modules`, stale internal digest, stale module pins, dropped
  go.mod requires — each repaired and re-verified; none bypassed.
- `go run ./sdk verify` → ok for discord, qq, tui and the eight untouched
  external modules.
- `go run ./sdk pack --recipe recipes/diva.vivy.yml --target shared --output /tmp/diva-artifact`
  → `vivy-shared.so` sha256 `d0155e26fddbb4426f7ddc2ab82615980a174d002bfc57d84c9386eac03659e5`.
- `go run ./sdk inspect-artifact /tmp/diva-artifact` → manifest decodes,
  `vivy/diva-cognitive` provides `core/cognitive-factory@v1` + 20
  `std/control-action@v1` ids `diva.cognitive.*`; portEdges bind them to
  `vivy/action-host`.
- `nm -D` on the .so → exactly the five exports; `vivy_abi.h` pin = 1.

## agent-diva (feat/dn-closure-wave1)

- `pnpm exec vue-tsc --noEmit` → clean.
- `pnpm test` → 67 files / 551 tests pass.
- `cargo test -p vivy-bridge -p diva-speech` → all green; the
  `real_artifact_*` tests exercised the newly staged `.so`
  (init → call/RPC → shutdown + event pump).
- `node scripts/ci/check_legacy_frontend_calls.mjs --selftest` → 8/8
  fixtures fired, gate clean.
- `just ci` → gui-test + gui-build + shell-bridge-test all pass.

## Not verified on this VM

- Whole-workspace `cargo check`/clippy (`shell-test`, `shell-clippy`,
  `tauri-build`): requires webkit2gtk-4.1 dev libs — absent.
- GUI candidate exercise and Windows x64 packaging: no WebView2/Windows
  runner.
