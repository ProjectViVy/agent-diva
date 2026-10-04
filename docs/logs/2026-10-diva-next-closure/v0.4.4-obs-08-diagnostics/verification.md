# Verification — OBS-08

- `pnpm --dir agent-diva-gui exec vitest run src/state/gui-diagnostics.test.ts src/components/console/DiagnosticsPanel.test.ts`
  -> 2 files, 14 tests, all green.
  Plan-named: partialAppendUnknownNoRetry, readerErrorNoRecursion,
  rotationGapVisible, queueBoundsAndDropCount, speechSentinelRedacted.
- `just gui-test` -> 61 files / 511 tests green.
- `npx vue-tsc --noEmit` -> clean.
- `just gui-build` -> green (pre-existing chunk-size warning only).
- Fixtures: closure-chat-obs.json seq 55-66 (append ack, logs page,
  invalid source/cursor, oversized-batch error, null records).
- NOT verified here (deferred): real filesystem rotation/reopen smoke on
  device — OBS-09 / developer smoke owns it.
