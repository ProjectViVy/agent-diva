# TODOLIST

Active gaps and deferred work for DIVA Next. Live status/dependencies live
only in [the existing index](docs/plans/diva-next/index.md); current host
architecture is [DN-W3](docs/plans/diva-next/p0-design.md). The previous
Tauri/C ABI source line is frozen at the paired archive refs. Checked rows
below describe prior delivered source, not acceptance of a Wails product.
Severity: P0 blocking correctness, P1 high, P2 medium, P3 low.

## Memory loop verification — execution started 2026-10-09

See the [Epic–Story package and live status](docs/plans/diva-next/memory-loop/README.md).
Most rows remain validation gaps. MEM-LOOP-USER-SOURCE has a verified repair; formal Story acceptance remains open. Environment/full-CI evidence is in docs/logs/2026-10-memory-loop-repair/v0.2.0-source-and-environment/. New reflection/Restart developer evidence and continuation are in docs/logs/2026-10-memory-loop-chain/v0.1.0-reflection-and-restart/. Historical failures remain unchanged.

- [ ] **MEM-LOOP-REAL-BACKEND** P0 — Reproduce w0-5's selected-backend
  unavailable result and prove real Mentle write/search/expand/reopen through
  the sealed host. Audit local-model initialization and read-only modes (S02).
- [ ] **MEM-LOOP-USER-SOURCE** P0 acceptance — Admitted durable user-source/role repair and random-fact canonical/process regression are implemented. Trusted accepted receipt recovery is repaired without rewriting old effects. Finish S04 V05–V09 and separately verify any legacy source backfill; do not mark full Story Done from partial proofs. See docs/logs/2026-10-memory-loop-repair/v0.2.0-source-and-environment/.
- [ ] **MEM-LOOP-REFLECTION-ACTMEM** P1 — Prove actual Pulse/Recap/Work,
  automatic reflection, effect receipts and persona-review continuity;
  domain API availability alone is not product wiring evidence (S05–S07).
- [ ] **MEM-LOOP-FAILED-WINDOW** P0 — Budget and unresolved-window fences are repaired; VIVY `d72add09` preserves run/window/watermark and exposes blocked status, Laputa `256d1f1` stops after unresolved work. Continue original-operation receipt recovery and six-cut crash matrix before acceptance. Do not infer crash idempotence from the now-green single-source reflection case (S06/S10).
- [ ] **MEM-LOOP-INFER-BOUNDS** P0 acceptance — Truncation and duplicate post-reflection evidence packets repaired: VIVY `e1de33ae`, Laputa `9bc39af`. Actual over-4-KiB Unicode-source reflection, runtime 75, selected composition 11, Laputa 64 and Garden 480 pass with zero skip. Complete remaining input limits/policy matrix and final new-source CI/sealing; this remains local work (S06).
- [ ] **MEM-LOOP-RECALL-INPUT** P0 — Prove stored ordinary memory reaches
  the model in a new session after process restart, with empty-profile and
  recall-disabled controls; do not substitute Frozen Core or BML (S08/S09).
- [ ] **MEM-LOOP-RECOVERY-ISOLATION** P0 — Six process-crash cuts, unknown
  effect handling, backend degradation, correction/deletion in later model input, foreign scope
  and untrusted memory content require integrated evidence (S09–S11). Public
  correction/tombstone/restart and accepted-source redelivery now have developer
  proof (VIVY 7a1086a5, v0.9 checkpoint); S08 and crash cuts remain open.
- [ ] **MEM-LOOP-NATIVE-LIVE** P1 — Verify the same sealed candidate on
  Windows and with a real model; historical 15/17 migration rows do not
  establish memory-loop correctness (S12/S13).
- [ ] **MEM-LOOP-PIN-DRIFT** P1 — DIVA's source lock, historical Windows
  artifact and current sibling repositories differ. Preserve exact baseline
  evidence and use supported candidate repinning before acceptance (S01).

