# DN-P verification

Platform: linux/amd64, Ubuntu 22.04, webkit2gtk-4.1 runtime present.
Artifact under test: `agent-diva-gui/src-tauri/vivy-runtime/vivy-shared.so` (sealed DN-L pin) + `pnpm tauri build` bundles.
Driver: `/home/ubuntu/diva-dnp-accept/lifecycle.py` (python ctypes, real C ABI; transcript `/home/ubuntu/diva-dnp-accept/transcript.jsonl`).
Model: sensenova / `sensenova-6.8-flash-lite`, key from `SENSENOVA_API_KEY`.

## Task 1 — packaged artifact vs DN-L manifest: PASS

`dpkg-deb -x DIVA_0.4.10_amd64.deb` → sha256 of `vivy-shared.so`, `vivy_abi.h`, `generation.json` all identical to the staged pins; `generation.json` byte-diff empty. No executable-only substitution detected (runtime trio all present).

## Task 2/3 — lifecycle chain + restart recovery: 21/21 PASS

- Phase A (live): session create → turn/start → `run.completed` → answer persisted in Journal → gated `write_file` blocked pre-approval → `review/respond approve` → run continued, file landed under `workspace/<run_id>/` → second gated run → `run/cancel` → `run.cancelled`, file never written → `run/get` agrees.
- Phase B (close/reopen): `VivyShutdown` → `VivyInit` on same `vivy.db` → history recovered (assistant answer present), `run/get` statuses persisted (`completed`, `cancelled`), decided reviews recoverable.
- Phase C (pending approval at close): run left with `tool.approval_required` open → shutdown → reopen → the approval is settled to terminal `cancelled` (not stuck `pending`, not silently approved); `run/get` = `cancelled`. Backend semantics verified via `review/list` + `run/get`, not UI.
- Phase D (in-flight run at close): long-form turn closed mid-flight → reopen → `run/get` = `cancelled` (no zombie `running`).
- Phase E: explicit `VivyShutdown` clean.

GUI window lifecycle: open → close (hide, process alive) → second launch (single-instance focus) → SIGINT ordered exit — all observed live on the release binary.

## Task 4 — live-model evidence: PASS

Recorded: provider `sensenova`, model `sensenova-6.8-flash-lite`, platform linux/amd64, artifact sha256 `af70a018…` (above). Transcript captures streamed deltas + `tool.approval_required` gating + journal persistence.

## Task 5 — boundary gate: PASS

`python3 scripts/ci/check_vivy_backend_boundary.py --src-tauri agent-diva-gui/src-tauri`:
- Cargo.lock dep graph: 421 packages, zero legacy `agent-diva-*` business crates;
- tauri.conf.json: resources = `vivy-runtime` only, no externalBin, single invoke handler `vivy_call`;
- deb contents: `usr/bin/agent-diva-shell` + required runtime trio, no stray `.so`, no `.db/.sqlite*` files.
CI: `boundary-gate` job added to `.github/workflows/ci.yml`.

## Finding (non-blocking, recorded)

`VivyShutdown` once returned `context deadline exceeded` while a background utility generation (session auto-title) was in flight; a later identical call returned instantly and the full chain re-passed clean. Transient; tracked as `VIVYSHUTDOWN-DEADLINE-UTIL` in TODOLIST.
