# Verification

- `pnpm vitest run src/components/planning/PlanApprovalCard.test.ts src/components/planning/PlanHistoryCard.test.ts`: passed, 2 files and 5 tests.
- `pnpm exec vue-tsc --noEmit`: passed.
- `pnpm vitest run`: passed, 44 files and 391 tests.
- `pnpm build`: passed; production build completed. Existing bundle-size warnings remain.
- `git diff --check`: passed.