## Approved migration work — planning delivered, execution not started

- [ ] **WAILS-NATIVE-GATE** P0 — W0 must prove pinned Windows native caller,
  media, lifecycle, keyring and WebView2 behavior before adoption.
- [ ] **GO-HOST-AND-SEALED-PACK** P0 — W1/W2 public Go lifetime owner and
  generated external host packaging; retain single Runtime/Journal/grants.
- [ ] **GO-DESKTOP-AND-SPEECH** P1 — W3/W4 frontend seam and DIVA Go speech;
  preserve raw audio, secret boundary, cancel and generation semantics.
- [ ] **WAILS-PARITY-AND-RECOVERY** P0 — W5 fresh FrozenCore, persisted-session
  admission, runtime logs, required chat/cognition/console/voice and truthful
  hide/quit/crash/restart evidence. No language-change shortcut closes these.
- [ ] **CABI-RUST-RETIREMENT** P1 — W6 remove retired host/ABI and replace
  CI/instructions after W5. Blocked until paired archive tags exist.
- [ ] **PAIRED-ARCHIVE-TAGS** P1 — archive/tauri-cabi branches created at
  DIVA 5444795a / VIVY fc559e6b. Annotated archive/tauri-cabi-20261004 tags
  remain pending tag-capable authenticated access; exact instructions in
  [archive record](docs/plans/diva-next/wails/archive.md). Old ignored binary
  is not preserved or reverified by the source freeze.
- [ ] **WINDOWS-WAILS-ACCEPTANCE** P1 — W7 clean installed final Go product;
  engineering preparation first, owner acceptance last.

The index contains eight executable Story plans with immediate dependencies,
six waves and verification gates. This publication authorizes no product
implementation and makes no Ready/Done implementation claim.

## Prior closure source deliveries and retained residuals

- [x] **COGNITIVE-COMPOSITION** P1 — Landed via DN-4A/DN-LC/DN-4C + sealed in
  DN-P-C (`vivy/diva-cognitive`, 20 control actions, one Garden owner per
  profile). Candidate-level residuals moved to OBS09 findings: F2 (Bootstrap
  capture seam unwired — first turn on a fresh session fails until a Frozen
  Core is captured), F3 (per-process action grants). See OBS-09 fixture;
  owner acceptance via NATIVE-ACCEPTANCE-HANDOFF.
- [ ] **COGNITIVE-EMBEDDED-LIFECYCLE** P1 — Current merged VIVY source
  already starts the cognitive loop through StartEmbeddedServices. W1/W5
  must prove single startup, policy-disabled behavior, stop/drain ordering,
  recovery and bounded close on the new Go host; do not add a duplicate loop.
- [x] **COGNITIVE-AUTHORITY-AND-RECOVERY** P0 — DN-4B landed per-run
  persisted authority/guard (ActiveRunID/PendingThrough instead of
  Attempt++), durable `unknown_outcome` block (`recovery_required`), and
  serialized admission; capture keeps `run_id:journal_seq` identity so no
  new-attempt replay after unknown outcome. Candidate-level OBS check did
  not re-drive a recovery scenario; exercised gate remains owner scope.
- [x] **COGNITIVE-SUPERVISOR-CAPTURE** P1 — DN-4B: supervisor session
  (`sess_cognitive_supervisor`) primaries excluded from capture with ack
  receipt; capture key stays `run_id:journal_seq` so restarts cannot feed
  strategy output back as fresh evidence.
- [x] **PINNED-NATIVE-BUILD-CLOSURE** P1 — DN-0P staged tree + CGO inventory
  (libc only, no ONNX/OpenSSL/libsqlite3) and DN-P-C repack/inspect
  (generation `331bb89d`, 21 modules, sha256 `d0155e26`); source pins in
  `closure-packaged-obs.json`. Windows x64 link remains under
  NATIVE-ACCEPTANCE-HANDOFF.
