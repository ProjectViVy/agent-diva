# TODOLIST

Active gaps and deferred work for DIVA Next. Stage state/dependencies live
only in [the existing index](docs/plans/diva-next/index.md); current scope and
closure rules are [DN-C2](docs/plans/diva-next/p0-design.md).
Severity: P0 blocking correctness, P1 high, P2 medium, P3 low.

## Closure work

The [DN-C2-P1 execution package](docs/plans/diva-next/index.md#executable-story-package)
maps these gaps to 20 concrete Plans. Planning is complete; all implementation,
captured contract/native/provider probes and owner acceptance remain unfinished.
Four independent roots are plan-Ready; other readiness lives only in that index.

- [ ] **COGNITIVE-COMPOSITION** P1 — Laputa #2 and VIVY #26 provide real
  foundations, but DIVA's recipe has no bound domain. DN-C2 specifies the
  same-owner Garden facade extension, generated factory/context/capture/actions and
  trusted human origin; implement and prove the existing runtime binding.
  No duplicate ACTMEM or automatic BML/backend switch. See DN-4 and Laputa S08.
- [ ] **COGNITIVE-EMBEDDED-LIFECYCLE** P1 — VIVY `StartEmbeddedServices()` starts
  sweeper/cron but not the new cognitive loop. Start selected cognition once;
  stop/drain before cancelling runs and closing storage; prove restart dedupe.
- [ ] **COGNITIVE-AUTHORITY-AND-RECOVERY** P0 — Current binding pins Mission
  at Service construction and failed runs may receive a new automatic attempt.
  Add per-run persisted authority/guard and durable unknown-outcome block;
  serialize admission/state reconciliation. Never replay ambiguous ACTMEM /
  Persona effects with a new key. DN-C2 sections 4.3–4.4.
- [ ] **COGNITIVE-SUPERVISOR-CAPTURE** P1 — Supervisor uses primary kind while
  current capture filters only non-primary kinds. Explicitly exclude its
  stored origin/session from capture and automatic speech; prove restart
  cannot feed strategy output back as fresh evidence. Source risk, not a
  reproduced runtime failure in this design iteration.
- [ ] **PINNED-NATIVE-BUILD-CLOSURE** P1 — VIVY/Garden/Laputa have sibling
  replacements (Laputa/Mentle/INOFY). Stage/hash exact transitive source graph;
  inspect native CGO/tokenizer/ONNX dependencies and loading on target. A
  missing local model does not prove link-time dependencies absent. DN-C2 §9.
- [ ] **CONSOLE-OBS-CONSUMERS** P1 — VIVY #27 producers exist; ConsoleView is
  still token-only. Consume actual `trajectory/session`, diagnostics and usage
  coverage via the typed client; preserve one subscription owner and gap /
  rotation behavior. Reuse OBS-06..09.
- [ ] **CHAT-ATTACHMENTS** P1 — App.vue warns/drops FileAttachmentDto; latest
  VIVY accepts image payloads. Wire actual bytes/MIME with ABI-size checks and
  explicit unsupported-file UI; never silently discard an attached file.
- [ ] **CHAT-CONTROLS** P2 — `_permissionMode` is ignored; regenerate appends
  the last user turn; start_goal has no visible entry; restart cancel needs
  authoritative semantics. Existing VIVY set_permission/rewind/work contracts
  must be verified and wired, not described as missing producers.
- [ ] **ONLINE-SPEECH-NATIVE** P1 — Reuse existing SiliconFlow STT and
  SiliconFlow/MiniMax TTS wrappers, replacing dead pet_* config/commands and
  frontend cloud fetch. DN-C2 specifies native Raw WAV/MP3, OS credentials
  and exact commands. Implement/probe provider/native fixtures; correlate
  request/run/utterance/generation; abort/drop late output,
  release audio and prevent history replay speech. Text mode remains usable.
- [ ] **VOICE-ASSET-BOUNDARY** P2 — DN-C2 specifies bounded reference-voice
  import/read/delete and leases; implement/probe it. General VRM asset management stays deferred.
  This is new media input, not cancelled historical database migration.
- [ ] **RUNTIME-REPIN-AND-GATES** P1 — Bundled Generation `1fd14fb2…` predates
  new cognitive/OBS integration. Repack/inspect/pin through VIVY; refresh
  acceptance. Amend AST/command/dep/package gates for narrow native speech;
  activated speech leaves the blanket dormant pet exemption.
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
  shell/bridge lockfiles; no fresh cargo audit has been run. Keep a clear
  current-dependency audit follow-up, without claiming the old versions are
  still shipped or that the whole new package is vulnerability-free.

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
