# Verification

通过：

- `npm test -- --run src/utils/pathDisplay.test.ts src/components/WorkspaceChip.test.ts src/components/settings/WorkspaceSettings.test.ts`
  （3 files / 15 tests passed）
- `npm run build`（Vue typecheck 与 Vite production build passed）
- `git diff --check`

覆盖 Windows drive verbatim path、verbatim UNC path、普通 Windows/POSIX path，以及
WorkspaceChip、设置页当前路径和候选路径的可见输出。