- [x] **CONSOLE-OBS-CONSUMERS** P1 — OBS-06/07/08 landed; OBS-09 verified on
  the packaged candidate: trajectory v2 + token totals equal journal, one
  event owner, bounded queries, gap flag, restart-stable replay. Residuals:
  runtime log sink absent (OBS09-F1), window hide/reopen + rotation pending.
- [x] **CHAT-ATTACHMENTS** P1 — DN-2A: image bytes/MIME wired through
  `vivy_call` with size guards and explicit unsupported-file UI.
- [x] **CHAT-CONTROLS** P2 — DN-2B: permission mode, regenerate-in-place,
  start_goal entry and authoritative cancel wired to VIVY contracts.
- [x] **ONLINE-SPEECH-NATIVE** P1 — DN-6A/B/C landed: exact native commands,
  keyring credentials with explicit unavailable state, SiliconFlow STT +
  SiliconFlow/MiniMax TTS request/cancel paths, generation-scoped playback in
  main chat. Live-provider/Windows gates remain under DN-0S-PENDING-GATES /
  NATIVE-ACCEPTANCE-HANDOFF.
- [x] **VOICE-ASSET-BOUNDARY** P2 — DN-6A: bounded `voice_asset_*` import/
  list/read/delete with leases; general VRM management stays deferred.
- [x] **RUNTIME-REPIN-AND-GATES** P1 — Repacked/inspected/staged under DN-P-C
  (v0.4.11): Generation `331bb89d…` with `vivy/diva-cognitive` + 20 actions;
  `vivy.default.yaml` gained the nine `diva.cognitive.*` write-action allow
  rules; inventory `dn_pc_refresh` recorded. GUI-level/Windows candidate
  exercise remains under NATIVE-ACCEPTANCE-HANDOFF.
- [ ] **NATIVE-ACCEPTANCE-HANDOFF** P1 — Superseded host procedure: W7
  prepares the sealed Go/Wails Windows x64 package, real-model/mic/STT/TTS,
  hide/reopen/Quit, cognition/restart and console checks. DLL/header/FFI are
  archived requirements, not tasks for the new mainline product.
- [ ] **GOVERNANCE-DOCS-STALE** P1 — AGENTS.md / AGENTS-ARCH.MD / CLAUDE.md /
  LAPUTA.md describe retired Rust crates, old memory rules and absent root
  Cargo recipes. DN-C2 records the conflict; reconcile stale descriptions
  against the authorized Next scope. Obsolete crate rules do not create a
  new permission gate or justify resurrecting retired code.
- [ ] **OLD-SECURITY-ISSUE-DISPOSITION** P2 — Issue #8 targets retired
  aws-lc-sys/rsa dependency chains. Neither package exists in the two current
  shell/bridge lockfiles. Fresh `cargo audit` + `pnpm audit` evidence lands
  with DN-M-C (see ledger); no vulnerability-free claim beyond the audited
  advisory sets.

## Deferred residuals (not completed)

- [ ] **GENERAL-REPORTS-RESOURCES** P2 — Broader notebook/report scheduling
  and generic resource management remain outside this closure. Cognitive
  reflection notes and required new voice assets are in scope; keep truthful
  unavailable states for other surfaces.
- [ ] **SKILL-MANAGEMENT-RESIDUAL** P2 — Upload/delete/content edit/request
  review lack verified DIVA contracts. Latest VIVY has revisions/list when
  its backing dependency is bound, so history is not universally absent.
- [ ] **COMMAND-RULES-AND-WIPE** P2 — No verified equivalent for the removed
  command-rule UI or backend wipe. LocalStorage reset does not wipe VIVY data.
- [ ] **SEARCH-MCP-TUNING-RESIDUAL** P3 — Legacy search roster/key/max-results,
  MCP timeout/form knobs, context ratio and runtime status columns lack full
  scoped parity. Keep as residuals; do not resurrect old commands.
