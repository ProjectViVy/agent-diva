---
status: final
updated: 2026-06-10
project_name: Agent-Diva Mask System
form_factor: desktop
ui_system: Tauri + Vue 3 + Tailwind CSS
---

# DESIGN.md — Mask System Visual Identity

> 继承 agent-diva-gui 现有设计系统，不引入新的视觉语言。

---

## Brand & Style

面具系统是 agent-diva 的功能扩展，视觉上延续现有风格：
- **Love 主题**：粉色渐变背景 + 毛玻璃面板 + 圆角卡片
- **Dark 主题**：深蓝背景 + 暗色面板 + 蓝色强调
- **Miku 主题**：青绿色调

面具图标使用 emoji，与现有 `topbar-avatar` 的 emoji 风格一致。

---

## Colors

不引入新颜色。复用现有 CSS 变量：

| 用途 | 变量 | Love 主题值 | Dark 主题值 |
|------|------|-------------|-------------|
| 面具选中背景 | `--accent-bg-hover` | `rgba(236,72,153,0.12)` | `rgba(96,165,250,0.12)` |
| 面具选中边框 | `--accent-border` | `rgba(236,72,153,0.4)` | `rgba(96,165,250,0.4)` |
| 面板背景 | `--panel` | `rgba(255,240,246,0.85)` | `rgba(15,23,42,0.88)` |
| 文字 | `--text` | `#6b2737` | `#e2e8f0` |
| 次要文字 | `--text-muted` | `#7a2f3e` | `#94a3b8` |

---

## Typography

复用现有字体系统，无新增。

---

## Layout & Spacing

面具相关组件的间距遵循现有 `--radius`（12px）和 `--radius-sm`（8px）。

---

## Components

### MaskSwitcherPopover

**位置**：Topbar 左侧，`topbar-avatar` 区域
**触发**：点击当前面具 emoji
**样式**：
- 弹出面板：`--panel` 背景 + `--glass-blur` + `--radius` 圆角
- 面具列表项：hover 时 `--accent-bg-hover`
- 选中项：`--accent-border` 左边框 + `--accent` 图标色
- 宽度：200px
- 最大高度：300px（可滚动）

### MaskSettingsPanel

**位置**：Settings 页面内，作为新 tab
**样式**：
- 卡片：复用 `ChannelCard` 的卡片风格
- 编辑器：复用 `ProviderWizardModal` 的表单风格
- 预置面具：网格布局，每个面具一张卡片

### MaskEditorModal

**位置**：Settings > Masks > 编辑/创建
**样式**：
- 复用 `ChannelWizardModal` 的弹窗结构
- 基础模式：表单字段（name, icon, description, model, tools）
- 高级模式：YAML 编辑器（monospace 字体）

---

## Do's and Don'ts

**Do：**
- 复用现有 CSS 变量和组件风格
- 使用 emoji 作为面具图标
- 保持毛玻璃效果和圆角一致性

**Don't：**
- 不引入新的颜色系统
- 不使用与现有风格冲突的扁平设计
- 不在面具切换器中放复杂编辑功能
