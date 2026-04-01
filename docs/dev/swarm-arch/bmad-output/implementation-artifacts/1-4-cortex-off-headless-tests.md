---
story_key: 1-4-cortex-off-headless-tests
story_id: "1.4"
epic: 1
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/implementation-artifacts/1-1-swarm-crate-workspace.md
  - _bmad-output/implementation-artifacts/1-2-cortex-state-persistence.md
  - _bmad-output/implementation-artifacts/1-3-gateway-swarm-sync-contract.md
---

# Story 1.4：「关大脑皮层」简化模式 — 行为与无头测试

Status: done

## 依赖前置故事

| 故事 | 角色（本 story 中） |
|------|---------------------|
| **1.1** 蜂群 crate 骨架 | 测试与语义文档落点 crate / workspace 成员须已存在并可 `cargo test`。 |
| **1.2** 皮层状态模型 | 无头测试须能 **程序化** 将 Cortex 设为 **关**（与默认真值一致或可覆盖）。 |
| **1.3** Gateway 同步契约 | 若 headless 路径经 gateway/DTO，须复用 **同一契约** 读写关状态，避免与 FR14 漂移。 |

**说明：** 实现顺序上应先完成 1.1–1.3 的可交付增量；本 story 在其上增加 **简化模式语义文档** 与 **FR3 / FR12 无头回归测试**。

## Story

作为一名 **开发者**，  
我希望 **在无 GUI 的测试中验证「大脑皮层关」下的行走路径**，  
以便 **FR3 与 FR12 可回归**，且错误时能 **明确区分开/关分支**。

## Acceptance Criteria

1. **Given** 测试环境使用 **固定模型或桩**（不依赖真实 GUI）  
   **When** 大脑皮层为 **关** 并触发一轮 **标准对话/工具路径**（与项目现有最小 turn 流程对齐）  
   **Then** 运行时行为符合 **本 story 交付的「简化模式语义」文档**（见下节「简化模式语义（实现登记）」）中登记的规则

2. **And（FR12）** 上述验证通过 **`cargo test`**（或 CI 等价）在 **无 GUI / headless** 条件下执行；不得将本 AC 绑定到 Tauri 窗口或浏览器

3. **And（FR3）** 「关大脑皮层」下的可测行为与文档 **一一可追溯**：文档中每条可执行断言在测试中 **有对应用例或显式 `// doc-ref:` 注释** 指向文档章节

4. **And** 至少 **一条** 自动化测试在 **故意走错分支**（例如在期望 **关路径** 的用例中 **错误地保持开**，或反之）时 **失败**，且 **断言消息或测试名** 须 **显式包含**「大脑皮层」「开」或「关」等可检索关键词，使 CI 日志能直接指向 **开/关分支错误**

## 简化模式语义（实现登记）

> **交付物：** 下列内容须在实现中 **落盘** 为单一维护者可读文档（例如 `agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md` 或 crate `docs/` 下等价路径），并与本 story 的测试 **交叉引用**。

### 1. 定义范围

- **简化模式** 指：**大脑皮层为关（Cortex OFF）** 时，运行时对 **用户 turn** 所采用的 **编排/执行策略**（与「开」路径的差异须 **可列举**）。
- **不在本 story 冻结：** FR21「强制轻量（ForceLight）」与 OFF **合并或独立** 的最终 ADR（由 Story 1.9 二选一封死）；本 story 文档中须 **单列一小节「与 FR21 的边界」**，写明：当前实现若仅有 OFF，则 ForceLight 行为 **待 1.9** 或 **与 OFF 暂同义**（二选一由实现者写明，避免静默歧义）。  
- **Story 1.9 交叉引用（冻结路径，写死）：** [`ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`](../../agent-diva/agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md)（仓库相对路径：`agent-diva/agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`）。实现登记正文见 [`CORTEX_OFF_SIMPLIFIED_MODE.md`](../../agent-diva/agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md) §3。

### 2. 建议最小语义条目（实现时勾选/改写）

维护者应在文档中 **逐条** 写明「关」时是否成立（是/否/不适用 + 一句理由）：

- **多参与者蜂群：** 关时 **不进入** 完整多代理 handoff/对弈链（与 FR19 精神一致；详细路由在 1.7 扩展）。
- **过程事件：** 关时 **是否** 仍发射过程类事件；若否，须写明 **订阅方应依赖的替代信号**（例如仅终态）。
- **工具调用：** 关时 **是否** 允许工具路径；若允许，与「开」路径的差异（例如仅直连、无蜂群仲裁）。
- **默认与持久化：** 与 Story 1.2 登记的默认值、持久化范围 **一致**，并链接至 1.2 实现说明。

