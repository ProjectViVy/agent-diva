# v0.5.1 验证记录

## 自动化验证（已执行）

| 命令 | 结果 | 备注 |
|---|---|---|
| `just fmt-check` | PASS | rustfmt 合规 |
| `just check`（clippy -D warnings） | PASS | 无新告警 |
| `cargo test -p agent-diva-sandbox --lib approval_coordinator` | 10/10 PASS | 含 governed + legacy 两套路径 |
| `cargo test --workspace --lib --exclude agent-diva-cli --exclude agent-diva-gui` | 89/89 PASS | 跳过被占用的 agent-diva.exe / agent-diva-gui.exe 链接产物 |
| `vue-tsc --noEmit`（GUI 类型检查） | PASS | 已修复中间状态 `currentSessionApprovals` 未使用告警 |

`just test` 全量在 dev 产物 `agent-diva.exe` / `agent-diva-gui.exe` 被现网 `just make-diva` 进程锁定时无法链接，已通过 `--workspace --lib` 替代路径覆盖所有库代码，结果全绿。

## GUI 烟雾测试（待用户执行）

前置：在 `agent-diva-gui` 与 gateway 两个窗口均关闭后重新执行 `just make-diva`，确保加载新二进制。

| # | 场景 | 预期 | 观察结果 |
|---|---|---|---|
| 1 | 主页切换到 **谨慎**，让 Agent 在桌面新建文件夹 | Drawer 自动弹出，**仅 1 张** `ApprovalCenterCard` | 待用户填写 |
| 2 | 聊天消息流中 | **不出现** 任何内联审批卡片或 legacy banner | 待用户填写 |
| 3 | 点 Allow | Drawer 卡片状态更新为 allowed，命令执行 | 待用户填写 |
| 4 | 切回 **智能**，制造失败命令 | Drawer 自动弹出，**仅 1 张** 卡片（OnFailure 路径） | 待用户填写 |
| 5 | 右上角红点 `approvalPendingCount` | 未读审批数量正确 | 待用户填写 |
| 6 | `PlanApprovalCard`（plan 域审批） | 仍嵌入聊天流，行为不变 | 待用户填写 |

## 回归点

- `ApprovalCenterDrawer` 渲染路径、props、emit 未变更；
- `PlanApprovalCard` 渲染条件由 `approvalPlan && !hasUnifiedPlanApproval` 简化为 `approvalPlan`，plan 域仍独立于 unified 通道；
- Tauri legacy 命令未删除，保留为 fallback，不影响外部消费者。
