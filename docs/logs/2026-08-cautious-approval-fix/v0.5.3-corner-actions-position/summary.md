# v0.5.3 — 历史记录/审批中心按钮移入输入栏工具栏右侧

## 变更内容

- 现象：ChatView 右上角悬浮的「历史记录」「审批中心」按钮堆叠遮挡右侧用户消息。
- 方案（用户指定）：不再悬浮，改为内嵌在聊天输入栏顶部工具栏（`.chat-input-toolbar`）
  的最右侧，与模式选择/附件/权限模式等按钮同一行；两按钮前加 `.toolbar-divider` 分隔。
- 按钮样式从 38px 悬浮卡（`.chat-corner-btn`）改为工具栏小按钮（`.toolbar-btn`，14px 图标），
  视觉与工具栏一致；审批中心 active 琥珀色与 pending 徽标保留。

## 实现

- `agent-diva-gui/src/components/ChatView.vue`：
  - 模板：删除 `.chat-main` 顶部悬浮 `.chat-corner-actions` 块；在 `chat-input-toolbar`
    内权限模式之后插入 `<span class="toolbar-divider" />` + `.chat-corner-actions`（横向）。
  - 样式：`.chat-corner-actions` 改为 `margin-left: auto` 右对齐行内 flex；
    删除 `.chat-corner-btn` 与 `.conv-sidebar-open` 偏移规则；
    `.approval-center-icon-btn` 保留 `position: relative`（徽标锚点）。
- `agent-diva-gui/src/components/ChatView.test.ts`：历史按钮选择器
  `.chat-corner-actions .chat-corner-btn` → `.chat-corner-actions .toolbar-btn`。

## 影响范围

- 仅 ChatView 模板/scoped 样式 + 1 个测试选择器；无脚本逻辑/API/Rust 变更。

## 说明

- 侧栏打开时工具栏随 chat-main 变窄，按钮自动跟随，无需再维护 right 偏移。
