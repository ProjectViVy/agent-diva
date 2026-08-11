# Verification

- `cargo fmt --all -- --check`：通过。
- `cargo check -p agent-diva-core -p agent-diva-agent -p agent-diva-manager`：通过。
- `cargo clippy -p agent-diva-core -p agent-diva-agent -p agent-diva-manager --all-targets -- -D warnings`：通过。
- `cargo test -p agent-diva-core -p agent-diva-agent`：通过；Core 704 tests，Agent 389
  unit tests，相关 integration/doc tests 全部通过。
- `cargo test -p agent-diva-agent --test compaction_integration`：5 项通过。
- `cargo test -p agent-diva-agent --test compaction_e2e`：3 项通过。
- deletion proof：非归档生产代码和测试中不存在旧压缩类型、旧 session 字段、
  `MetaCompactor`、多摘要 marker 或旧双入口。

- `just fmt-check`：通过。
- `just check`：通过。
- `just test`：等价的 `cargo test --all` 通过；随后 `just ci` 完整通过（含 feature gate、
  Laputa clean-break gate 和 health benchmark gate）。
- `just ci`：通过。
- `cargo run -p agent-diva-cli -- --help`：通过，CLI help 正常输出。
