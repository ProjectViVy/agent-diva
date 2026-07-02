# Plan + TodoList Pre-Implementation Research

> 生成日期: 2026-06-01
> 调研负责人: Hermes Agent（通过 3 个 Claude Code 子任务并行执行）
> 调研目标: 为 agent-diva 滚动式规划能力（Plan + TodoList）第一期正式编码前做窄口径技术预检

---

## Executive Summary

基于三份设计文档和仓库当前代码状态，第一期 Plan + TodoList 的实现路径已经非常清晰：

| 维度 | 结论 | 信心 |
|------|------|------|
| **存储方案** | SQLite（sqlx）→ `.agent-diva/planning.db`，Markdown 仅为投影 | 高 |
| **类型放置** | `agent-diva-core::planning` — 共享 domain 类型 + store contract | 高 |
| **Runtime** | `agent-diva-agent::planning` — orchestrator + context injection | 高 |
| **工具注册** | `agent-diva-tools/src/planning.rs`，通过 `BuildInToolsConfig.planning` 控制 | 高 |
| **Manager API** | `/api/plans` 路由组，走 `ManagerCommand` → `PlanningService` 模式 | 高 |
| **CLI** | 第一期不做 CLI 命令，只做 Manager API + Agent Tool | 高 |
| **迁移策略** | MVP 用 `CREATE TABLE IF NOT EXISTS`，schema 第二次变更时引入正式 migration | 高 |
| **外部参考** | Claude Code V1 式全量替换 Todo 模式最适配当前阶段 | 高 |

**关键风险已识别并可控**：过度设计（用最小 schema + 单 active plan 约束）、AgentLoop 复杂化（hook 点明确且侵入小）、模型绕过状态机（五层防御设计）。

---

## Code Map

### 仓库当前结构（与 planning 相关）

```
agent-diva/
├── Cargo.toml                         # workspace: sqlx 0.7 (sqlite+migrate+chrono)
├── agent-diva-core/src/
│   └── lib.rs                         # ← 新增 pub mod planning
│       ├── planning/
│       │   ├── mod.rs                 #     re-exports
│       │   ├── ids.rs                 #     PlanId, TodoId
│       │   ├── model.rs               #     Plan, PlanStep, TodoList, TodoItem + status enums
│       │   ├── events.rs              #     PlanEvent, TodoEvent
│       │   ├── store.rs               #     PlanningStore trait + SqlitePlanningStore
│       │   └── render.rs              #     todo.md / plan.md 投影生成器
├── agent-diva-agent/src/
│   ├── agent_loop.rs                  #     AgentLoop 结构体 (928行)
│   ├── agent_loop/
│   │   ├── loop_turn.rs               # ← HOOK-1~5 挂载点 (825行)
│   │   └── loop_tools.rs              #     工具结果处理
│   ├── context.rs                     # ← Planning context injection 接入点 (705行)
│   ├── tool_assembly.rs               # ← 注册 planning tools 的入口 (255行)
│   ├── tool_config/
│   │   └── builtin.rs                 # ← 新增 planning: bool 开关
│   └── planning/                      #     新增模块
│       ├── mod.rs
│       ├── orchestrator.rs            #     PlanOrchestrator
│       ├── todo_planner.rs            #     模型辅助生成 TodoItems
│       ├── context.rs                 #     Plan/Todo 上下文渲染
│       └── verifier.rs                #     验证逻辑
├── agent-diva-tools/src/
│   └── planning.rs                    # ← 新增: TodoShow, TodoWrite 工具
├── agent-diva-manager/src/
│   ├── server.rs                      # ← runtime_routes() 中添加 /api/plans 路由
│   ├── handlers.rs                    # ← 新增 handlers: list/create/get/update/delete_plan
│   ├── state.rs                       # ← ManagerCommand 新增 5 个 Planning 变体
│   └── planning_service.rs            # ← 新增: PlanningService（仿 skill_service/mcp_service）
└── agent-diva-files/src/
    └── index.rs                       #   参考: SqliteIndex 初始化模式（655行）
```

### 接入点汇总

