# v0.5.1 — 审批 UI 三重显示去重

**状态**：已实现（后端 + 前端）
**关联**：`v0.5.0-cautious-approval`（同主题上一轮）
**范围**：GUI + sandbox coordinator（不涉及 CLI/Channel）

## 背景

v0.5.0 让"谨慎"模式能够弹出审批，但同一个 `ExecTool` 请求在 GUI 同时出现三种视觉形态：

1. 右侧 Approval Center Drawer 内的 `ApprovalCenterCard`；
2. 聊天流底部的内联 `ApprovalCenterCard`（compact 样式）；
3. 聊天流底部、短暂闪现的 legacy `ApprovalBanner`。

根因：后端 `approval_coordinator` 对同一请求同时写入 governance ledger（统一通道）和 broadcast 队列（legacy 通道），GUI 维护两套状态、四个渲染点，互斥守卫无法完全消除视觉重复。

## 修复要点

### 后端：单通道

- `agent-diva-sandbox/src/approval_coordinator.rs:335`：仅当 governance ledger **未挂载** 时才向 legacy broadcast channel 发送 `command_approval_requested`；挂载时由统一 `approval-event` SSE 权威发布，避免双源事件。
- Tauri 命令 `start_command_approval_stream` / `get_command_approvals` / `resolve_command_approval` 保留为 fallback API（本迭代不删）。

### 前端：单渲染点

- 保留 `ApprovalCenterDrawer.vue` 为唯一全量审批中心（含角标 `approvalPendingCount` 与自动弹出逻辑）。
- 删除 `ChatView.vue` 内联两份重复渲染：
  - 内联 unified `ApprovalCenterCard` 块；
  - legacy `ApprovalBanner` 块；
  - 相关 props（`commandApprovals` / `resolvingApprovalIds` / `commandApprovalErrors` / `unifiedApprovals` / `unifiedApprovalDetails` / `unifiedSubmittingIds` / `unifiedOutcomeUnknownIds` / `unifiedActionErrors`）、emit、computed（`hasUnifiedCommandApproval` / `hasUnifiedPlanApproval`）。
- `PlanApprovalCard` 保留在 `ChatView.vue`，仅移除 `!hasUnifiedPlanApproval` 守卫（plan 域通道独立）。
- `NormalMode.vue` 透传链路同步清理。
- `App.vue`：
  - 删除 `commandApprovals` / `resolvingApprovalIds` / `commandApprovalErrors` 三个 ref；
  - 删除 `sortCommandApprovals` / `upsertCommandApproval` / `reconcileCommandApprovals` / `approvalErrorMessage` / `resolveCommandApproval` / `currentSessionApprovals` 等 legacy 辅助函数与计算属性；
  - 删除 `command-approval-requested` / `command-approval-stream-connected` Tauri 监听；
  - 保留 unified 通路（`unifiedApprovals` / `handleUnifiedApprovalEvent` / Drawer 自动弹出 / 角标计数）。
- 删除 `ApprovalBanner.vue` 与 `ApprovalBanner.test.ts`。
- `ChatView.test.ts`：内联审批测试改写为"ChatView 不再内联渲染审批卡片"。

## 影响面

- **用户可见**：同一审批请求仅在 Drawer 内出现一次；聊天流保持纯消息；未读红点与自动弹出体验保留。
- **回归**：`PlanApprovalCard` 渲染路径不变；`ApprovalCenterDrawer` 渲染路径不变；CLI/Channel 未受影响。
- **契约**：Tauri 旧版命令保留为兜底，外部消费者（如果有）不中断。

## 验证

- `just fmt-check` / `just check`：pass；
- `cargo test -p agent-diva-sandbox --lib approval_coordinator`：10/10 pass；
- `cargo test --workspace --lib --exclude agent-diva-cli --exclude agent-diva-gui`：89/89 pass；
- `vue-tsc --noEmit`：pass（修复 `currentSessionApprovals` 未使用后清零）。

完整 GUI 烟雾测试（`just make-diva` 重启）需在用户环境执行；详见 `verification.md`。

## 不在范围

- Tauri 旧版 approval 命令删除（保留为 fallback）；
- `api/desktop.ts` 接口字段清理；
- 聊天流内"跳转 Drawer"角标 UX。
