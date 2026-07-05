---
title: "Persona & Memory — Experience Specification"
status: draft
created: 2026-07-05
updated: 2026-07-05
version: 0.1.0
project: agent-diva-pro
related_prds:
  - docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md
related_architecture:
  - docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md
---

# EXPERIENCE.md: 人格与记忆（Persona & Memory）

> 体验规格文档：定义新页面“人格与记忆”的信息架构、行为、状态、交互和文案。视觉 token 引用 `DESIGN.md`。

## Foundation

- **形态**：桌面端 Webview（Tauri），单窗口应用内页面；不支持独立窗口或移动端。
- **UI 系统**：基于现有 `agent-diva-gui` 的自定义 CSS 变量 + Tailwind 工具类，无第三方组件库。
- **视觉身份引用**：所有颜色/圆角/阴影使用 `{DESIGN.md#Colors}`、`{DESIGN.md#Shapes}`、`{DESIGN.md#Components}` 中定义的 token。
- **语言**：支持中英文切换，通过 `vue-i18n` 读取 `locales/zh.ts` 与 `locales/en.ts`。

## Information Architecture

### 入口
- 位置：侧边栏“功能”板块最上方，名为“人格与记忆”（英文 “Persona & Memory”）。
- 图标：[ASSUMPTION] 使用 `BookUser` 或 `Brain` lucide 图标，与“人格/记忆”语义相关。
- 点击后 `activeMenu = 'persona-memory'`，主内容区替换为 `PersonaMemoryView.vue`。

### 页面结构
```
PersonaMemoryView.vue
├── Header（页面标题 + 全局刷新按钮 + 保存反馈）
├── Body（list/detail 双栏）
│   ├── Left: SectionGroupList.vue
│   │   └── 4 个分组，每组可折叠
│   │       └── 14 个 section 项（名称 + 状态徽章 + 最后更新时间）
│   └── Right: SectionEditor.vue
│       ├── Toolbar（section 标题 + 状态 + 历史按钮 + 保存按钮）
│       ├── Editor Pane（textarea）
│       └── Preview Pane（渲染后 Markdown）
└── HistoryModal.vue（遮罩弹窗，条件渲染）
```

### 14 Section 分组
| 分组 | 中文名 | Sections |
|------|--------|----------|
| 人格 | Persona | identity, relationship, commitment, preferences |
| 记忆 | Memory | memory_md, history_md |
| 周期 | Periodic | daily, weekly, monthly |
| 索引 | Indexes | journal_reflective, proposal_inbox, changelog, report_indexes, aaak_summaries |

- 默认全部展开。
- 首次进入默认选中 `identity`。
- 刷新/返回页面后保留最后选中的 section（可选，本期可不做）。

## Voice and Tone

- 语气：助手式、温和、直接。避免技术黑话，也避免过度亲昵。
- 操作按钮：使用动词开头，如“保存”、“查看历史”、“复制内容”。
- 空状态：说明“还没有内容”以及用户能做什么。
- 错误：说明发生了什么 + 下一步怎么做，不堆砌堆栈。

### 文案 Key 建议（新增到 locales）
```ts
laputa: {
  title: '人格与记忆',
  subtitle: '管理 Diva 的长期人格与记忆内容',
  loading: '正在加载 Laputa 数据…',
  loadError: '无法加载记忆数据',
  retry: '重试',
  save: '保存',
  saved: '已保存',
  saveFailed: '保存失败：{message}',
  saving: '保存中…',
  history: '历史',
  copy: '复制内容',
  copied: '已复制',
  emptyTitle: '此 section 还没有内容',
  emptyDesc: '在右侧编辑器中输入 Markdown 内容，然后点击保存。',
  uninitializedTitle: 'Laputa 尚未初始化',
  uninitializedDesc: '保存任意 section 后，系统将自动创建记忆骨架。',
  groups: {
    persona: '人格',
    memory: '记忆',
    periodic: '周期',
    indexes: '索引',
  },
  sections: { /* 14 section 中英文映射 */ },
  status: {
    owned: '已就绪',
    tbd: '待定',
  },
  confirm: {
    title: '确认保存',
    message: '确定要覆盖 {section} 的当前内容吗？此操作会生成一条审计记录。',
  },
  historyModal: {
    title: '{section} 的变更历史',
    empty: '暂无变更记录',
  },
}
```