### 3. 与测试的契约

- 文档末尾须有 **「测试对照表」**：列 `文档章节 / 断言摘要 / 测试模块或函数名`。
- 至少一行对应 **「错误分支」用例**（见 Tasks），确保失败时 **可grep「开」「关」**。

## Tasks / Subtasks

- [x] **依赖与基线**（AC: 全局）  
  - [x] 确认 1.1 crate 可测；1.2 提供 Cortex OFF 的测试 API；若走 gateway，1.3 DTO/命令已可用  
  - [x] 在 README 或本 story 完成说明中 **链接** 1.2/1.3 的契约片段路径  

- [x] **编写《简化模式语义》文档**（AC: #1, #3, 简化模式节）  
  - [x] 创建/更新 `CORTEX_OFF_SIMPLIFIED_MODE.md`（或选定路径并在本文件「File List」登记）  
  - [x] 填写「定义范围」「最小语义条目」「与 FR21 的边界」「测试对照表」  
  - [x] 与 `prd.md` FR3、FR17 指向关系一致：FR17 偏 **用户说明**；本文档偏 **实现者与 headless 断言**  

- [x] **Headless 测试：关路径正向用例**（AC: #1, #2, FR12）  
  - [x] 固定桩或 mock provider，避免网络与非确定性  
  - [x] `#[test]` 或集成测试：Cortex **关** → 触发标准路径 → 断言 **不违反** 文档中「关」的禁止项（例如未进入全蜂群图式，若文档如此规定）  

- [x] **Headless 测试：开/关分支错误必现**（AC: #4）  
  - [x] 实现 **成对** 或 **参数化** 用例之一：  
    - **方案 A：** 同一输入下，`cortex_on` 与 `cortex_off` 各断言不同可观测行为（例如调用的执行层、事件种类计数）；**故意**在 OFF 用例中保留 ON 配置时，断言失败且消息含 **「大脑皮层」+「关」或「开」**  
    - **方案 B：** 单一负向用例，注释写明「模拟实现错误：OFF 请求却走 ON 分支」，期望 `panic!`/`assert!` 文案含开/关关键词  
  - [x] CI 中运行：`cargo test -p <相关包> -- <过滤可选>`，确保无 GUI  

- [x] **验证**（AC: #1–#4）  
  - [x] `cargo clippy -p <包> -- -D warnings`（与 workspace 策略一致）  
  - [x] 本地与 CI 均执行 headless 测试矩阵中与 1.4 相关的子集  
  - [x] 人工检查：故意改错一行「分支条件」后，**负向用例** 是否 **清晰失败**（红测）  

## Dev Notes

### Epic 1 上下文

Epic 1 要在后端确立 **大脑皮层开/关** 真相源、**简化模式**、过程事件、执行分层与收敛等。本 story **聚焦** FR3（关时语义可测）与 FR12（无 GUI 验证开/关与核心分支），**不** 要求完成完整 FR19 路由（1.7）或 FR20 收敛（1.8）。

### 架构合规

| 主题 | 要求 | 来源 |
|------|------|------|
| ADR-A | 测试与编排辅助代码仍在 swarm 边界内时 **不** 依赖 `agent-diva-meta` | `architecture.md` |
| 真相源 | Cortex 状态以 1.2/1.3 为准；测试 **不得** 在前端伪造长期 OFF 状态 | FR14、Additional Requirements |
| 无 GUI 优先 | 与架构「实现顺序」：(1) 版本化契约 + 无 GUI 测试 | `epics.md` Additional Requirements |

### 禁止事项

- 将本 story 的「可回归」仅落在 **手动** GUI 点击验收。  
- 断言消息含糊（如仅 `assert_eq!(a, b)` 而无开/关语境）导致 CI 无法区分 **分支错误** 与 **数据错误**。  
- 在文档中描述与 1.2 默认值 **冲突** 的 OFF 语义而不同步改状态模型 story。

### Testing Requirements

- **至少一条** 测试必须在 **错误分支** 下失败且 **日志可检索** 开/关关键词（见 AC #4）。  
- 优先 **单元或集成级** Rust 测试；若必须起进程，须 **headless** 且无显示依赖。  
- 与 Story 1.7、1.8 的后续无头测试 **命名空间区分**（模块前缀如 `cortex_off_`）。

### References

- `epics.md` — Epic 1, Story 1.4；FR3、FR12；FR Coverage Map  
- `prd.md` — FR3、FR12、FR17（用户向说明与本 story 文档互补）  
- `architecture.md` — 真相源、实现顺序、DTO 版本化  
- `1-1-swarm-crate-workspace.md`、`1-2-cortex-state-persistence.md`、`1-3-gateway-swarm-sync-contract.md` — 前置交付假设  

