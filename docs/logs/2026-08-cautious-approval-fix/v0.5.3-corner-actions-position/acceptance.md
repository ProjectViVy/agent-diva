# v0.5.3 — 验收

## 验收清单（用户视角）

- [ ] 历史记录与审批中心按钮位于聊天输入栏工具栏右侧，不再悬浮遮挡消息。
- [ ] 两按钮与工具栏风格一致，点击行为正常（侧栏开合 / Drawer 开关与徽标）。
- [ ] 审批流程（自动弹出 → 同意 → 自动关闭）不受影响。

## 回滚方案

- 变更集中于 ChatView.vue 模板 + scoped 样式 + 1 个测试选择器。
- 回滚：恢复悬浮 `.chat-corner-actions`（absolute top/right + `.chat-corner-btn` 38px 样式），
  或直接 revert 提交。
