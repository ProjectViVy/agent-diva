---
title: "agent-diva-pro 自进化 + 沙箱 UI — 视觉设计"
status: final
created: "2026-06-02"
updated: "2026-06-02"
project: agent-diva-pro
source: "../planning-artifacts/prds/prd-my-project-2026-06-02/prd.md"
experience_ref: "EXPERIENCE.md"

colors:
  brand: "#ec4899"
  brand-light: "#f472b6"
  accent: "#ec4899"
  danger: "#ef4444"
  success: "#22c55e"
  warning: "#f59e0b"
  bg-primary: "var(--bg-primary)"
  bg-panel: "var(--bg-panel)"
  text-primary: "var(--text)"
  text-muted: "var(--text-muted)"
  border-subtle: "var(--border)"

typography:
  family: "'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif"
  size-base: "0.875rem"
  size-sm: "0.8125rem"
  size-xs: "0.75rem"
  size-lg: "1rem"
  weight-normal: 400
  weight-medium: 500
  weight-semibold: 600
  line-height-base: 1.6

rounded:
  card: "12px"
  card-sm: "8px"
  button: "8px"
  input: "var(--radius-sm)"
  bubble-user: "18px 18px 4px 18px"
  bubble-assistant: "18px 18px 18px 4px"

spacing:
  unit: 4
  card-padding: "12px 16px"
  section-gap: "16px"
  input-padding: "8px 12px"

components:
  decision-card:
    background: "var(--bg-panel)"
    border: "1px solid var(--border)"
    border-radius: "{rounded.card}"
    padding: "{spacing.card-padding}"
    shadow: "0 4px 16px rgba(236, 72, 153, 0.12)"
    hover-shadow: "0 6px 20px rgba(236, 72, 153, 0.18)"
    risk-dot-size: "8px"
    risk-dot-margin: "0 4px 0 0"
  todo-card:
    background: "var(--bg-panel)"
    border: "1px solid var(--border)"
    border-radius: "{rounded.card}"
    padding: "{spacing.card-padding}"
    checkbox-size: "18px"
    checkbox-border-radius: "4px"
    done-strikethrough: "line-through"
    done-color: "var(--text-muted)"
  approval-banner:
    height: "64px"
    background: "var(--bg-panel)"
    border: "1px solid var(--border)"
    border-left-width: "4px"
    border-radius: "8px"
    risk-bar-green: "#22c55e"
    risk-bar-yellow: "#f59e0b"
    risk-bar-red: "#ef4444"
  notebook-list-item:
    height: "56px"
    padding: "8px 12px"
    active-border-left: "4px solid {colors.accent}"
    active-background: "rgba(236, 72, 153, 0.08)"
  settings-section-card:
    background: "var(--bg-panel)"
    border: "1px solid var(--border)"
    border-radius: "{rounded.card}"
    padding: "16px"
    margin-bottom: "16px"
---

# DESIGN.md

## Brand & Style

agent-diva-pro 的视觉身份为**"病娇二次元 + 现代玻璃拟态"**。品牌色锚定粉色系，通过四套主题（Love / Dark / Default / Miku）提供情感多样性，毛玻璃 + 柔和阴影贯穿所有面板。

**本次新增组件**不引入新主题或新色调，完全继承现有视觉系统。所有 token 使用 CSS 变量覆盖：
- `var(--bg-primary)` / `var(--bg-panel)` — 背景层次
- `var(--text)` / `var(--text-muted)` — 文字层次
- `var(--accent)` — 品牌色强调
- `var(--border)` — 边框线
- `var(--radius)` / `var(--radius-sm)` — 圆角系统
- `var(--sidebar-width)` — 侧边栏宽度

**情感基调**：专业但不冷酷，精致但不繁复。粉色系为主色调但不过度甜腻，四主题支持用户切换人格。

## Colors

### 语义色（跨主题恒定）

| 用途 | Token | 色值 |
|------|-------|------|
| 品牌/强调 | `accent` | `#ec4899` |
| 危险/删除 | `danger` | `#ef4444` |
| 成功/完成 | `success` | `#22c55e` |
| 警告/风险中 | `warning` | `#f59e0b` |
| 链接 | `link` | `#3b82f6` |

### 风险等级色（审批/决策卡专用）

