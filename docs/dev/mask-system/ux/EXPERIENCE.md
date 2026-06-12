---
status: final
updated: 2026-06-10
project_name: Agent-Diva Mask System
form_factor: desktop
sources:
  - docs/dev/mask-system/prd.md
  - docs/dev/mask-system/architecture.md
  - docs/dev/mask-system/epics.md
---

# EXPERIENCE.md — Mask System Experience Design

---

## Foundation

**Form-factor**：Desktop（Tauri + Vue 3）
**UI System**：现有 agent-diva-gui 组件库
**Visual Identity**：见 `DESIGN.md`

---

## Information Architecture

### 面具系统入口

```
主界面
├── Topbar（顶部栏）
│   └── 面具快速切换器（点击 emoji 弹出）
│       ├── 当前面具显示
│       ├── 面具列表（最近使用 + 全部）
│       └── "管理面具" 链接 → 跳转 Settings
│
└── Settings（设置页面）
    └── Masks（面具管理 tab）
        ├── 面具卡片网格
        │   ├── 预置面具（researcher, coder, reviewer, assistant）
        │   └── 自定义面具
        ├── 创建面具按钮
        └── 面具编辑器弹窗
            ├── 基础模式（表单）
            └── 高级模式（YAML）
```

### 页面清单

| 页面 | 入口 | 核心功能 |
|------|------|----------|
| MaskSwitcherPopover | Topbar emoji 点击 | 快速切换面具 |
| MaskSettingsPanel | Settings > Masks | 面具列表、管理、创建 |
| MaskEditorModal | Settings > Masks > 编辑 | 面具文件编辑 |

---

## Voice and Tone

面具系统的文案风格与现有 agent-diva 一致：
- 简洁、友好、带一点可爱（emoji 风格）
- 中文为主
- 错误提示友好，不吓人

**示例文案：**
- 面具切换提示："🎭 已切换为「研究员」模式"
- 空状态："还没有面具呢～点击创建你的第一个面具"
- 加载失败："面具文件读取失败，请检查文件格式"

---

## Component Patterns

### MaskSwitcherPopover

**行为：**
- 点击 `topbar-avatar` 弹出
- 点击外部区域关闭
- Escape 键关闭
- 选择面具后立即切换并关闭

**内容结构：**
```
┌─────────────────────────┐
│ 🎭 当前：我就是我      │
├─────────────────────────┤
│ 最近使用                │
│   🔍 研究员             │
│   💻 程序员             │
├─────────────────────────┤
│ 全部面具                │
│   🔍 研究员             │
│   💻 程序员             │
│   📝 审查员             │
│   🤖 助手               │
├─────────────────────────┤
│ ⚙️ 管理面具 →          │
└─────────────────────────┘
```

**状态：**
- 默认：显示当前面具 emoji + 名称
- 弹出：显示面具列表，当前面具高亮
- 切换中：显示 loading 状态
- 切换成功：短暂 toast 提示

### MaskSettingsPanel

**行为：**
- 作为 Settings 页面的一个 tab
- 显示所有面具的卡片网格
- 支持创建、编辑、删除操作

**卡片结构：**
```
┌──────────────────────┐
│ 🔍 研究员            │
│ 专注调研与分析        │
│──────────────────────│
│ 模型：deepseek-chat  │
│ 工具：4 个允许        │
│──────────────────────│
│ [编辑] [删除]        │
└──────────────────────┘
```

**状态：**
- 加载中：骨架屏
- 空状态：引导创建第一个面具
- 正常：卡片网格
- 删除确认：二次确认弹窗

### MaskEditorModal

**行为：**
- 从 Settings > Masks 打开
- 支持基础模式和高级模式切换
- 实时预览面具效果

**基础模式字段：**
- 名称（必填）
- 图标（emoji 选择器）
- 描述（可选）
- 模型（下拉选择，可选）
- 工具限制（allow/deny 列表编辑）
- Prompt 内容（文本区域）

**高级模式：**
- 直接编辑 YAML frontmatter + Markdown body
- 语法高亮
- 格式校验

---

## State Patterns

### 面具切换状态机

```
[无面具/默认] → wear → [切换中] → success → [已佩戴]
                          ↓
                        error → [无面具/默认] + toast 错误

[已佩戴] → off → [切换中] → success → [无面具/默认]
                    ↓
                  error → [已佩戴] + toast 错误
```

### 面具列表加载状态

```
[idle] → load → [loading] → success → [loaded]
                           → error → [error] + 重试按钮
```

---

## Interaction Primitives

### 面具切换动画

- 弹出：`scale(0.95) → scale(1)` + `opacity(0) → opacity(1)`，150ms
- 收起：反向，100ms
- 面具选中：左侧 `--accent` 边框滑入，200ms

### 面具切换反馈

- 视觉：emoji 变化 + 名称更新
- 文字：toast 提示 "🎭 已切换为「XXX」模式"
- 声音：无（保持安静）

---

## Accessibility Floor

- 键盘导航：Tab 切换面具，Enter 确认，Escape 关闭
- ARIA：弹出菜单 `role="menu"`，菜单项 `role="menuitem"`
- 焦点管理：弹出时焦点锁定在菜单内

---

## Key Flows

### Flow 1: 快速切换面具

**主角**：大湿，正在使用 agent-diva 进行代码审查

1. 大湿点击 Topbar 的 emoji（当前显示 😊）
2. 弹出面具选择菜单，显示 "我就是我"（当前）+ 最近使用的面具
3. 大湿点击 "📝 审查员"
4. emoji 变为 📝，名称变为 "审查员"
5. toast 提示 "🎭 已切换为「审查员」模式"
6. 大湿继续对话，agent 表现为审查模式（只读工具）

**高潮**：面具切换后，agent 的行为立即改变，但人格不变。

### Flow 2: 创建新面具

**主角**：大湿，需要一个专门的翻译助手面具

1. 大湿进入 Settings > Masks
2. 点击 "创建面具" 按钮
3. 弹出编辑器，基础模式
4. 填写：名称="翻译官"，图标=🌐，描述="专业翻译助手"
5. 设置工具限制：只允许 read_file, web_search
6. 编写 prompt："你是一个专业的翻译官，擅长中英互译..."
7. 点击保存
8. 面具出现在 Settings 列表中
9. 大湿点击 emoji → 面具列表中出现 "🌐 翻译官"

**高潮**：创建的面具立即可用，无需重启。

### Flow 3: 面具切换失败

**主角**：大湿，尝试切换到一个损坏的面具文件

1. 大湿点击 emoji，选择 "🔧 工具人"
2. 系统尝试加载面具文件
3. frontmatter 解析失败
4. toast 提示 "面具文件读取失败，请检查文件格式"
5. 面具保持不变（仍是之前的面具）
6. 大湿进入 Settings > Masks，看到 "🔧 工具人" 卡片显示错误状态
7. 大湿点击编辑，修复 YAML 格式
8. 保存后，面具恢复正常

**高潮**：失败时优雅降级，不影响当前工作。

---

## Responsive & Platform

面具系统仅支持 Desktop（Tauri），无响应式需求。

窗口最小宽度：800px
弹出菜单自适应：窄窗口时菜单向上展开
