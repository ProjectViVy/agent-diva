# W0-2 Acceptance — row-by-row outcome (Windows x64 leg)

Fixture `docs/plans/diva-next/fixtures/wails-candidate-acceptance.json`
was updated to this candidate and re-validated with
`python scripts/ci/check_wails_candidate.py` (see verification.md).

| row | outcome | Windows evidence |
|-----|---------|------------------|
| W5-T1-CANDIDATE | passed | diva.exe built at diva 81c381f8 / vivy 3862b768 / laputa 6f2eed2; sha256 + generation recorded |
| W5-T2-FRESH-PROFILE | passed | `%APPDATA%\DIVA\vivy.yaml` seeded, `~/.vivy` workspace created, `vivy host opened` |
| W5-T2-FIRST-TURN | pending | no model credentials + F1: sealed binary cannot render the UI to drive a turn |
| W5-T2-GRANTS-CATALOG | pending | same harness need as FIRST-TURN |
| W5-T3-CHAT-MATRIX | pending | requires working UI (F1) |
| W5-T3-COGNITIVE | pending | requires working UI (F1); `garden/` dir created on boot |
| W5-T4-SINGLE-INSTANCE | passed | second launch → `organism lease held` (-32086) and exits |
| W5-T4-CRASH-RESTART | passed | `taskkill /F` → TTL-gated rejection → post-TTL `vivy host opened`, same profile |
| W5-T4-CLEAN-QUIT | passed | CTRL_BREAK → `Quitting application...` → speech teardown → `teardown complete`; lease released, instant relaunch |
| W5-T4-HIDE-REOPEN | **failed** | hide works (X + WM_CLOSE; runtime survives, tray icon present) but no reopen affordance exists — tray unattached/no menu; second instance blocked by lease before `OnSecondInstanceLaunch` |
| W5-T4-WEBVIEW-RELOAD | pending | live F5 driven — reloads, runtime stable; context invalidation/turn dedup needs real frontend (F1) |
| W5-T4-EVENT-LOSS | pending | host-level queue-loss drive unchanged (w4-1 Go-level evidence stands) |
| W5-T5-LOGS | passed | `.vivy/logs/vivy.log.2026-10-04` written during Windows runs |
| W5-T5-VOICE-SCRIPTED | passed | `go test -tags vivy_headless ./internal/speech` green on Windows |
| W5-T5-VOICE-REAL | pending | no audio endpoints (Audiosrv disabled, zero devices) + operator credentials needed |
| W5-T5-VOICE-LIFECYCLE | passed | unchanged (w4-1 service-level evidence) |
| W5-WINDOWS-MATRIX | pending | leg ran end-to-end: keyring green, lifetime mostly proven; blocked by F1/F2/F3 + F4 (see summary) |

Probe fixture `wails-native-probe.json` updates: `keyring.
windowsCredentialManager` → PASS (wincred + real namespace verified via
cmdkey, no plaintext), `keyring.lockedStore` → not exercisable (wincred
has no lock surface; errors map to credential_unavailable), `lifetime`
rows annotated with the Windows outcomes, `windowsX64.status`/`verdict`
updated.
