# Plan/Todo UI 范围提取摘要

> 来源: `pre-implementation-research.md` + `planning-gui-design-supplement.md`
> 提取日期: 2026-06-02

---

## 一、本期定义范围（What's In）

### 后端核心（PR #1-#6 顺序交付）

1. **类型 + 存储层** (PR #1)
   - `agent-diva-core::planning` 模块：PlanId/TodoId、Plan/PlanStep/TodoList/TodoItem 领域类型
   - `SqlitePlanningStore`（SQLite, `.agent-diva/planning.db`，5 表 + 外键约束）
   - Markdown 投影生成（`plan.md`/`todo.md`，投影至 `.agent-diva/plans/<plan-id>/`）

2. **Agent 集成** (PR #2-#4)
   - ContextBuilder 中注入 ActivePlan + TodoList 上下文（≤800 字符，位于 Memory block 之后）
   - AgentLoop 5 个 hook 点：HOOK-1 注入 Plan context，HOOK-2~5 处理工具调用/状态同步
   - 2 个 MVP 工具：`todo_write`（全量替换模式，仿 Claude Code V1）+ `todo_show`
   - PlanOrchestrator gate 规则（禁止多 in_progress、blocked 必须有 reason、无审批不执行）
   - NAG 机制：连续 3 轮有 pending 项但不调 planning 工具时，注入催促 prompt

3. **Manager API** (PR #5)
   - RESTful 端点：`GET/POST /api/plans`, `GET/PUT/DELETE /api/plans/:plan_id`
   - `ManagerCommand` 新增 5 个变体：ListPlans/GetPlan/CreatePlan/UpdatePlan/DeletePlan
   - `PlanningService`（仿 SkillService/McpService 懒创建模式）

4. **五层防绕过机制**
   - 文件系统拦截 → 工具层统一入口 → PlanOrchestrator gate 校验 → 不暴露裸路径 → 事件审计日志

### GUI 范围

#### Phase A（PR #2 后启动，约 2-3 天）

| 交付物 | 说明 |
|--------|------|
| `desktop.ts` DTO + invoke | `PlanDto`, `PlanDetailDto`, `TodoItemDto`, `PlanStepDto` 类型定义；`get_plans()`, `get_plan()`, `get_active_plan()` 三个 Tauri invoke |
| Rust Tauri commands | `get_plans`, `get_plan`, `get_active_plan`，通过 `api_tx` → `ManagerCommand` → `PlanningService` |
| `TodoInlineCard.vue` | ChatView 中当 `toolName === 'todo_write'` 或 `'todo_show'` 时，做结构化卡片渲染（替代纯文本截断） |
| i18n 字符串 | `zh.ts` / `en.ts` 新增 `planning` 命名空间（标题、阶段、优先级、状态等） |

#### Phase B（PR #4 后启动，约 3-4 天）

| 交付物 | 说明 |
|--------|------|
| `PlanningView.vue` | 主视图：双栏布局（左 280px Plan 列表 + 右详情），仿 `CronTaskManagementView` |
| `PlanStatusCard.vue` | Active Plan 概览卡片（phase/status/进度/验证 四格状态指示器） |
| `TodoListPanel.vue` | TodoList 进度面板，按状态分组（进行中 → 待处理 → 阻塞 → 已完成） |
| `TodoItemRow.vue` | 单条 TodoItem 行：序号 + 状态图标 + 标题 + 证据链接 + 优先级标签 + 阻塞原因 tooltip |
| 侧边栏入口 | `NormalMode.vue` 侧边栏新增 `planning` section（Chat → Settings → Console → Neuro → Cron → **Planning**） |
| 轮询刷新 | 仿 `CronTaskManagementView` 的 `LOG_POLL_MS` 模式，轮询 `get_active_plan` |

---

## 二、延后范围（What's Out）

| 项目 | 延后原因 | 重新触发条件 |
|------|---------|-------------|
| GUI 完整视图中 Plan 创建/编辑 | 第一期 GUI 只读，修改通过 Agent Chat 完成 | PR #4 完成后，Phase C |
| Plan 审批流程 UI | 用户通过聊天文字确认即可 | 需要显式审批交互时 |
| Kanban 看板 UI | 本期范围外，单 worker 串行足够 | autonomous-evolution loop 成熟后 |
| DAG / 依赖图 | 串行执行已覆盖当前场景 | 出现可并行工作量证明 |
| CLI `plan` 命令 | Manager API + agent tool 已覆盖核心场景 | 用户反馈需要终端操作 |
| `plan_update` 工具 | PlanOrchestrator 拥有 Plan 所有变更权 | 需要模型动态调整 Plan 时 |
| 正式 sqlx migration | `CREATE TABLE IF NOT EXISTS` 足够 | schema 第二次破坏性变更 |
| 实时 WebSocket 推送 | 轮询 `get_active_plan` 足够 | 需要低延迟推送时 |
| Plan 历史列表 + 归档 | Phase C 范围 | Phase B 完成后 |
| Verification 面板 | Phase C 范围 | Phase B 完成后 |

### 第一期明确不做的 GUI 项（来自 supplement §10）

- Plan 创建表单弹窗（但侧边栏预留 `[+]` 按钮占位）
- GUI 内编辑 Todo（但 TodoItemRow 预留 `@click` handler）
- 批量操作 checkbox
- Verification 面板（但底部预留区域）

---

## 三、GUI 组件树 / 信息架构

### 技术栈

```
Vue 3 (Composition API) + TypeScript + Vite 6 + Tailwind CSS 3
+ Tauri v2 + lucide-vue-next v0.575
+ markdown-it + highlight.js (GitHub Dark)
+ vue-i18n (zh/en)
+ 主题: 'love' 模式（粉色系 yandere palette + 樱花/爱心粒子动画）
```

### 组件树（新增）

```
src/components/planning/
├── PlanningView.vue          # 主视图：双栏（左 Plan 列表 + 右详情）
│   ├── PlanStatusCard.vue    # Active Plan 概览卡片（4 格状态指示器）
│   │   └── StatusBadge       # 子组件：k-v 对格子（label + 语义色 value）
│   ├── TodoListPanel.vue     # TodoList 进度面板（分组列表）
│   │   └── TodoItemRow.vue   # 单条 TodoItem 行（序号/状态图标/标题/证据/优先级/tooltip）
│   └── PlanHistoryList.vue   # 历史 Plan 列表（Phase C）
└── TodoInlineCard.vue        # ChatView 中工具结果的结构化卡片渲染
```

### 挂载点

- **`NormalMode.vue`** 侧边栏新增 `'planning'` section（在 Cron 按钮旁）
- **`ChatView.vue`** L332 区域：当 `msg.toolName` 为 planning 工具时，插入 `TodoInlineCard` 替代纯文本截断

### 导航结构变化

```
Sidebar (NormalMode.vue):
├── Chat        → ChatView
├── Settings    → SettingsView
├── Console     → ConsoleView
├── Neuro       → (占位)
├── Cron        → CronTaskManagementView
└── Planning    → PlanningView           ← 新增
```

### 状态颜色体系（全 GUI 统一）

| 状态 | 背景 | 文字色 | 图标 |
|------|------|--------|------|
| pending | `gray-100` | `gray-600` | `Circle` |
| in_progress | `pink-100` | `pink-700` | `Loader2` (动画) |
| completed | `green-100` | `green-700` | `CheckCircle2` |
| blocked | `amber-100` | `amber-700` | `AlertCircle` |
| canceled | `gray-100` | `gray-400` | `XCircle` |
| failed | `red-100` | `red-700` | `XCircle` |

### Tauri 命令通道

```
Vue (desktop.ts)               Rust (Tauri commands)            Manager
─────────────────────────────────────────────────────────────────────
getPlans()          ──invoke──→ get_plans        ──api_tx──→ ListPlans      → PlanningService
getPlan(id)         ──invoke──→ get_plan         ──api_tx──→ GetPlan        → PlanningService
getActivePlan()     ──invoke──→ get_active_plan  ──api_tx──→ (active_plan表) → PlanningService
```

---

## 四、PR 阶段与依赖关系

### 后端 PR 序列

```
PR #1: Types + Store           [~300行]  无用户可见变化，纯基础设施
  └── 依赖: 无
  └── 产出: domain 类型、SqlitePlanningStore、Markdown 渲染、5 表 CRUD

PR #2: Context + todo_show     [~150行]  注入 + 只读工具
  └── 依赖: PR #1
  └── 产出: ContextBuilder 改造、HOOK-1 实现、todo_show 工具

PR #3: todo_write + NAG        [~200行]  模型可操作 TodoList
  └── 依赖: PR #2
  └── 产出: todo_write 全量替换工具、NAG 机制、HOOK-2~5

PR #4: PlanOrchestrator gates  [~250行]  审批 + 验证 + 阶段转换
  └── 依赖: PR #3
  └── 产出: PlanOrchestrator、gate 规则、阶段转换逻辑

PR #5: Manager API             [~200行]  HTTP 端点
  └── 依赖: PR #1 (仅 Store)，可与 PR #2-#4 部分并行
  └── 产出: /api/plans 路由、ManagerCommand 变体、PlanningService

PR #6: Verification + Closure  [~150行]  终止态转换
  └── 依赖: PR #4
  └── 产出: 验证流程、completed/failed 终止、事件日志完整性
```

### GUI 阶段与后端依赖

```
Phase A (PR #2 后) ──── 依赖 PR #2
  ├── desktop.ts DTO + invoke
  ├── Rust Tauri commands
  ├── TodoInlineCard.vue
  └── i18n 字符串

Phase B (PR #4 后) ──── 依赖 PR #4 + Phase A
  ├── PlanningView.vue
  ├── PlanStatusCard.vue
  ├── TodoListPanel.vue + TodoItemRow.vue
  └── NormalMode.vue 侧边栏入口

Phase C (延后) ─────── 依赖 Phase B
  ├── Plan 创建表单
  ├── 历史列表 + 归档
  ├── 审批 UI
  └── Verification 面板
```

### 关键依赖链

```
PR #1 ──→ PR #2 ──→ Phase A (GUI)
  │         │
  │         └──→ PR #3 ──→ PR #4 ──→ Phase B (GUI)
  │                         │
  └──→ PR #5 (可并行)       └──→ PR #6
```

---

## 五、关于合并到 agent-diva-pro 的决策

在 `pre-implementation-research.md` 及 `planning-gui-design-supplement.md` 中，**未找到任何关于合并到 agent-diva-pro 的显式决策或讨论**。文档中搜索 "agent-diva-pro"、"merge"、"合并" 均无匹配。

当前所有设计均以 `agent-diva` 仓库内的 `agent-diva-gui` (Tauri) 为目标平台。如果后续需要将 Planning UI 迁移至 agent-diva-pro，需额外评估：
- Tauri invoke 通道 vs agent-diva-pro 的通信机制差异
- Vue 组件复用策略（共享组件库 vs 复制适配）
- Manager API 是否对 agent-diva-pro 的网络访问开放

当前无此决策记录。

---

## 附录：关键设计原则（来自原文）

1. **最小 schema**：V1 用 5 表，不做 DAG/worker/claim 字段
2. **单 Active Plan 约束**：`active_plan` 表保证全局唯一
3. **全量替换 Todo**：仿 Claude Code V1，每次 `todo_write` 完整覆盖列表
4. **≤800 字符上下文注入**：防止主上下文污染
5. **第一期 GUI 只读**：修改仅通过 Agent 聊天工具完成，GUI 只是查询视图
6. **无过度设计**：触发条件明确后才扩展到 Kanban/DAG/多 worker
