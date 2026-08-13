# 验证记录

- `pnpm exec vue-tsc --noEmit`：通过。
- `pnpm vitest run src/components/NormalMode.test.ts --run`：通过，6 个测试。
- `git diff --check`：通过。
- 截图复核：确认问题来自滚动条槽位预留导致的导航内容宽度减少；移除预留后导航项恢复与菜单按钮同宽。
