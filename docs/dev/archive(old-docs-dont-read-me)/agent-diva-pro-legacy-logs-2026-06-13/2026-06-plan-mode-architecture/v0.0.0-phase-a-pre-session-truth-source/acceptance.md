# Acceptance

## User-Facing Acceptance Steps

- Open `docs/dev/agent-plan/phase-a-pre-session-truth-source-fix.md`.
- Confirm it defines backend session history as the authoritative source.
- Confirm it limits localStorage session cache to display optimization and
  backend-unavailable fallback.
- Confirm it covers backend-first `loadSession()`.
- Confirm it covers optimistic message reconciliation and failed-send cleanup.
- Confirm it covers cache invalidation on send, delete, reset, new session, and
  session switch.
- Confirm it defines tests and manual GUI smoke validation.
- Confirm `docs/dev/README.md` links to the document.

## Acceptance Result

Accepted when the above documentation checks pass and no code changes are
included in the scoped diff.
