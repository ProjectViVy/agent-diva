# DN-5 — Tauri thin shell + vivy-bridge (iteration summary)

Branch: `DIVA-NEXT-P0`. Commits: `c2c3cc21` (vivy-bridge), `0f012484`
(shell crate), `8faa177d` (frontend seam + build wiring). Unpushed.

## Delivered

- `agent-diva-gui/src-tauri/crates/vivy-bridge` — runtime-dlopen bridge
  over ABI v1 (`VivyInit/Call/PollEvents/Shutdown/Free`). Loads only the
  staged triple `vivy-shared.{so,dll,dylib}` + `vivy-shared.h` +
  `vivy_abi.h`; parses `VIVY_ABI_VERSION` from the staged header at load
  (`build.rs` additionally fails when `VIVY_ABI_HEADER` is pinned to a
  different version). Every `char*` return is wrapped in a single-owner
  guard freed exactly once; the library stays resident for process
  lifetime (no `dlclose` before exit). Error envelopes map to typed
  `ErrorKind` + numeric code; null/malformed/non-UTF-8 returns degrade to
  `transport_lost`/`internal`. A single dedicated pump thread drains
  `VivyPollEvents` (25 ms tick, batch 500) and emits domain events, a
  sticky `gap`, and a one-shot `transport_lost`.
- `agent-diva-gui/src-tauri` (`agent-diva-shell` v0.4.10) — Tauri v2,
  single window + tray + `tauri-plugin-single-instance` (second launch
  focuses the existing window). One command `vivy_call` executes on a
  blocking worker and forwards the method name verbatim — the shell never
  interprets business methods. One event `vivy:event` carries
  `{kind:"vivy"}` payloads and `{kind:"bridge", status:"gap"|"lost"}`
  separately. Window close hides; only tray Quit flips the close action
  and runs the ordered shutdown (pump stop → join → `VivyShutdown` ≤5 s)
  on a teardown thread, then `exit(0)`. `RunEvent::Exit` is the belt for
  any other exit path. Startup failure is process-fatal (`exit(1)`) — no
  in-process backend restart.
- `agent-diva-gui/src/platform/desktop-host.ts` — sole frontend↔native
  seam (`vivyCall`, `onVivyEvent`, `isTauriShell`). Retired legacy invoke
  consumers are untouched pending DN-1..6 rewiring.
- Build wiring: `tauri` pnpm script, `@tauri-apps/cli` devDep,
  `@tauri-apps/api` bumped to `~2.12` to match the resolved `tauri` crate
  minor (CLI refuses mismatched minors). just: `shell-bridge-test`,
  `shell-test`, `shell-clippy`, `shell-stage-runtime`, `tauri-build`;
  `ci` now also runs the pure-Rust bridge tests. CI gains a
  `shell-bridge-check` job (no webkit needed); the shell crate itself
  stays native-runner verified.
- `src-tauri/vivy-runtime/` is the pack-staging dir consumed by
  `bundle.resources`; ABI headers + `generation.json` are committed,
  binaries and generated payload are gitignored.

## Verification

- `cargo test --workspace` — 10/10 green: 6 bridge FFI tests (missing
  dir/header → `load_failed`, ABI mismatch → `incompatible_abi`, happy
  path + gap + null/malformed/UTF-8/non-envelope → `internal`/
  `transport_lost`, `allocs == frees`, idempotent shutdown, closed
  rejection, event pump, env-gated real-artifact init/call/shutdown
  asserting `protocol_version == "vivy.rpc.v1"`) and 4 lifecycle tests
  (pre-start rejection, startup failure → `load_failed` + clean quit,
  hide-until-quit, late-call rejection + idempotent/racing shutdown).
- `cargo clippy --all-targets -- -D warnings` — clean.
- `pnpm build` (vue-tsc + vite) — clean.
- `pnpm tauri build` — release binary + `DIVA_0.4.10_amd64.deb` +
  `.rpm`. Deb inspection confirms `usr/lib/DIVA/vivy-runtime/` carries
  the full staged artifact (the 92 MB `vivy-shared.so`, both headers,
  `generation.json`, assembly files).
- Native transcript (this Linux box, real sealed .so): process launch →
  `DIVA` window appears; `wmctrl -c` → window hides, process persists;
  second launch → single-instance exit 0, existing window refocused;
  `SIGINT` → `RunEvent::Exit` → ordered shutdown, process exits with no
  crash output.

## Rulings

- Task 1: runtime `dlopen` (libloading) over link-time loading — missing/
  mismatched artifacts are testable failure modes, not loader aborts;
  library deliberately kept resident to process end (avoids
  free-after-unload), documented per contract.
- Task 1: ABI version is read from the *staged* `vivy_abi.h` at load time
  and echoed-check inside `VivyInit` — the header stays the single ABI
  source; `build.rs` refuses a pinned `VIVY_ABI_HEADER` ≠ 1, and the
  committed `abi.rs` constant only backstops headerless dev builds.
- Task 2: failure injection via a deterministic fake C library
  (`tests/fake_vivy.c`) — real .so paths are env-gated so no fake passes
  as live acceptance.
- Task 3: `vivy_call` runs on `spawn_blocking`; the pump thread is the
  only `VivyPollEvents` caller — the UI executor and frontend never touch
  FFI. Bridge status rides inside `vivy:event` as a distinct `kind`, not
  a second channel.
- Task 4: close → hide; tray Quit is the only explicit-exit path and owns
  the ordered teardown on a separate thread so `prevent_close` never
  deadlocks the UI; `RunEvent::Exit` repeats shutdown idempotently.
- Task 5: `@tauri-apps/api` bumped `~2.11` → `~2.12` to satisfy the CLI's
  minor-match check against `tauri 2.12.1`; pinning the older crate was
  tried and 2.11.6 fails to compile against the 2.12 build/codegen pair.

## Not verified here (deferred, recorded)

- Live streamed answer + pending-approval window close/reopen — needs a
  provider credential; covered structurally (pump independent of window)
  and scheduled for DN-2 acceptance.
- Two windows: by design this shell owns exactly one `main` window;
  duplicate launch is the single-instance focus path (verified).
- Windows/amd64 build + bundle + lifecycle: no Windows runner in this
  environment (same gate as DN-L `P0-NATIVE-VERIFICATION`).
- `pnpm-lock.yaml`/`Cargo.lock` updated as required by the new deps.
