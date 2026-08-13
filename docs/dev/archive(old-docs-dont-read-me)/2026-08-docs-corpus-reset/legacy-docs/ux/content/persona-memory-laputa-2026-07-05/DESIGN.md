---
title: "Persona & Memory — Visual Identity"
status: draft
created: 2026-07-05
updated: 2026-07-05
version: 0.1.0
project: agent-diva-pro
---

# DESIGN.md: 人格与记忆（Persona & Memory）

> 视觉身份文档：定义新页面“人格与记忆”的外观 token。所有 token 继承自 `agent-diva-gui/src/styles.css`，确保与现有 GUI 主题一致。

## Brand & Style

- **设计基调**：浪漫、轻盈、略带赛博感的个人 AI 助手界面。玻璃拟态面板、粉蓝渐变背景、圆润卡片、柔和阴影。
- **信息密度**：中低密度；单屏可见 1 个编辑目标 + 辅助列表，优先聚焦内容。
- **视觉优先级**：强调色仅用于激活态、主按钮、保存反馈；正文与辅助信息使用灰/粉暗色，避免过度装饰。
- **动效原则**：过渡 150–200ms， easing 使用 `ease`；hover 使用轻微上浮/阴影；加载使用脉冲骨架屏而非无限 spinner。

## Colors

### Core Tokens（默认 love 主题）

| Token | Value | Usage |
|-------|-------|-------|
| `--bg-app` | `linear-gradient(160deg, #fff5f7 0%, #ffe4ef 60%, #ffd6e8 100%)` | 页面背景渐变 |
| `--panel` | `rgba(255, 240, 246, 0.85)` | 玻璃面板背景 |
| `--panel-solid` | `#fff0f6` | 实心面板/输入框背景 |
| `--line` | `rgba(255, 182, 193, 0.5)` | 边框、分割线 |
| `--text` | `#6b2737` | 主文本 |
| `--text-muted` | `#7a2f3e` | 辅助文本 |
| `--accent` | `#ec4899` | 主强调色（激活态、主按钮、保存） |
| `--accent-light` | `#f472b6` | hover 高亮 |
| `--accent-glow` | `rgba(236, 72, 153, 0.15)` | 阴影/光晕 |
| `--accent-border` | `rgba(236, 72, 153, 0.4)` | 选中态边框 |
| `--accent-bg-light` | `rgba(236, 72, 153, 0.08)` | 轻柔背景 |
| `--accent-bg-hover` | `rgba(236, 72, 153, 0.12)` | hover 背景 |
| `--danger` | `#ef4444` | 危险/错误 |
| `--success` | `#22c55e` | 成功 |
| `--warning` | `#f59e0b` | 警告 |

### Theme Variants

| Theme | `--bg-app` | `--panel-solid` | `--text` | `--accent` | `--radius` |
|-------|------------|-----------------|----------|------------|------------|
| love（默认） | 粉渐变 | `#fff0f6` | `#6b2737` | `#ec4899` | `12px` |
| dark | `#0f172a` | `rgba(15,23,42,0.88)` | `#e2e8f0` | `#60a5fa` | `12px` |
| default | 白→粉渐变 | `rgba(255,255,255,0.9)` | `#6b2737` | `#ec4899` | `10px` |
| miku | `#0d1117` | `#161b22` | `#e6edf3` | `#39c5bb` | `14px` |

## Typography

| Element | Font | Size | Weight | Line-height |
|---------|------|------|--------|-------------|
| 页面标题 | Inter, sans-serif | `1.25rem` (20px) | 700 | 1.3 |
| 分组标题 | Inter, sans-serif | `0.875rem` (14px) | 600 | 1.4 |
| Section 名称 | Inter, sans-serif | `0.875rem` (14px) | 500 | 1.4 |
| 正文 | Inter, sans-serif | `0.875rem` (14px) | 400 | 1.6 |
| 辅助说明 | Inter, sans-serif | `0.75rem` (12px) | 400 | 1.5 |
| 徽章/标签 | Inter, sans-serif | `0.625rem` (10px) | 600 | 1 | uppercase, tracking-wide |
| 编辑器内容 | `ui-monospace`, SFMono-Regular, Menlo, Consolas, monospace | `0.875rem` (14px) | 400 | 1.6 |

## Layout & Spacing

- **页面内边距**：`p-6`（24px）。
- **卡片内边距**：`1rem`（16px）或 `1.5rem`（24px）。
- **元素间隙**：`8px`、`12px`、`16px` 三档。
- **左侧分组列表宽度**：`280px`（参考 NotebookView 左侧列表）。
- **编辑器区域**：右侧剩余空间 `flex: 1`，最小宽度 `320px`。
- **历史弹窗宽度**：`640px`，最大宽度 `calc(100vw - 48px)`。
- **最大内容宽度**：无硬性限制；编辑器区域随窗口自适应。

