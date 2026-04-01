---
story_key: 1-3-gateway-swarm-sync-contract
story_id: "1.3"
epic: 1
status: done
generated: "2026-03-30T12:00:00+08:00"
depends_on:
  - 1-2-cortex-state-persistence
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
  - agent-diva/project-context.md
---

# Story 1.3：Gateway 与蜂群状态同步契约

Status: done

## 依赖

- **必须先完成 Story 1.2**（大脑皮层状态模型与持久化边界）：本故事在 **已可查询、可切换的 Cortex 真相源** 之上，定义 **对外契约** 与 **gateway / Tauri 薄适配**，避免在 1.2 未落地时重复定义状态语义。

## Story

As a **集成方**,  
I want **GUI 与 gateway 通过同一契约读写大脑皮层状态**,  
So that **不存在长期双端漂移（FR14）**。

## Acceptance Criteria

1. **Given** 已定义 **可序列化 DTO** 以及 **与现有 gateway 模式对齐** 的接口面（Tauri `invoke` / 事件，和/或 gateway 内部消息路径 —— **以实现前检索 `src-tauri` 与 gateway 既有命名为准**）  
   **When** 从 **GUI 测试客户端** 或 **无 GUI 测试钩子** 发起状态切换/查询  
   **Then** **gateway（或 Tauri 背后同一 Rust 真相源）** 侧查询结果与请求 **一致**，且无第二份长期分叉的「权威皮层状态」

2. **And** **FR13：** GUI **仅** 通过 **已文档化** 的 API/事件获取蜂群相关状态；契约正文写入 **`docs/` 片段** 或 **相关 crate 的 `README.md` 章节**（须含：命令/事件名清单 v0、请求/响应形状、错误形态与 **NFR-R1** 切换失败语义引用）

3. **And** **FR14 / NFR-I1：** 对外 DTO 含 **版本字段**，与 `architecture.md`「Format Patterns — 版本字段」一致：**`schema_version` 或 `api_version` 二选其一，全项目统一一种**（建议与架构示例 `CortexState { …, schema_version: u32 }` 对齐）；**NFR-I2：** 字段与端点 **白名单** 可列清单；契约演进须同步 **CHANGELOG 或 ADR**（与架构 Enforcement 一致）

4. **And** **Tauri 2：** 新 `command` / `listen` 事件在注册前 **对照** `agent-diva-gui/src-tauri` 现有命名与 **`Result` 错误类型**；成功路径 **直接返回强类型序列化结果**（除非全应用已统一包装层）；**禁止** Vue 侧长期维护与后端不一致的皮层布尔为第二真相源（对齐架构「State Management — 以后端为准」）

5. **And** **Gateway 单点注入：** 皮层状态读写路径在文档中标明 **唯一注入/适配点**（避免多处各写各的），与 `architecture.md`「Gateway：蜂群状态注入点应单点文档化」一致

## Tasks / Subtasks

- [x] **复核 Story 1.2 产出**（AC: #1, #5）  
  - [x] 确认 Cortex 状态由 **Rust（gateway 或新蜂群适配层）** 持有，与 epics 1.2 中「持久化范围」说明一致  
  - [x] 若 1.2 使用独立类型名，本故事 DTO **复用或透明包装** 该类型，避免重复语义

- [x] **定义 v0 同步 DTO**（AC: #1, #3）  
  - [x] 在 **可被 Tauri + gateway 共享** 的 crate/模块中定义结构体（如 `CortexSyncDto` 或复用 `CortexState`），字段含 **`enabled`（或项目既定命名）+ `schema_version`**，及 1.2 要求的最小附属字段（若有）  
  - [x] `serde` 默认与全链路 JSON 约定一致；若前端需 camelCase，**全链路统一** `rename_all` 并在此 story 文档中写明  
  - [x] **白名单：** 在文档中列出 v0 字段表；新增字段走版本 bump 或 ADR

