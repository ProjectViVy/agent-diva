# Verification

## Commands

由于当前 macOS 环境下 `justfile` 依赖 `powershell.exe`，本轮继续使用等价 `cargo` 命令验证。

执行结果：

- `cargo fmt --all -- --check`
- `cargo test -p agent-diva-memory -- --nocapture`
- `cargo test -p agent-diva-agent loop_turn:: -- --nocapture`
- `cargo test -p agent-diva-agent context:: -- --nocapture`
- `cargo test -p agent-diva-tools memory:: -- --nocapture`
- `cargo clippy --all -- -D warnings`
- `CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0 cargo test --all`

以上命令均通过。

## Notes

- 直接执行默认参数的 `cargo test --all` 会因本机磁盘空间不足在 `target/debug` 阶段失败，不是代码错误。
- 为完成全量验证，先清理了当前 worktree 的可再生产物 `target/debug`，随后使用：
  - `CARGO_INCREMENTAL=0`
  - `CARGO_PROFILE_DEV_DEBUG=0`
  - `CARGO_PROFILE_TEST_DEBUG=0`
- 这些环境变量只降低构建产物体积，不改变测试内容和断言范围。

## Coverage Highlights

- `agent-diva-memory`
  - `FileRecallEngine` 的 diary recall
  - `MEMORY.md` heading chunk / paragraph chunk recall
  - 混合排序与近期 diary 优先
- `agent-diva-agent`
  - 自动 recall policy 未回归
  - recall context 注入仍可命中 memory 拆分相关问题
- `agent-diva-tools`
  - `memory_recall` / `diary_read` / `diary_list` contract 未破坏
