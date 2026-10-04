# v0.4.10 verification (DN-4D)

- `pnpm vitest run src/api/cognitive.test.ts src/state/vivy-cognitive.test.ts
  src/components/persona-memory/PersonaMemoryView.test.ts`: 18 green.
  - api: all 20 captured fixture envelopes replay verbatim; module/
    action/prefix/mutation flags asserted; timeout→unknown (1 call,
    no replay); -32009 denials rethrown; malformed envelopes rejected;
    unsafe integers rejected before transport; recovery payload on
    failed owner.save preserved.
  - controller: humanSetupGate, casConflictRetainsDraft,
    unknownSaveNoReplay, currentVersusFrozen, scopeAndReceiptStates,
    unavailable-navigable, stale-session discard.
  - component: setup gate shown on uninitialized; capability_unavailable
    renders navigable notice.
- `just gui-test`: 67 files / 551 tests green. vue-tsc clean.
- `just gui-build`: green.
- `node scripts/ci/check_legacy_frontend_calls.mjs --selftest` + live:
  clean; no new native commands (cognitive goes over vivy_call).
- Pending: developer-native smoke (setup→primary-turn→edit→new-session→
  review/cancel/reopen) requires a live VIVY host — cannot run on this
  VM without the packaged shell; recorded as a gap, not evidence.
