# Verification

- `cargo check -p agent-diva-core -p agent-diva-manager`：通过。
- `cargo check -p agent-diva-agent -p agent-diva-cli -p agent-diva-e2e`：通过。
- `cargo check -p agent-diva-gui`：通过。
- `pnpm vitest run`（`agent-diva-gui`）：41 files / 385 tests passed。

GUI 测试仍报告已有 locale 重复 `mode` key 警告，已记录到 `TODOLIST.md`，未混入本次恢复提交。
