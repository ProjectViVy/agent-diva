# 修复"谨慎"模式下审批不生效

## 变更摘要

修复了 GUI 主页面将权限模式切到 **"谨慎"** 后，沙箱直接以 `exit code 1` 拒绝所有命令、从不弹出审批询问的回归。根因链：

1. `permissionMode` 是 ChatView 的本地 UI 状态，从未传到后端；
2. `ExecTool::with_approval_backend` 硬编码 `AskForApproval::OnFailure`；
3. `ToolOrchestrator::should_offer_escalation` 不覆盖 `ExecutionFailed`，Windows RestrictedToken 沙箱返回 `code=1` 即被当作普通失败直接返回；
4. `AskForApproval::OnRequest.allows_sandbox_failure_retry()` 为 `false`，即便配置为 cautious，沙箱失败后也无法升级审批；
5. `ApprovalCenterDrawer` 在收到 pending 事件时不会自动弹出，即审批真发出用户也易错过。

## 修复点（最小集）

| 文件 | 修复 |
|---|---|
| `agent-diva-gui/src/components/ChatView.vue` | emit `send` 透传 `permissionMode` |
| `agent-diva-gui/src/App.vue` | `sendMessage` 映射 cautious→`on-request`/smart→`on-failure`/trusted→`unless-trusted` 并透传 `approvalPolicy`；`handleUnifiedApprovalEvent` / `upsertCommandApproval` 收到 pending 时自动打开 drawer |
| `agent-diva-gui/src-tauri/src/commands.rs` | `send_message` Tauri 命令新增 `approvalPolicy` 参数 |
| `agent-diva-manager/src/handlers.rs` | `ChatRequest.approval_policy` 字段；`parse_approval_policy` 支持中文模式名+kebab 别名+大小写不敏感；通过 `InboundMessage` metadata 传给 agent loop |
| `agent-diva-agent/src/agent_loop.rs` | `ToolConfig.approval_policy`；`set_approval_policy` + `apply_approval_policy_from_metadata`；`handle_inbound` 每轮消息读取策略 |
| `agent-diva-agent/src/tool_assembly.rs` | `ToolAssembly.approval_policy`；`with_approval_policy` setter；ExecTool 注册时传入 |
| `agent-diva-agent/src/runtime_control.rs` + `loop_runtime_control.rs` | 新增 `RuntimeControlCommand::SetApprovalPolicy` |
| `agent-diva-tools/src/shell.rs` | `with_approval_backend` 接收外部 `AskForApproval` |
| `agent-diva-sandbox/src/policy.rs` | `OnRequest` 也允许沙箱失败后重试 |
| `agent-diva-sandbox/src/orchestrator.rs` | `should_offer_escalation` 覆盖 `ExecutionFailed` |

## 影响范围

- ExecTool 命令执行路径：所有 GUI/CLI/API 渠道都会受到影响（默认 `OnFailure`，向后兼容）；
- Approval Center UI：收到新 pending 时 drawer 自动弹出；
- 新增 `RuntimeControlCommand` 变体，外部若 `match` 该 enum 需补 `SetApprovalPolicy` 分支。

## 复用的现有工具

- `AskForApproval` 枚举（policy.rs）+ `CommandApprovalCoordinator`（approval_coordinator.rs）+ governance SSE，原样复用；
- `InboundMessage::with_metadata` 作为 metadata 通道，避免为 `approval_policy` 引入新协议；
- `ToolTurnOptions` / `rebuild_tools_for_turn` 作为策略刷新路径。
