# Verification

- `pnpm test -- src/components/NormalMode.test.ts`
  - Passed: 9 tests.
- `pnpm exec vue-tsc --noEmit`
  - Passed.

Observations:

- The disconnected indicator renders only in the error state.
- The model dropdown still opens while the indicator is visible.
