# Verification

- `cargo fmt --all -- --check` — passed.
- `cargo test -p agent-diva-manager --lib` — passed, 60 tests.
- `pnpm exec vue-tsc --noEmit` — passed.
- `pnpm vitest run src/components/NormalMode.test.ts src/components/ConversationSidebar.test.ts --run` — passed, 2 files and 9 tests.
