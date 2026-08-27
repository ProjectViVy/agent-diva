# Verification

通过：

- `npm test -- --run`（69 个测试文件，491 项通过）
- `npm test -- --run src/components/ConversationSidebar.test.ts src/components/ChatView.test.ts src/components/NormalMode.test.ts`（32 passed）
- `npm run build`（`vue-tsc --noEmit` 与 Vite production build 均通过）
- `git diff --check`

新增测试覆盖 workspace/channel/child 三层显示、child 搜索时保留 parent ancestor，以及
旧 flat session props 的 root 兼容表现。
