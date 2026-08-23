# Verification

- `cargo test -p agent-diva-cli --lib unified_list_parses`：通过。
- `cargo test -p agent-diva-cli --lib explicit_allow_once`：通过。
- `cargo test -p agent-diva-cli --lib explicit_queue_returns`：通过。
- 未宣称全仓库 `just test` 全绿（其它 flake 仍可能存在）。
