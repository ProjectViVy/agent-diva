# Verification

## 自动化

- 运行 `WorkspaceChip.test.ts`，覆盖 `process-cwd`、`legacy-default`、`configured` 与 `explicit-cli`。
- 运行 GUI production build，验证类型与模板编译。

## 结果

- `pnpm test -- --run src/components/WorkspaceChip.test.ts src/components/NormalMode.test.ts` 通过：2 个文件、14 个测试。
- `pnpm run build` 通过：`vue-tsc --noEmit` 与 Vite production build 完成，2416 个模块转换成功。
- 构建仅保留既有的大 chunk warning；本次未改变分包策略。
