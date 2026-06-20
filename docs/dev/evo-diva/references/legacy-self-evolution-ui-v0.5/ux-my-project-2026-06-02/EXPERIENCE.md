---
title: "agent-diva-pro 自进化 + 沙箱 UI — 体验设计"
status: final
created: "2026-06-02"
updated: "2026-06-02"
project: agent-diva-pro
source: "../planning-artifacts/prds/prd-my-project-2026-06-02/prd.md"
design_ref: "DESIGN.md"
---

# EXPERIENCE.md

## Foundation

**Form-factor**: Desktop only（Tauri v2 桌面应用，Windows/macOS/Linux）。
**UI system**: agent-diva-pro 现有 GUI（Vue 3 Composition API + Tailwind CSS + 四主题 CSS 变量）。
**Visual identity**: `DESIGN.md` 持有视觉规范，本文件持有行为和交互。所有视觉 token 通过 `{section.key}` 引用 DESIGN.md。

## Information Architecture

### 侧边栏（NormalMode.vue 扩展）

```
Chat          ← 现有，增强
  ├─ 决策卡    (DecisionCard.vue)
  ├─ Todo 卡   (TodoCard.vue)
  └─ 审批横幅  (ApprovalBanner.vue)
记事本        ← 新增
Journal       ← 保留不动
Pet           ← 保留不动
Settings      ← 扩展
  └─ Self Evolution 子页  ← 新增
      ├─ 自进化开关/策略
      ├─ Memory Changelog
      └─ 沙箱设置分段
Console       ← 保留
Neuro         ← 保留
Cron          ← 保留
```

### 页面层级

| 页面 | 路由 | 类型 | 复用模式 |
|------|------|------|---------|
| ChatView | 默认视图 | 修改 | 在原 tool 消息渲染区插入卡片槽 |
| NotebookView | 新页面 | 新建 | 参考 Journal 的双栏模式（左列表 + 右详情） |
| SelfEvolutionSettings | Settings 子页 | 新建 | 参考现有 ProvidersSettings 的表单模式 |
| SandboxSettingsSection | SelfEvolution 子分段 | 新建 | 表单字段集 + 列表编辑 |

## Voice and Tone

- **决策卡标题**: 动词开头，陈述 Diva 的意图（"计划执行以下操作""建议修改长期记忆"）
- **审批按钮**: 简洁动词（"允许""拒绝""同意执行"），不解释后果（后果在卡片正文）
- **记事本报告**: 叙事语气，第一人称（Diva 视角），保留 AutoDream 原文风格
- **设置标签**: 名词短语（"自进化开关""审批策略""触发频率"），不解释含义（tooltip 负责）
- **空态提示**: 友好但克制（"暂无本周报告""还没有变更记录"），不用拟人化过度表达
- **错误提示**: 技术中性（"加载失败，请检查 gateway 连接"），不用可爱化错误信息

## Component Patterns

### DecisionCard（决策卡）

**位置**: ChatView 消息流内，作为 assistant 消息的内联组件。
**触发**: Diva 进入 Plan 模式后，生成结构化计划时。

| 属性 | 说明 |
|------|------|
| 状态 | `draft` → `approved` / `rejected` |
| 布局 | 卡片容器（`{DESIGN.md.components.decision-card}`），标题 + 摘要 + 步骤列表 + 风险等级 + 操作按钮 |
| 风险标识 | 三色圆点：绿（low）/ 黄（medium）/ 红（high），位于标题右侧 |
| 步骤列表 | 有序列表，每项带编号圆圈 |
| 操作 | 底部右对齐 `[拒绝]` `[同意执行]` |
| 交互 | 点击按钮 → emit `card-action` 事件 → ChatView 转发到 manager API |
| 状态变更 | `approved` → 卡片收起为"已批准"摘要行，自动插入 TodoCard |
| 异常 | 请求超时（5秒）→ 按钮 disabled + toast "操作超时，请重试" |

### TodoCard（Todo 卡）

**位置**: ChatView 消息流内，可出现在任意 assistant 消息后。
**触发**: 决策卡 `approved` 后自动生成，或 Diva 直接生成（日常对话中）。

| 属性 | 说明 |
|------|------|
| 状态 | `active` → `completed` |
| 布局 | 卡片容器，标题 "📋 待办" + Todo 项列表 |
| 每项 Todo | 复选框 + 文本 + 完成时间（done 态） |
| 勾选行为 | 点击复选框 → emit `todo-check` → 当前项 toggle `pending`/`done` → 即时更新 |
| 全部完成 | 底部 "全部完成" 按钮 → 所有项标记 done → 卡片折叠 |
| 持久化 | Todo 状态通过 manager API 写入 session 存储 |
| 折叠 | 全部完成后 3 秒自动折叠为 "已完成" 摘要行（可展开） |
| 异常 | 写入失败 → toast "保存失败" + 复选框回退到之前状态 |

### ApprovalBanner（即时审批横幅）

