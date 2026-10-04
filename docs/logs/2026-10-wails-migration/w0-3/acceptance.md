# W0-3 Acceptance — Windows x64 row outcomes, round 2

Fixture: `docs/plans/diva-next/fixtures/wails-candidate-acceptance.json`
(updated in this leg).

## Rows moved

| row | outcome | evidence |
|-----|---------|----------|
| W5-T4-HIDE-REOPEN | **passed** | tray AttachWindow + Show/Quit menu live: click toggles reopen, Quit releases lease; clean relaunch accepted instantly |
| W5-T4-WEBVIEW-RELOAD | **passed** | real Vue UI embedded and rendered; F5 reload re-renders, runtime unaffected |
| W5-T2-FIRST-TURN | **failed** | F5: sealed `turn/start` on every fresh session → `runtime: prepare session authority: no captured frozen core for this session`; run row never created. Post-capture workaround completes turns via mock (DB-verified) |
| W5-T3-CHAT-MATRIX | **failed** | F6: runs complete + persist (vivy.db), but GUI renders no messages/echo and `connect()` never binds a session (status stuck "连接中"; post-restart send created a new session) |

## Rows still pending (honest)

| row | reason |
|-----|--------|
| W5-T2-GRANTS-CATALOG | `tools/list` + governance catalog verified on the shared dispatch table (embedded probe); sealed-GUI drive blocked by F6 |
| W5-T3-COGNITIVE | garden + persona store created; `persona.Initialize` + lazy `SessionProvider.Get` capture proven via module-level probe; full matrix blocked by F5/F6 |
| W5-T4-EVENT-LOSS | `run/subscribe` accepted with seq-replay (`sub_…`); live GUI event lane cannot be exercised while F6 persists |
| W5-T5-VOICE-REAL | VM has zero audio endpoints (unchanged from w0-2); needs audio-capable machine + operator credentials |
| W5-WINDOWS-MATRIX | leg executed; rollup reflects F5/F6 as the remaining blockers |

## GO/NO-GO

- Regression gate for upstream F1/F2/F3 fixes: **GO** — all three verified fixed on the real Windows desktop.
- Candidate chat acceptance (W5-T2/T3 drive rows): **NO-GO** — F5 and F6 are reproducible, platform-independent blockers with root cause localized to the sealed cognitive-assembly path and the GUI binding/projection path respectively.
