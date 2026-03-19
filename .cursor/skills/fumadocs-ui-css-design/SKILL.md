---
name: fumadocs-ui-css-design
description: Guide UI layout, component styling, and CSS/Design system decisions for Fumadocs-based documentation sites, including responsive layout, typography, color system, and theme customization. Use when the user is designing or refining Fumadocs UI or styles.
---

# Fumadocs UI 与 CSS 设计

## 使用场景

在以下情况使用本 Skill：

- 需要为 Fumadocs 文档站设计整体 UI 风格与布局
- 需要编写/重构文档站的 CSS、Tailwind、设计系统或主题配置
- 需要优化文档阅读体验（排版、对比度、响应式、暗色模式等）
- 需要为 Agent Diva 相关文档统一视觉风格

## 设计原则

1. **优先可读性**
   - 正文字号适中，行宽控制在合理范围（一般 60–80 字符）
   - 段落间距清晰，标题层级明显
   - 链接、按钮等交互元素有足够的对比度和悬停反馈

2. **与 Fumadocs 生态对齐**
   - 优先使用 Fumadocs/框架已有的布局组件与主题扩展接口
   - 不要在全局范围内随意覆盖核心组件的基础样式，使用可配置化方式（如 theme tokens、CSS 变量）
   - 尽可能避免引入与现有栈冲突的 UI 框架

3. **响应式与多设备适配**
   - 先设计移动端/窄屏布局，再扩展到桌面端
   - 确保导航栏、侧边栏在小屏幕下有合理的折叠/抽屉方案
   - 表格、代码块等宽内容在小屏下需支持横向滚动或折叠

4. **暗色模式与主题切换**
   - 使用 CSS 变量或 design tokens 管理颜色，而不是直接写死具体色值
   - 保证亮/暗两种模式下的对比度均符合可读性要求
   - 主题切换的状态应可持久（如 localStorage）且不会闪屏严重

5. **与 Agent Diva 品牌/语气匹配**
   - 视觉风格偏工程化、理性、简洁，避免花哨视觉噪音
   - 重点突出信息结构和可用性，而不是炫技动画
   - 如需使用品牌色，保持在按钮、链接、强调标记等有限区域内使用

## CSS / Tailwind 编写规范

1. **结构与职责**
   - 布局层（Layout）与组件层（Components）样式分离
   - 避免在同一选择器中混合过多职责（布局 + 颜色 + 动画）

2. **命名与可维护性**
   - 若使用原生 CSS/SCSS，推荐 BEM 风格或语义化 class 名
   - 若使用 Tailwind，提取重复组合为组件或 `@apply`，避免极长的 utility 列表

3. **主题变量**
   - 使用 `--color-bg`, `--color-fg`, `--color-accent` 等语义化变量名
   - 不直接写如 `#fff`, `#000`，而是引用变量，方便全局主题调整

## Fumadocs UI 设计工作流（建议）

当用户请求设计或调整 Fumadocs UI 时，按以下步骤工作：

1. **分析现状**
   - 查看现有布局组件、导航结构、主色调
   - 确认是否已有设计系统或 Tailwind 配置

2. **明确目标**
   - 是新建整体皮肤，还是微调现有风格？
   - 优先目标是可读性、品牌一致性，还是功能密度？

3. **提出方案**
   - 以简洁清单形式给出几个可选方向（如 “极简”、“工程化”、“文档中心型”）
   - 对每个方向简要说明配色、排版、布局特点

4. **给出样例代码**
   - 提供关键组件或页面的样例（如布局 shell、导航栏、内容区域）
   - 使用项目实际使用的技术栈（如 Tailwind/纯 CSS）

5. **校验与微调**
   - 检查在亮/暗模式、不同屏幕宽度下的表现
   - 对齐 Agent Diva 与 Fumadocs 的既有视觉/交互约定

## 输出格式建议

当用户请求 UI 或 CSS 方案时，推荐输出结构：

```markdown
## 设计目标
- 目标 1
- 目标 2

## 布局方案
[文字说明 + 简要结构示意]

## 颜色与排版
- 颜色：主色 / 辅色 / 背景 / 边框 / 链接
- 字体：正文字号、标题层级、行高

## 样例代码
```css
/* 或 Tailwind 组合示例 */
```

## 使用建议
- 如何在现有 Fumadocs 项目中接入
```

## 与代码开发 Skill 的配合

本 Skill 专注于 “长什么样” 和 “怎么排版”，与 `fumadocs-code-dev` Skill 的职责划分如下：

- `fumadocs-code-dev`：负责文档结构、内容组织、代码示例和路由
- `fumadocs-ui-css-design`：负责布局、颜色、排版、交互细节

在回答综合问题时，先确保信息架构与内容正确，再进行 UI 与样式层面的优化建议。

