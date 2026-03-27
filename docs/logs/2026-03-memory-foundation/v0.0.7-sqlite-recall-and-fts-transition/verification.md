# v0.0.7 Verification

## Validation Commands

当前 macOS 环境下 `justfile` 仍依赖 `powershell.exe`，本轮继续使用等价 `cargo` 命令验证。

- `cargo fmt --all`
- `cargo test -p agent-diva-memory -- --nocapture`
- `cargo test -p agent-diva-agent diary:: -- --nocapture`
- `cargo test -p agent-diva-agent loop_turn:: -- --nocapture`
- `cargo test -p agent-diva-tools memory:: -- --nocapture`
- `cargo fmt --all -- --check`
- `cargo clippy --all -- -D warnings`
- `cargo test --all`

## Results

- `cargo fmt --all`：通过
- `cargo test -p agent-diva-memory -- --nocapture`：通过
- `cargo test -p agent-diva-agent diary:: -- --nocapture`：通过
- `cargo test -p agent-diva-agent loop_turn:: -- --nocapture`：通过
- `cargo test -p agent-diva-tools memory:: -- --nocapture`：通过
- `cargo fmt --all -- --check`：通过
- `cargo clippy --all -- -D warnings`：通过
- `cargo test --all`：通过

## Focused Coverage

- SQLite schema 从 `v1` 升级到 `v2` 的迁移测试通过
- FTS5 索引与 triggers 可正常工作
- `SqliteRecallEngine` 可按 query/filter 命中
- diary backfill 幂等，不会重复膨胀
- `MEMORY.md` chunk backfill 幂等
- `WorkspaceMemoryService` 的混合 recall 去重逻辑通过
- `RationalDiaryExtractor.persist_if_relevant()` 成功后，对应 SQLite 记录已生成

## Notes

- `clippy` 输出的 warning 仍是现有 workspace 级别的重复告警与 MSRV 提示，不构成本轮失败。
- 本轮验证未使用 `just`，原因同前述环境限制。
