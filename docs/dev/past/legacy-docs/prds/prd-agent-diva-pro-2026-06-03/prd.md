---
title: 桌宠内嵌视图全屏交互优化
status: final
created: 2026-06-03
updated: 2026-06-03
---

# PRD: 桌宠内嵌视图全屏交互优化

## 0. Document Purpose

本文档定义 agent-diva GUI 中桌宠内嵌视图（`DivaPetView`，通过主窗口侧边栏进入）的交互行为升级。受众为前端开发。结构：Glossary 定义关键术语，Features 按交互场景分组，FRs 全局编号供下游引用。

现有相关代码：
- `NormalMode.vue` — 导航壳（侧边栏 + topbar + 内容区）
- `DivaPetView.vue` — 宠物内嵌视图（VRM 模型 + 聊天面板）
- 参考模式：`NormalMode.vue.before-refactor`（原始 overlay 侧边栏实现）

## 1. Vision

当前侧边栏是常驻折叠模式——点击"宠物"后侧边栏和顶部栏仍在，宠物视图只能占据剩余空间，浪费了主窗口面积，削弱沉浸感。本项目将宠物视图升级为**全屏沉浸模式**：点击侧边栏"宠物"→ 侧边栏自动折叠，topbar 消失，宠物视图铺满整个主面板。侧边栏变为 overlay 浮层（汉堡按钮呼出，5 秒无操作自动消失）。页面顶部只保留浮动迷你状态栏（情绪 + 连接状态 + 模型选择）。

核心价值：最大化宠物画面可视面积，减少无关 UI 干扰，同时保留快速切换模型的便利性。

## 2. Target User

### 2.1 Jobs To Be Done

- 作为 DiVA 用户，我想在查看宠物时获得最大的视觉沉浸感，不被侧边栏和顶部栏挤占空间
- 作为 DiVA 用户，我偶尔需要切模型或查看连接状态，但不希望这些控件占据固定屏幕空间
- 作为 DiVA 用户，我在宠物页面短暂离开后回来，侧边栏应该自动消失以恢复沉浸体验

### 2.2 Key User Journeys

- **UJ-1. 大湿进入宠物页面开始对话。**
  - **Persona + context:** 大湿，日常使用 agent-diva，想和 DiVA 聊天。
  - **Entry state:** 在主窗口任意页面，侧边栏可见。
  - **Path:** 点击侧边栏"宠物" → 侧边栏自动折叠，topbar 消失 → DivaPetView 铺满整个主面板 → 迷你状态栏浮现在左上区域（显示"😊 开心 · 在线" + 模型选择按钮）。
  - **Climax:** 宠物全屏呈现，用户可以立即看到完整的 VRM 模型和聊天面板。
  - **Resolution:** 处于全屏沉浸模式，迷你状态栏持续浮动显示。

- **UJ-2. 大湿在全屏宠物页面想切换模型。**
  - **Entry state:** 宠物全屏模式，迷你状态栏浮动显示。
  - **Path:** 点击迷你状态栏上的模型按钮 → 下拉菜单弹出 → 选择目标模型 → 菜单关闭。
  - **Climax:** 模型切换完成，迷你状态栏更新显示新模型名。
  - **Resolution:** 回到全屏沉浸模式。

- **UJ-3. 大湿在全屏宠物页面需要打开侧边栏。**
  - **Entry state:** 宠物全屏模式，侧边栏隐藏。
  - **Path:** 点击左上角汉堡按钮（DivaPetView 现有 Menu 按钮）→ 侧边栏 overlay 从左侧滑出 → 5 秒内任意鼠标移动/键盘操作重置计时器 → 5 秒无操作侧边栏自动消失。
  - **Climax:** 侧边栏浮层出现，可以进行导航。
  - **Resolution:** 5 秒无操作后侧边栏消失，回到全屏模式。若在侧边栏内点击了非宠物页面，侧边栏关闭且 NormalMode 恢复正常顶栏+侧边栏。

