# Verification

执行与结果：

- `cargo fmt --all`
  - 结果：通过
- `cargo check -p agent-diva-core -p agent-diva-providers`
  - 结果：通过
- `cargo check -p agent-diva-cli -p agent-diva-manager`
  - 结果：通过
- `cargo check -p agent-diva-gui`
  - 结果：通过
- `cd agent-diva-gui && pnpm run build`
  - 结果：通过

待执行的完整仓库门禁：

- `just fmt-check`
- `just check`
- `just test`

说明：

- 当前环境下 `just` 无法直接运行，因为 `just` 找不到默认 shell；如需严格走 `just` 配方，需要先修复本机 `just` shell 配置。
- 真实 OAuth smoke test 需要人工完成浏览器授权。
- `cargo clippy --workspace -- -D warnings` 仍被仓库现有的 `agent-diva-agent/src/diary.rs` / `agent-diva-memory` 依赖问题阻断，不是本轮 GUI/provider 对齐直接引入的问题。
