# Plan + TodoList GUI Design Supplement

> 补充日期: 2026-06-01
> 关联文档: `docs/dev/agent-plan/pre-implementation-research.md`

---

## 1. GUI 当前架构

| 层级 | 技术 | 文件 |
|------|------|------|
| 框架 | Vue 3 (Composition API) + TypeScript | `src/App.vue` (1382行) |
| 构建 | Vite 6 + vue-tsc | `vite.config.ts` |
| 桌面壳 | Tauri v2 (`@tauri-apps/api`) | `src-tauri/` |
| 样式 | Tailwind CSS 3 + 自定义 `yandere` 粉色系 | `tailwind.config.js` |
| 图标 | lucide-vue-next v0.575 | 全局 |
| 渲染 | markdown-it + highlight.js (GitHub Dark) | `ChatView.vue` |
| 国际化 | vue-i18n (zh/en) | `src/locales/zh.ts`, `en.ts` |
| 主题 | `love` 模式：樱花/爱心粒子动画，粉白渐变 | `NormalMode.vue`, `ChatView.vue` |
| 路由 | 无 vue-router，使用 `activeTab`/`activeMenu` 状态切换 | `NormalMode.vue` |

### 当前导航结构

```
Sidebar (NormalMode.vue L122):
├── Chat        (activeTab === 'chat')       → ChatView
├── Settings    (activeTab === 'settings')   → SettingsView
├── Console     (activeMenu === 'console')   → ConsoleView
├── Neuro       (activeMenu === 'neuro')     → (未实现，占位)
└── Cron        (activeMenu === 'cron')      → CronTaskManagementView

Header (NormalMode.vue):
├── 模型切换 Dropdown
├── 历史会话 Dropdown
└── Emotion 显示
```

### 关键 UI 模式总结

| 模式 | 参考组件 | 适用场景 |
|------|---------|---------|
| **Card + Icon 标题** | GatewayControlPanel: 带大图标的 section header | Planning 面板标题区 |
| **状态标签** | CronTaskManagementView: `running/scheduled/paused/completed/failed` | TodoItem 状态显示 |
| **列表 + 操作按钮** | CronTaskManagementView: job 列表 + Play/Pause/Delete/Edit 按钮 | Plan/TodoList 列表操作 |
| **表单弹窗** | CronTaskManagementView: create/edit job form | 创建 Plan 弹窗 |
| **工具结果内嵌** | ChatView: tool 消息展开/折叠 toggle | Todo 工具执行结果 |
| **下拉菜单** | NormalMode.vue: 模型/历史 session 下拉 | Plan selector |
| **侧边栏导航** | NormalMode.vue sidebar | Planning 导航入口 |
| **API 封装** | `desktop.ts`: `invoke<T>()` 模式 | Planning API 调用 |

---

## 2. 第一期 UI 范围定义

**第一期做**：
- 聊天视图中 **Todo 工具结果的美化渲染**（ChatView 已有 tool 消息 UI）
- **Planning 独立视图**（新的 sidebar section，仿 CronTaskManagementView）
- **Plan 状态仪表盘**（在 Planning 视图中显示 active plan + todo 进度）
- **Tauri 后端命令补充**（`get_plans`、`get_plan`、`create_plan` 等）

**第一期不做**：
- Plan Mode 审批流程 UI（用户通过聊天文字确认即可）
- Kanban 看板 UI
- GUI 内编辑 Plan/Todo（第一期只读查询视图，修改通过 agent chat 完成）
- 实时 Todo 进度推送（轮询 `get_plan` 即可）

---

## 3. 组件树设计

### 3.1 新增组件

```
src/components/planning/
├── PlanningView.vue           # 主视图（替代 CronTaskManagementView 的位置）
│   ├── PlanStatusCard.vue     # Active Plan 概览卡片
│   ├── TodoListPanel.vue      # TodoList 进度面板
│   │   └── TodoItemRow.vue    # 单个 TodoItem 行
│   └── PlanHistoryList.vue    # 历史 Plan 列表（可选第一期）
```

