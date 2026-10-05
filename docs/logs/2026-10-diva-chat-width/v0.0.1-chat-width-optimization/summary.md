# 迭代总结：聊天栏宽度与消息流居中对齐优化 (v0.0.1-chat-width-optimization)

## 1. 背景与目标
在先前的版本重构与回滚中，确认了 Peach Sakura Pink（蜜桃樱花粉）设计语言作为默认主题的主体基线。用户在确认当前主题样式满意的前提下，明确指出先前的版本中“聊天栏宽度修正做得比较好，需要恢复一下”。

针对宽屏显示器（1080p / 2K / 4K）下输入框铺满整屏、视觉过空、视线涣散，以及消息流左右边界与输入栏中轴脱节的问题，本迭代统一实施**综合聊天宽度与中轴阅读列宽优化**。

## 2. 修改范围
- **`agent-diva-gui/src/styles.css`**：
  - `.chat-input-container`：将宽屏下无限制撑满的 `margin: 12px 16px;` 升级为 `width: min(52rem, calc(100% - 32px)); margin: 12px auto;`。大屏上维持 52rem (832px) 黄金阅读列宽并水平居中，小屏自适应两端留白 16px。
  - `.chat-input-toolbar`：将内边距从 `8px 12px` 微调为紧凑舒适的 `6px 12px`。
- **`agent-diva-gui/src/components/ChatView.vue`**：
  - 聊天列表 `.chat-list` 内部引入 `.chat-message-stream` 包装层，设置 `width: min(52rem, 100%); margin: 0 auto; min-height: 100%;`，使消息流与下方输入框、上方 TODO 栏形成完全一致的中轴视觉带。
  - 活动计划栏 `.active-plan-todo-panel` 与状态条 `.compaction-status-line` 宽度协同对齐（`width: min(52rem, calc(100% - 32px)); margin: 0 auto 8px;`）。
  - 维持短消息气泡紧凑收敛包裹（`width: max-content; max-width: 100%;`）与思考卡片列宽（`width: min(100%, 52rem); min-width: min(100%, 34rem);`）。
- **`LOCK.md`**：任务排他锁登记与维护。

## 3. 影响评估与兼容性
- 视觉排版：在大屏上彻底消除横跨整个屏幕的涣散感，输入框与消息流上下中轴垂直对齐；小屏下自动回退至 100% 弹性布局，无任何溢出或横向滚动条。
- 无任何第三方依赖变动，无裸写 PostCSS `@apply` 问题。
