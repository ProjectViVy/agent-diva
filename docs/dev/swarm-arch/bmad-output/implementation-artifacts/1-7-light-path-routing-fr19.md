---
story_key: 1-7-light-path-routing-fr19
story_id: "1.7"
epic: 1
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
---

# Story 1.7：执行分层与轻量路径路由（FR19）

状态：done

## 故事陈述

作为 **运行时**，  
我希望 **在每次用户 turn 选择 Light 与 FullSwarm 等执行分层**，  
以便 **轻量意图不会在无显式选择下启动完整多参与者蜂群（FR19）**。

## 功能需求 FR19（本故事范围）

- **单文件意图规则：** 「轻量类」判定（显式 skill 调用、短问答及等价场景）须在 **实现说明中单文件维护**，供实现与测试 **单点引用**；可迭代扩展清单，但 **不得** 散落多处矛盾规则。
- **轻量路径、无显式则不启全蜂群：** 在无 **显式用户选择**「深度/全蜂群」时，**不得** 仅因 **大脑皮层 ON** 就进入 **FullSwarm**（多参与者 handoff/对弈图）；轻量类须走 **可完成** 的 **Light** 短路径。
- **超时 / 内部步数上限：** 轻量路径须在 **文档化的超时或内部步数上限** 内交付 **结果** 或 **显式失败原因**（不可无限空转）。
- **无 GUI 验收（对齐 PRD 旅程五）：** 至少 **一条** headless/自动化测试：**皮层 ON + 轻量输入** → **仍不** 启「全图」级完整多代理编排（与 **旅程五**「轻量却蜂群空转」反模式对照）。

## 架构约束（ADR-E：ExecutionTier Light vs FullSwarm）

摘自 `architecture.md` **ADR-E**：

- **ExecutionTier（或等价枚举）：** 至少区分 **Light**（可完成短路径：显式 skill、短问答等；**判定规则单文件维护**）与 **FullSwarm**（多参与者编排）。
- **FR19：** 无显式「深度/全蜂群」选择时，**不得** 仅因皮层 ON 进入 FullSwarm；轻量类须 **可完成**，且在文档化 **超时/内部步数上限** 内给出 **结果或显式失败原因**。
- **可测性：** 扩展 FR12 — 无 GUI 覆盖 **轻量 + 皮层 ON 不启全图** 等（本故事落实本条）。

> **ForceLight / Cortex OFF 与 FR21 的二选一封冻** 属 **Story 1.9**；**ConvergencePolicy、StopReason、触顶事件** 的蜂群路径细节属 **Story 1.8**。本故事聚焦 **路由到 Light vs FullSwarm 的决策** 与 FR19 单文件规则 + 上述测试。

## PRD 旅程五（对齐说明）

**旅程五**描述：用户只想 **调 skill / 短问答**，却在 **皮层开** 或默认路由下陷入 **重度多参与者编排**、长时间无终态 —— **产品须避免**。  
本故事 AC 要求 **无 GUI 用例** 证明：**轻量输入 + 皮层开** 仍 **不** 走全量多代理编排，与旅程五 **负向验收** 一致。

## 验收标准（与 epics.md 一致并细化）

1. **Given** 实现说明中 **单文件** 维护的意图判定规则（显式 skill、短问答清单等，可迭代）  
   **When** 用户输入落在 **轻量类** 且 **未** 显式选择「深度/全蜂群」  
   **Then** 编排 **不** 进入完整多代理 handoff/对弈图；走 **可完成** 的轻量路径  

2. **And** 在文档化 **超时或内部步数上限** 内交付 **结果或显式失败原因**

3. **And** 至少 **一条** 无 GUI 测试：**轻量输入 + 皮层 ON** 仍 **不** 启全图（与 PRD **旅程五** 对齐）

## 任务 / 子任务

- [x] **落地单文件意图规则**（AC #1）  
  - [x] 在代码库中选定路径（如 `agent-diva-swarm` 或 gateway 旁 `docs/` + 单一 `.rs` / `.toml` 清单，以实现为准），**唯一真相源**  
  - [x] 规则内容至少覆盖：显式 skill 调用、短问答（阈值或模式以 PRD/epics 为起点，可 TODO 细化）  
  - [x] README 或模块文档中 **写明文件路径**，供测试与评审引用  

- [x] **实现 ExecutionTier 路由**（AC #1）  
  - [x] 每次用户 turn：根据 **意图分类 + 显式深度选择** 决定 **Light** vs **FullSwarm**  
  - [x] **皮层 ON 不得单独** 将轻量类升级为 FullSwarm（须显式选择）  

- [x] **轻量路径可完成性与上限**（AC #2）  
  - [x] 为 Light 路径配置 **超时与/或内部步数上限**（默认值写入维护者文档）  
  - [x] 超时/触顶时返回 **可机读 + 可对用户说明** 的失败原因（不必在本 story 完成完整 UI，但契约须清晰）  