- [ ] **GUI-PET-DORMANT** P2 — Pet windows/neuro-link remain dormant by ruling.
  Online voice must work from main chat/settings without restoring this host.
- [ ] **LOCAL-VOICE-OLVRS** P3 — Local ONNX/sherpa and OLVRS follow a separate
  design; no implementation or dependency added in online voice closure.
- [ ] **VIVYSHUTDOWN-DEADLINE-UTIL** P3 — Prior auto-title in-flight shutdown
  deadline occurred once. Track bounded teardown on the new artifact; do not
  present driver retries as a product fix.
- [ ] **UI-COPY-CLEANUP** P3 — Sweep unused DN-3 locale keys and misleading
  controls only alongside touched flows; no unrelated broad frontend rewrite.
- [ ] **LAPUTA-ACTMEM-EMPTY-RENDER** P3 — Upstream defect found by DN-4A:
  laputa `evolution.ActmemDocument.Render()` emits `entries:` null for an
  empty entry map, producing an ACTMEM head that its own reader rejects.
  Deferred per DN-4A handoff (no ownership change); fix upstream in laputa.
- [ ] **GARDEN-CONSOLE-DIST** P3 — laputa `garden/console` fails
  `go build ./...` until the console UI build produces `dist/`; recorded in
  `closure-build-inputs.json` captured_failures, not patched.
- [ ] **DN-0S-PENDING-GATES** P2 — Windows WebView2 IPC/audio roundtrip,
  Credential Manager store path, MSVC+cmake aws-lc-sys link, and
  authenticated SiliconFlow/MiniMax calls remain pending; DN-6/DN-8C own.
- [ ] **DN-6B-SHELL-COMPILE** P2 — `src/speech/{mod,commands}.rs` +
  `src/lib.rs` wiring (4 new commands, diagnostic sink, Hide/Quit hooks)
  is rustfmt-clean but compile-unverified: tauri cannot link
  GTK/WebKitGTK on this VM. `just shell-test` / `just shell-clippy`
  whole-workspace runs remain a DN-8C obligation.

## Recorded delivery and cancellation

- [x] **DN-C2-P1-PLANS** — 20 executable Story plans supplied under existing
  DN/OBS stages with requirement coverage, immediate dependencies, waves,
  file ownership, checks and handoff. This closes planning only, not product work.

- [x] Initial shared host/bridge/core/settings/masks delivery: DIVA #16 and
  VIVY #28 merged; prior Linux evidence retained at its original pins.
- [x] **HISTORICAL-IMPORT-CANCELLED** — Owner decision 2026-10-03: no old data
  import, because there are no current DIVA users. DN-7 cancelled and removed
  from DN-8 prerequisites; do not build an importer or delete old data.
- [x] Previous mixed backlog preserved as an explicit historical snapshot:
  [2026-10-03-before-next-closure.md](docs/archive/todolist/2026-10-03-before-next-closure.md).
  Old Rust-specific tests/tasks are not active Next work by default; product
  residuals above remain visible. Historical boxes are not new acceptance.


## DN-W3 migration residuals (2026-10-04)

