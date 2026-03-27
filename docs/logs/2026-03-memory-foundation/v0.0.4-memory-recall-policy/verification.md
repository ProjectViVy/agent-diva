# Verification

## 已执行

- `cargo fmt --all`
- `cargo fmt --all -- --check`
- `cargo test -p agent-diva-memory service:: -- --nocapture`
- `cargo test -p agent-diva-agent loop_turn:: -- --nocapture`
- `cargo test -p agent-diva-agent context:: -- --nocapture`
- `cargo test -p agent-diva-tools memory:: -- --nocapture`
- `cargo clippy --all -- -D warnings`
- `cargo test --all`

## 结果

- `cargo fmt --all -- --check` 通过。
- `agent-diva-memory` 定向测试通过，覆盖 recall 摘要格式、`MEMORY.md` fallback、mixed-language query 命中与 limit 行为。
- `agent-diva-agent` 的 `loop_turn` 定向测试通过，覆盖：
  - 历史型问题触发自动 recall
  - 非触发型请求不自动 recall
  - recall 摘要被注入首条 system message
- `agent-diva-agent` 的 `context` 定向测试通过，确认 prompt 中已包含 memory 工具使用规约。
- `agent-diva-tools` 的 memory tool 定向测试通过，说明 contract 与工具层未被本轮 policy 接线破坏。
- `cargo clippy --all -- -D warnings` 通过。
- `cargo test --all` 在当前 worktree 下全量通过。

## 说明

- 本仓库当前 `justfile` 仍依赖 `powershell.exe`，在当前 macOS 环境中不能作为直接验证入口，因此本轮继续使用等价 `cargo` 命令完成验证。
- `cargo test --all` 中的 `integration_logs` 为长耗时测试，但最终通过。
