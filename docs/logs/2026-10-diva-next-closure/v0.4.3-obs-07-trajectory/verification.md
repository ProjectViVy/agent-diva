# Verification — OBS-07

- `pnpm --dir agent-diva-gui exec vitest run src/state/vivy-trajectory.test.ts src/components/console/TrajectoryPanel.test.ts src/state/vivy-chat.test.ts`
  -> 3 files, 41 tests, all green.
  Plan-named cases: stableIdsOnSnapshotReplay, callFinishBeforeMessage,
  waitVsActiveCall, gapRefetchSingleOwner, lateSessionResponseIgnored.
- `just gui-test` -> 59 files / 497 tests green.
- `npx vue-tsc --noEmit` -> clean (exit 0).
- `just gui-build` -> green; pre-existing >500kB chunk warning unchanged.

Fixture source: `docs/plans/diva-next/fixtures/closure-chat-obs.json`
requests_responses[50-52] (trajectory/session, sess_missing, child/list).
