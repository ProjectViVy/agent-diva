# v0.1.0 行为表征总结（Wave B）

## 范围

Phase 0 现状快照：钉住 CLI/AGENTS/Shell 三条路径的当前合同，为后续 Wave 提供
回归基线。不做行为变更，只写表征测试。

## 改动

- `agent-diva-cli/tests/effective_workspace.rs`（新建）：
  `CliRuntime::effective_workspace()` 四种场景（default/显式/~ 展开/绝对路径）。
- `agent-diva-agent/src/context.rs`（测试追加）：AGENTS.md 注入矩阵
  （缺失/空/存在一次/超限截断）。
- `agent-diva-tools/src/shell.rs`（测试追加）：`working_dir` 越界负向测试，
  先 `#[ignore]` 待 Wave C2 转正。

## 影响范围

- 仅测试代码；未触及生产逻辑。
- 覆盖 3 个 crate 的公开合同。

## 验证

- `cargo test -p agent-diva-agent --lib agents_md`：4/4 通过。
- `cargo test -p agent-diva-tools --lib shell`：15/15 通过 + 1 ignored。
- `cargo test -p agent-diva-cli --test effective_workspace`：4/4 通过。
- `cargo fmt` / `cargo clippy --lib -D warnings`：干净。

## 已知遗留

- Wave B 钉住的 CLI 默认仍为 `~/.agent-diva/workspace`，将在 Wave C1 切换为进程 CWD。
- Wave B 钉住的 CLI 显式覆盖行为不做绝对化，Wave C1 将统一 canonicalize。
