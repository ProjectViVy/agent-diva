# W0-4 — Windows x64 leg, round 3 (F5 verify + F6 root cause & fix)

Date: 2026-10-04 · Executor: Devin (Windows Server 2022, amd64) on behalf of mastwet
Pins: agent-diva `dda9d7da` (feat/wails-go-host) · agent-vivy `e3b60280` (feat/wails-migration) · laputa `6f2eed2`

## Outcome

Both round-2 blockers are resolved and verified on the sealed Windows candidate:

- **F5 (sealed `turn/start` → "no captured frozen core")** — fixed upstream in
  vivy `e3b60280` (`fix(diva-cognitive): lazily capture frozen core on first Prepare`).
  Verified live: the first GUI send on a fresh session wrote a
  `frozen_core_sessions` row at first Prepare (`sess_4d13a567743cd881`,
  captured_at 2026-10-04T08:36:22Z), the turn reached the model call
  (mock provider), `run_a81e8bf6` completed, both bubbles rendered.
- **F6 (GUI never binds/renders session)** — root-caused via WebView2 CDP and
  fixed in agent-diva `dda9d7da`. The defect was a mount-time startup race,
  not an RPC rejection (see verification.md for the exact mechanism and error).

## Rows moved this leg

| Row | w0-3 | w0-4 |
|-----|------|------|
| W5-T2-FIRST-TURN | failed | **passed** |
| W5-T3-CHAT-MATRIX | failed | **passed** (scope in fixture evidence) |
| W5-T4-EVENT-LOSS | pending | **passed** (scope in fixture evidence) |
| W5-WINDOWS-MATRIX | pending | pending (audio gap only) |

Fixture after w0-4: **13 passed / 4 pending / 0 failed** — checker OK.

## Residuals

- `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` env var is not honored by the wails3
  loader path; CDP access required a temporary
  `WindowsOptions.AdditionalBrowserArgs` patch (reverted before commit).
- CHAT-MATRIX sub-lanes not exercised on Windows: image input, approval
  accept/reject, edit/regenerate, rewind conflict.
- VOICE-REAL still pending — this VM exposes no audio endpoints.
