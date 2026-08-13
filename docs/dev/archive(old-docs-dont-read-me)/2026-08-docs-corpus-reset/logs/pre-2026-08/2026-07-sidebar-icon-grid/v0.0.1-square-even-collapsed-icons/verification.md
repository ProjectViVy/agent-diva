# 验证记录

- `pnpm exec vue-tsc --noEmit`：通过。
- `pnpm vitest run src/components/NormalMode.test.ts --run`：通过，6 个测试。
- `git diff --check`：通过。
- 样式核对：折叠态入口均使用 40×40 方形盒，图标使用 24×24 固定尺寸。