## 3. Glossary

- **全屏沉浸模式 (Fullscreen Immersive Mode):** 宠物页面特有状态——侧边栏隐藏、topbar 消失、DivaPetView 占据整个 main-panel、迷你状态栏浮动显示。
- **迷你状态栏 (Mini Status Bar):** 宠物全屏模式下浮动在视图左上区域的紧凑 UI，包含：情绪 emoji + 情绪标签 + 连接状态圆点 + 在线/离线文字 + 模型选择下拉按钮。视觉风格沿用 DivaPetView 现有 `pet-glass` 半透明样式。
- **Overlay 侧边栏:** 从左侧滑出的浮层侧边栏（与原始 `before-refactor` 版本相同模式），带半透明遮罩，点击遮罩关闭，5 秒无操作自动消失。
- **自动消失计时器 (Auto-Dismiss Timer):** 5 秒倒计时，每次鼠标移动或键盘按键重置。计时归零自动关闭 overlay 侧边栏。仅在全屏沉浸模式下生效。

## 4. Features

### 4.1 宠物全屏沉浸模式

**Description:** 用户从侧边栏点击"宠物"导航项后进入的特殊布局状态。侧边栏自动折叠隐藏，topbar 消失，DivaPetView 铺满 NormalMode 的 main-panel 全部空间。迷你状态栏浮动在视图左上。此模式仅影响主窗口内嵌宠物视图，不影响独立桌面弹出窗口（DesktopPetOverlay）。实现 UJ-1。

`[ASSUMPTION: desktopPetActive prop 当前未从 NormalMode 传入 DivaPetView——本次改动不涉及 prop 传递，仅改变 NormalMode 的布局逻辑。]`

**Functional Requirements:**

#### FR-1: 进入宠物页面自动进入全屏

用户点击侧边栏导航项进入宠物页面时，NormalMode 自动切换到全屏沉浸模式。

**Consequences (testable):**
- `activeMenu === 'pet'` 时，侧边栏 CSS class 添加 `sidebar-hidden`（display:none 或 transform 移出视口）
- `activeMenu === 'pet'` 时，topbar（`<header class="topbar">`）不渲染（`v-if` 排除 pet）
- DivaPetView 容器占满 `.main-panel` 的 100% 宽高
- 迷你状态栏在 DivaPetView 内渲染

#### FR-2: 离开宠物页面恢复正常布局

用户从宠物页面导航到任意其他页面（chat、settings、console 等）时，NormalMode 恢复标准布局：侧边栏回到折叠/展开状态，topbar 重新显示。

**Consequences (testable):**
- `activeMenu !== 'pet'` 时，侧边栏恢复 `sidebarCollapsed` 之前的状态
- `activeMenu !== 'pet'` 时，topbar 正常渲染
- 切换回 chat 页面后，消息列表和输入框正常可用

#### FR-3: 迷你状态栏浮动显示

宠物全屏模式下，迷你状态栏浮动在视图左上区域，z-index 高于宠物内容但低于 overlay 侧边栏。

**Consequences (testable):**
- 显示当前情绪 emoji + 情绪标签（如 "😊 开心"）
- 显示连接状态圆点 + 在线/离线文字
- 显示模型选择下拉按钮（当前模型名 + provider 名）
- 点击模型按钮弹出下拉菜单，菜单行为与当前 topbar 模型下拉一致（复用 `savedModels`、`selectSavedModel` 逻辑）
- 视觉风格使用 `pet-glass` class（半透明毛玻璃效果）
- `[ASSUMPTION: 模型下拉的 savedModels 数据由 NormalMode 通过 props 传入 DivaPetView，或在 DivaPetView 内通过 inject/provide 获取——优先 props，与 existing pattern 一致。]`

**Out of Scope:**
- 不改变模型切换的后端逻辑

