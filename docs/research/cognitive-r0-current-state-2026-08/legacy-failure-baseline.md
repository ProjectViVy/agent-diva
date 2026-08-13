# R0 旧架构失败基线

- 状态：`Research Draft / Source-backed`
- 日期：2026-08-13
- 性质：把 2026-08-13 人工桌面症状固化为旧架构证据；**禁止沿旧模型打补丁**
- EPIC 出处：`cognitive-workspace-reset-epic-2026-08/epic-orchestration.md` §旧故障的处置

## 处置政策

三条症状是本 EPIC 的输入证据，不是独立 hotfix 队列。除非它们阻断数据导出、研究取证或危险工具审批，不再沿旧 Persona / Evolution / Memory governance 架构做临时修补。

已落地的恢复提交（`78e2bcf5`、`ab4705e4`、`d6f82ea3`）保留为历史。它们**不构成**必须维持的兼容义务，也不能阻止 clean break。

最终验收必须证明：

1. Persona 正文全链路为 Markdown，不可能把对象隐式渲染成字符串；
2. Evolution 不加载或投影 Persona / Memory / 旧 AutoDream proposal；
3. Memory CRUD、STM、Persona 初始化与直接保存不查询 Governance Ledger；
4. Chat Approval Center 的危险工具授权仍独立可用。

## 症状一：Persona 主内容显示 `[object Object]`

### 观察

人工测试：Persona 中央编辑区出现字面量 `[object Object]`。  
EPIC / TODOLIST / `docs/logs/2026-08-governance-persona-recovery/v0.0.2-persona-evolution-usability/` 将其记为桌面症状。

### 触发链（源码事实）

人格正文在服务端是 `LaputaSection.content: serde_json::Value`（`service.rs:945-948`）。  
种子为 JSON `null`；Owned 判定是「非 null」（`service.rs:1037-1042`）。  
Frozen Core 捕获再 `serde_json::to_string`（`frozen_core.rs:62-66`）。  
GUI 必须把对象变成字符串才能放进 textarea。

当前防护：

| 位置 | 行为 | 标签 |
| --- | --- | --- |
| `PersonaMemoryView.vue:66-67` | `JSON.stringify(value ?? {}, null, 2)` | 源码事实 |
| `SectionEditor.vue:77-88` | `JSON.parse` 校验 + stringify 格式化 | 源码事实 |
| `errorMessage.ts` | 抽 `message` / `error` 字段 | 源码事实 |
| `PersonaMemoryView.test.ts:96` | 错误条不得含 `[object Object]` | 源码事实 |

`v0.0.2-persona-evolution-usability`（`d6f82ea3`）把结构化 Tauri 错误改成可读字符串，并让 JSON `null` 显示为可编辑 `{}`。这是旧链修补，不是 Markdown 权威。

### 仍可复现的路径（推断 + 源码）

| 路径 | 机制 | 标签 |
| --- | --- | --- |
| Vue `{{ section.content }}` | 直接插值对象 → `[object Object]` | 推断；当前 Persona/Evolution 模板未这样写 |
| `MemoryView.vue:58` | `String(err.message)`，message 为对象时 | 源码事实 |
| `App.vue` `String(error)` / `String(payload)` | 对象被强制转字符串 | 源码事实 |
| `ApprovalCenterCard.vue:64` | `String(presentation.title)` | 源码事实 |
| `formatContent(null)` → `"{}"` | 保存 `{}` 后 status 从 Tbd 变 Owned | 源码事实 + 推断 |

根因不是某一处漏写 stringify，而是**正文被建模为 JSON 对象**。只要 GUI 或 Prompt 边界漏一次，症状就会回来。

### 数据影响

不损坏 sqlite 或 section 文件本身。会误导用户把 `{}` 存成「已拥有」的空对象，使 first-run 空检测失效（Frozen Core `is_empty` 把非空 JSON 当有内容）。

### 验收证伪

Persona 全链路只传递 Markdown 字符串；类型上不再出现 `serde_json::Value` 正文、JSON.parse 门或 `JSON.stringify` 编辑器。

## 症状二：Evolution 数据加载失败且页面挤作一团

### 观察

人工测试：Evolution 页提示加载失败，内容挤成一团。  
文案：`evolution.errorTitle` =「进化数据加载失败」（`locales/zh.ts:326`）。

### 触发链（源码事实）

`EvolutionView.refresh()` 主路径是 `listLaputaProposals()`。失败写入 `loadError`。  
辅路径独立失败：`pollLaputaEvents` 三路、health、feedback、runs → `auxiliaryError`。  
`d6f82ea3` 后：提案是主数据；辅失败保留上次成功数据。