## Elevation & Depth

| Token | Value | Usage |
|-------|-------|-------|
| `--shadow` | `0 8px 32px rgba(236, 72, 153, 0.15)` | 主面板/卡片阴影 |
| `--accent-glow` | `rgba(236, 72, 153, 0.15)` | hover 光晕 |
| 模态阴影 | `0 20px 60px rgba(0,0,0,0.22)` | 历史弹窗 |
| 玻璃模糊 | `blur(16px)` | `--panel` 背景 |

## Shapes

| Token | Value | Usage |
|-------|-------|-------|
| `--radius` | `12px`（love/dark）、`10px`（default）、`14px`（miku） | 卡片、容器 |
| `--radius-sm` | `8px`（love/dark/miku）、`6px`（default） | 按钮、输入框、徽章 |
| 胶囊 | `9999px` | 状态徽章 |

## Components

### Primary Button（保存）
```css
background: var(--accent);
color: white;
border-radius: var(--radius-sm);
padding: 0.5rem 1rem;
font-size: 0.875rem;
font-weight: 500;
transition: filter 0.15s ease;
:hover { filter: brightness(1.1); }
:disabled { opacity: 0.5; cursor: not-allowed; }
```

### Secondary Button（历史、重试）
```css
background: transparent;
color: var(--text-muted);
border: 1px solid var(--line);
border-radius: var(--radius-sm);
padding: 0.5rem 0.75rem;
font-size: 0.875rem;
transition: background 0.15s ease;
:hover { background: var(--accent-bg-light); }
```

### Section List Item
```css
padding: 0.5rem 0.75rem;
border-radius: var(--radius-sm);
color: var(--text);
font-size: 0.875rem;
transition: background 0.15s ease;
:hover { background: var(--accent-bg-light); }
&.active {
  background: var(--panel-solid);
  border-left: 3px solid var(--accent);
  color: var(--accent);
  box-shadow: 0 2px 8px var(--accent-glow);
}
```

### Section Status Badge
| Status | Background | Text | Border |
|--------|------------|------|--------|
| owned | `var(--accent-bg-light)` | `var(--accent)` | `1px solid var(--accent-border)` |
| tbd | `transparent` | `var(--text-muted)` | `1px solid var(--line)` |

### Markdown Editor（新建组件）
```css
.editor-textarea {
  width: 100%;
  min-height: 320px;
  padding: 1rem;
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  background: var(--panel-solid);
  color: var(--text);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.875rem;
  line-height: 1.6;
  resize: vertical;
  transition: border-color 0.15s ease, box-shadow 0.15s ease;
}
.editor-textarea:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px var(--accent-glow);
}
```

### Preview Pane
```css
.preview-pane {
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: var(--radius-sm);
  padding: 1rem;
  overflow-y: auto;
}
```

### History Modal
- 遮罩：`fixed inset-0 bg-black/45 backdrop-blur-[2px] z-[600]`
- 卡片：`max-w-[640px] w-full bg-[var(--panel-solid)] rounded-[var(--radius)] shadow-[0_20px_60px_rgba(0,0,0,0.22)]`
- 列表项：`border-b border-[var(--line)] last:border-b-0 py-3`
- 复制按钮：Secondary Button 样式，成功切换为 `--success` 色 1 秒后恢复。

### Empty State
```css
.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem 1rem;
  color: var(--text-muted);
  text-align: center;
}
.empty-state-icon {
  opacity: 0.5;
  margin-bottom: 1rem;
}
```

### Skeleton
```css
.skeleton-line {
  height: 12px;
  border-radius: 4px;
  background: var(--accent-bg-light);
  animation: skeleton-pulse 1.5s ease-in-out infinite;
}
@keyframes skeleton-pulse {
  0%, 100% { opacity: 0.4; }
  50% { opacity: 0.8; }
}
```

## Do's and Don'ts

- **Do** 使用 CSS 变量，不要硬编码颜色或圆角。
- **Do** 保存按钮仅在内容变更且未保存时启用。
- **Do** 对 hover/active 态使用 `--accent-bg-light` / `--accent-glow`，保持低对比度光晕。
- **Don't** 在编辑器区域使用纯白色背景，应使用 `--panel-solid` 以融入主题。
- **Don't** 对“待定”section 使用红色/警告色，它只是未初始化，不是错误。
- **Don't** 在历史弹窗中展示 diff/回滚按钮（本期 deferred）。

## Dependencies

- `vue-i18n@11.4.4`
- `lucide-vue-next@0.575.0`（图标）
- `markdown-it` + `highlight.js`（Markdown 渲染）
- `tailwindcss`（布局/间距工具类）
- 项目全局样式：`agent-diva-gui/src/styles.css`
