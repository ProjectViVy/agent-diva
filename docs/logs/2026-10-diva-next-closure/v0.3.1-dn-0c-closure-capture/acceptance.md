# Acceptance — v0.3.1 DN-0C

## Self-review evidence

- Fixture produced by the pinned VIVY source (`1db8b55`), `capture_kind`
  honestly labelled `source_host` — it is not bundled-runtime proof and
  does not claim to be.
- All 25 closure assertions embedded in the fixture are `ok: true`.
- `go test ./internal/app ./internal/rpc -count=1` is green at the pin.
- Fixture is valid JSON and contains no secrets/audio/raw-thinking.

## Honest limitations

- Provider traffic is the scripted `newPlanGoalScriptServer` wire; this is
  the sanctioned source-host harness, not a live-provider capture.
- `diagnostics/logs` stale-cursor behavior is captured as observed in the
  transcript (see record for `after: bogus-cursor`); no semantic asserted
  beyond what the wire returned.
- Zero `notifications` were captured on the client peer in this run; the
  fixture records that fact rather than fabricating frames.

## Owner acceptance

Pending. Final acceptance belongs to the owner; this capture is evidence,
not acceptance.
