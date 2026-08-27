# Verification

## 自动化

- WorkspaceChip 测试验证按钮发出目录选择意图。
- NormalMode 测试验证目录选择 → candidate inspect → 权威 switch action 的调用顺序与路径。
- GUI production build 验证类型、模板与 CSS。

## 结果

- 首次测试暴露 `NormalMode.test.ts` 缺少 WorkspaceChip 弹层图标 mock；补齐夹具后重跑通过，产品逻辑未因此改变。
- `pnpm test -- --run src/components/WorkspaceChip.test.ts src/components/NormalMode.test.ts src/components/settings/WorkspaceSettings.test.ts` 通过：3 个文件、19 个测试。
- `pnpm run build` 通过：`vue-tsc --noEmit` 与 Vite production build 完成，2416 个模块转换成功。
- 构建仅保留既有的大 chunk warning；本次未改变分包策略。