| 接入点 | 文件 | 行号/区域 | 改动类型 |
|--------|------|-----------|----------|
| Domain types | `agent-diva-core/src/lib.rs` | L6-L18 区域 | 新增 `pub mod planning` |
| SQLite store | `agent-diva-core/src/planning/store.rs` | 新文件 | 仿照 `agent-diva-files/src/index.rs` |
| Context injection | `agent-diva-agent/src/context.rs` | L144 之后 | 在 Memory block 后、tool guidance 前插入 |
| Turn hook | `agent-diva-agent/src/agent_loop/loop_turn.rs` | L119 之后 | 注入 Plan/Todo context system message |
| Tool registration | `agent-diva-agent/src/tool_assembly.rs` | L120 区域 | `if builtin_config.planning { ... }` |
| Tool config toggle | `agent-diva-agent/src/tool_config/builtin.rs` | struct 定义 | 新增 `planning: bool` |
| Tool impl | `agent-diva-tools/src/planning.rs` | 新文件 | 实现 `Tool` trait |
| Manager routes | `agent-diva-manager/src/server.rs` | `runtime_routes()` L85-142 | 追加 `/api/plans*` 路由 |
| Manager commands | `agent-diva-manager/src/state.rs` | `ManagerCommand` 枚举末尾 | 新增 5 个变体 |
| Manager handler | `agent-diva-manager/src/handlers.rs` | 末尾 | 新增 handler 函数 |
| Manager service | `agent-diva-manager/src/planning_service.rs` | 新文件 | 仿 `skill_service.rs` / `mcp_service.rs` |

---

## Store And Persistence Recommendation

### 结论：SQLite（Option B），与 roadmap 一致

- **方案**: SQLite 作为 canonical store，Markdown 仅为人脸投影
- **位置**: `.agent-diva/planning.db`（全局，与 config.json / files/ 同级）
- **投影**: `.agent-diva/plans/<plan-id>/plan.md`, `todo.md`（per-plan 目录）

### 仓库现有 sqlx 模式分析

仓库中 `sqlx 0.7` 已在 workspace `Cargo.toml` 声明（features: `runtime-tokio-rustls`, `sqlite`, `migrate`, `chrono`），但**实际使用仅限** `agent-diva-files/src/index.rs` 的 `SqliteIndex`（655 行）。

**SqliteIndex 初始化模式**（可直接复用）：

```rust
// 1. 确保父目录存在
tokio::fs::create_dir_all(parent).await?;

// 2. 配置连接选项
let options = SqliteConnectOptions::new()
    .filename(&db_path)
    .create_if_missing(true);

// 3. 创建连接池
let pool = SqlitePoolOptions::new()
    .max_connections(5)
    .connect_with(options)
    .await?;

// 4. 初始化 schema
sqlx::query("CREATE TABLE IF NOT EXISTS ...")
    .execute(&pool).await?;
```

**关键发现**：仓库**没有正式使用** `sqlx::migrate!()` 宏。`SqliteIndex` 通过 `PRAGMA table_info()` 查询列名 + `ALTER TABLE ADD COLUMN` 实现轻量迁移。

### planning.db 差异化需求

| 维度 | SqliteIndex | planning.db |
|------|------------|-------------|
| 表数量 | 1（files） | 5（plans, plan_steps, todo_items, planning_events, active_plan） |
| 外键 | 无 | 有（plan_steps→plans, todo_items→plan_steps, etc） |
| 连接数 | 5 | 2（单写入者） |
| foreign_keys pragma | 不需要 | **必须** `.foreign_keys(true)` |
| 迁移策略 | PRAGMA + ALTER TABLE | 初期同模式，schema 第二次变更时引入正式 migration |

### 初始化伪代码

```rust
// agent-diva-core/src/planning/store.rs
pub struct SqlitePlanningStore {
    pool: SqlitePool,
}

impl SqlitePlanningStore {
    pub async fn open(db_path: PathBuf) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .foreign_keys(true);  // ← 关键差异
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .connect_with(options)
            .await?;
        let store = Self { pool };
        store.init_schema().await?;  // CREATE TABLE IF NOT EXISTS x5
        Ok(store)
    }
}

pub fn default_planning_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".agent-diva")
        .join("planning.db")
}
```

### 迁移策略决策

- **第一阶段（MVP）**：沿用 `CREATE TABLE IF NOT EXISTS` + 必要时 `PRAGMA table_info` + `ALTER TABLE`（与仓库现行模式一致）
- **第二阶段**：schema 第二次破坏性变更时引入 `sqlx::migrate!()` + `migrations/` 目录（`migrate` feature 已在 Cargo.toml 声明，随时可用）
- **判断信号**：当 planning schema 需要**第二次** `ALTER TABLE ADD COLUMN` 时，即引入正式迁移

---

## Runtime Integration Points

### ContextBuilder Injection

