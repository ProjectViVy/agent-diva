# Verification

- `just fmt-check`：通过。
- `just check`：通过（workspace Clippy `-D warnings`）。
- `cargo test -p agent-diva-agent --lib`：397 passed。
- `cargo test -p agent-diva-providers --lib`：通过。
- `cargo test -p agent-diva-core --lib`：通过。
- 定向 `tool_results` 测试：materialization failure、成功幂等与边界场景通过。

一次 180s 独立 `just test` 尝试在 Windows 全量 doctest 阶段被命令超时关闭了 pipe；
不是代码测试断言失败。随后最终 `just ci` 完整通过（含 workspace test、feature gate
和 BML clean-break gate）。
