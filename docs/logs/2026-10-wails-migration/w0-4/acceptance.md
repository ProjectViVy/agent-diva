# W0-4 acceptance notes

Fixture: `docs/plans/diva-next/fixtures/wails-candidate-acceptance.json`
Checker: `python scripts/ci/check_wails_candidate.py <fixture> --build-report artifacts/diva-go-host-r4/build-report.json --binary artifacts/diva-go-host-r4/diva.exe`

Result: **13 passed / 4 pending / 0 failed — OK**

## Passed (Windows x64, w0-2…w0-4 cumulative)

- W5-T1-CANDIDATE, W5-T2-FRESH-PROFILE — sealed artifact pins + fresh profile boot
- W5-T2-FIRST-TURN — first GUI turn passes frozen-core prepare (lazy capture), reaches model call, renders
- W5-T3-CHAT-MATRIX — text multi-turn, echo+reply render, reload resync, restart restore (scope: image/approval/edit/rewind lanes not driven)
- W5-T4-SINGLE-INSTANCE, W5-T4-CRASH-RESTART, W5-T4-CLEAN-QUIT, W5-T4-HIDE-REOPEN, W5-T4-WEBVIEW-RELOAD — lifetime matrix
- W5-T4-EVENT-LOSS — reload/restart resync with zero duplicates (scope: overflow/timeout lanes on Go-level w4-1 evidence)
- W5-T5-LOGS, W5-T5-VOICE-SCRIPTED, W5-T5-VOICE-LIFECYCLE

## Pending (with reason)

- W5-T2-GRANTS-CATALOG — dispatch-table coverage proven via probe; denied-tool boundary drive not exercised in GUI this leg (owner E2E)
- W5-T3-COGNITIVE — lazy capture now auto-fires (gate gone); cognitive projection/controls matrix not driven (owner E2E)
- W5-T5-VOICE-REAL — this VM exposes no audio endpoints; needs an audio-capable machine
- W5-WINDOWS-MATRIX — rollup pending only on the audio gap; all other lanes proven

## Findings closed

- F1 embedded UI (vivy 42c263f2) — verified w0-3
- F2/F3 tray AttachWindow + Show/Quit (diva 8db11f51) — verified w0-3
- F5 frozen-core gate (vivy e3b60280 lazy capture) — verified w0-4
- F6 mount-time `_wails.environment` race (diva dda9d7da) — verified w0-4
