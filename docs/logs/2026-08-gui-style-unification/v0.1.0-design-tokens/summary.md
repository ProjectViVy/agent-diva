# Summary: GUI 样式体系统一 v0.1.0（设计令牌基建 + 头部文件收敛）

**日期**: 2026-08-17
**分支**: `agent-diva-pro`
**目标**: 把 agent-diva-gui 中散落的配色、CSS 样式收敛到统一的设计令牌（Design Token）体系，为四主题（love / dark / default / miku）建立单一来源，深度优化整体 UI 一致性。

## 本次交付（阶段一）

### 1. 设计令牌基建（styles.css + tailwind.config.js）
- `styles.css` 新增主题无关的排版/间距令牌：`--font-size-xs..xl`、`--line-height-*`、`--font-weight-*`、`--space-1..6`。
- 四个主题块各自补齐语义令牌：`--success/-bg`、`--warning/-bg`、`--info/-bg`、`--surface-raised/-sunken`、`--text-faint`、`--border-strong`、`--overlay`，以及聊天区专用令牌（`--chat-bg/-text`、`--chat-bar-*`、`--chat-input-*`、`--chat-avatar-*`、`--bubble-user/-assistant-*`、`--chat-empty-*`、`--subview-*`）。
- Tailwind 通过 `var()` 接入令牌：新增 `surface/ink/line/brand/accent/state` 颜色族与 `tk-*` 前缀的字号/字重/圆角/阴影 scale（避免覆盖默认 `text-sm` 等造成回归）。

### 2. 主题切换治理（useTheme）
- 新增 `src/composables/useTheme.ts`：单一 `data-theme` 属性入口、localStorage 持久化（`agent-diva-theme`）、非法值回退 love。
- `NormalMode.vue` 改用 `useTheme()`，删除本地重复的 themeMode 状态；附 5 个单测。

### 3. `.theme-*` 覆盖层退役
- 删除约 540 行遗留覆盖（`.app-shell-old` 等死代码、全部 `.theme-X .chat-*`、四个 subview 主题块、死工具类 `.text-yandere` 等）。
- 聊天区/输入框/头像/气泡/空态/子视图基类全部改为令牌引用，带精确回退链（零色值漂移）。
- 保留 DEPRECATED 兼容桥（gray-utility 类 → 令牌），供尚未迁移的组件过渡。

### 4. 头部组件语义色令牌化
- `ConversationSidebar.vue`、`ChatView.vue`、`settings/McpManagementCard.vue` 中与令牌值一致的 danger/success/warning/info 硬编码改为 `var(--token, fallback)`。
- 深色对比色阶（#dc2626/#d97706/#047857/#b91c1c 等）与 WelcomeWizard 粉色身份色板按零回归原则保留字面量，列入二期。

## 提交记录（未推送）

| Commit | 说明 |
| --- | --- |
| `edbf0134` | Step 1: 令牌基建 + Tailwind 接入 |
| `feat(gui): centralize theme switching` | Step 2: useTheme |
| `e8e4b337` | Step 3: .theme-* 覆盖层退役（+160/−446） |
| `785b954b` | Step 4: 头部组件语义色令牌化 |

## 影响范围

- 仅 `agent-diva-gui`（styles.css、tailwind.config.js、composables、NormalMode、ChatView、ConversationSidebar、McpManagementCard）。
- 四主题视觉保持收敛（love/default 完全不变；dark/miku 聊天区由覆盖层改为令牌驱动，像素值一致）。
- 未触碰 Rust 工作区与其他并行任务的脏文件。

## 遗留（二期，见 TODOLIST.md）

- 其余约 31 个组件的硬编码色值迁移。
- 宠物装饰层（DivaPetView / DesktopPetOverlay）rgba 白色系色板。
- 组件内 scoped 的按主题覆盖块（如 ConversationSidebar 尾部 `.theme-*` 段）。
- WelcomeWizard 粉色身份色板的主题适配方案。
- 深色对比色阶（#dc2626 等）的语义令牌扩展（如 --danger-strong）。
- `tk-*` 字号 scale 的渐进迁移（本期只建基建，不动现有字号，零回归）。
