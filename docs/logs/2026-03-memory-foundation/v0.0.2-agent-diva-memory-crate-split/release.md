# Release

## 当前状态

本次为工作区内部 crate 拆分与分层调整，不涉及独立发布流程，也不包含外部部署步骤。

## 发布注意事项

- 后续若需要单独发布 `agent-diva-memory`，应补充 crate 级 README、版本策略和公开 API 稳定性说明。
- 当前 worktree 已完成 `cargo fmt --all -- --check`、`cargo clippy --all -- -D warnings`、`cargo test --all`，可作为本轮拆分的完整绿灯基线。
- 若仅从 memory 子系统视角评估，本次拆分已经完成主要收口，可作为后续 recall / SQLite / embedding 工作的分层基线。