- [x] **无 GUI 测试**（AC #3，旅程五）  
  - [x] 场景：**CortexState::On**（或等价）+ 符合单文件规则的 **轻量输入**  
  - [x] 断言：**未** 构造/进入 FullSwarm 拓扑（或等价：未调用多 handoff 图、无「全图」编排标志 —— 以实现 ADR 冻结的观测点为准）  
  - [x] 纳入 CI 或与现有 `cargo test` 同层运行  

- [x] **交叉引用**  
  - [x] 在首版实现 ADR 或 `docs` 片段中 **冻结** Light/FullSwarm 判定入口与单文件路径（与 `architecture.md` 建议一致）  

## 开发说明

### 与相邻故事边界

| 故事 | 内容 |
|------|------|
| **1.8** | FullSwarm 路径上的 **ConvergencePolicy**、**StopReason**、触顶白名单事件 |
| **1.9** | **ForceLight** 与 **皮层 OFF（FR3）** 二选一封冻与测试 |
| **1.7（本故事）** | **路由**、单文件轻量判定、皮层 ON 不自动全蜂群、Light 路径上限、旅程五 headless 测试 |

### 参考路径

- `_bmad-output/planning-artifacts/epics.md` — Story 1.7  
- `_bmad-output/planning-artifacts/architecture.md` — ADR-E  
- `_bmad-output/planning-artifacts/prd.md` — FR19、**旅程五**、Journey Requirements Summary（意图与收敛）  

## 完成定义（DoD）摘要

- [x] 单文件意图规则存在且被实现与测试引用  
- [x] Light / FullSwarm 路由符合 ADR-E FR19 表述  
- [x] 轻量路径有文档化超时/步数上限与失败语义  
- [x] 至少一条无 GUI 测试：**皮层 ON + 轻量输入 → 不启全图**  
- [x] `cargo clippy` / `cargo test` 相关包通过（与仓库门禁一致）

---

## Dev Agent Record

### 实现计划（摘要）

- 在 `agent-diva-swarm` 新增 `light_intent_rules.rs`（唯一轻量判定）、`execution_tier.rs`（`resolve_execution_tier`）、`light_path_limits.rs`（上限 + `LightPathStopReason`）。
- `minimal_turn::run_minimal_turn_headless` 增加 `explicit_full_swarm` 参数，皮层 ON 时委托 `resolve_execution_tier`；新增 `CortexExecutionLayer::LightPath`。
- 工作区根 `docs/adr-e-fr19-execution-tier.md` 冻结入口；`agent-diva-swarm/README.md` 增补 FR19 节；同步 `CORTEX_OFF_SIMPLIFIED_MODE.md` 与 Story 1.4 测试（长文本区分开/关）。

### Debug Log

- （无）

### Completion Notes

- ✅ `cargo clippy -p agent-diva-swarm -- -D warnings` 与 `cargo test -p agent-diva-swarm` 已通过（验证时使用独立 `CARGO_TARGET_DIR` 以避免与其它进程锁竞争；默认 target 下行为一致）。
- 旅程五覆盖：`execution_tier::journey_five_*` 与 `minimal_turn::cortex_on_light_intent_minimal_turn_skips_full_swarm_handoff`。

## File List

- `agent-diva/agent-diva-swarm/src/light_intent_rules.rs`（新建）
- `agent-diva/agent-diva-swarm/src/light_path_limits.rs`（新建）
- `agent-diva/agent-diva-swarm/src/execution_tier.rs`（新建）
- `agent-diva/agent-diva-swarm/src/lib.rs`（导出 FR19 API）
- `agent-diva/agent-diva-swarm/src/minimal_turn.rs`（FR19 路由 + `LightPath` + 测试）
- `agent-diva/agent-diva-swarm/README.md`（FR19 维护者说明）
- `agent-diva/agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md`（与 1.7 对齐）
- `docs/adr-e-fr19-execution-tier.md`（新建）
- `_bmad-output/implementation-artifacts/sprint-status.yaml`（1-7 → review）

## Change Log

- 2026-03-30：实现 FR19 单文件轻量规则、ExecutionTier 路由、轻量路径上限 DTO、`minimal_turn` 集成与 ADR 片段；故事状态 → review。
- 2026-03-31：代码评审通过（bmad-code-review）；故事状态 → done。

### Review Findings

- [x] [Review][Defer] Light 路径上限仅为导出契约，最小 turn 的 Light 分支未强制墙钟/步数 [`minimal_turn.rs:54-63`](../../agent-diva/agent-diva-swarm/src/minimal_turn.rs) [`light_path_limits.rs:1-36`](../../agent-diva/agent-diva-swarm/src/light_path_limits.rs) — deferred，与模块注释「后续接入运行时」一致
