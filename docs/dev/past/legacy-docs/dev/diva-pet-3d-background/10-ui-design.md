# 10 — UI 设计文档

## 1. 目标组件

### 1.1 DivaPetView — 场景快速切换

DivaPetView 是主窗口中的内嵌桌宠面板。当前已有功能按钮：
- **左上角齿轮** (Settings): 模型管理器
- **右上角情绪标签**: 当前情绪显示

**新增**: 齿轮按钮旁边，场景快速切换按钮。

### 1.2 布局变化

```
┌─ DivaPetView ──────────────────────────┐
│ ┌─────────────────────────────────────┐│
│ │ [⚙] [🖼▼]              😊 neutral ││  ← 按钮行
│ ├─────────────────────────────────────┤│
│ │                                     ││
│ │          Canvas (Three.js)          ││  ← 3D 场景背景 + VRM
│ │          [场景快速下拉]              ││
│ │                                     ││
│ ├─────────────────────────────────────┤│
│ │ 🎤 语音 | 🔊 TTS | 👁 ASR         ││
│ ├─────────────────────────────────────┤│
│ │ 消息列表...                         ││
│ ├─────────────────────────────────────┤│
│ │ [输入框__________________] [发送]  ││
│ └─────────────────────────────────────┘│
└────────────────────────────────────────┘
```

### 1.3 场景下拉菜单

点击场景按钮后展开：

```
┌─────────────────┐
│ 🖼️ 透明背景    │  ← 默认 (active: 蓝色高亮)
│ 🏠 室内场景    │
│ 🌊 海边场景    │
│ 🚀 太空场景    │
└─────────────────┘
```

**样式**: 毛玻璃背景 + 圆角卡片，与现有模型管理器风格一致：
- `bg-white/90 backdrop-blur-sm rounded-lg border border-gray-100 shadow-lg`
- 选中项: `bg-pink-50 text-pink-600`
- 悬停项: `bg-gray-50`
- 过渡: `transition-colors duration-150`

### 1.4 交互行为

- 点击按钮 → toggle 下拉
- 点击场景项 → 设置 `petConfig.selectedGaussSceneId` → 关闭下拉
- 点击场景外区域 → 关闭下拉
- 当前选中项高亮

---

## 2. PetSettings — 场景配置

### 2.1 位置

在现有 "基本设置" section 之后，ASR section 之前。

### 2.2 布局

```
┌─ PetSettings ────────────────────────────┐
│ 🐱 Diva Pet 设置                         │
│                                           │
│ ┌─ 基本设置 ─────────────────────────────┐│
│ │ 启用桌宠                    [ON/OFF]   ││
│ │ 当前模型: Alice                        ││
│ └────────────────────────────────────────┘│
│                                           │
│ ┌─ 📺 3D 背景场景 ──────────────────────┐│
│ │                                         ││
│ │  ○ 🖼️ 透明背景                        ││
│ │     默认模式，仅显示 VRM 模型和阴影     ││
│ │                                         ││
│ │  ● 🏠 室内场景                        ││
│ │     温馨的室内环境，Gaussian Splat 渲染 ││
│ │                                         ││
│ │  ○ 🌊 海边场景                        ││
│ │     阳光海滩环境，约 500K splats        ││
│ │                                         ││
│ │  ○ 🚀 太空场景                        ││
│ │     深邃太空环境，约 100K splats        ││
│ │                                         ││
│ │  💡 切换立即生效，场景加载约 1-3 秒    ││
│ └────────────────────────────────────────┘│
│                                           │
│ ┌─ 🎤 ASR 语音输入 ─────────────────────┐│
│ └────────────────────────────────────────┘│
└───────────────────────────────────────────┘
```

### 2.3 样式

复用现有的 `.settings-card`, `.settings-title`, `.scene-option` 风格：

```css
.scene-option {
  @apply flex items-center gap-3 p-3 rounded-lg border cursor-pointer
         transition-all duration-150;
  @apply border-gray-200 bg-white;
}
.scene-option:hover {
  @apply border-pink-200 bg-pink-50/30;
}
.scene-option.selected {
  @apply border-pink-300 bg-pink-50;
}
.scene-radio {
  @apply accent-pink-500 w-4 h-4;
}
.scene-icon {
  @apply text-xl w-8 text-center;
}
.scene-label {
  @apply text-sm font-medium text-gray-800;
}
.scene-desc {
  @apply text-xs text-gray-400;
}
.section-hint {
  @apply text-xs text-gray-400 mt-3 italic;
}
```

---

## 3. 视觉一致性

| 元素 | 现有风格 | 新增 | 一致 |
|------|----------|------|------|
| 按钮尺寸 | 28×28 圆角 | 28×28 圆角 | ✅ |
| 下拉菜单 | 毛玻璃卡片 | 白底卡片 | ⚠️ 轻微不同 (适配内嵌) |
| 选中高亮 | 粉色系 | 粉色系 | ✅ |
| 过渡曲线 | ease-out | ease-out | ✅ |
| 字体大小 | 11-14px | 11-14px | ✅ |
| 圆角 | rounded-lg (8px) | rounded-lg | ✅ |
| 图标 | lucide-vue-next | Emoji (🖼️🏠🌊🚀) | ⚠️ 暂用 Emoji |
