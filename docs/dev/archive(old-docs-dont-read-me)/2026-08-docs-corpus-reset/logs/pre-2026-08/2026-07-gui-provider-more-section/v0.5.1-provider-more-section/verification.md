# Verification

- `pnpm vue-tsc --noEmit` passed for `agent-diva-gui`.
- `pnpm test` passed (358 tests); the only failure is a pre-existing `NormalMode.test.ts` issue unrelated to this change.
- Verified that the hidden provider set matches `agent-diva-providers/src/providers.yaml` internal names.
