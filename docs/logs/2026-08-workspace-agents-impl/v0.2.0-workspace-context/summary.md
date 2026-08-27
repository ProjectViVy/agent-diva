# v0.2.0 WorkspaceContext 总结（Wave C1）

## 交付

- `agent-diva-core/src/workspace.rs`（新建）：
  - `WorkspaceSource` 枚举（`explicit_cli` / `configured` / `process_cwd` / `legacy_default`）
  - `WorkspaceContext { root, source, agents_md }` 解析结果
  - `resolve_workspace(cli_override, config_workspace)` 单一解析函数
  - `legacy_default_doctor_hint()` 为 `agent-diva doctor` 提供提示文案
  - 立即 absolutize / canonicalize；`LegacyDefault` 触发 `tracing::warn`
- `agent-diva-cli/src/cli_runtime.rs`：
  - `CliRuntime::workspace_context(&config)` 委托给 core 模块
  - `effective_workspace()` 返回 `workspace_context().root`（向后兼容）
  - ExplicitCli / Configured 场景自动 `create_dir_all`（保持旧行为）
- `agent-diva-cli/src/main.rs` + `chat_commands.rs`：
  - 从 `run_gateway` / `run_tui` / `chat_commands::build_*` 启动链移除
    `ensure_workspace_templates` 自动调用
  - 模板写入仅保留在 `run_onboard` / `run_config_refresh` / `workspace create`
- `agent-diva-cli/tests/effective_workspace.rs`：更新为 Wave C1 合同
  （默认取进程 CWD、显式覆盖 canonicalize、相对覆盖绝对化、`WorkspaceContext::source` 报告）

## 影响

- 运行时：Gateway/TUI/chat 启动不再自动写入 workspace 目录和 AGENTS.md 模板。
- 用户：旧默认 `~/.agent-diva/workspace` 触发一次性 `tracing::warn`，建议通过
  `agent-diva config set --workspace <dir>` 钉死项目工作区。
- 向后兼容：`CliRuntime::effective_workspace(&config) -> PathBuf` 签名不变。

## 验证

- core workspace 测试：6/6 通过。
- cli effective_workspace 测试：6/6 通过。
- cli workspace_commands 集成测试：7/7 通过（未破坏 onboard/create/switch/delete）。
- core 全量测试：707+5+2 通过。
- `cargo fmt --check` 干净；`cargo clippy --lib -D warnings` 干净。

## 已知遗留

- GUI 侧尚未消费 `WorkspaceContext`，仍走 `effective_workspace()` 旧 API。
- `LegacyDefault` 的 warn 日志未去重（每次启动都会输出一次）。
