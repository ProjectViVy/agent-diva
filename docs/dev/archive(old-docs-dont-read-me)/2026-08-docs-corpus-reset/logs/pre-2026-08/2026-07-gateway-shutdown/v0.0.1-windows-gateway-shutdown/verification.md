# 验证记录

- `cargo test -p agent-diva-manager --lib runtime::shutdown::tests -- --nocapture`：通过，1 个 shutdown 超时回归测试通过。
- `cargo check -p agent-diva-cli`：通过。
- `cargo fmt --all -- --check`：通过。

待执行的人工 Windows 验收：启动 `just diva-gate`，按两次 Ctrl+C，确认 `cargo.exe` 与 `agent-diva.exe gateway run` 均退出，再重新构建 CLI。
