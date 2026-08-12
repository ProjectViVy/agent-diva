# Verification

- `just fmt-check`：通过。
- `just check`：通过。
- `cargo test -p agent-diva-agent --lib`：397 passed。
- `cargo test -p agent-diva-core --lib`：通过，含 ContextCompaction serde roundtrip。
- GUI targeted tests：12 passed。
- `npm run build`：通过。
- `cargo run -p agent-diva-cli -- --help`：CLI 用户入口 smoke 通过。
