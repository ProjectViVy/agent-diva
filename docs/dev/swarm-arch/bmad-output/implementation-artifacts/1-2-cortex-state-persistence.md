---
story_key: 1-2-cortex-state-persistence
story_id: "1.2"
epic: 1
status: done
generated: "2026-03-30T14:22:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/implementation-artifacts/1-1-swarm-crate-workspace.md
  - agent-diva/project-context.md
---

# Story 1.2: 大脑皮层状态模型与持久化边界

Status: done

## Story

As a **运行时**,  
I want **在会话/进程边界内维护 Cortex（大脑皮层）开/关状态**,  
So that **gateway 与后续 GUI 能查询一致状态**。

## Acceptance Criteria

1. **Given** 一次用户会话或等价生命周期  
   **When** 切换开/关（API 或测试钩子）  
   **Then** 状态可查询且 **符合 PRD 定义的真值**（含默认值）  
   **And** 持久化范围（仅内存 / 本地存储）在实现说明中写明并与 FR14 一致

## Tasks / Subtasks

> **前置依赖：** 须先完成 **Story 1.1**（`agent-diva-swarm` 或架构选定之蜂群 crate 已作为 workspace 成员存在且可编译）；本故事在该 crate（或架构指定的共享模块）内落地类型与状态，不得在 1.1 未合入前假设路径不存在。

- [x] **定义 Cortex 状态模型与默认值**（AC: #1）  
  - [x] 在 `agent-diva-swarm`（或 `architecture.md` 与 1.1 决议一致的包）中引入 **会话/进程内** 权威状态类型（名称以实现为准，可与架构示例 `CortexState` 对齐）  
  - [x] 明确 **默认开/关** 与 PRD/实现说明一致，并在代码或模块文档中单点写明（真值含默认值）

- [x] **实现切换与查询 API（内部或测试可见）**（AC: #1）  
  - [x] 提供 **切换开/关** 入口（内部函数、测试钩子或最小 `pub` API，不要求本故事暴露 Tauri；Story 1.3 再对齐 gateway 契约）  
  - [x] 提供 **查询当前状态**，保证与同一会话内写入一致

- [x] **持久化边界与 FR14 对齐说明**（AC: #1）  
  - [x] 在 **README 片段、`lib.rs` 模块文档或 `docs/` 短节** 中写明：状态当前为 **仅进程内内存**，还是 **会话期 + 本地存储**（若选用后者须与 `architecture.md` 数据架构「不新增专用 DB、延续既有存储」一致）  
  - [x] 显式对照 **FR14**：Rust 侧为 **单一真相源**；本故事不写 Vue 第二份权威状态

- [x] **Serde 与版本化字段（若适用）**（AC: #1，衔接 FR13/架构）  
  - [x] 若类型需为后续 IPC/JSON 共用，采用 **`serde` 可序列化形状**，并包含 **`schema_version`（或等价 u32）** 等与 `architecture.md`「Pattern Examples」一致的演进位  
  - [x] 版本字段策略与 **NFR-I1** 后续变更路径一致（本故事可先固定 v0）

- [x] **验证**（AC: #1）  
  - [x] `cargo test -p agent-diva-swarm`（或实际包名）：覆盖 **默认 → 切换 → 查询** 的单元测试，**无需 GUI / WebView**  
  - [x] `cargo clippy -p agent-diva-swarm -- -D warnings`（与 workspace 惯例一致）

## Dev Notes

### Epic 1 上下文

本故事在 **1.1  crate 骨架** 上建立 **大脑皮层（Cortex）** 的 **运行时真相源** 第一步：状态模型、默认值、会话/进程边界内的持有方式，以及持久化范围的文档化。**Gateway / Tauri 暴露与 GUI 同步** 属 **Story 1.3**；本故事可为内部 API + 测试钩子，不强制完成对外 command。

### FR14 与单一真相源

- **FR14：** 大脑皮层状态须与 **gateway/后端** 保持 **单一真相源**，避免长期双端不一致。  
- 本故事在 **Rust（蜂群 crate 或指定模块）** 内建立 **唯一权威副本** 的落点；前端不得在本故事阶段另立持久权威副本。  
- 架构要求：**权威状态在 Rust（gateway/新蜂群适配层）**；GUI 仅通过后续文档化 API/事件消费（见 `architecture.md` — Core Architectural Decisions / 运行时真相源）。

### 默认值与会话范围

- **默认值：** 须在实现与文档中与 PRD「大脑皮层」产品语义一致（例如默认开或关由 ADR/实现说明冻结）；验收要求 **真值含默认值** —— 未显式切换前查询结果须可预测且可测。  
- **会话范围：** 「一次用户会话或等价生命周期」—— 状态至少 **在同一会话/同一持有者实例** 内一致；跨进程是否恢复为 **本地存储** 若启用，须在实现说明中写明并与架构「延续既有存储」一致。

### Serde 与版本化类型

