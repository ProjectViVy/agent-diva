# v0.5.1 验收步骤

## 验收目标

确保"同一审批请求只出现一次、位置直观、视觉一致"的用户预期得到满足。

## 验收清单

- [ ] **审批唯一性**：谨慎模式触发任意 ExecTool 命令，GUI 仅 Drawer 渲染一张卡片，聊天流无内联卡片或 legacy banner；
- [ ] **Drawer 自动弹出**：新审批到达时 Drawer 自动展开，角标显示待审批数量；
- [ ] **Allow / Deny 闭环**：Drawer 内点击 Allow 后命令执行，Deny 后命令被拒绝，卡片状态即时更新；
- [ ] **智能模式回归**：智能模式下失败命令仍能弹出 Drawer 单卡（OnFailure 路径）；
- [ ] **Plan 审批回归**：plan 域审批卡片（`PlanApprovalCard`）仍按原逻辑嵌入聊天流，未被 Drawer 替代；
- [ ] **无控制台告警**：GUI 启动后关键路径（loadSession / deleteSession / 审批 SSE）无新的 JS / Rust 告警；
- [ ] **CLI / Channel 回归**：CLI 与 Channel 适配器未受影响（`agent-diva-cli` 未改，`agent-diva-channels` 未改）。

## 验收方式

由用户在开发机上通过 `just make-diva` 完整重启后逐项确认，结果回填到 `verification.md` 观察结果列。

## 回滚预案

- 若 Drawer 渲染异常或 Allow 无法下发，回退到 v0.5.0 分支代码（git revert 本次 commit）即可恢复原三渲染点行为；
- Tauri 旧版命令未删除，回退无需数据迁移。

## 后续迭代建议（不在本迭代范围）

- 彻底删除 `start_command_approval_stream` / `get_command_approvals` / `resolve_command_approval` 三个 Tauri 命令；
- 清理 `api/desktop.ts` 中 `CommandApprovalRequest` / `CommandApprovalResolution` 接口；
- 聊天流内增加"快速跳转到 Drawer"角标 UX。
