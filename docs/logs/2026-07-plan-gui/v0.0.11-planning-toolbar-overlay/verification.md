# Verification

- `pnpm exec vue-tsc --noEmit` — passed.
- `pnpm vitest run src/components/NormalMode.test.ts src/components/ConversationSidebar.test.ts --run` — passed, 2 files and 9 tests.
- `git diff --check` — passed.
- Manual smoke path: open Chat, click the clipboard planning button in the toolbar, confirm the planning list/detail view appears in a backdrop dialog, then close it.