### 3.2 组件挂载点

`NormalMode.vue` 中：

```typescript
// L122: SidebarSection 类型新增
type SidebarSection = 'chat' | 'settings' | 'console' | 'neuro' | 'cron' | 'planning';

// L125: 新增
const activeMenu = ref<'console' | 'neuro' | 'cron' | 'planning' | null>(null);

// L226-236: navigateTo 新增 case
if (section === 'planning') {
  activeMenu.value = 'planning';
}

// Template 中 sidebar nav list 新增按钮
// 在 Cron 按钮旁添加 Planning 按钮
```

---

## 4. 各组件详细设计

### 4.1 PlanningView.vue（主视图）

**布局**: 双栏（左侧 Plan 列表，右侧详情）

```
┌──────────────────────────────────────────────┐
│  📋 规划管理                                  │
│                                              │
│  ┌─────────────┐  ┌─────────────────────────┐│
│  │ Plan 列表    │  │ Plan 详情                ││
│  │             │  │                         ││
│  │ [Active]    │  │ 标题: xxx               ││
│  │  Plan A  ●  │  │ 状态: 执行中             ││
│  │  3/5 done   │  │ 创建: 2026-06-01        ││
│  │             │  │                         ││
│  │  Plan B  ✓  │  │ TodoList (5 items)      ││
│  │  5/5 done   │  │ ┌───────────────────┐   ││
│  │             │  │ │ ✓ 已完成: 3 items  │   ││
│  │  Plan C  ✗  │  │ │   item-001 ✓       │   ││
│  │  2/5 done   │  │ │   item-002 ✓       │   ││
│  │             │  │ │   item-003 ✓       │   ││
│  │             │  │ │                   │   ││
│  │ [+ 新建]    │  │ │ ● 进行中: 1 item   │   ││
│  │             │  │ │   item-004 ●       │   ││
│  │             │  │ │                   │   ││
│  │             │  │ │ ○ 待处理: 1 item   │   ││
│  │             │  │ │   item-005 ○       │   ││
│  │             │  │ └───────────────────┘   ││
│  └─────────────┘  └─────────────────────────┘│
└──────────────────────────────────────────────┘
```

**关键设计点**:
- 左栏宽 280px，右侧填充剩余空间
- Plan 列表项高亮 `active_plan`（单例约束），用粉色边框标识
- TodoList 面板使用 Card + 分组列表风格
- 状态颜色沿用 CronTaskManagementView 的语义色

### 4.2 PlanStatusCard.vue

仿照 GatewayControlPanel 的 section header 风格：

```html
<section class="bg-white border border-gray-100 rounded-xl p-5 space-y-4 shadow-sm">
  <div class="flex items-center gap-3">
    <div class="w-10 h-10 rounded-lg bg-pink-100 text-pink-600 flex items-center justify-center">
      <ClipboardList :size="20" />  <!-- lucide icon -->
    </div>
    <div>
      <h3 class="text-lg font-semibold text-gray-800">{{ plan.title }}</h3>
      <p class="text-sm text-gray-500">{{ plan.goal }}</p>
    </div>
  </div>

  <!-- 状态指示器网格 (仿 CronTaskManagementView) -->
  <div class="grid gap-3 md:grid-cols-4">
    <StatusBadge label="阶段" :value="plan.phase" />
    <StatusBadge label="状态" :value="plan.status" />
    <StatusBadge label="进度" :value="`${completedCount}/${totalCount}`" />
    <StatusBadge label="验证" :value="plan.verification_verdict" />
  </div>
</section>
```

**StatusBadge** 子组件（仿 GatewayControlPanel L210-235 的 k-v 对格子）：

```html
<div class="rounded-lg border border-gray-100 bg-gray-50 px-4 py-3">
  <div class="text-xs text-gray-500">{{ label }}</div>
  <div class="mt-1 text-sm font-medium" :class="statusColor(value)">
    {{ value || '—' }}
  </div>
</div>
```