## Component Patterns

### SectionGroupList.vue
- 每个分组有一个可点击标题，点击切换展开/折叠，右侧显示 `ChevronDown` / `ChevronRight`。
- Section 项显示：
  - 左侧图标（[ASSUMPTION] 所有 section 统一使用 `FileText` 或按分组使用不同图标）。
  - 中文 section 名（来自 `locales`）。
  - 右侧状态徽章（`owned` / `tbd`）。
- 选中项：左边框 3px accent + 背景 panel-solid + 文字 accent。
- hover：背景 `--accent-bg-light`。

### SectionEditor.vue
- 顶部 toolbar：
  - 左侧：section 中文名 + 状态徽章 + 最后更新时间。
  - 右侧：历史按钮（Secondary Button）+ 保存按钮（Primary Button）。
- 编辑区：textarea，最小高度 320px，可垂直拉伸。
- 预览区：实时渲染 Markdown（使用 `markdown-it` + `highlight.js`）。
- 当右侧空间不足时，[ASSUMPTION] 可切换为“编辑/预览”双 tab，或默认上下堆叠。

### HistoryModal.vue
- 触发：点击 toolbar“历史”按钮。
- 遮罩：`fixed inset-0 bg-black/45 backdrop-blur-[2px] z-[600]`，点击遮罩关闭。
- 内容：
  - 标题：`{section} 的变更历史`
  - 列表：时间、动作（apply）、操作者、内容摘要（前 120 字符）。
  - 每行操作：复制该版本 `after` 内容到剪贴板。
- 关闭：右上角 `X` 按钮 或 点击遮罩。
- 加载：使用 skeleton-line。
- 空态：显示“暂无变更记录”。

## State Patterns

### 初始加载
```
加载 snapshot → 渲染左侧分组列表 → 默认选中 identity → 加载 section content
```
- `loading` 时左侧显示 skeleton，右侧显示居中大 spinner。
- `error` 时全屏错误态，提供重试按钮。

### Section 切换
- 若当前 editor 有未保存变更（`isDirty === true`），[ASSUMPTION] 弹出 `appConfirm` 确认是否放弃修改。
- 切换后清空 error，重新加载目标 section。

### 编辑与 Dirty 状态
- 本地 `draftContent` 与原始 `originalContent` 比较：`isDirty = draftContent !== originalContent`。
- 保存按钮启用条件：`isDirty && !saving && !loading`。
- 离开页面或切换 section 前，若 `isDirty`，弹出确认。

### 保存流程
```
点击保存 → saving=true → 调用 writeLaputaSection → 成功：刷新内容 + toast 已保存 + isDirty=false → 失败：显示错误 + 保留 draft
```
- 保存前 [ASSUMPTION] 是否二次确认？建议对非空覆盖弹出 `appConfirm`；首次创建可直接保存。
- 保存失败时，保留用户输入，不重置 draft。

### 历史弹窗状态
- 打开时异步加载 `listLaputaChangelog({ target_section })`。
- 每行 copy 按钮点击后：
  - 调用 `navigator.clipboard.writeText(record.after)`；
  - 按钮文案临时变为“已复制”，1 秒后恢复。

### 空状态
- `.laputa` 未初始化：显示 uninitialized 文案 + 引导保存。
- section 无内容：显示 empty 文案 + 引导输入。
- changelog 为空：显示“暂无变更记录”。

## Interaction Primitives

### 点击
- 点击 section 项：切换选中并加载内容。
- 点击分组标题：切换展开/折叠。
- 点击“保存”：提交变更。
- 点击“历史”：打开弹窗。
- 点击遮罩/关闭按钮：关闭历史弹窗。

