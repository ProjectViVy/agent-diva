# W0-5 — Windows x64 leg, round 4 (GRANTS-CATALOG + COGNITIVE GUI drive)

Date: 2026-10-04 · Executor: Devin (Windows Server 2022, amd64) on behalf of mastwet
Pins: agent-diva `819cda37`+ (feat/wails-go-host) · agent-vivy `e3b60280` · laputa `6f2eed2`
Artifact: `artifacts/diva-go-host-r4/diva.exe` (same binary as w0-4; driven via the
identical `diva-go-host-r3dbg` build which only adds
`WindowsOptions.AdditionalBrowserArgs` for CDP access)

## Outcome

The two remaining driveable rows were exercised through the real sealed UI:

- **W5-T2-GRANTS-CATALOG → passed.** Provider catalog, model selector,
  permission presets, and the sandbox/grants surface in the GUI all render
  backend data verbatim — verified field-by-field against `settings/providers`,
  `settings/get`, and `tools/list` payloads (rpc-dump.log) plus the sealed
  `generation.json` module-action catalog.
- **W5-T3-COGNITIVE → passed.** Cognitive context projection and controls are
  live: persona view (status ready, doc revisions, frozen-core state, ACTMEM),
  memory view (search/mutate/receipt), evolution view (status/policy/trigger/
  cancel/results). Denied/invalid/unavailable outcomes surface explicitly
  (`invalid_schema`, `backend_unavailable` verbatim). Frozen-core lazy capture
  on first turn already proven in w0-4.

Fixture after w0-5: **15 passed / 2 pending / 0 failed** — checker OK.
Remaining pendings are environmental only: W5-T5-VOICE-REAL (no audio
endpoints on this VM) and the W5-WINDOWS-MATRIX rollup that waits on it.