### 4.3 TodoListPanel.vue

**列表行样式**（仿 CronTaskManagementView 的 job 列表 L340-440）：

```
┌─────────────────────────────────────────────┐
│ #  状态  标题                证据   优先级   │
│                                             │
│ 1  ✓     读取设计文档       [查看]  Normal  │ ← 已完成，绿色
│ 2  ●     实现 TodoListStore  —      High    │ ← 进行中，粉色脉冲
│ 3  ○     添加 context 注入    —      Normal  │ ← 待处理，灰色
│ 4  ⊘     等待用户审批       [原因]  High    │ ← 阻塞，橙色 + reason tooltip
│ 5  ✗     已取消              —      Low     │ ← 取消，删除线
└─────────────────────────────────────────────┘
```

**TodoItemRow.vue** 伪代码：

```html
<div class="flex items-center justify-between px-4 py-3 border-b border-gray-50
            hover:bg-gray-50/50 transition-colors"
     :class="{ 'bg-pink-50/30': item.status === 'in_progress' }">
  <!-- 序号 -->
  <span class="w-8 text-xs text-gray-400">{{ index + 1 }}</span>

  <!-- 状态图标 -->
  <div class="w-8 flex justify-center">
    <CheckCircle2 v-if="item.status === 'completed'" class="text-green-500" :size="18" />
    <Loader2 v-else-if="item.status === 'in_progress'" class="text-pink-500 animate-spin" :size="18" />
    <Circle v-else-if="item.status === 'pending'" class="text-gray-300" :size="18" />
    <AlertCircle v-else-if="item.status === 'blocked'" class="text-amber-500" :size="18" />
    <XCircle v-else class="text-gray-400" :size="18" />
  </div>

  <!-- 标题 + detail -->
  <div class="flex-1 min-w-0 px-3">
    <div class="text-sm text-gray-800 truncate" :class="{ 'line-through text-gray-400': item.status === 'canceled' }">
      {{ item.title }}
    </div>
    <div v-if="item.detail" class="text-xs text-gray-500 truncate mt-0.5">{{ item.detail }}</div>
  </div>

  <!-- 证据链接 -->
  <button v-if="item.evidence_ref"
          class="text-xs text-pink-500 hover:text-pink-700 px-2 py-1 rounded hover:bg-pink-50">
    查看证据
  </button>
  <span v-else class="w-16" />

  <!-- 优先级标签 -->
  <span class="text-xs px-2 py-0.5 rounded-full"
        :class="priorityColor(item.priority)">
    {{ priorityLabel(item.priority) }}
  </span>

  <!-- 阻塞原因 tooltip -->
  <span v-if="item.status === 'blocked' && item.block_reason"
        class="text-xs text-amber-600 ml-2 truncate max-w-[120px]"
        :title="item.block_reason">
    {{ item.block_reason }}
  </span>
</div>
```

**优先级颜色**：

| Priority | Tailwind |
|----------|----------|
| High | `bg-red-100 text-red-700` |
| Normal | `bg-gray-100 text-gray-600` |
| Low | `bg-blue-100 text-blue-600` |

### 4.4 聊天视图中 Todo 工具结果的增强渲染

**现状**：ChatView.vue 已支持 `role === 'tool'` 的消息渲染（L298-380），包括：
- tool 状态图标（running/success/error）
- tool 名称 badge
- 结果截断 + 展开/折叠
- tool 参数和原始 meta

**增强方案**：当 `msg.toolName === 'todo_write'` 或 `msg.toolName === 'todo_show'` 时，在 `toolResult` 区域插入 `TodoInlineCard` 子组件，做结构化渲染。

