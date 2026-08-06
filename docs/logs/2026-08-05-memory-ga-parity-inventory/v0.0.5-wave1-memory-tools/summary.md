# Summary — GA-MEM-PARITY Wave 1（Agent 记忆工具 CRUD）

- 版本：`v0.0.5-wave1-memory-tools`
- 日期：2026-08-06
- 类型：功能对齐核心（4 切片，每切片独立提交）
- 前置：`v0.0.4-wave0-honesty`（诚实与契约）

## 做了什么

Wave 1 实现六个独立 memory 工具（add / list / search / update / remove /
distill），使「记住 X / 忘掉 X / 你还记得吗」有确定落点；混合分级写入
（低风险即时 apply，高风险 proposal + 审批）；Legacy 模式 proposal-first。
决策基线：inventory §10.2（W1-1..W1-4）+ §10.3（G1、G3 勘误）。

### S1 core：CRUD trait 扩展面 — `1e40bf16`
- 新建 `agent-diva-core/src/memory/crud.rs`：6 请求类型（MemoryAddRequest /
  MemoryListRequest / MemorySearchRequest / MemoryUpdateRequest /
  MemoryRemoveRequest / MemoryDistillRequest）、`MemoryEntry`、
  `MemoryCrudOutcome { Applied{entry} / Listed{entries} /
  ProposalCreated{proposal_id} / Failed{reason} }`（serde tag="status"）、
  `MemoryCrudContext{workspace_root}`、`unsupported()` helper。
- `MemoryProvider` trait 加 6 个 async **默认方法**（返回 `Failed{unsupported}`），
  10 个既有实现零改动编译（对齐 `record_recall_outcome` 先例）。
- 测试：默认方法返回 Failed；类型 serde roundtrip。

### S2 laputa：Typed 实现 — `5a742c82`
- `TypedLaputaMemoryProvider` 持有 `TypedMemoryStore` 第二连接
  （crud_store）+ `MemoryGovernanceCoordinator::open_lazy(...).ok()`
  （与 manager 共享 `.laputa/governance.db`）。
- add：构造 `MemoryRecord`（AppliedAuthority / LaputaAppliedSection，
  validate_at）→ `store.put` 即时权威 + audit_sink ToolInvoked 事件。
- list / search：`store.list(limit)` / `store.search_visible(query, scope, limit)`，
  tombstone 已过滤（G3 验收点）。
- update：`ProposalType::MemoryPatch` + RiskLevel::High → create_proposal +
  `coordinator.submit` → `ProposalCreated`（HITL apply 复用既有基建）。
- remove：`ProposalType::Deprecation`（patch schema_version=1 /
  target_record_id / reason）→ 同上；tombstone 由既有 apply 基建产生。
- distill：新建 skill 写 `<ws>/skills/<name>/SKILL.md`（即时）+ audit；
  覆盖已有 skill → `ProposalType::SopCreate` proposal + submit（W1-3）。
- 测试（tempdir + 真实 store）：add→list 可见且 AppliedAuthority；search
  排除 tombstone（G3）；update/remove→proposal 存在 + submit 映射；distill
  新建写入 / 覆盖 proposal；启动渲染与 list 一致性。

### S3 agent：Legacy proposal-first + 治理接线 — `522df4d0`
- `agent-diva-agent/src/memory_boundary.rs` 新增
  `LegacyCrudMemoryProvider{workspace, legacy: MemoryManager,
  service: Option<LaputaService>, coordinator: Option<MemoryGovernanceCoordinator>}`：
  读路径保留 MemoryManager（visible_lines 解析 MEMORY.md 行）；写路径全走
  async proposal（MemoryPatch / Deprecation / SopCreate + `submit`）。
- `memory_provider_for_mode`：Legacy 分支返回 wrapper；Cutover/Degraded 保持
  默认 Failed（文档化）。
- 测试：legacy add → proposal 落在 `.laputa`；10 个现有实现仍编译。

### S4 tools：六工具 + 接线 + prompt 恢复 — `bd04cfc5`
- `agent-diva-tools/src/memory_{add,list,search,update,remove,distill}.rs`：
  统一模式（Option provider + workspace，new / with_provider，Tool trait）；
  结果 JSON 三态：`{"status":"applied"|"proposal_created","proposal_id"?}`
  / `{"status":"failed","reason"}`（ask_user JSON 先例）。
- ToolAssembly：`memory_provider: Option<Arc<dyn MemoryProvider>>` +
  `with_memory_provider` builder + build_internal 注册（mask 默认启用）；
  builtin config 加 `memory: bool` gate（minimal/none/for_subagent 关）。
- `build_agent_tools`（agent_loop.rs）传 provider；3 处调用点 + loop_tools
  注册补参。
- context.rs 恢复工具指引（原诚实降级文案替换为「用 memory_add 记住…」），
  含三态结果语义说明 + 回归测试改写。
- 测试：每工具直接构造单测（mock provider / without provider → failed /
  invalid args → error）；ToolAssembly 注册 + mask deny 生效。

## 验证

- 每切片 `cargo fmt --check` + `cargo clippy -D warnings`（改动 crate）
  + `cargo test -p <crate>`。
- 全 workspace 测试：core 682+ / laputa 20+ / agent 378+ / tools 12+ 全绿；
  CLI 6 个既有 wiremock 502 失败（`CLI-WIREMOCK-502-PREEXISTING`，
  2026-08-05 已在干净树复现，与本迭代无关）原样复现。
- 修复 tools crate 3 个既有 clippy lint（cron nth(0)、filesystem
  items-after-test-module、shell identical blocks）随 S4 一并提交。

## 影响范围

- core：CRUD 请求/结果类型 + trait 默认方法（新增，无破坏）。
- laputa：Typed provider 6 方法 + coordinator 接线（写路径三路）。
- agent：Legacy wrapper、ToolAssembly/builtin config/agent_loop 接线、
  prompt 工具指引恢复。
- tools：6 个新工具 + 既有 lint 修复。
- docs：TODOLIST WAVE1 勾选；`memory-write-paths-contract.md` 无变化。
