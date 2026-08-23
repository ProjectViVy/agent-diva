# Verification

- `cargo clippy -p agent-diva-laputa --all-targets -- -D warnings`：通过。
- `cargo fmt -p agent-diva-laputa`：已跑。
- `cargo test -p agent-diva-laputa`：32 lib + 5 集成套件通过，`typed_store`
  1 ignored（既有 10k 性能门）。
- `just bml-boundary-check` 等价于 `--test bml_boundary_guard`，该套件 1/1 通过。
- 未跑全仓库 `just test`（本 slice 只动 laputa 测试 helper）。