`agent-diva-agent/src/context.rs::build_system_prompt()` 当前拼装顺序：

1. Identity header（from IDENTITY.md）
2. 时间 + workspace
3. Soul sections（AGENTS.md → SOUL.md → IDENTITY.md → USER.md → BOOTSTRAP.md）
4. Always-loaded Skills
5. Available Skills list
6. Memory provider block（`SystemPromptBlock`）
7. **通用 tool guidance**（IMPORTANT / be helpful 等）

**推荐注入点**：在第 6 步（Memory block）之后、第 7 步（tool guidance）之前（约 L144-L146 之间）。

理由：此时 Memory/Skills/Identity 已加载完毕，Planning 上下文作为执行指引紧贴在 tool usage guidance 之前，语义上自然。

```rust
// context.rs 中伪代码
if let Some(plan_context) = self.active_plan_context.as_ref() {
    prompt.push_str("\n\n## Active Plan\n");
    prompt.push_str(plan_context);
    prompt.push_str("\n## Active TodoList\n");
    prompt.push_str(todo_context);
}
prompt.push_str(r#"IMPORTANT: When responding..."#);
```

### AgentLoop Turn Hooks

`process_inbound_message_inner()` (loop_turn.rs, 825 行) 中的 5 个 hook 点：

| Hook | 位置 | 用途 | 优先级 |
|------|------|------|--------|
| **HOOK-1** | 消息构建后、迭代循环前（L87-L119 之后） | 注入 Plan+TodoList system message | ★ 最关键 |
| **HOOK-2** | LLM 响应后、工具执行前（L264-L275） | 检测模型是否调用了 planning 工具 | ★★ |
| **HOOK-3** | 工具执行后、结果添加前（L342-L374） | 检测 `changed_plan_state()` | ★★ |
| **HOOK-4** | 最终响应后、session 保存前（L427-L439） | 同步 planning store | ★ |
| **HOOK-5** | consolidation 触发前（L441-L458） | 确保 Plan/Todo 不被压缩丢弃 | ★★ |

**HOOK-1 的具体实现**（最小侵入）：

```rust
// loop_turn.rs L119 之后，while 循环之前
if let Some(plan_ctx) = self.get_active_plan_context().await {
    messages.insert(1, Message::system(plan_ctx));
}
while iteration < self.max_iterations { ... }
```

### Plan/Todo 防上下文压缩策略

agent-diva 当前的 consolidation（`consolidation.rs`）在 `session.messages.len() >= memory_window` 时触发，保留最近 50 条消息，将旧消息交给 LLM 压缩。

**Plan/Todo 防丢失**：

1. **压缩前快照**：consolidation 触发前，将 Active Plan + TodoList 写入 `planning_events` 表
2. **压缩后恢复**：ContextBuilder 从 `planning.db` / `todo.json` 重建上下文，不依赖 session 历史中的 plan 引用
3. **最小保留**：Plan goal + active TodoItem + pending items 的紧凑摘要必须在 compaction 后仍可注入

### Plan State Machine Bypass Prevention（五层防御）

| 层级 | 机制 | 说明 |
|------|------|------|
| 1. 文件系统 | SecurityPolicy 拒绝直接写 `.agent-diva/plans/` | 模型 write_file 工具被拦截 |
| 2. 工具层 | Todo 工具不直接写 store，调用 PlanOrchestrator API | 统一入口 |
| 3. PlanOrchestrator | Phase gate 校验 | Execute 阶段不允许修改 Plan 结构 |
| 4. ToolAssembly | Planning tool 不暴露裸文件路径 | 通过 store API 操作 |
| 5. 事件审计 | 所有状态变更写入 events.jsonl / planning_events | 事后审计追踪 |

---

## Tool Surface Recommendation

### MVP 最小工具集（强烈推荐极简方案）

基于 Claude Code V1、GenericAgent 和 roadmap 文档的交叉验证，第一期只需要 **2 个工具**：

#### 1. `todo_write`

Claude Code V1 风格的全量替换模式：

```
参数: {
  todos: [{id, title, status, detail?}]
}
status: "pending" | "in_progress" | "completed" | "blocked"
block_reason: 包含在 detail 字段中
```

行为:
- **全量替换**：每次调用传入完整 TodoItem 列表，完全覆盖之前状态
- **全部 completed 自动清空**：所有项都 completed 时列表归空
- **invariant 强制**：最多 1 个 in_progress；blocked 必须带 reason；completed 需要 evidence
- **不暴露内部 ID**：id 由工具内部生成并返回给模型