- 架构示例：`CortexState { enabled: bool, schema_version: u32 }`（`protocol` 或共享模块，前后端共用 serde 形状）。  
- 本故事若仅 crate 内 Rust 使用，仍可 **预先** 加入 `schema_version` 与 `serde`，减少 Story 1.3 DTO 漂移。  
- **serde 版本：** 使用 workspace 统一的 `serde` / `serde_json`（与 Story 1.1 `{ workspace = true }` 一致）。

### FR12 与测试

- **FR12：** 开/关分支须 **无 GUI** 可测。  
- 本故事单元测试 **仅 Rust**，不依赖 Tauri、不启动 GUI。

### 架构合规（摘要）

| 主题 | 要求 | 来源 |
|------|------|------|
| 真相源 | Rust 为编排与皮层状态权威；禁止 GUI 长期第二真相源 | `architecture.md` — 运行时真相源、FR12–FR14 |
| 反模式 | 前端 `cortexOn` 与后端 `cortex_enabled` 长期不同步且无协议 | `architecture.md` — Anti-Patterns |
| 数据持久化 | 不新增本特性专用 DB；会话/配置延续既有实践 | `architecture.md` — Data Architecture |
| ADR-A | 蜂群编排 crate 不依赖 `agent-diva-meta`（继承 1.1） | `architecture.md` — ADR-A |

### 禁止事项

- 不在本故事完成 **Tauri command / HTTP 对外契约**（留给 1.3）。  
- 不在 Vue 中实现皮层权威状态。  
- 不引入与 **1.1** 冲突的依赖（尤其 `agent-diva-meta`）。

### Testing Requirements

- 至少 **1** 组测试：**默认值**、**切换**、**再查询** 一致。  
- **无需** E2E、Playwright 或 GUI。

### References

- `d:\newspace\_bmad-output\planning-artifacts\epics.md` — Epic 1, Story 1.2  
- `d:\newspace\_bmad-output\planning-artifacts\architecture.md` — 运行时真相源、FR12–FR14、`CortexState` 示例、Implementation Sequence 步骤 1–2、Anti-Patterns  
- `d:\newspace\_bmad-output\planning-artifacts\prd.md` — FR12、FR14、大脑皮层真值与可测性  
- `d:\newspace\_bmad-output\implementation-artifacts\1-1-swarm-crate-workspace.md` — 前置 crate 与 ADR-A  
- `d:\newspace\agent-diva\project-context.md` — workspace 依赖与质量门禁

## Dev Agent Record

### Agent Model Used

Cursor 内联代理（Composer）

### Debug Log References

（无）

### Completion Notes List

- 在 `agent-diva-swarm` 新增 `cortex` 模块：`CortexState`（`enabled` + `schema_version`，serde + `#[serde(default)]`）、`CORTEX_DEFAULT_ENABLED`/`CORTEX_STATE_SCHEMA_VERSION_V0`、`CortexRuntime`（`snapshot` / `set_enabled` / `toggle`，进程内 `RwLock`）。
- README 与 `lib.rs`  crate 文档补充 **仅进程内内存**、**FR14** 与默认 `enabled = true` 说明。
- 单元测试：`default_toggle_query_matches_session`、`serde_json_round_trip`；`cargo test -p agent-diva-swarm` 与 `cargo clippy -p agent-diva-swarm -- -D warnings` 已通过。
- 工作区 `cargo test --all`：曾因仓库缺少默认 `skills/` 布局导致 `agent-diva-agent` / `agent-diva-manager` 技能相关测试失败；已补充 `agent-diva/skills/weather/SKILL.md`（与既有单测假定的内置 `weather` 一致），全量测试现已通过。

### File List

- `agent-diva/agent-diva-swarm/src/cortex.rs`（新增）
- `agent-diva/agent-diva-swarm/src/lib.rs`（导出 cortex、更新 crate 文档）
- `agent-diva/agent-diva-swarm/Cargo.toml`（dev-dependencies `serde_json`）
- `agent-diva/agent-diva-swarm/README.md`（Cortex / FR14 / 持久化边界）
- `agent-diva/skills/weather/SKILL.md`（默认内置技能布局，满足 SkillsLoader / SkillService 单测对 `weather` builtin 的假定）

### Change Log

- 2026-03-30：实现 Story 1.2 皮层状态模型、进程内运行时、文档与测试；sprint `1-2-cortex-state-persistence` → `review`。
- 2026-03-30：补充 `agent-diva/skills/weather/` 默认内置技能，修复干净 clone 下 `cargo test --all` 技能相关回归。

### Review Findings

- [x] [Review][Defer] 工作区相对 `HEAD` 的未提交改动在 `lib.rs` / `README.md` 中混入后续故事（如 `convergence`、`neuro_overview`、README FR20 段落）— deferred, pre-existing（合并前建议拆分提交或暂存无关变更，便于 bisect 与按 story 审查）

---

_Context: Ultimate BMad Method story context — `bmad-create-story` 于 2026-03-30 生成。_
