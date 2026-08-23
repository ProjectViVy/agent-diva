# v0.4.0 WorkspaceInstructions + Manager `/api/workspace` 总结（Wave C3 + Wave D backend）

## 交付

### Wave C3：WorkspaceInstructions 模块与安全合同

- `agent-diva-agent/src/workspace_instructions.rs`（新建）：
  - `WorkspaceInstruction { source, digest, truncated, char_count, body }`
  - `AGENTS_MD_MAX_CHARS = 4000`（与原 `WORKSPACE_MD_MAX_CHARS` 对齐）
  - `DIGEST_PREFIX_LEN = 16`（SHA-256 hex 前 16 字符）
  - `SECURITY_CONTRACT` 常量：声明 AGENTS.md 是项目指导而非无条件权威
  - `load_workspace_instructions(root)` 与 `load_from_path(path)`
  - `format_header()`：注入 `Source: <path> (SHA256: <digest>, truncated: <bool>). <contract>`
  - 空文件/纯空白/不可读返回 `None`
- `agent-diva-agent/src/context.rs`：
  - `append_agent_rules` 改为委托新模块
  - 新增 `tracing::info` 事件（source/digest/truncated/char_count）
  - 删除 `read_workspace_markdown` / `read_trimmed_markdown` 死代码
  - 新增 `agents_md_injection_carries_digest_and_security_contract` Wave C3 合同测试

### Wave D backend：Manager `/api/workspace` 端点

- `agent-diva-manager/src/handlers/workspace.rs`（新建）：
  - `GET /api/workspace` 返回 `WorkspaceStatusResponse { root, source, legacy_hint, agents_md }`
  - `agents_md: { path, digest, truncated, char_count, present }`
  - 直接读 `state.config_dir/config.json` + workspace root 的 AGENTS.md，无 ManagerCommand 回环
  - 3 个 handler 测试（AGENTS.md 存在/缺失/legacy-default 提示）
- `agent-diva-manager/src/server.rs` 路由注册

## 影响

- 每次 AGENTS.md 注入会留下可审计的 digest / 截断标记。
- 安全合同明确项目指令不得授予工具权限、覆盖系统安全策略或修改 BML/Persona 权威。
- GUI 可通过 `/api/workspace` 获取当前 workspace 与 AGENTS.md 状态。

## 验证

- `cargo test -p agent-diva-agent --lib workspace_instructions`：6/6 通过。
- `cargo test -p agent-diva-agent --lib agents_md`：5/5 通过（含 Wave C3 合同测试）。
- `cargo test -p agent-diva-manager --lib handlers::workspace`：3/3 通过。
- `cargo fmt --check` / `cargo clippy --lib -D warnings`：干净。

## 已知遗留

- GUI WorkspaceChip / Settings / 切换流程未实现（见 TODOLIST `WORKSPACE-GUI`）。
- `/api/workspace` 的 source 分类是 best-effort（从 root 推断，非完整 `WorkspaceContext`）。
