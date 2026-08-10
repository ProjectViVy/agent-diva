# C1c Verification

## 定向验证

- `cargo test -p agent-diva-agent context::tests::c1c -- --nocapture`：6 passed。
- `cargo test -p agent-diva-laputa memory_add_visible_in_next_startup_rendering -- --nocapture`：1 passed。
- `cargo test -p agent-diva-core -p agent-diva-laputa -p agent-diva-agent`：通过；Agent
  库 403 tests 全绿，Laputa 性能门按既有标记 ignored。
- `cargo clippy -p agent-diva-core -p agent-diva-laputa -p agent-diva-agent --lib -- -D warnings`：通过。

## 已知基线

- 三 crate `--all-targets -D warnings` 会命中 Laputa 既有测试目标的 dead-code 与
  `cmp_owned` 告警；与本迭代差异无关，库级严格 Clippy 通过。

## 工作区门禁

- `just fmt-check`：通过。
- `just check`：通过（根 workspace 全成员 Clippy，warnings denied）。
- `just test`：通过（`cargo test --all`，exit 0）。
- `cargo run -p agent-diva-cli -- --help`：通过，帮助页正常输出并退出 0。

## CI 汇总

- 最终 `just ci` 中 fmt、workspace Clippy、`cargo test --all`、Manager benchmark 和
  feature gate 均通过；最后 `laputa-clean-break-check` 因当前 HEAD 已存在的
  `agent-diva-laputa/src/bml/mod.rs:3` 退休术语命中而返回 1。
- 单独重跑 `just laputa-clean-break-check` 稳定复现同一命中；该文件不在本迭代差异中，
  已登记 `LAPUTA-CLEAN-BREAK-BML-DOCSTRING`，未放宽 gate 或混入无关修复。