布局：`.evolution-inbox-shell` 双列 `minmax(320px) + minmax(520px) = 840px`，仅 `@media (max-width: 900px)` 单列。  
推断：900–1180px 窄窗 + 错误条 + workspace 状态条会在人工分辨率下「挤作一团」。仓库内无逐步截图日志。

加载失败的常见上游：

- Laputa 服务 / `.laputa/proposals` IO
- 治理投影对缺失 approval 的失败（见症状三）
- Tauri 未连上 gateway（非 Tauri 时 Persona 直接空数据）

### 数据影响

读失败本身不写坏提案。用户无法审查/应用时，AutoDream 新提案堆积在 `PendingReview`。

### 验收证伪

Evolution 不再加载 Persona/Memory/旧 AutoDream proposal；页面只服务 Skill 领域对象。加载失败不得再依赖治理账本是否存在 request。

## 症状三：`governance ledger failed: approval request not found`

### 观察

人工测试与归档 TODOLIST：「Memory governance 双账本导致 approval request not found」。  
组合错误串仍由当前类型产生。

### 触发链（源码事实）

```text
ApprovalLedgerError::NotFound
  → "approval request not found"          ledger.rs:193

MemoryGovernanceError::Ledger
  → "governance ledger failed: {0}"       governed_apply.rs:35

组合
  → "governance ledger failed: approval request not found"
```

双文件（仍在）：

| 文件 | 角色 | 打开者 |
| --- | --- | --- |
| `.laputa/governance.db` | 生产统一 ApprovalCoordinator 账本 | Manager `bootstrap.rs:99` |
| `.laputa/governance.sqlite3` | proposal↔request 映射；`open_lazy` 可当独立账本 | `LaputaPaths::governance_database` / `MemoryGovernanceCoordinator` |

历史根因（提交事实，`docs/logs/2026-08-governance-persona-recovery/v0.0.1-unified-governance/summary.md`）：Agent 把审批事件写进 `governance.sqlite3`，Manager 读 `governance.db`，decide/apply 按 request_id 查找失败。

`78e2bcf5` 把生产 Memory/Laputa 写入改为共享 `ApprovalCoordinator`；启动调和把可审核 pending 迁入共享账本，不导入旧 allow receipt。  
`ab4705e4` 把 AutoDream 注册提前到提案创建边界。

现生产决策应落 `governance.db`。`governance.sqlite3` 仍是映射库，`open_lazy` 仍可把它当备用账本。错误串在「映射有 proposal、共享账本无 request」或「request 过期/已消费」时仍会出现。

GUI 测试：`errorMessage.test.ts` 断言能抽出这两段字符串，而不是再渲染成 `[object Object]`。

### 数据影响

- 提案文件仍在 `.laputa/proposals/`
- 用户无法 decide/apply；提案留在 PendingReview 或 `needs_attention`
- 不自动回写 section JSON 或 BML
- 危险工具账本与 Memory 域共用 `governance.db`：Memory 查找失败不应破坏 command/plan 授权，但共享 coordinator 使故障面相邻

### 验收证伪

Memory CRUD、STM、Persona 初始化与直接保存的代码路径零查询 Governance Ledger。Evolution 不再 submit `MemoryApply`。Chat Approval Center 仍能独立完成危险工具授权。

## 已落地修补与本基线的关系

| 提交 | 做了什么 | 对本 EPIC |
| --- | --- | --- |
| `78e2bcf5` | 统一 Memory 审批到 Manager coordinator | 历史修补；不阻止删除 Memory 治理 |
| `ab4705e4` | AutoDream 提案边界注册审批 | 同上 |
| `d6f82ea3` | Persona/Evolution 错误不再 `[object Object]`；辅失败保活 | 旧 GUI 修补；JSON 正文仍在 |

这些提交证明旧链**可被缓解，但不能被产品化为目标**。

## 研究级复现观察点（不修）

本包未做新的真机桌面复测。若需取证，只读观察：

1. Persona：打开 `persona-memory`，看中央是否 JSON / `{}` / 对象字符串；不要保存。
2. Evolution：缩到约 1000px 宽，断开 gateway 或清空 proposals 权限，看错误条与双列是否重叠。
3. 审批：对一条无共享账本映射的旧提案点 apply，抓 `governance ledger failed: approval request not found`。

若复现阻断导出或危险工具审批，记入 TODOLIST 为运维取证，仍不沿旧模型加兼容层。

## 与 R4 deletion-proof 的接口

R4 的零残留证明至少扫描：

- 字面量 `[object Object]` 出现在 Persona 主内容渲染路径
- Evolution 页面依赖 `listLaputaProposals` / MemoryApply
- 生产路径字符串 `governance ledger failed` 与 Memory/Persona 调用点
- `governance.sqlite3` 作为审批权威（映射库删除后，危险工具只留 `governance.db`）
