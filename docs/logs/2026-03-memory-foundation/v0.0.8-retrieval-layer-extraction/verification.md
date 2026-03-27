# v0.0.8 Verification

## Validation Commands

当前环境下 `just` 仍因 shell 配置问题无法直接执行，本轮使用等价 `cargo` 命令完成验证。

- `cargo fmt`
- `cargo test -p agent-diva-memory --lib`
- `cargo fmt --all -- --check`
- `cargo clippy --all -- -D warnings`
- `cargo test --all`

## Results

- `cargo fmt`：通过
- `cargo test -p agent-diva-memory --lib`：通过
- `cargo fmt --all -- --check`：通过
- `cargo clippy --all -- -D warnings`：通过，保留现有 workspace 级 MSRV/future-incompat 提示，不构成本轮失败
- `cargo test --all`：通过

## Focused Coverage

- retrieval 层扩限逻辑由新增单测覆盖
- hybrid rerank 在 semantic 异常时的自动降级由新增单测覆盖
- hybrid rerank 在 semantic 可用时的重排行为由新增单测覆盖
- `WorkspaceMemoryService` 现有 recall/search/get/snapshot 恢复相关单测全部继续通过
- 全仓测试通过，确认 facade 调整未破坏 agent/tools/gui/manager 现有集成行为

## Notes

- `just fmt-check`、`just check`、`just test` 在当前环境均因 `just` 找不到配置 shell 而失败，这属于环境问题，不是代码问题。
- `cargo test --all` 与 `cargo clippy --all -- -D warnings` 输出的 future-incompat/MSRV 提示来自现有依赖与工作区配置，未由本轮引入。