```html
<!-- ChatView.vue L332 区域，tool result 位置 -->
<template v-if="msg.toolStatus !== 'running' && msg.toolResult">
  <!-- 新增：planning 工具的结构化渲染 -->
  <TodoInlineCard v-if="isPlanningTool(msg.toolName)"
                  :tool-name="msg.toolName"
                  :result="msg.toolResult" />
  <!-- 原有：普通工具截断文本 -->
  <div v-else class="px-3 pb-1 text-xs text-gray-600 break-all whitespace-pre-wrap">
    {{ truncate(msg.toolResult, 160) }}
  </div>
</template>
```

**TodoInlineCard.vue** 设计：

```
┌─────────────────────────────────────┐
│ 📋 Todo List Updated (5 items)      │
│                                     │
│ ● item-003   Implement Store   [进行中]
│ ○ item-004   Add context inj    [待处理]
│ ○ item-005   Add manager API    [待处理]
│ ✓ item-001   Domain types       [已完成]
│ ✓ item-002   Schema design      [已完成]
└─────────────────────────────────────┘
```

这是一个紧凑的卡片式渲染，不需要像 PlanningView 那样完整，但比纯文本截断更直观。

---

## 5. Tauri 后端命令补充

### 5.1 新增 `invoke` 命令

在 `desktop.ts` 中追加（仿现有模式）：

```typescript
// ===== Planning API =====

export interface PlanDto {
  id: string;
  title: string;
  goal: string;
  phase: string;        // "explore" | "plan" | "awaiting_approval" | "execute" | "verify" | "completed" | "failed"
  status: string;       // "pending" | "in_progress" | "completed" | "failed" | "canceled"
  verification_verdict?: string | null;  // "pass" | "fail" | "partial"
  todo_count: number;
  todo_completed: number;
  created_at: string;
  updated_at: string;
  is_active: boolean;
}

export interface PlanDetailDto extends PlanDto {
  strategy?: string | null;
  todos: TodoItemDto[];
  steps: PlanStepDto[];
}

export interface TodoItemDto {
  id: string;
  title: string;
  detail?: string | null;
  status: string;       // "pending" | "in_progress" | "completed" | "blocked" | "canceled"
  priority: string;     // "low" | "normal" | "high"
  evidence_ref?: string | null;
  block_reason?: string | null;
  ordinal: number;
  updated_at: string;
}

export interface PlanStepDto {
  id: string;
  title: string;
  status: string;
  ordinal: number;
}

// Tauri invoke wrappers
export const getPlans = () =>
  invoke<PlanDto[]>('get_plans');

export const getPlan = (planId: string) =>
  invoke<PlanDetailDto>('get_plan', { planId });

export const getActivePlan = () =>
  invoke<PlanDetailDto | null>('get_active_plan');
```

### 5.2 Rust 后端对应的 Tauri command

在 `src-tauri/src/main.rs` 或专门的 `commands/planning.rs` 中添加：

```rust
#[tauri::command]
async fn get_plans(state: tauri::State<'_, AppState>) -> Result<Vec<PlanDto>, String> {
    // 通过 api_tx → ManagerCommand::ListPlans → PlanningService
}

#[tauri::command]
async fn get_plan(plan_id: String, state: tauri::State<'_, AppState>) -> Result<PlanDetailDto, String> {
    // 通过 api_tx → ManagerCommand::GetPlan
}

#[tauri::command]
async fn get_active_plan(state: tauri::State<'_, AppState>) -> Result<Option<PlanDetailDto>, String> {
    // 查询 active_plan 表 → 返回当前活跃 Plan
}
```

---

## 6. 国际化字符串（zh.ts / en.ts 补充）

在 `src/locales/zh.ts` 中追加：

