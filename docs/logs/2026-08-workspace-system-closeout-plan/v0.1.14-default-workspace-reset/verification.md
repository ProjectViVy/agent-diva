# Verification

已完成：

- `npm test -- --run src/components/settings/WorkspaceSettings.test.ts src/components/WorkspaceChip.test.ts src/utils/pathDisplay.test.ts`
  — 3 个测试文件、17 个测试通过。
- `npm test` — 71 个测试文件、509 个测试通过。
- `npm run build` — `vue-tsc --noEmit` 与 Vite production build 通过；仅保留既有大 chunk 提示。
- `cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml --lib` — 56 个测试通过，覆盖默认
  目录稳定性、legacy 投影、session override 与回滚 source 解析。
- `just fmt-check`、`just check`、`just test` — 根工作区格式、Clippy（warnings denied）和完整
  测试门禁通过；测试阶段仅出现既有测试代码 unused-variable / future-incompat 警告。
- `git diff --check` — 通过。

桌面烟测：

- `npm run tauri -- dev` 成功启动 Tauri 与内嵌 Gateway。
- 实际监听端口的 `/api/workspace` 返回当前用户已保存目录
  `C:\Users\Administrator\Pictures`，source 为 `configured`；未修改该默认配置。
- 烟测产生的 GUI、Gateway 和 Vite 进程已停止。
