# v0.3.1 — DN-0C closure chat/OBS contract capture

## What changed

- VIVY (`agent-vivy` @ `1db8b55`, branch `feat/dn-closure-wave1`):
  `internal/app/dn0_capture_test.go` gains `TestDN0CaptureClosureTranscript`,
  a source-host transcript capture for every closure-relevant chat, work and
  OBS producer contract. Historical `TestDN0CaptureCoreTranscript` /
  `fixtures/core-rpc.json` are untouched.
- DIVA (this repo): new redacted fixture
  `docs/plans/diva-next/fixtures/closure-chat-obs.json`
  (`capture_kind: source_host`, `vivy_pin`, `protocol_version: vivy.rpc.v1`,
  110 ordered request/response/error records + 25 embedded assertions).
- Ledger C2-1 gained a "DN-0C closure capture — source-pinned semantics"
  table mapping each frozen contract to its owning Go symbol.

## Scope covered

turn/start image bytes + MIME/base64 errors, session/set_permission enum,
inclusive session/rewind (busy conflict + invalid cutoff), session/edit
atomic text edit (+ missing-message error), session/fork inclusive copy,
run/cancel terminal-not-found + reopen-via-fresh-turn, plan/decide
`start_goal` (resume plan run → admitted Goal round → report_goal),
trajectory/session v2 (+ unknown-session empty projection), stats/tokens v2
coverage (+ invalid period), diagnostics gui append/read bounds (+ bad
source, >500 batch, cursor probe), child/list + child/get read DTOs.

## Rulings observed (recorded in C2-1)

- No `reopen` RPC exists; a fresh `turn/start` after a cancelled run is the
  admitted reopen path.
- `trajectory/session` on an unknown session returns an empty v2 projection,
  not an error.
- `session/edit` on a missing message_id surfaces `InternalError` (-32603),
  not a typed not-found.
- `plan/decide` resumes the interrupted plan run with a reviewer
  tool-result; the Goal driver then admits a separate round run — only that
  run may `report_goal`.
