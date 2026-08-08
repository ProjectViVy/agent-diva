# Garden 记忆工作台批次 · Summary

## 变更内容

按冻结三层架构（BML / Laputa / Garden）落地 Garden facade 首个实质切片：**人格与记忆版块按真实架构图管理**（用户 2026-08-09 决策：侧栏平级两入口 + 记忆仓库只读 + 删除走 proposal）。

### 后端（agent-diva-laputa + agent-diva-manager）

- `LaputaService` 新增只读记忆方法（`agent-diva-laputa/src/service.rs`）：
  - `MemoryListFilter { query, kind, limit }` 查询过滤类型
  - `list_memories`：经 `.laputa/memory.sqlite3` 权威库只读列出/搜索（FTS5 `search_visible`），排除 tombstone 与 superseded，按 `effective_at` 倒序；库缺失时返回空列表且**不创建数据库**
  - `get_memory`：按 id 读取单条记录
- `LaputaError::MemoryStore(#[from] TypedMemoryStoreError)` 错误变体（`code() = "memory_store_error"`）
- `StoredMemoryRecord` 增加 `Serialize`（API 输出面）
- Manager 新增 `handlers/bml.rs` 三个端点（挂入 `build_router`）：
  - `GET /api/bml/memories?query=&kind=&limit=` → 列表/搜索
  - `GET /api/bml/memories/:id` → 详情（404 当不存在）
  - `POST /api/bml/memories/:id/remove { reason }` → `TypedLaputaMemoryProvider::memory_remove` 生成 governed proposal → 返回 `proposal_id`（**不直接删除记录**）

### GUI（agent-diva-gui）

- 侧栏拆「人格」（Laputa）/「记忆」（BML）平级双入口；PersonaMemoryView 治理树由虚构架构（garden→laputa/mempalace/rag）对齐为真实三层架构（Garden → Laputa / BML），BML 节点点击跳转记忆版块
- 新增 `components/memory/MemoryView.vue` 记忆仓库：搜索（300ms 防抖）、类型过滤、列表、详情（内容/元数据/provenance/证据/取代记录）、删除入口（reason 必填 → proposal → toast + 跳转 Evolution 审批中心）
- `appDialog` 扩展 `prompt` 类型（带文本输入），`AppDialogLayer` 渲染 reason 输入框
- `commands.rs` 新增 `bml_list_memories` / `bml_get_memory` / `bml_remove_memory`（manager HTTP 转发）；`desktop.ts` 新增 `BmlMemory` 类型族与 API 封装
- i18n：zh/en 新增 `memory.*` 命名空间；`nav` 拆 `persona` / `memory`；`laputa.nodes` 移除虚构节点

## 影响范围

- 只读路径 + 受治理删除路径；不改变任何既有 BML 写面；`bml_boundary_guard` 保持零违规
- 兼容：`pub mod typed_store` 与全部顶层 re-export 不变，4 个外部 crate 零改动

## 关键提交

- 提交 1：`feat: add BML memory read/remove endpoints (laputa service + manager)`
- 提交 2：`feat: add Garden memory repository view (persona + memory split)`
- 提交 3：`docs: record Garden memory UI iteration`
