# Verification

## 自动化

- WorkspaceChip、ChatView、NormalMode 与工作区相关 GUI 测试。
- `npm run build`，包含 `vue-tsc --noEmit` 与 Vite production build。

## 结果

- 首次 `npm test` 因根工作树 `node_modules` 的 `.bin/vitest.cmd` 与 `@babel/parser` 链接缺失而无法启动；不是测试断言失败。
- `pnpm install --frozen-lockfile` 使用本地缓存补回 7 个依赖链接，未修改 lockfile。
- `pnpm test -- --run ...` 通过：5 个测试文件、40 个测试全部通过。
- `pnpm run build` 通过：`vue-tsc --noEmit` 与 Vite production build 完成，2416 个模块转换成功。
- 构建仅保留既有的大 chunk warning；本次未改变分包策略。
- 真实 Windows Tauri 窗口中的最终视觉位置待用户验收。
