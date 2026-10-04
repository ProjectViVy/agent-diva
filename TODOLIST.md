# TODOLIST

Active gaps and deferred work for DIVA Next. Mainline stage state/dependencies live
only in [the existing index](docs/plans/diva-next/index.md); current scope and
closure rules are [DN-C2](docs/plans/diva-next/p0-design.md).
Severity: P0 blocking correctness, P1 high, P2 medium, P3 low.

## Closure work

The [DN-C2-P1 execution package](docs/plans/diva-next/index.md#executable-story-package)
maps these gaps to 20 concrete Plans. Planning is complete; all implementation,
captured contract/native/provider probes and owner acceptance remain unfinished.
Four independent roots are plan-Ready; other readiness lives only in that index.

- [x] **COGNITIVE-COMPOSITION** P1 — Landed via DN-4A/DN-LC/DN-4C + sealed in
  DN-P-C (`vivy/diva-cognitive`, 20 control actions, one Garden owner per
  profile). Candidate-level residuals moved to OBS09 findings: F2 (Bootstrap
  capture seam unwired — first turn on a fresh session fails until a Frozen
  Core is captured), F3 (per-process action grants). See OBS-09 fixture;
  owner acceptance via NATIVE-ACCEPTANCE-HANDOFF.
- [ ] **COGNITIVE-EMBEDDED-LIFECYCLE** P1 — Bundle arms on every `VivyInit`
  from the sealed manifest (one lazy owner per handle); cognitive trigger
  exists but is policy-disabled, so no autonomous loop runs. Shutdown drains
  runs via CancelAll and closes the domain. Restart dedupe + drain-before-
  cancel ordering not yet proven end-to-end.
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
- [ ] **NATIVE-ACCEPTANCE-HANDOFF** P1 — Prepare fresh install, Windows/amd64
  DLL+header+FFI and packaging, real-model GUI reopen/pending approval,
  mic/STT/both TTS/cancel/Quit, cognition/restart and console scenarios. Owner
  performs final product acceptance after all engineering work/checks.
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


## MyGo observation branch

- [ ] **MYGO-RESEARCH-HOST** P2 — Maintain the optional research/mygo lane over
  the same accepted public VIVY Host and retained frontend. Wails remains the
  delivery path. The [MY-P1 plan](docs/plans/diva-next/mygo/index.md) and
  [MY-D1 design](docs/plans/diva-next/mygo/design.md) define scope, prerequisite
  evidence and acceptance; this publication does not implement the prototype.
- [ ] **MYGO-NATIVE-EVIDENCE** P2 — Compiler/generator closure, trusted native
  caller/media, hide/reopen/Quit, VRM/microphone, configured providers and
  installed Windows candidate have no experiment evidence yet. Await accepted
  W1/W2/W3, W4/W6 and W7 outputs as specified in MY-P1; keep missing rows pending.
- [ ] **MYGO-UPKEEP-DECISION** P3 — Use relevant upstream/mainline interface
  changes as manual triggers. Record actual maintenance/build/native costs
  before proposing a framework switch; no scheduled monitor or mainline
  promotion is configured. Observation evidence lives in
  [the ledger](docs/plans/diva-next/mygo/observations.md).
