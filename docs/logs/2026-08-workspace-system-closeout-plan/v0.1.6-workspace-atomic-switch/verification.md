# Verification

通过：

- `cargo fmt --all -- --check`
- `cargo check -p agent-diva-gui`
- `cargo test -p agent-diva-gui workspace_switch_tests`（3 passed）
- `pnpm test -- --run src/components/settings/WorkspaceSettings.test.ts`（3 passed）
- `pnpm test -- --run`（最终门禁在提交前复跑）
- `pnpm build`（`vue-tsc --noEmit` 与 Vite production build 均通过）
- `git diff --check`

Rust focused tests 覆盖 unsafe runtime guard、候选失败路径和 canonical/AGENTS 预检；GUI focused
tests 覆盖阻塞时不触发 commit。完整切换成功与重建失败回滚依赖 release Tauri embedded runtime，
列入 WS-06 真机 smoke，不在 debug 外部 gateway 中伪造通过。
