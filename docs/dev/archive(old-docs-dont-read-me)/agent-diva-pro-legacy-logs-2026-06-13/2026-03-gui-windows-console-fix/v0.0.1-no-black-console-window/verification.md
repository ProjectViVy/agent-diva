# 验证记录

- `cargo fmt --all --check`
- `cargo check -p agent-diva-gui`
- `cargo test -p agent-diva-gui`

## 结果

- `cargo fmt --all --check`：通过。
- `cargo check -p agent-diva-gui`：通过。
- `cargo test -p agent-diva-gui`：通过（当前 crate 无实际测试用例，结果为 0 tests）。

## 手动 smoke

- 需要在 Windows 打包版 GUI 中手动验证：
  - 打开设置中的网关/服务控制入口。
  - 点击启动网关，确认桌面不再弹出黑色控制台窗口。
  - 执行服务状态查询、服务启动、服务停止，确认不再弹出黑窗。
- 当前仓库环境未执行真实打包 GUI 交互 smoke，因此该项仍需人工确认。