| 等级 | 色值 | 用途 |
|------|------|------|
| 低风险 (low) | `#22c55e` | 左侧色条 + 文字标签 |
| 中风险 (medium) | `#f59e0b` | 左侧色条 + 文字标签 |
| 高风险 (high) | `#ef4444` | 左侧色条 + 文字标签 |

### 新增背景色

| Token | 色值 | 用途 |
|-------|------|------|
| 审批横幅底色 | `var(--bg-panel)` | 标准面板背景 |
| 卡片 hover 背景 | `var(--bg-panel)` + brightness(0.98) | 微暗反馈 |
| 强制确认提醒条 | `#fef3c7` (light) / `#3d2e00` (dark) | 沙箱模式变更提示 |

### 主题适配规则

所有新组件通过 CSS 变量接入四主题。不添加硬编码色值（除语义色外）。暗色主题下的新组件由现有变量自然覆盖，无需额外配置。

## Typography

继承现有字体栈。新增文本层级：

| 层级 | 字号 | 字重 | 用途 |
|------|------|------|------|
| 卡片标题 | `0.9375rem` (15px) | 600 | DecisionCard / TodoCard 标题 |
| 卡片正文 | `0.875rem` (14px) | 400 | 决策步骤描述、审批操作描述 |
| 风险标签 | `0.75rem` (12px) | 500 | low / medium / high 标签 |
| 报告标题 | `1.125rem` (18px) | 600 | 记事本详情页报告标题 |
| 报告日期 | `0.8125rem` (13px) | 400 | 记事本列表项日期 |
| 设置标签 | `0.875rem` (14px) | 500 | 表单字段标签 |
| 倒计时 | `0.75rem` (12px) | 500 | 审批超时倒计时 |

所有尺寸使用 `rem` 单位，`font-size: 16px` 基准。

## Layout & Spacing

### 新页面布局

**NotebookView** — 双栏布局：
```
┌──────────────────────────────────────┐
│ [日报] [周报] [月报]                   │  ← 标签栏 48px
├────────────┬─────────────────────────┤
│ 报告列表    │ 报告详情                 │
│ (280px)    │ (flex 1)                │
│            │                         │
│ 02-01 周报 │ ## 本周总结              │
│ 01-25 周报 │ ...markdown...          │
│ 01-18 周报 │                         │
│            │ [固化为SOP] [固化为技能]  │  ← 底部操作栏 56px
└────────────┴─────────────────────────┘
```

**SelfEvolutionSettings** — 单栏表单：
```
┌──────────────────────────────────────┐
│ 自进化设置                      [保存] │  ← 顶栏 56px
├──────────────────────────────────────┤
│ ┌─ 自进化控制 ──────────────────────┐ │
│ │ 总开关     [===========]          │ │
│ │ 频率       [每日 ▾]              │ │
│ │ 阈值       [━━━━●━━━] 50         │ │
│ └──────────────────────────────────┘ │
│ ┌─ 学习策略 ────────────────────────┐ │
│ │ 信任度     [━━━━●━━━] 0.95       │ │
│ │ 强制确认   ☑ 身份 ☑ 关系 ☐ SOP  │ │
│ └──────────────────────────────────┘ │
│ ┌─ 沙箱设置 ────────────────────────┐ │
│ │ 模式       [WorkspaceWrite ▾]    │ │
│ │ ...更多字段...                     │ │
│ └──────────────────────────────────┘ │
│ ┌─ Memory Changelog ───────────────┐ │
│ │ 2024-06-02  来源: AutoDream       │ │
│ │ 2024-06-01  来源: 手动            │ │
│ │ ...(分页)                         │ │
│ └──────────────────────────────────┘ │
└──────────────────────────────────────┘
```

### 间距继承

- 卡片内边距：`12px 16px`（与现有 `settings-card` 一致）
- 分组间距：`16px`（与现有表单分组一致）
- 列表项间距：`8px`（与现有侧边栏项一致）
- 操作按钮间距：`8px`（按钮组内 button gap）

## Elevation & Depth

继承现有三层阴影体系：

| 层级 | 用途 | shadow |
|------|------|--------|
| 基础 | 卡片（默认） | `0 4px 16px rgba(236,72,153,0.12)` |
| 悬浮 | 卡片（hover） | `0 6px 20px rgba(236,72,153,0.18)` + translateY(-1px) |
| 焦点 | 输入框 focus | `0 0 0 2px var(--accent)` |

