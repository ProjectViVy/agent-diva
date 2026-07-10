# Verification

- `pnpm exec vue-tsc --noEmit` — passed.
- `pnpm vitest run src/components/NormalMode.test.ts src/components/ConversationSidebar.test.ts --run` — passed, 2 files and 9 tests.
- Manual smoke path: active plan bar is rendered in the chat panel; clicking it expands task rows; clicking a task opens the centered backdrop status dialog; clicking the backdrop or close button dismisses it.