- [x] **对齐现有 gateway / Tauri 模式**（AC: #1, #4）  
  - [x] 检索 `commands.rs`（及 lib 注册）现有 command 命名风格（snake_case vs camelCase）与错误返回类型  
  - [x] 实现 **查询** 与 **设置**（或合并为单一 command，以实现与现有模式更贴近者为准）**薄适配**：仅转调 1.2 真相源 API，**不** 在适配层复制业务状态机  
  - [x] （可选，若架构 ADR 已冻结推送）对状态变更 emit **`cortex_toggled`** 类事件时，payload **小而稳定** 且含 `schema_version`

- [x] **文档化契约**（AC: #2, #3）  
  - [x] 在 `docs/`（如 `docs/swarm-cortex-contract-v0.md`）**或** `agent-diva-swarm` / `agent-diva-gui` README 中增加 **「大脑皮层同步契约 v0」** 节：端点名、DTO JSON 示例、`schema_version` 含义、与 FR13/FR14 的对应关系  
  - [x] 注明 **GUI 禁止** 绕过该契约长期自写皮层状态（引用 UX-IMPL-1）  
  - [x] 契约变更流程：CHANGELOG 或 ADR 链接

- [x] **测试**（AC: #1）  
  - [x] **至少一条** 无 GUI 集成/单元测试：模拟客户端切换后 **gateway 侧（或共享状态句柄）** 查询与预期一致  
  - [x] 若已有 Tauri 测试夹具，可选补一条 **invoke 往返**（非本 story 阻塞时可标为 follow-up）— **Follow-up：** 可后续增加 `tauri::test` invoke 往返用例。

- [x] **验证**（AC: #1–#5）  
  - [x] `cargo clippy` / `cargo test` 覆盖改动 crate  
  - [x] 人工走查：文档中的命令名与 Rust 注册名 **一字一致**

## Dev Notes

### Epic 1 上下文

本故事落实 **FR13（文档化 API/事件）** 与 **FR14（单一真相源、无长期双端不一致）** 在 **gateway ↔ GUI 边界** 的契约层；**不** 在本 story 内实现完整蜂群编排、过程事件总线（属 1.5+）或 Vue 完整 UI（属 Epic 2）。

### 架构合规（必须遵守）

| 主题 | 要求 | 来源 |
|------|------|------|
| 真相源 | 皮层状态权威在 **Rust**；GUI 仅 Tauri command / 事件消费 | `architecture.md` — Core Decisions、API & Communication |
| FR13–FR14 | 契约文档化 + 与 gateway 查询一致 | `epics.md` Story 1.3；`architecture.md` — Requirements Mapping |
| DTO 版本 | `schema_version` 或 `api_version`，**全项目统一一种** | `architecture.md` — Format Patterns、Pattern Examples |
| Tauri | `invoke` / `listen`；命名与错误形态与现有 `src-tauri` 一致 | `architecture.md` — API Naming、API Response Formats |
| Gateway | 状态注入 **单点**、与现有消息/会话中心模式对齐 | `architecture.md` — Service Boundaries、Integration Points |
| 兼容 | NFR-I1 不破坏现有 provider/MCP/channels；NFR-I2 白名单 | `epics.md` — NFR；`architecture.md` — ADR 摘要 |

### 禁止事项

- 在 Vue/Pinia 中引入 **与后端长期不同步** 的皮层状态为唯一权威。  
- 未含版本字段的「临时 JSON」作为对外稳定契约。  
- 在 swarm 编排 crate 内为「迁就 GUI」直接依赖 `agent-diva-meta`（仍受 ADR-A 约束）；本 story 适配层留在 **gateway / Tauri 组合边界**。

### Project Structure Notes

- 契约类型优先放在 **Rust 核心 + Tauri 可共享** 位置（`architecture.md` — Structure Patterns）。  
- 工作区根以用户仓库为准；规划产物：`_bmad-output/planning-artifacts/`。

### Testing Requirements

- **至少 1 条** 无 GUI 测试证明切换后 gateway/共享状态可读且一致（FR12 延伸）。  
- 全量 E2E 非本 story 阻塞。