### 4.2 Overlay 侧边栏（宠物页面专用）

**Description:** 在全屏沉浸模式下，复用 DivaPetView 现有的左上角 Menu 按钮（`pet-edge-button absolute top-4 left-4`），点击呼出 overlay 侧边栏。实现模式参考 `NormalMode.vue.before-refactor` 的 overlay sidebar 实现。实现 UJ-3。

**Functional Requirements:**

#### FR-4: 汉堡按钮呼出 overlay 侧边栏

点击 DivaPetView 左上角 Menu 按钮 → overlay 侧边栏从左侧滑入。

**Consequences (testable):**
- DivaPetView 的 Menu 按钮 `@click` emit `toggle-sidebar` → 由 NormalMode 处理
- Overlay 侧边栏包含与常驻侧边栏相同的导航项
- 半透明遮罩覆盖主内容区，点击遮罩关闭侧边栏
- z-index: 遮罩 < overlay 侧边栏，且两者均在 DivaPetView 上方
- `[ASSUMPTION: overlay 侧边栏的 DOM 结构直接复用 NormalMode 现有的 `<aside class="sidebar">` 但改为 absolute 定位 + 遮罩，而不是另写一套。]`

#### FR-5: 5 秒无操作自动消失

Overlay 侧边栏打开后启动 5 秒计时器。每次 `mousemove` 或 `keydown` 事件重置计时器。计时归零自动关闭侧边栏。

**Consequences (testable):**
- 侧边栏打开后 5 秒内无鼠标/键盘事件 → 侧边栏关闭
- 鼠标移动（任意位置）重置计时器为 5 秒
- 键盘按键重置计时器为 5 秒
- 关闭后计时器清除（不泄漏）
- 仅在全屏沉浸模式（activeMenu === 'pet'）下生效

**Out of Scope:**
- 其他页面（chat / settings / console 等）不启用 5 秒自动消失
- 桌面弹出窗口（DesktopPetOverlay）不受影响

## 5. Non-Goals (Explicit)

- 不改变桌面独立弹出窗口（DesktopPetApp / DesktopPetOverlay）的任何行为
- 不改变 NormalMode 在非宠物页面的侧边栏/顶栏行为
- 不改变 ConversationSidebar（会话列表侧边栏）
- 不添加新的后端 API 或配置项——纯前端布局改动

## 6. MVP Scope

### 6.1 In Scope

- 宠物页面进入时侧边栏自动折叠 + topbar 消失
- 迷你浮动状态栏替代 topbar
- Overlay 侧边栏（汉堡呼出 + 5s 自动消失）
- 离开宠物页面时恢复正常布局

### 6.2 Out of Scope for MVP

- 侧边栏动画/过渡效果（v1 用 instant toggle，后续可加 slide 动画）
- 5 秒超时时间可配置化（v1 硬编码 5000ms）
- 迷你状态栏位置可拖拽

## 7. Success Metrics

**Primary**
- **SM-1**: 用户进入宠物页面后，不额外操作即可看到全屏宠物视图（侧边栏自动隐藏率 100%）。验证 FR-1。

**Secondary**
- **SM-2**: 侧边栏 overlay 在 5 秒无操作后自动关闭（自动关闭成功率 100%）。验证 FR-5。

**Counter-metrics (do not optimize)**
- **SM-C1**: 不因全屏模式引入额外的页面切换延迟。SM-1 和 SM-2 的实现不应增加宠物页面渲染时间 > 50ms。

## 8. Assumptions Index

- [§4.1] `desktopPetActive` prop 当前未从 NormalMode 传入——本次不改 prop 传递，仅改 NormalMode 布局
- [§4.1 FR-3] 模型下拉数据通过 props 从 NormalMode 传入 DivaPetView
- [§4.2 FR-4] Overlay 侧边栏复用 NormalMode 现有 `<aside class="sidebar">`，改为 absolute 定位