- [ ] **W5 promotion gate** — after w0-5: 15 passed / 2 pending / 0 failed in `fixtures/wails-candidate-acceptance.json` (VOICE-REAL [no audio endpoints on VM] + Windows rollup pending — both environmental); `scripts/ci/check_wails_candidate.py --require-all-passed` is the gate.
- [x] **W0-F5 sealed turn/start frozen-core gate** — FIXED upstream (vivy `e3b60280`: lazy capture on first Prepare on agentapi not_found); verified w0-4 — fresh-session GUI turn wrote `frozen_core_sessions` row at first Prepare and reached the model call.
- [x] **W0-F6 GUI never binds/renders session** — FIXED in diva `dda9d7da`. Root cause (CDP capture, w0-4): wails3 injects `window._wails.environment` via execJS on NavigationCompleted, after mount; `isTauri()` sampled at mount was false → browser-mode return → `vivyChat.connect()` never ran. Fix waits for `wails:runtime-config-ready` (3s bound); verified: online on boot + history renders + reload/restart resync clean.
- [x] **W0-F1 sealed frontend not embedded** — FIXED upstream (vivy `42c263f2`: pack overlays built dist into staged tree); verified live w0-3 (real Vue UI renders).
- [x] **W0-F2 no reopen affordance** — FIXED upstream (diva `8db11f51`: tray AttachWindow + Show/Quit menu); verified live w0-3 (click toggles, Quit releases lease).
- [x] **W0-F3 no explicit-Quit affordance** — FIXED upstream with F2 (tray Quit → graceful teardown + lease release); verified w0-3.
- [ ] **W0-F4 mic/audio untestable on build VM** — Audiosrv disabled + zero PNP audio devices on Windows Server 2022; needs an audio-capable machine (plus operator credentials for real-provider rows).
- [ ] **Windows deps-link gotcha** — `mklink /J` junctions break `hashSourceTree` (`Incorrect function`); real directory symlinks (`mklink /D`, elevated) work. Worth a setup note or scripted step for Windows builds.
- [ ] **W3 Windows candidate acceptance** — lifetime + UI/chat rows green on Windows (w0-2…w0-4); remaining pending rows are grants-catalog/cognitive drives + VOICE-REAL (no audio endpoints).
- [ ] **W4 real-provider evidence** — SiliconFlow STT + SiliconFlow/MiniMax TTS need operator-held credentials; scripted fixtures landed (w4-1).
- [ ] **W4 Windows speech leg** — Credential Manager proven (w0-2 Task 5); speech lifecycle on Windows blocked by W0-F4.
- [ ] **`desktop_pet_*` dispatch noop** — pet commands still `not_ready`; pet lands with drag/expansion (deferred).
- [ ] **`desktop_pet_start_drag` noop** — DesktopPetOverlay drag calls a stub; Wails native drag (`--wails-draggable` regions) lands with pet expansion (deferred).
- [ ] **`tauriVoiceFileReader`/`isTauri*` naming** — functional seam is Wails; leftover cosmetic names + locale strings ("Tauri 运行时") should be renamed in W4/W5 cleanup.
- [ ] **W6 cleanup pending** — `src-tauri/`, `@tauri-apps/*` deps, Rust/tango recipes in justfile, `scripts/tauri` helpers all still present until W6 removal.
- [ ] **Generated bindings drift** — `src/generated/wails/` must be regenerated (`just desktop-bindings`) after any bound-method change; no drift check in CI yet.

## GUI follow-ups

- [ ] **GUI-SESSION-PIN-PERSISTENCE** P3 — no verified session pin write contract exists; the no-op pin action and stale grouping were removed until persistence is available.


## Memory execution environment and fixture blockers

- [x] **MEM-LOOP-NATIVE-BUILD-DEPS** — Resolved task-locally without system/HOME changes: extracted native headers/libraries, supported sealed Linux host test/race and development build pass. Original product lock is unchanged. Windows/native UI/live-model acceptance remains separate (S01/S12/S13).
- [ ] **MEM-LOOP-FIXTURE-COMPLETE** P1 — ack/full App/real storage evidence works; reflection/recall modes, reflected observation and real process Restart are explicitly pending. Complete S03 contract before downstream gate acceptance.
- [ ] **MEM-LOOP-PINNED-REGRESSION** acceptance — Underlying blockers repaired: Laputa 63 pass; Mentle 536 pass / 0 skip; Garden 480 pass / 0 skip and process e2e 2 pass. Exact dependency closure/model/config bindings preserved. Managed ancestor instruction/VCS and Codeface home-write fixtures repaired; complete frozen-source just ci now exits 0. Formal same-candidate Story evidence remains open.