### References

- `epics.md` — Epic 1, Story 1.3；FR13、FR14；Coverage Map  
- `architecture.md` — API & Communication、Format Patterns、Communication Patterns、Project Structure、Pattern Examples（`CortexState` + `schema_version`）  
- `prd.md` — Rust 真相源、集成面可列清单  
- Story 1.1：`1-1-swarm-crate-workspace.md`；Story 1.2：`1-2-cortex-state-persistence.md`（依赖）  
- `agent-diva-gui/src-tauri` — `commands.rs`、`lib.rs` 注册与既有错误类型

## Dev Agent Record

### Agent Model Used

Cursor 内联代理（Composer）

### Debug Log References

（无）

### Completion Notes List

- 复用 `agent_diva_swarm::CortexState`，增加 `CortexSyncDto` 类型别名；JSON 全链路 `camelCase`（`schemaVersion`）。
- Tauri 单点注入：`Arc<CortexRuntime>` + `get_cortex_state` / `set_cortex_enabled` / `toggle_cortex`；变更后 `emit("cortex_toggled", payload)`。
- 契约文档：`docs/swarm-cortex-contract-v0.md`；`agent-diva-swarm` README 链接；`agent-diva/CHANGELOG.md` [Unreleased] 记录。
- 无 GUI 测试：`agent-diva-gui/src-tauri/tests/cortex_sync_contract.rs`（跨线程写入后查询一致）。
- 验证：`cargo test -p agent-diva-swarm`、`cargo test -p agent-diva-gui`；`cargo clippy -p agent-diva-swarm -- -D warnings` 与 `cargo clippy -p agent-diva-gui --no-deps -- -D warnings`（全 workspace `-p agent-diva-gui -p agent-diva-swarm` 会因依赖 crate 既有 clippy 告警失败，非本 story 引入）。

### File List

- `agent-diva/agent-diva-swarm/src/cortex.rs`
- `agent-diva/agent-diva-swarm/src/lib.rs`
- `agent-diva/agent-diva-swarm/README.md`
- `agent-diva/agent-diva-gui/src-tauri/Cargo.toml`
- `agent-diva/agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva/agent-diva-gui/src-tauri/tests/cortex_sync_contract.rs`
- `docs/swarm-cortex-contract-v0.md`
- `agent-diva/CHANGELOG.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-03-30：实现 Story 1.3 — 皮层同步 DTO/Tauri 薄适配、事件、`docs` 契约与集成测试；sprint 状态 → review。

### Review Findings

- [x] [Review][Decision] `get_neuro_overview_snapshot` 与 v0 契约表不一致 — **已裁定选项 A**（2026-03-31）：`docs/swarm-cortex-contract-v0.md` 已补充该 command、`NeuroOverviewSnapshotV0` / `NeuroActivityRowV0` 白名单与 JSON 示例。

- [x] [Review][Patch] `cortex_toggled` 发射失败静默 — **已修复**：`set_cortex_enabled` / `toggle_cortex` 在 `emit` 失败时 `tracing::warn!`（target `agent_diva_gui::cortex`）。

- [x] [Review][Patch] `AGENT_DIVA_TEST_CORTEX_SYNC_FAIL` 并行竞态 — **已修复**：`agent-diva-gui` 增加 `serial_test` dev-dependency，相关 4 个单元测试加 `#[serial]`。

- [x] [Review][Defer] 相对 `main` 的同一 diff 中混入与 Story 1.3 无直接关系的改动（例如 `collect_status_report(&runtime, false)` 签名变更、`capability_commands` 注册、CHANGELOG 中 Story 1.5 长条目）— 增加审查与回滚耦合；非本 story 单独引入时可接受，但建议在后续拆 commit 或拆 PR — deferred, pre-existing（混合批次）。

---

_Context: Ultimate BMad Method story context — 依据 epics Story 1.3 与 architecture.md（API、DTO 版本、gateway、FR13–FR14、Tauri）生成；依赖 Story 1.2。_