毛玻璃效果 (`backdrop-filter: blur(16px)`) 仅用于侧边栏和设置导航栏（现有约定），新卡片使用纯色背景 + 边框线 + 阴影。

## Shapes

### 圆角继承

| 元素 | 圆角 | 来源 |
|------|------|------|
| 大卡片 | `12px` | `--radius` |
| 小卡片/按钮 | `8px` | `--radius-sm` |
| 输入框 | `var(--radius-sm)` | 现有 settings-input |
| 复选框 | `4px` | 方形微圆 |
| 风险色条 | `4px 0 0 4px`（仅左侧圆角） | 审批横幅左侧 |

### 边框

- 标准卡片：`1px solid var(--border)`
- 审批横幅：标准 + 左侧 4px 色条（风险颜色）
- 列表选中项：左侧 4px accent 色条 + 浅 accent 背景
- 强制提醒条：`1px solid var(--warning)` + 警告背景

## Components

### DecisionCard（决策卡）

```
┌────────────────────────────────────────┐
│ 计划执行以下操作              ● medium │  ← 标题 + 风险
│                                        │
│ Diva 将执行以下步骤来完成你的请求：      │  ← 摘要
│                                        │
│ ① 创建 src/components/NewFeature.vue  │  ← 步骤
│ ② 注册到路由和导航                     │
│ ③ 添加 i18n 翻译                      │
│                                        │
│ 依据：对话中你提到了需要新功能           │  ← 证据
│                    [拒绝] [同意执行]    │  ← 操作
└────────────────────────────────────────┘
```

**颜色**: 背景 `var(--bg-panel)` / 边框 `var(--border)` / 阴影 `0 4px 16px rgba(236,72,153,0.12)`
**圆角**: `12px`
**风险圆点**: 8px 直径，颜色 = `success`/`warning`/`danger`，标题右侧对齐
**步骤编号**: 18px 直径圆圈，accent 色填充，白色数字
**按钮**: `[拒绝]` secondary 样式，`[同意执行]` primary 渐变样式

**已批准态（收起）**:
```
┌────────────────────────────────────────┐
│ ✅ 计划已批准 · 3 项待完成   [展开 ▾]   │
└────────────────────────────────────────┘
```
高度收至 48px，绿色 checkmark，灰色文字，可展开回完整视图。

### TodoCard（Todo 卡）

```
┌────────────────────────────────────────┐
│ 📋 待办                        2/3     │  ← 标题 + 进度
│                                        │
│ ☑ 创建 src/components/NewFeature.vue  │  ← 已完成项
│   刚刚完成                             │
│ ☑ 注册到路由和导航                     │  ← 已完成项
│   2 分钟前                             │
│ ☐ 添加 i18n 翻译                      │  ← 待处理项
│                                        │
│                         [全部完成]      │  ← 操作（仅当有未完成项时显示）
└────────────────────────────────────────┘
```

**复选框**: 18px 方形，accent 色填充（checked）/ 灰色边框（unchecked），白色勾
**已完成文本**: `line-through` + `var(--text-muted)` 颜色
**完成时间**: 12px，灰色，右对齐
**进度**: 标题右侧 `2/3` 灰色小字

**已折叠态（全部完成后 3 秒自动）**:
```
┌────────────────────────────────────────┐
│ ✅ 全部完成 · 3/3              [展开 ▾] │
└────────────────────────────────────────┘
```

### ApprovalBanner（审批横幅）

```
┌────────────────────────────────────────┐
│ ▌ Diva 请求执行：删除 /tmp/test-project/│  ← 左侧 4px 风险色条
│ ▌ 影响范围：临时目录，不含用户代码       │
│ ▌ 风险：低                      [允许] [拒绝] │
└────────────────────────────────────────┘
```

**高度**: 64px（紧凑单行模式）或自适应（多行模式，最多 3 行）
**左侧色条**: 4px 宽，颜色 = 风险等级色
**按钮**: 右对齐，`[允许]` primary，`[拒绝]` secondary
**倒计时**: 最后 60 秒在 `[拒绝]` 按钮左侧显示 "59s"

**过期态**:
```
┌────────────────────────────────────────┐
│ ⏱ 已过期（自动拒绝）                    │  ← 灰色文字，无按钮
└────────────────────────────────────────┘
```