## Dev Agent Record

### Implementation Plan

在 `agent-diva-swarm` 内增加 **最小 turn 桩** `run_minimal_turn_headless`（接 `CortexRuntime` + FR19 `resolve_execution_tier`），产出可断言的 `MinimalTurnTrace`（含 `LightPath`）；语义与 FR21 边界写入 `docs/CORTEX_OFF_SIMPLIFIED_MODE.md`。测试模块 `minimal_turn::cortex_off_tests`（`cortex_off_*` 前缀），满足 FR12 无 GUI、AC#4 借助 `should_panic` 固定错误分支文案。

### Agent Model Used

Cursor / Composer（bmad-dev-story 工作流）

### Debug Log References

（无）

### Completion Notes List

- 新增 `docs/CORTEX_OFF_SIMPLIFIED_MODE.md`：定义范围、§2 最小语义条目、与 FR21 边界、测试对照表；与 `README` 中 1.2/1.3 路径表交叉引用。
- 新增/演进 `src/minimal_turn.rs`：`run_minimal_turn_headless(rt, text, explicit_full_swarm)` 读取 `CortexRuntime` 与 `resolve_execution_tier`（FR19）；OFF → `Simplified`；ON → `LightPath` 或 `FullSwarmOrchestration`；关路径仍无过程事件计数、不进入 handoff。
- Headless 单测模块 `cortex_off_tests`：`doc-ref` 指向文档章节；`cortex_on_and_off_minimal_turn_observable_layers_differ` 满足方案 A 式开/关可区分；`cortex_off_wrong_branch_panics_with_cortex_keywords` 以 `#[should_panic(expected = "大脑皮层")]` + `buggy_always_swarm` 固定 **错误分支** 并校验 panic 文案含「大脑皮层」「关」「开」关键词（AC#4）。
- 若将 `run_minimal_turn_headless` 内 `enabled` 分支颠倒，正向用例会红；`cargo clippy -p agent-diva-swarm -- -D warnings` 与 `cargo test -p agent-diva-swarm` 已通过。

### File List

- `agent-diva/agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md`（新增/与 FR19、`process_events` 对齐）
- `agent-diva/agent-diva-swarm/src/minimal_turn.rs`（新增/演进）
- `agent-diva/agent-diva-swarm/src/lib.rs`（导出 `minimal_turn`）
- `agent-diva/agent-diva-swarm/src/process_events.rs`（为 `ProcessEventPipeline` 补 `Debug`：`ThrottleState` + 手工 `fmt::Debug`，消除 `dyn Sink` 派生失败）
- `agent-diva/agent-diva-swarm/README.md`（1.2/1.3 基线表、Story 1.4 简化模式链接）
- `agent-diva/agent-diva-gui/src-tauri/src/capability_commands.rs`（补 `use std::sync::Arc`，便于全 workspace 编译）
- `_bmad-output/implementation-artifacts/sprint-status.yaml`（1-4 → done）
- `_bmad-output/implementation-artifacts/1-4-cortex-off-headless-tests.md`（本文件任务与状态）

### Change Log

- 2026-03-30：实现 Story 1.4 简化模式文档、`minimal_turn` headless 桩与 `cortex_off_*` 测试；README 链接 1.2/1.3；sprint 状态 1-4 → review。
- 2026-03-31：code review patch — `CORTEX_OFF_SIMPLIFIED_MODE.md` §4 测试对照表路径与 `cargo test` 一致；story / sprint 1-4 → done。

### Review Findings

_（bmad-code-review，2026-03-31；无 git diff，以当前文件对照 story；`cargo test` / `clippy -D warnings` 已通过。）_

- [x] [Review][Patch] 《简化模式语义》测试对照表中的 Rust 模块路径与 `cargo test` 过滤器不一致 — 已改为 `minimal_turn::cortex_off_tests::…` 并补充过滤说明。（2026-03-31 已修复）[`agent-diva/agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md` 测试对照表]

- [x] [Review][Defer] `full_swarm_cap_observable_via_pipeline_without_gui` 未列入 §4 测试对照表且无 `// doc-ref:` — 该用例主要覆盖 FR20 收敛/管道终局事件，与 Story 1.4 §4 可执行断言清单未逐行对齐；建议在 FR20/整合文档中补一行对照或补 `doc-ref`。[`agent-diva/agent-diva-swarm/src/minimal_turn.rs` ~238] — deferred, pre-existing scope overlap with 1.8

---

_Context: Ultimate BMad Method story context — 对齐 `bmad-create-story` / `epics.md` Story 1.4；简体中文。_