返回: `"Todo list updated (N items)"` + 格式化列表文本

#### 2. `todo_show`

参数: 无

返回: 当前 TodoList 格式化文本（供模型和用户阅读）

```
## Active TodoList (Plan: <plan-id>)
Current: [todo-003] Implement TodoListStore serialization tests
Pending:
- [todo-004] Add context injection renderer
- [todo-005] Add manager status endpoint
Blocked:
- [todo-002] Needs user approval
Completed:
- [todo-001] Add domain types
```

### 工具注册路径（最小改动）

```
1. agent-diva-agent/src/tool_config/builtin.rs
   添加: pub planning: bool,  // +3行

2. agent-diva-agent/src/tool_assembly.rs
   在 build_internal() 中添加:
   if self.builtin_config.planning {
       registry.register(TodoShowTool::new(orch.clone()));
       registry.register(TodoWriteTool::new(orch.clone()));
   }

3. agent-diva-tools/src/planning.rs (新文件)
   实现 TodoShowTool, TodoWriteTool (各 ~50行)
   内部通过 Arc<PlanOrchestrator> 调用服务边界
```

### 第一期暂不开放

- `plan_update` — 模型不应直接修改 Plan；PlanOrchestrator 拥有 Plan 的所有变更
- `todo_start/todo_complete/todo_block` 单独工具 — `todo_write` 的全量替换模式已经足够，避免 chat 中出现过多 tool call
- 用户审批 — 通过现有 ask_user / channel 机制实现，不作为 planning 专属工具

### NAG 机制（可选但推荐）

当模型连续 3 轮不调用任何 planning 工具且有 pending 项时，在 context 中注入 prompt：

```
"You have pending TodoList items. Pick up the next one now."
```

---

## Context Injection Recommendation

### 注入格式（紧凑原则）

在 system prompt 末尾、tool usage guidance 之前插入：

```
## Active Plan
Goal: <plan.goal, 截断至 200 字符>
Phase: <plan.phase>
Strategy: <plan.strategy, 截断至 300 字符>

## Active TodoList (Revision: <revision>)
Current: [<id>] <title>
Pending:
- [<id>] <title>
Blocked:
- [<id>] <title> (Reason: <block_reason>)
```

### 规则

- 不超过 800 字符（防止污染主上下文）
- 只包含 active、pending、blocked 项，不包含已完成项
- 已完成项仅在上下文中保留 1 轮作为进度确认，之后自动清除
- 不注入完整 evidence 文本；仅链接 evidence artifact 名称
- 每次 `todo_write` 调用后自动刷新注入

---

## Manager/CLI Recommendation

### Manager API（第一期）

在 `server.rs` 的 `runtime_routes()` 中添加（与 cron 路由并列）：

```
GET    /api/plans              → list_plans_handler
POST   /api/plans              → create_plan_handler
GET    /api/plans/:plan_id     → get_plan_handler
PUT    /api/plans/:plan_id     → update_plan_handler
DELETE /api/plans/:plan_id     → delete_plan_handler
```

### ManagerCommand 扩展

在 `state.rs` 的 `ManagerCommand` 枚举末尾追加：

```rust
// Planning / task orchestration
ListPlans(oneshot::Sender<Result<Vec<PlanDto>, String>>),
GetPlan(String, oneshot::Sender<Result<Option<PlanDto>, String>>),
CreatePlan(CreatePlanRequest, oneshot::Sender<Result<PlanDto, String>>),
UpdatePlan(String, UpdatePlanRequest, oneshot::Sender<Result<PlanDto, String>>),
DeletePlan(String, oneshot::Sender<Result<(), String>>),
```

### AppState

- **第一期不需要扩展**。Planning 状态查询通过 `api_tx` → `ManagerCommand` → Manager 事件循环完成
- Manager 内部通过 `PlanningService`（仿 `SkillService` / `McpService` 模式懒创建）处理

### CLI

- **第一期不做 CLI 命令**。Planning 核心场景是 agent 内部工具调用 + Manager HTTP API 供 GUI/外部查询
- 第二期可参考 `cron` 模式：本地模式直接操作 `PlanningService`，远程模式通过 `ApiClient` 调 Manager

---

## Test Plan

### 第一期测试清单

