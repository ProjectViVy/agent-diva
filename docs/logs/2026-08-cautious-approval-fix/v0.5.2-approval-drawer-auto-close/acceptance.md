# v0.5.2 — 验收

## 验收清单（用户视角）

- [ ] 审批自动弹出后，点击「同意」→ 命令执行，右侧审批栏自动关闭。
- [ ] 手动打开审批栏时同意 → 审批栏不自动关闭。
- [ ] 自动弹出后拒绝/失败 → 审批栏保持打开，错误信息可见。
- [ ] 审批中心角标计数与 Drawer 状态一致。
- [ ] 聊天流中仍无内联审批卡片/横幅（v0.5.1 行为不回退）。

## 回滚方案

- 本次为单文件前端变更（`agent-diva-gui/src/App.vue` + 日志 + LOCK）。
- 回滚：`git revert <commit>` 或手动删除 `approvalDrawerAutoOpened` 相关 5 处逻辑，
  将 `@update:approval-center-open` 恢复为 `approvalCenterOpen = $event`。
- 无数据迁移，回滚无副作用。
