# C1d Verification

## 定向验证

- `cargo test -p agent-diva-tooling -p agent-diva-core -p agent-diva-providers -p agent-diva-agent`：通过。
- `cargo clippy -p agent-diva-tooling -p agent-diva-core -p agent-diva-providers -p agent-diva-agent --lib -- -D warnings`：通过。
- Agent 408、Core 692、Provider 125、Tooling 32 个库测试通过；相关集成与 doc tests 通过。

## 工作区门禁

- `just fmt-check`：通过。
- `just check`：通过。
- `just test`：通过。首次运行受执行器 124 秒超时中断，使用 300 秒时限重跑后完整通过，耗时约 120 秒。
- `cargo run -p agent-diva-cli -- --help`：通过，CLI 正常输出帮助信息。
- `just ci`：格式、Clippy、全量测试、manager health benchmark 和 8 组 feature gate 均通过；最终仅在既有 `laputa-clean-break-check` 基线问题处失败，位置为 `agent-diva-laputa/src/bml/mod.rs:3`。

## 已知基线

- `just ci` 如预期命中既有 `LAPUTA-CLEAN-BREAK-BML-DOCSTRING`。本迭代未放宽 gate，
  也未混入与 C1d 无关的术语修复；该事项继续由根 `TODOLIST.md` 跟踪。
