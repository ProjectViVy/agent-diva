# Acceptance

1. 检查四个 Clippy 报错位置只做等价机械重写。
2. 运行 `just fmt-check` 和 `just check`，应全部通过。
3. 运行 `cargo test -p agent-diva-core` 或确认 workspace 输出中的 701 个 core 测试全部通过。
4. 将 Windows Restricted Token 两项失败视为独立环境待办，不把它们归因于本次 Clippy 修复。
