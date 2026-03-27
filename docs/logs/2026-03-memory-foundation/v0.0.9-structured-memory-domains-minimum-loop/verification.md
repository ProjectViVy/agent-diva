# v0.0.9 Verification

## Validation Commands

当前环境下 `just` 仍因 shell 配置问题无法直接执行，本轮继续使用等价 `cargo` 命令验证。

- `cargo fmt`
- `cargo test -p agent-diva-memory --lib`
- `cargo test -p agent-diva-agent diary:: -- --nocapture`
- `cargo fmt --all -- --check`
- `cargo clippy --all -- -D warnings`
- `cargo test --all`

## Results

- `cargo fmt`：通过
- `cargo test -p agent-diva-memory --lib`：通过
- `cargo test -p agent-diva-agent diary:: -- --nocapture`：通过
- `cargo fmt --all -- --check`：通过
- `cargo clippy --all -- -D warnings`：通过，保留现有 workspace 级 MSRV/future-incompat 提示，不构成本轮失败
- `cargo test --all`：通过

## Focused Coverage

- rational diary 可提炼 `relationship` / `self_model` / `soul_signal`
- crate 名称如 `agent-diva-memory` 不会误触发 `self_model`
- `sync_diary_entry_to_sqlite()` 会连带写入提炼出的结构化 memory
- `backfill_workspace_sources()` 可重建这些 record，且幂等
- `memory_recall` / `memory_search` 可直接按新 domain 召回对应记录

## Notes

- `just fmt-check`、`just check`、`just test` 在当前环境均因 `just` 找不到配置 shell 而失败，这属于环境问题，不是代码问题。
