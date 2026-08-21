# Workspace 与 AGENTS.md：验证记录

## 证据来源

- Codex：`.workspace/codex` HEAD `a9c111da544c976d591343db5493a7da283b72e5`；核对
  `codex-rs/utils/cli/src/shared_options.rs`、`codex-rs/core/src/config/mod.rs`、
  `codex-rs/core/src/agents_md.rs` 及其测试。
- Diva：核对 `agent-diva-cli/src/cli_runtime.rs`、`agent-diva-cli/src/main.rs`、
  `agent-diva-cli/src/chat_commands.rs`、`agent-diva-agent/src/agent_loop.rs`、
  `agent-diva-agent/src/context.rs`、`agent-diva-agent/src/tool_assembly.rs`、
  `agent-diva-tools/src/shell.rs` 和 `agent-diva-core/src/config/schema.rs`。

## 结果

- [x] 确认 Diva 已有全局 `--workspace` 当前命令覆盖。
- [x] 确认 Diva 当前默认 workspace 是 `~/.agent-diva/workspace`。
- [x] 确认 Diva 只读取 workspace 根 `AGENTS.md`，最多 4000 字符，缺失时不追加段落。
- [x] 确认 Codex 默认当前 CWD、显式 `--cd`、session CWD 选择和层级 AGENTS 规则。
- [x] 确认工作区选择与 AGENTS 注入应共享一个 canonical workspace root。

本迭代为文档-only；`just fmt-check/check/test` 不因生产代码未改而运行。进入 Phase 0
施工后恢复完整 workspace gate 与 CLI/GUI/channel smoke test。