| 测试类型 | 内容 | 关键验证点 |
|----------|------|-----------|
| **单元测试** | `SqlitePlanningStore` CRUD | 5 表 CRUD + 外键约束 + invariant enforcement |
| **单元测试** | Domain 类型序列化 | Plan/TodoItem JSON 往返 + unknown variant 错误处理 |
| **单元测试** | Markdown 投影渲染 | `todo.md` 生成内容与 typed state 一致、确定性输出 |
| **单元测试** | PlanOrchestrator gate 规则 | 禁止无审批执行、禁止无 evidence 完成、禁止多 in_progress |
| **集成测试** | AgentLoop + Planning context | 完整 turn 中 context 包含 plan/todo 块 |
| **集成测试** | 3-item TodoList 串行执行 | agent 调用 todo_write 完成 Pending→InProgress→Completed 流程 |
| **集成测试** | Manager API endpoint | HTTP 请求→ManagerCommand→PlanningService→响应 |
| **恢复测试** | Session resume | 重启后 ContextBuilder 从 planning.db 恢复 active plan context |
| **恢复测试** | Consolidation 后恢复 | 上下文压缩后 plan/todo 仍可注入 |

### 测试基础设施

- 使用 `tempfile` 创建临时 `planning.db`
- 每个测试 `#[tokio::test]` + 独立 db 实例
- 参考 `agent-diva-files` 的测试模式

---

## First PR Slice

### PR #1: Types + Store（最小可验证基础）

**目标**：创建 durability base，无 user-facing 变化

```
PR #1 内容:

1. agent-diva-core/src/planning/
   ├── mod.rs          # pub mod ids, model, events, store, render
   ├── ids.rs          # PlanId(String), TodoId(String)
   ├── model.rs        # Plan, PlanStep, TodoList, TodoItem + status enums + serde
   ├── events.rs       # PlanEvent, TodoEvent enum + serde
   ├── store.rs        # PlanningStore trait + SqlitePlanningStore::open()
   └── render.rs       # render_todo_md(TodoList) -> String

2. agent-diva-core/Cargo.toml
   添加 sqlx 依赖（workspace 级别已声明，直接 use）

3. 测试
   - model JSON 序列化往返测试
   - SqlitePlanningStore CRUD 测试（tempfile sqlite）
   - render_todo_md 确定性输出测试
   - invariant: 最多一个 in_progress TodoItem
   - invariant: blocked 必须有 reason

4. 不包含
   - AgentLoop 集成
   - Tool 实现
   - Manager API
   - Context injection
```

### 验收标准

- `cargo test -p agent-diva-core` 绿色
- SQLite 5 表创建 + CRUD 通过
- Markdown 投影与 typed state 一致
- 序列化往返无信息丢失

### 后续 PR 建议顺序

```
PR #1 → Types + Store           (当前，约300行)
PR #2 → Context + todo_show     (注入 + 只读可见)
PR #3 → todo_write + NAG        (模型可操作 TodoList)
PR #4 → PlanOrchestrator gates  (审批 + 验证 + 阶段转换)
PR #5 → Manager API             (/api/plans 端点)
PR #6 → Verification + Closure  (终止态转换)
```

---

## GUI Design

> 完整设计见: `docs/dev/agent-plan/planning-gui-design-supplement.md`

### 技术栈

- **Vue 3 + TypeScript + Vite + Tailwind CSS 3 + Tauri v2**
- **图标**: lucide-vue-next v0.575
- **渲染**: markdown-it + highlight.js (GitHub Dark)
- **i18n**: vue-i18n (zh/en)
- **主题**: `love` 模式（粉色系 yandere palette + 樱花/爱心粒子动画）

### 新增组件

```
src/components/planning/
├── PlanningView.vue          # 主视图（双栏：左 Plan 列表 + 右详情）
│   ├── PlanStatusCard.vue    # Active Plan 概览卡片（phase/status/进度/验证）
│   ├── TodoListPanel.vue     # TodoList 进度面板
│   │   └── TodoItemRow.vue   # 单个 TodoItem 行（状态图标 + 标题 + 证据 + 优先级）
│   └── PlanHistoryList.vue   # 历史 Plan 列表（可选第一期）
└── TodoInlineCard.vue        # ChatView 中 Todo 工具结果的结构化卡片渲染
```

### 导航入口

`NormalMode.vue` 侧边栏新增 `planning` section，与 Cron 并列：
```
Chat → Settings → Console → Neuro → Cron → [Planning] ← 新增
```

### Tauri 后端命令

在 `desktop.ts` 中新增 3 个 `invoke` 命令：
- `get_plans()` → `PlanDto[]`
- `get_plan(planId)` → `PlanDetailDto`
- `get_active_plan()` → `PlanDetailDto | null`

