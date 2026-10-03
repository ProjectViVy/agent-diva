# TODOLIST

Active gaps and deferred work for DIVA Next. Stage state/dependencies live
only in [the existing index](docs/plans/diva-next/index.md); current scope and
closure rules are [DN-C1](docs/plans/diva-next/p0-design.md).
Severity: P0 blocking correctness, P1 high, P2 medium, P3 low.

## Closure work

- [ ] **COGNITIVE-COMPOSITION** P1 — Laputa #2 and VIVY #26 provide real
  foundations, but DIVA's recipe has no bound domain. Freeze the minimum
  exported Garden/Laputa facade, context/capture/domain actions and trusted
  human identity; wire the existing runtime binding. No duplicate ACTMEM or
  automatic BML/backend switch. See DN-4 and Laputa S08.
- [ ] **COGNITIVE-EMBEDDED-LIFECYCLE** P1 — VIVY `StartEmbeddedServices()` starts
  sweeper/cron but not the new cognitive loop. Start selected cognition once;
  stop/drain before cancelling runs and closing storage; prove restart dedupe.
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
  frontend cloud fetch. Freeze native credential/media transport and command
  fixtures; correlate request/run/utterance/generation; abort/drop late output,
  release audio and prevent history replay speech. Text mode remains usable.
- [ ] **VOICE-ASSET-BOUNDARY** P2 — Existing reference voice and VRM media
  paths need a scoped native import/read/delete/lifetime contract where used.
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
  Cargo recipes. DN-C1 records the conflict; instruction-file rewrite needs
  explicit authorization. Do not silently reuse obsolete rules for new code.
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

## Recorded delivery and cancellation

- [x] Initial shared host/bridge/core/settings/masks delivery: DIVA #16 and
  VIVY #28 merged; prior Linux evidence retained at its original pins.
- [x] **HISTORICAL-IMPORT-CANCELLED** — Owner decision 2026-10-03: no old data
  import, because there are no current DIVA users. DN-7 cancelled and removed
  from DN-8 prerequisites; do not build an importer or delete old data.
- [x] Previous mixed backlog preserved as an explicit historical snapshot:
  [2026-10-03-before-next-closure.md](docs/archive/todolist/2026-10-03-before-next-closure.md).
  Old Rust-specific tests/tasks are not active Next work by default; product
  residuals above remain visible. Historical boxes are not new acceptance.