### Hover
- section 项 hover：背景变亮。
- 按钮 hover：brightness 提升或背景变亮。
- 历史列表项 hover：背景 `--accent-bg-light`。

### Focus
- textarea focus：边框 accent + 外发光。
- 保存按钮 focus：标准浏览器焦点环或自定义 outline。

### 键盘
- textarea 内：支持标准 Markdown 编辑快捷键（无需自定义）。
- 弹窗内：Escape 关闭。
- 全局：无新增快捷键。

### 滚轮/滚动
- 左侧列表可独立滚动。
- 编辑区和预览区可独立滚动。
- 历史弹窗内容超出时内部滚动。

## Accessibility Floor

- 所有交互元素可通过键盘 Tab 到达。
- 按钮和输入框有明确的 focus 样式。
- 状态徽章不依赖颜色 alone：使用文字 + 颜色。
- 错误信息不只用颜色，使用图标 + 文案。
- [ASSUMPTION] 为 section 列表和编辑器区域添加语义化 `role`/`aria-label`（如 `role="navigation"`、`aria-label="Laputa section editor"`）。

## Key Flows

### Flow 1: 首次查看并编辑 Identity
1. 用户点击侧边栏“人格与记忆”。
2. 页面加载 snapshot，左侧高亮 `identity`，右侧加载其 Markdown 内容。
3. 用户在 textarea 中修改内容，保存按钮自动启用。
4. 用户点击“保存”，后端合成 proposal 并 apply，返回 changelog。
5. 右侧刷新为最新内容，显示“已保存”toast，左侧最后更新时间更新。

### Flow 2: 查看变更历史并复制
1. 用户在 `identity` 页面点击“历史”。
2. 弹窗加载该 section 的 changelog。
3. 用户看到刚保存的记录，点击“复制内容”。
4. 剪贴板获得该版本的完整内容，按钮短暂显示“已复制”。
5. 用户点击遮罩或关闭按钮关闭弹窗。

### Flow 3: 处理未保存切换
1. 用户修改 `identity` 后未保存。
2. 用户点击 `relationship`。
3. 系统检测到 `isDirty`，弹出确认：“当前修改未保存，是否放弃？”
4. 用户选择“放弃”，切换并加载 `relationship`；选择“取消”，保持当前编辑状态。

### Flow 4: 未初始化状态
1. 用户首次进入页面，`.laputa` 不存在。
2. 右侧显示“Laputa 尚未初始化”空态。
3. 用户在任意 section 输入内容并保存。
4. 后端创建 `.laputa` 骨架并写入 section，页面进入正常状态。

## Responsive & Platform

- 最小窗口宽度：[ASSUMPTION] 支持 1024px 以上；低于 1024px 时左侧列表可折叠为图标栏或切换为上下布局。
- 高度：页面占满 `app-shell` 主内容区，无纵向外层滚动；编辑区内部滚动。
- 平台：仅桌面 Tauri；无需触控优化。

## Open Questions / Assumptions

1. [ASSUMPTION] 侧边栏图标使用 `BookUser`；可替换为 `Brain`、`Library` 或用户指定图标。
2. [ASSUMPTION] 保存前对“覆盖非空内容”进行二次确认；首次创建不确认。
3. [ASSUMPTION] 切换 section 时若存在未保存内容，弹出放弃确认。
4. [ASSUMPTION] Markdown 预览默认与编辑区并排；空间不足时切换为上下堆叠或 tab。
5. [ASSUMPTION] 历史弹窗每次打开都重新加载 changelog，不做本地缓存。
6. [ASSUMPTION] 14 个 section 的中文名统一维护在 `locales` 中，不来自后端。

## Related Artifacts

- PRD: `docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md`
- Architecture Spine: `docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md`
- Visual Identity: `DESIGN.md`（本目录）
- Implementation Reference:
  - `agent-diva-gui/src/components/NotebookView.vue`
  - `agent-diva-gui/src/components/EvolutionView.vue`
  - `agent-diva-gui/src/components/console/ConfigEditor.vue`
  - `agent-diva-gui/src/components/settings/SelfEvolutionSettings.vue`
