# Verification

- `cargo fmt --all -- --check` — passed.
- `cargo test -p agent-diva-core planning::store::tests::test_clear_active_plan_only_clears_matching_plan` — passed.
- `cargo test -p agent-diva-manager planning` — 5 passed.
- `cargo check -p agent-diva-gui` — passed.
- `pnpm vitest run src/components/planning/PlanApprovalCard.test.ts` — 2 passed.
- `pnpm test` — 42 files, 387 tests passed.
- `pnpm build` — passed.

The full `agent-diva-core` test run had one unrelated pre-existing failure in `supervised::executor::tests::test_executor_fails_when_no_handler`; 585 other tests passed.
