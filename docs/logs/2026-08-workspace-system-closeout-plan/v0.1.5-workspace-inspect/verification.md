# Verification

通过：

- `cargo fmt --all`
- `cargo check -p agent-diva-gui`
- `cargo test -p agent-diva-gui workspace_switch_tests`（3 passed）
- `pnpm test -- --run src/components/settings/WorkspaceSettings.test.ts`（2 passed）
- `pnpm test -- --run`（69 个测试文件，491 项通过）
- `pnpm build`（`vue-tsc --noEmit` 与 Vite production build 均通过）
- `git diff --check`

新增 Rust 测试覆盖 canonical candidate、AGENTS metadata、空/不存在/文件路径拒绝和切换阻塞
合同；GUI 测试覆盖候选预览不覆盖当前 workspace，以及旧预检响应被新选择丢弃。
