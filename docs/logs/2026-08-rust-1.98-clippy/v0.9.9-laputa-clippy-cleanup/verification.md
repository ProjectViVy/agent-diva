# Verification

- `just fmt-check`：通过。
- `just check`：通过，workspace Clippy `-D warnings` 通过。
- `cargo test -p agent-diva-laputa --lib`：32 passed。
- `just test`：通过；workspace 测试与 doc-tests 均无失败。
