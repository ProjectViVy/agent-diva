# v0.5.2 — 审批 Drawer 同意后自动关闭

## 变更内容

- 场景：统一审批（unified approval）事件到达时 Drawer 自动弹出（v0.5.1 保留的行为）。
- 变更：当 Drawer 是**自动弹出**（`handleUnifiedApprovalEvent` 收到 pending 事件触发打开）时，
  用户点击「同意（allow）」且后端决策成功返回后，Drawer 自动关闭。
- 边界：
  - 手动打开 Drawer（点击审批中心图标）→ 同意成功后**不**自动关闭（仅自动弹出场景生效）。
  - deny / cancel / 决策失败（错误、outcome unknown）→ 不自动关闭，保留上下文展示错误。
  - 自动弹出后用户手动关闭 Drawer → 标记复位；后续同意不再触发自动关闭。
  - 自动弹出后走「在源头编辑」（editUnifiedApproval）→ Drawer 关闭且标记复位。

## 实现

全部在 `agent-diva-gui/src/App.vue`：

- 新增 `approvalDrawerAutoOpened: Ref<boolean>`（自动弹出标记）。
- `handleUnifiedApprovalEvent`：pending 事件自动打开 Drawer 时置位标记。
- `decideUnifiedApproval`：allow 决策成功后，若标记为真则关闭 Drawer 并复位标记。
- 新增 `onApprovalCenterOpenChange(open)`：用户手动打开 Drawer 时复位标记；
  替换 NormalMode 的 `@update:approval-center-open` 内联赋值。
- `editUnifiedApproval`：程序化关闭时同步复位标记。

## 影响范围

- 仅 `agent-diva-gui/src/App.vue`（前端单文件）。
- 无后端 / Rust / API / 协议变更；无迁移；向后兼容。
- 关联：`docs/logs/2026-08-cautious-approval-fix/v0.5.1-approval-ui-dedup/`（单一渲染点修复）。
