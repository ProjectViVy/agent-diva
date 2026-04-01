---
story_key: 1-1-swarm-crate-workspace
story_id: "1.1"
epic: 1
status: done
generated: "2026-03-30T06:58:16+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
  - agent-diva/project-context.md
---

# Story 1.1: 蜂群 crate 骨架接入 workspace

Status: done

## Story

As a **维护者**,  
I want **在 agent-diva workspace 中新增蜂群相关 crate 并能被依赖编译**,  
So that **后续故事可在清晰边界内增量实现**。

## Acceptance Criteria

1. **Given** 当前 agent-diva 多 crate workspace  
   **When** 添加新 crate（名称以实现为准，建议 `agent-diva-swarm`）并声明 **单向依赖** 现有基础 crate  
   **Then** `cargo build --workspace`（或至少 `cargo build -p <新crate>` + 受影响成员）与 CI 等价检查通过，且 **无循环依赖**

2. **And** crate 根目录 **README.md**（或 `src/lib.rs` 顶部模块文档 + README 二选一，至少一处对用户可读）须 **显式链接或路径引用**：  
   - `_bmad-output/planning-artifacts/prd.md`  
   - `_bmad-output/planning-artifacts/architecture.md`  
   并写明 **ADR-A**：编排 swarm 实现 **不得** 依赖 `agent-diva-meta`（Meta 仅在 runtime/gateway 组合层边界触发）

3. **And** 新 crate **Cargo.toml** 的 `[dependencies]` 中 **不得** 出现 `agent-diva-meta`（直接 or 间接由本 story 引入的依赖树均不可；若仅用 `core`，仍须在 PR/说明中承诺后续拉入 `agent` 时也不违反 ADR-A）

## Tasks / Subtasks

- [x] **决议 crate 物理位置（本 story 内写死并落文）**（AC: #1）  
  - [x] **推荐路径 A：** `agent-diva/agent-diva-swarm/` 作为 workspace 新 member（与 `architecture.md`「Project Structure」示例一致）  
  - [x] **备选路径 B：** 在 `newspace/agent-diva-swarm/` 补全 `Cargo.toml` + `src/lib.rs`，并以 `path` 加入 `agent-diva` 根 workspace（须在 README 说明为何选 B）— **已决议不采用；README 已说明选用路径 A 及与 `agent-diva-swarm/docs/` 设计目录的对应关系**  
  - [x] 在 `agent-diva/Cargo.toml` 的 `members` 数组中加入新成员（若选 A）；若选 B，在根 workspace 中加 `path` 成员指向 sibling crate

- [x] **生成最小可编译库骨架**（AC: #1）  
  - [x] `cargo new agent-diva-swarm --lib`（或等价目录结构）  
  - [x] `lib.rs`：crate 级文档注释说明职责边界（蜂群编排、皮层契约后续故事填充）；可暂为 `//!` + 空模块或占位 `pub fn crate_version() -> &'static str`  
  - [x] `edition = "2021"`，`rust-version` 与 workspace 对齐（继承 `[workspace.package]` 或显式 `1.80.0`）

- [x] **配置依赖（单向、无 meta）**（AC: #1, #3）  
  - [x] 仅添加 **必要** 基础依赖：优先 **`serde` / `thiserror` / `tracing`** 等通过 `{ workspace = true }`（与现有 crate 一致）  
  - [x] 若需共享类型，优先 **`agent-diva-core`** 的 `path` 依赖；**禁止** `agent-diva-meta`  
  - [x] 运行 `cargo tree -p agent-diva-swarm`（或所选包名）人工确认树中无 `agent-diva-meta`

- [x] **文档与门禁说明**（AC: #2）  
  - [x] README：PRD、架构路径、ADR-A 一句话 + 指向 `agent-diva-swarm/docs/` 设计文档（可选链接）  
  - [x] （可选，epics Additional Requirements）在实现说明或 README 中预留 **CI：`cargo tree` 断言不依赖 meta** 的脚本位置，不必本 story 启用 CI

- [x] **验证**（AC: #1–#3）  
  - [x] `cd agent-diva && cargo clippy -p agent-diva-swarm -- -D warnings`（包名随实现调整）  
  - [x] `cargo test -p agent-diva-swarm`（至少通过空测试或 `#[test] fn smoke()`）

## Dev Notes

### Epic 1 上下文（交叉故事）

本 Epic 目标：在后端确立大脑皮层真相源、简化模式、过程事件、执行分层与收敛、遥测挂点，并向 GUI 暴露文档化契约。  
**本 story 仅做骨架**：不实现皮层状态、gateway 同步或事件总线；后续 **1.2–1.9** 在此外延。

### 架构合规（必须遵守）

| 主题 | 要求 | 来源 |
|------|------|------|
| ADR-A | swarm 编排 crate **不** `use` / 不依赖 `agent-diva-meta` | [Source: `_bmad-output/planning-artifacts/architecture.md` — Core Architectural Decisions / Swarm 与 Meta 边界] |
| 棕地 | 在既有 workspace 内 **增量**；单向依赖 | [Source: 同上 — Technical Constraints] |
| Crate 位置 | (A) `agent-diva/agent-diva-swarm` 或 (B) `newspace/agent-diva-swarm` + path member；须 **本 story 内** 选定 | [Source: `architecture.md` — Project Structure & Boundaries L348–349] |
| MSRV / 质量 | Rust 1.80.0，`clippy -D warnings`，workspace dependencies 模式 | [Source: `agent-diva/project-context.md` — 技术栈与质量规则] |

