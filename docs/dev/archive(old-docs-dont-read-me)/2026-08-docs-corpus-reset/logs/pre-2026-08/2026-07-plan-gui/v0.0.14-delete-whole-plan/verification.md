# Verification

- `pnpm exec vue-tsc --noEmit` — passed.
- `pnpm vitest run src/components/NormalMode.test.ts src/components/ConversationSidebar.test.ts --run` — passed, 2 files and 9 tests.
- `cargo fmt --all -- --check` — passed.