Rust 端通过 `ManagerCommand::ListPlans/GetPlan` 转发到 `PlanningService`。

### 状态颜色（全 GUI 统一）

| 状态 | 样式 |
|------|------|
| pending | `gray-100` bg, `Circle` icon |
| in_progress | `pink-100` bg, `Loader2` animated, 行背景高亮 |
| completed | `green-100` bg, `CheckCircle2` icon |
| blocked | `amber-100` bg, `AlertCircle` icon + tooltip 显示原因 |
| canceled | `gray-400` text, 删除线 |

### 实现优先级

- **Phase A**（PR #2 后）：`TodoInlineCard` + Tauri commands + i18n
- **Phase B**（PR #4 后）：`PlanningView` + `PlanStatusCard` + `TodoListPanel` + 侧边栏入口
- **Phase C**（延后）：Plan 创建表单、历史列表、审批 UI、Verification 面板

---

## Risks And Deferrals

### 已识别风险与对策

| 风险 | 当前控制措施 | 触发升级信号 |
|------|-------------|-------------|
| TodoList 变成第二套规划系统 | 必须关联 PlanId + render under plan dir | Todo 中出现策略性描述 |
| 模型绕过 PlanOrchestrator | 五层防御 + 工具不直接操作 store | 审计日志发现异常写入 |
| AgentLoop 复杂化 | 5 个明确 hook 点 + 最小 context 注入 | AgentLoop.rs 行数再增 20%+ |
| SQLite schema 过度设计 | V1 用 5 表，不做 DAG/worker/claim 字段 | 需要 DAG 时再扩展 |
| Context injection 污染 | 硬上限 800 字符 + active/pending/blocked 过滤 | context 利用率超 70% |
| 过早变成 Kanban | PlanOrchestrator 是唯一入口 + 无 dispatcher | 出现多 worker 需求时 |

### 明确延后

| 项目 | 延后原因 | 触发条件 |
|------|---------|---------|
| Kanban 多 worker | 本期范围外 | autonomous-evolution loop 成熟后 |
| DAG / 依赖图 | 串行执行已足够 | 出现可并行工作量证明 |
| GUI 完整视图 | 第一期只做 ChatView inline + Tauri commands；PlanningView 在 Phase B | PR #4 完成后 |
| CLI `plan` 命令 | Manager API + agent tool 覆盖核心场景 | 用户反馈需要终端操作 |
| 正式 sqlx migration | CREATE TABLE IF NOT EXISTS 足够 | schema 第二次破坏性变更 |
| `plan_update` 工具 | PlanOrchestrator 拥有 Plan 变更 | 需要模型动态调整 Plan 时 |
| 自动复杂度判断 | 用户显式触发即可 | 出现可量化复杂度信号 |

---

## References

- `docs/dev/agent-plan/plan-mode-architecture.md` — Plan Mode 生命周期设计
- `docs/dev/agent-plan/todolist-runtime-architecture.md` — TodoList 运行时架构
- `docs/dev/agent-plan/plan-todo-implementation-roadmap.md` — 技术实施路线图
- `agent-diva-files/src/index.rs` — sqlx SqliteIndex 参考实现（655行）
- `agent-diva-agent/src/context.rs` — ContextBuilder 实现（705行）
- `agent-diva-agent/src/agent_loop/loop_turn.rs` — Per-turn 处理（825行）
- `agent-diva-agent/src/tool_assembly.rs` — 工具注册（255行）
- `agent-diva-agent/src/tool_config/builtin.rs` — 工具开关配置
- `agent-diva-manager/src/server.rs` — HTTP 路由定义
- `agent-diva-manager/src/state.rs` — ManagerCommand 枚举
- `docs/dev/agent-plan/planning-gui-design-supplement.md` — GUI 设计补充（组件树、状态颜色、Tauri commands）
- `agent-diva-gui/package.json` — Vue 3 + Tauri v2 依赖确认
- `agent-diva-gui/src/components/NormalMode.vue` — 侧边栏导航结构（629行）
- `agent-diva-gui/src/components/ChatView.vue` — 聊天视图 tool 消息渲染（598行）
- `agent-diva-gui/src/api/desktop.ts` — Tauri invoke API 封装（200行）
- `.workspace/agent-diva/` — Claude Code V1 TodoWrite 参考（全量替换模式）
- `.workspace/GenericAgent/` — plan_sop.md（Markdown checkbox 计划系统）