```typescript
planning: {
  title: '规划管理',
  desc: '查看和管理执行规划',
  navPlanning: '规划',
  activePlan: '当前规划',
  noActivePlan: '暂无活跃规划',
  planList: '规划列表',
  planDetail: '规划详情',
  planPhase: '阶段',
  planStatus: '状态',
  planProgress: '进度',
  planVerification: '验证结果',
  todoList: '任务清单',
  todoPending: '待处理',
  todoInProgress: '进行中',
  todoCompleted: '已完成',
  todoBlocked: '已阻塞',
  todoCanceled: '已取消',
  todoPriority: '优先级',
  todoPriorityHigh: '高',
  todoPriorityNormal: '中',
  todoPriorityLow: '低',
  todoEvidence: '查看证据',
  todoBlockReason: '阻塞原因',
  planPhaseExplore: '探索',
  planPhasePlan: '规划',
  planPhaseAwaitingApproval: '等待审批',
  planPhaseExecute: '执行',
  planPhaseVerify: '验证',
  planPhaseCompleted: '已完成',
  planPhaseFailed: '失败',
  todoUpdated: '任务清单已更新',
},
```

---

## 7. 颜色与图标约定

### 状态颜色（全 GUI 统一）

| 状态 | 背景色 | 文字色 | 图标 (lucide) |
|------|--------|--------|--------------|
| pending | `gray-100` | `gray-600` | `Circle` |
| in_progress | `pink-100` | `pink-700` | `Loader2` (animated) |
| completed | `green-100` | `green-700` | `CheckCircle2` |
| blocked | `amber-100` | `amber-700` | `AlertCircle` |
| canceled | `gray-100` | `gray-400` | `XCircle` |
| failed | `red-100` | `red-700` | `XCircle` |

### Planning 专属图标

| 元素 | lucide icon | 说明 |
|------|------------|------|
| Planning 导航 | `ClipboardList` | 侧边栏导航图标 |
| Active Plan 卡片头 | `Target` 或 `Compass` | Plan 概览区 |
| TodoList 面板 | `ListChecks` | Todo 分组标题 |
| Plan phase 指示 | `GitBranch` | 阶段流转箭头 |
| 新建 Plan | `Plus` | 创建按钮（第一期末必做，但预留） |

---

## 8. 实现优先级

### Phase A（在 PR #2 之后启动，约 2-3 天）

1. `desktop.ts` 新增 Planning DTOs + `invoke` 命令
2. Rust 端 Tauri commands（`get_plans`, `get_plan`, `get_active_plan`）
3. `TodoInlineCard.vue` — ChatView 中 Todo 工具结果的增强渲染
4. `zh.ts` / `en.ts` 国际化字符串

### Phase B（在 PR #4 之后启动，约 3-4 天）

1. `PlanningView.vue` 主视图
2. `PlanStatusCard.vue` — Active Plan 概览
3. `TodoListPanel.vue` + `TodoItemRow.vue` — TodoList 进度
4. `NormalMode.vue` 侧边栏新增 Planning 入口
5. 轮询 `get_active_plan` 实现近实时更新（仿 `CronTaskManagementView` 的 `LOG_POLL_MS` 模式）

### Phase C（延后）

1. Plan 创建表单弹窗（仿 `CronTaskManagementView` 的 create form）
2. Plan 历史列表 + 归档
3. Plan 审批状态可视化
4. Verification 面板

---

## 9. 与 CronTaskManagementView 的差异

| 维度 | Cron | Planning |
|------|------|---------|
| 列表项 | Job（独立实体） | Plan（1 active + N 历史） |
| 操作按钮 | Play/Pause/Edit/Delete | 第一期只读，后期可 Cancel/Re-plan |
| 实时更新 | 轮询 `list_cron_jobs` | 轮询 `get_active_plan` |
| 详情区 | Job detail inline expand | 右侧详情面板（常驻） |
| 数据源 | CronService | PlanningService → planning.db |

---

## 10. 第一期不做但预留扩展点

- **Plan 创建 UI**：Sidebar 中 Planning nav item 旁预留 `[+]` 按钮
- **实时 WebSocket 推送**：`/api/events` SSE 流预留 `PlanEvent` 类型
- **GUI 内编辑 Todo**：TodoItemRow 的点击事件预留 `@click="selectItem"` handler
- **Verification 面板**：Plan 详情底部预留 `VerificationCard` 区域
- **批量操作**：列表头预留 checkbox 列（display: none by default）