### NotebookView（记事本页面）

**报告列表项**:
```
┌──────────┐
│ 06-02     │  ← 56px 高，12px 内边距
│ 本周报告  │  ← 14px 500 字重
│ 本周完成 Rust workspace... │  ← 13px 400 灰色，截断 100 字
│           │
│ 05-26     │  ← hover：浅粉背景
│ 上周报告  │  ← 选中：左 4px accent 边框 + accent 浅背景
└──────────┘
```

**报告详情**:
- 顶部：报告标题（18px 600）+ 日期（13px 400 灰色）
- 正文：Markdown 渲染（复用 `.markdown-body` 样式）
- 底部固定栏：56px 高，灰色背景，三个操作按钮右对齐

**空态**:
```
┌─────────────────────────────┐
│                             │
│       📋                    │
│    暂无周报                  │  ← 16px 灰色
│    AutoDream 尚未生成报告    │  ← 13px 更浅灰色
│                             │
└─────────────────────────────┘
```

### SelfEvolutionSettings

**设置分组卡片**（参考现有 `settings-card` 样式）：
- 卡片容器：`var(--bg-panel)` + `1px solid var(--border)` + `12px` 圆角 + `16px` 内边距
- 卡片标题：15px 600，底部 12px 间距，下方 1px `var(--border)` 分隔线
- 表单项：`label (14px 500)` + `input/select/toggle`，垂直间距 12px

**Toggle 开关**（复用现有）：
```
[===========●]  自进化
  off → on
```
宽 48px，高 24px，accent 渐变填充（on）/ 灰色（off），12px 圆角

**Memory Changelog 时间线**:
```
● 2026-06-02 14:30    来源: AutoDream
  更新了 Rust workspace 结构规范
  ──────────────────────────────
● 2026-06-01 09:15    来源: 手动
  添加了新的项目命名约定
  ──────────────────────────────
```
左边 accent 圆点 (8px) + 竖线连接，每条记录间距 12px

### SandboxSettingsSection

嵌入 SelfEvolutionSettings 页面内作为独立分段卡片，布局同上。字段：

| 字段 | 控件 | 宽度 |
|------|------|------|
| 沙箱模式 | `<select>` 下拉 | 240px |
| 审批策略 | `<select>` 下拉 | 240px |
| 网络访问 | `<toggle>` | auto |
| 可写根目录 | 标签列表 + 添加输入框 | 100% |
| 保护路径 | 标签列表 + 添加输入框 | 100% |
| 拒绝模式 | `<textarea>` 4 行 | 100% |
| 超时 | `<input type="number">` | 120px |

**标签列表**：水平排列，每项 `[路径名 ✕]`，粉色浅背景 + accent 边框，12px 圆角

**模式变更提醒条**（黄色）：
```
┌────────────────────────────────────────┐
│ ⚠ 沙箱模式变更需要重启 gateway 才能生效  │
└────────────────────────────────────────┘
```
背景 `#fef3c7` / 暗色 `#3d2e00`，1px 黄色边框，12px 内边距

## Do's and Don'ts

### Do

- ✅ 所有新组件使用 CSS 变量引用颜色，不硬编码
- ✅ 卡片使用 `var(--bg-panel)` + 边框线 + 阴影，不用毛玻璃（保留给侧边栏）
- ✅ 风险等级同时使用颜色 + 文字标签
- ✅ 操作按钮使用现有 `btn-primary` 和 `btn-secondary` 样式
- ✅ 列表选中态使用左 4px accent 边框 + 浅 accent 背景
- ✅ 空态使用居中布局 + 图标 + 说明文字
- ✅ 所有交互元素有 focus ring（2px accent box-shadow）
- ✅ 过渡统一 `0.15s ease`

### Don't

- ❌ 不新增自定义颜色族（不扩展 tailwind.config.js 的 yandere 调色板）
- ❌ 不在新卡片上使用毛玻璃效果（保留给侧边栏和导航栏）
- ❌ 不使用 `position: fixed` 覆盖现有层叠上下文
- ❌ 不修改现有 ChatView 的气泡布局和间距
- ❌ 不引入新的字体
- ❌ 不添加装饰性动画（浮动爱心/樱花/初音）——仅 Love/Miku 主题保留
- ❌ 不硬编码 px 值用于间距和字号——使用 rem 或 CSS 变量