**位置**: ChatView 消息流内，作为独立的系统消息卡片。
**触发**: Diva 执行敏感工具操作前（shell 命令 / 文件写 / 网络请求）。

| 属性 | 说明 |
|------|------|
| 状态 | `pending` → `approved` / `rejected` / `expired` |
| 布局 | 紧凑横向卡片（高度 ~64px），操作描述 + 风险标签 + 两个按钮 |
| 风险标签 | 左侧色条 (绿/黄/红) + 文字标签 |
| 操作 | `[允许]` (primary) `[拒绝]` (secondary) |
| 超时 | 5 分钟倒计时 → 自动 `expired` → 卡片替换为"已过期（自动拒绝）" |
| 倒计时显示 | 最后 60 秒显示剩余秒数 "59s 后自动拒绝" |
| 键盘 | Enter = 允许，Escape = 拒绝（仅当审批卡片处于焦点时） |
| 不阻断 | 审批卡片不阻止消息流滚动，用户可跳过继续阅读 |

### NotebookView（记事本页面）

**位置**: 侧边栏"记事本"入口 → 新页面。
**导航**: 侧边栏点击 → 路由 `/notebook`。

| 属性 | 说明 |
|------|------|
| 布局 | 左侧 280px 报告列表 + 右侧详情（参考 Journal 双栏模式） |
| 标签栏 | 三标签切换：日报 / 周报 / 月报，默认"周报" |
| 报告列表 | 按时间倒序，每项显示：日期 + 标题 + 摘要（截断 100 字） |
| 当前选中 | 粉色左边框（`{DESIGN.md.colors.accent}`）+ 浅粉背景 |
| 详情区 | Markdown 渲染（复用现有 markdown-body 样式） |
| 底部操作栏 | 固定于详情区底部，三个按钮：`[固化为 SOP]` `[固化为技能]` `[更新长期记忆]` |
| 固化弹窗 | 点击按钮 → `appConfirm()` 弹窗（复用现有 AppDialogLayer），展示变更内容 + 影响范围 |
| 空态 | "暂无{日报/周报/月报}" + 说明文字 "AutoDream 尚未生成报告" |
| 加载 | skeleton 占位（三行灰色条） |
| 错误 | "加载失败" + 重试按钮 |

### SelfEvolutionSettings（自进化设置子页面）

**位置**: Settings → Self Evolution（SettingsView 第 10 个子页面）。
**导航**: Settings dashboard 卡片点击进入。

| 属性 | 说明 |
|------|------|
| 布局 | 表单式垂直排列，分组卡片（参考 ProvidersSettings 模式） |
| 分组 1: 自进化控制 | Toggle 开关（总开关） + 频率下拉选择（每日/每周/手动） + 触发阈值滑块 |
| 分组 2: 学习策略 | 信任度滑块（0.0-1.0） + 强制确认类型多选（身份/关系/承诺/SOP/deprecate） |
| 分组 3: Memory Changelog | 只读时间线列表，分页 20 条/页 |
| 保存 | 右上角固定 "保存设置" 按钮（`{DESIGN.md.components.btn-primary}`） |
| 反馈 | 保存成功 → toast "设置已保存" + 按钮变为 checkmark 1.5 秒后恢复 |
| 异常 | 保存失败 → toast "保存失败: {原因}" |

### SandboxSettingsSection（沙箱设置分段）

**位置**: SelfEvolution 页面内，作为独立分段卡片。
**参考**: 沙箱分支 `SandboxSettings.vue` 的配置项映射。

| 属性 | 说明 |
|------|------|
| 布局 | 表单字段集，分组在 `sandbox` 卡片内 |
| 沙箱模式 | 下拉选择：DangerFullAccess / ReadOnly / WorkspaceWrite |
| 审批策略 | 下拉选择：Never / OnFailure / OnRequest / UnlessTrusted |
| 网络访问 | Toggle 开关 |
| 可写根目录 | 标签式列表（添加 + 删除按钮），每项显示路径 |
| 保护路径 | 同上 |
| 拒绝模式 | 多行文本输入（textarea），每行一个模式 |
| 超时 | 数字输入框（秒） |
| 模式变更提醒 | 切换沙箱模式时底部显示提醒条"⚠ 需重启 gateway 生效"（黄色背景） |
| 空态 | deny_patterns 为空时显示 placeholder "每行一个 shell 命令前缀，如 rm -rf" |

## State Patterns

### 所有新组件的统一状态矩阵

| 状态 | DecisionCard | TodoCard | ApprovalBanner | NotebookView | Settings 表单 |
|------|-------------|----------|---------------|-------------|-------------|
| **加载中** | skeleton 占位（灰色卡片） | — | — | skeleton 列表 | spinner + disabled |
| **正常** | 渲染卡片 | 渲染列表 | 渲染横幅 | 渲染双栏 | 可编辑表单 |
| **空** | — | — | — | "暂无报告" | 默认值 |
| **成功** | "已批准" 摘要 | "已完成" 折叠 | "已允许" | — | "已保存" checkmark |
| **错误** | toast + 重试 | toast + 回退 | — | toast + 重试 | toast + 保留输入 |
| **禁用** | 按钮 disabled（请求中） | — | 按钮 disabled（已过期） | — | 非管理员/网关断开 |
| **过期** | — | — | "已过期（自动拒绝）" | — | — |

