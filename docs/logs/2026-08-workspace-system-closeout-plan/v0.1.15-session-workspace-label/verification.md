# Verification

## 自动化验证

- `cargo fmt --manifest-path agent-diva-gui/src-tauri/Cargo.toml`
- `cargo fmt --manifest-path agent-diva-manager/Cargo.toml`
- `cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml workspace_switch_tests --lib` — 7 passed
- `cargo test --manifest-path agent-diva-manager/Cargo.toml handlers::workspace::tests --lib` — 5 passed
- `npm test -- --run src/components/WorkspaceChip.test.ts src/components/settings/WorkspaceSettings.test.ts src/composables/useWorkspaceContext.test.ts` — 17 passed
- `npm test` — 72 files / 512 tests passed
- `npm run build` — `vue-tsc` and Vite production build passed
- `just fmt-check`、`just check`、`just test` — 通过；全量 Rust 测试无失败（既有 warnings / ignored tests 保持不变）

## Windows Tauri smoke

- `AGENT_DIVA_EXTERNAL_GATEWAY=0; npm run tauri -- dev` — embedded GUI/Gateway started successfully.
- `GET http://127.0.0.1:<ephemeral>/api/workspace` — HTTP 200，返回
  `source: configured`、`uses_default_workspace: true` 及 profile-local Diva workspace 根目录。
- 冒烟进程已按精确 PID 停止，端口释放。

## 验收观察点

- 默认目录重置后，活动旧 runtime 仍在 `Pictures` 时，接口返回 `source: configured`、`uses_default_workspace: false`，入口显示 `Pictures`。
- 选择当前已运行目录时，Tauri 返回 `source: explicit-cli`、`uses_default_workspace: false`；后续刷新不会恢复“默认工作区”标签。
- 设置页刷新当前 session 状态不会改写默认目录之外的 runtime/session authority。