### 与 `agent-diva-swarm/docs` 的关系

`newspace/agent-diva-swarm/docs/` 已有设计文档（ARCHITECTURE_DESIGN、CAPABILITY_ARCHITECTURE_DEEP_DIVE 等）。**本 story 的 Rust crate** 可与该目录 **同前缀命名** 但物理位置按架构 **(A)/(B)** 决议；README 中说明 **设计文档仍位于** `agent-diva-swarm/docs/` 即可，避免贡献者混淆。

### 禁止事项

- 在本 story 中实现完整编排、Tauri 命令或 Vue 变更（属后续 Epic）。  
- 引入 `agent-diva-meta` 或循环依赖（例如新 crate ← agent ← swarm ← agent）。  
- 跳过 workspace 注册导致 `cargo build --workspace` 在 CI 中不编译新 crate。

### Project Structure Notes

- 工作区根：`d:\newspace\agent-diva\`（以用户机器为准）。  
- 规划产物：`d:\newspace\_bmad-output\planning-artifacts\`。

### Testing Requirements

- 至少 **1** 个单元测试或 `cargo test` 可运行的空测试目标，证明 crate 接入测试矩阵。  
- 无需 E2E / GUI。

### Latest Tech Notes（2026-03-30）

- 仓库 **MSRV 1.80.0** 优先于本机更新的 stable；新代码须在该版本下通过。  
- Tauri / Vue 本 story **不触碰**。

### References

- `epics.md` — Epic 1, Story 1.1  
- `architecture.md` — Starter、ADR-A、Project Structure、Implementation Sequence 步骤 1–2  
- `prd.md` — 棕地、Rust 真相源、范围策略  
- `agent-diva/project-context.md` — workspace 依赖与 clippy 规则  
- `agent-diva-swarm/docs/ARCHITECTURE_DESIGN.md` — 长期设计与 MVP 裁剪对照（只读，非本 story 交付物）

## Dev Agent Record

### Agent Model Used

Cursor / Composer（bmad-dev-story 工作流）

### Implementation Plan

- 采用 **路径 A**：`agent-diva/agent-diva-swarm`，在根 `Cargo.toml` 注册 workspace member。  
- 依赖：`agent-diva-core`（path）+ `serde` / `thiserror` / `tracing`（`workspace = true`）；`cargo tree -p agent-diva-swarm` 确认无 `agent-diva-meta`。  
- `lib.rs`：`//!` 职责说明、ADR-A、`CoreResult` 再导出、`crate_version()`、占位类型与 `SwarmError`，配套单元测试。  
- 回归：`cargo clippy -p agent-diva-swarm -- -D warnings`、`cargo test -p agent-diva-swarm`、`cargo check --all`。

### Debug Log References

（无）

### Completion Notes List

- 已满足 AC #1–#3：新 crate 接入 workspace、README 含 PRD/架构链接与 ADR-A、依赖树不含 meta。  
- 全工作区 `cargo check --all` 通过（约 4m48s，含既有 future-incompat 提示来自 `imap-proto`，非本 story 引入）。

### File List

- `agent-diva/Cargo.toml`（members 增加 `agent-diva-swarm`）  
- `agent-diva/agent-diva-swarm/Cargo.toml`  
- `agent-diva/agent-diva-swarm/src/lib.rs`  
- `agent-diva/agent-diva-swarm/README.md`  
- `_bmad-output/implementation-artifacts/sprint-status.yaml`（`1-1-swarm-crate-workspace`: review）  
- `_bmad-output/implementation-artifacts/1-1-swarm-crate-workspace.md`（本文件状态与任务勾选）

### Change Log

- 2026-03-30：新增 `agent-diva-swarm` crate 与 README；sprint / story 状态更新为 review。  
- 2026-03-30：代码评审 patch 已处理 — Git 提交 `18fe139`（`agent-diva-swarm` + workspace 成员）；story / sprint 标为 done。

### Review Findings

- [x] [Review][Patch] 将 `agent-diva-swarm/` 与根目录 `Cargo.toml` 的 workspace 成员变更提交到 Git — 已于分支 `feature-swarm-humanlike` 提交 `18fe139`。

- [x] [Review][Defer] [`agent-diva/agent-diva-swarm/README.md`] — README 中 `../../agent-diva-swarm/docs/ARCHITECTURE_DESIGN.md` 依赖 `newspace` 级并排目录（`agent-diva` 与 `agent-diva-swarm` 同级）；仅克隆 `agent-diva` 单仓时该链接可能 404。可选在 README 增加「完整 monorepo 布局」说明。 — deferred, pre-existing

- [x] [Review][Defer] [`_bmad-output/implementation-artifacts/1-1-swarm-crate-workspace.md` § File List] — File List 仅列骨架文件，仓库中已含后续故事引入的 `src/*.rs` 等；与严格「1.1 仅骨架」叙述并存时易造成审计混淆，可选更新 File List 或加注「后续故事已合入」。 — deferred, pre-existing

---

_Context: Ultimate BMad Method story context — `bmad-create-story` 于 2026-03-30 生成。_