## Interaction Primitives

| 原语 | 规格 | 来源 |
|------|------|------|
| 过渡 | `0.15s ease`（hover/active/focus），`0.2s ease`（展开/折叠） | 现有约定 |
| 按钮反馈 | hover scale(1.02) + brightness(1.1)，active scale(0.98) | 现有 btn-primary |
| 卡片 hover | 上移 1px + 边框变色 + 阴影增强 | 现有 settings-card |
| 列表选中 | 左 4px accent 边框 + 浅 accent 背景 | 现有 channel 列表 |
| 失败回退 | checkbox 状态恢复 / 表单保留输入 / toast 显示原因 | — |
| 焦点环 | 2px accent box-shadow，无 outline | 现有 settings-input |
| 模态框 | scale(0.95) → scale(1) + opacity 0→1，0.2s | 现有 AppDialogLayer |
| 键盘 | Tab 导航，Enter 确认，Escape 关闭/拒绝 | 标准 |

## Accessibility Floor

- Tab 顺序：侧边栏 → 主内容区 → 操作按钮。新增组件插入自然位置。
- 焦点指示器继承现有 `focus:shadow-[0_0_0_2px_var(--accent)]`。
- 所有交互元素有 `aria-label`（审批按钮："允许执行 {操作描述}"；todo 复选框："标记 {todo内容} 为已完成"）。
- 颜色不单独传达信息——风险等级同时使用颜色 + 文字标签。
- 键盘可完成任务流：Tab 到审批横幅 → Enter 允许 / Escape 拒绝；Tab 到 Todo 列表 → Space 勾选。
- 对比度：确保 accent 色 (`#ec4899`) 在白色背景上 ≥4.5:1 的文字对比度（按钮使用白色文字 on accent 背景）。

## Key Flows

### Flow 1: Plan → Decision → Todo

```
1. ChatView: 用户输入 "帮我做 X"
2. Diva 生成 DecisionCard（draft 态）插入消息流顶部
3. 用户滚动到卡片，评估风险（黄色 medium），点击 [同意执行]
4. 卡片 → approved 态，收起为摘要行，下方自动插入 TodoCard（active 态）
5. Diva 逐个执行，每完成一步 → 对应 Todo 项自动标记 done
6. 全部完成 → 卡片折叠为 "已完成" 摘要行
```

**高潮节拍**: 第三步——用户看到风险后仍然点同意，信任感建立。

### Flow 2: 日常对话 Todo

```
1. ChatView: 用户 "提醒我三件事：A、B、C"
2. Diva 回复文本的同时生成 TodoCard，三项 pending
3. 用户手动勾掉 A（点击复选框）→ 显示删除线 + "刚刚完成"
4. 用户手动勾掉 B → 同上
5. 用户点击 [全部完成] → 三项全部 done → 卡片折叠
```

**高潮节拍**: 无——这是工具性流程，高潮在日常完成感。

### Flow 3: 记事本节律治理

```
1. ChatView: Diva 发送系统消息 "📋 本周记事本已生成" + [查看记事本] 按钮
2. 用户点击 → 路由跳转 `/notebook`，默认选中 "周报"
3. 报告列表自动定位到本周报告（高亮）
4. 右侧渲染完整报告 Markdown，底部显示操作栏
5. 用户看到 "本周 3 次提到 Rust workspace 结构" → 点击 [固化为 SOP]
6. appConfirm() 弹窗：展示变更内容 + 影响范围
7. 用户确认 → toast "SOP 已固化" → 操作栏按钮变灰（已处理）
```

**高潮节拍**: 第七步——确认后，用户意识到 "Diva 学到了一项新的长期知识"。

### Flow 4: 即时审批

```
1. ChatView: 对话中 Diva 需要执行 `rm -rf /tmp/test/`
2. 消息流底部插入 ApprovalBanner（pending 态）
3. 横幅显示 "Diva 请求执行：删除临时目录 /tmp/test/" + 风险 low 标签 + [允许] [拒绝]
4. 用户点击 [允许] → 横幅 → approved 态 → "已允许" 2 秒后自动消失
5. Diva 继续执行
```

**高潮节拍**: 第三步——用户看到操作描述后瞬间做出信任判断。

---

## 设计决策确认

- ✅ 记事本报告数据源 = `.laputa/rhythm/` 目录的 Markdown 文件
- ✅ 审批超时默认 5 分钟，可通过 Settings 调整 `approval_timeout_seconds`
- ✅ 沙箱模式变更后需重启 gateway（当前架构不支持热切换 SandboxConfig）
- ✅ ChatView 卡片渲染复用现有 tool 消息区组件插槽
