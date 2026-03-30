# 简化模式语义（大脑皮层关 / Cortex OFF）

**维护者说明：** 本文档登记 **实现者可断言** 的「关大脑皮层」行为，与 **FR3**（关时语义可测）、**FR12**（无 GUI 验证）及 **FR17**（用户向说明，见 `prd.md`）互补：FR17 偏产品语言，本文偏 **headless 断言与代码分支**。

**前置契约（须一致）：**

- Story **1.2** 状态模型、默认值、持久化边界：`agent-diva/agent-diva-swarm/src/cortex.rs` 与 [`README.md`](../README.md)「大脑皮层」节。
- Story **1.3** Gateway / DTO v0：仓库根 [`docs/swarm-cortex-contract-v0.md`](../../../docs/swarm-cortex-contract-v0.md)。

---

## 1. 定义范围

- **简化模式** 指：**大脑皮层为关（Cortex OFF，`CortexState.enabled == false`）** 时，对 **单次用户 turn** 所采用的 **编排/执行策略**（与「开」路径的差异见下节逐条）。
- **Story 1.7（FR19）已接入：** `run_minimal_turn_headless` 在皮层 **开** 时按 [`docs/adr-e-fr19-execution-tier.md`](../../../docs/adr-e-fr19-execution-tier.md) 区分 **LightPath**（轻量输入、不进入多 handoff 全图）与 **FullSwarmOrchestration**。**不在本文冻结：** FR20 收敛策略（Story 1.8）。

---

## 2. 最小语义条目（当前实现）

| 条目 | 关（OFF）时 | 理由（一句） |
|------|-------------|--------------|
| **多参与者蜂群** | **否** — 不进入完整多代理 handoff/对弈链 | 与 FR19 一致；皮层 **开** 时仅当 **非轻量** 且未强制其它策略时，headless 桩标记 **FullSwarmOrchestration**；轻量输入走 **LightPath**（见 `minimal_turn` + `execution_tier`）。 |
| **过程事件** | **否** — 本最小路径不发射中间过程类事件计数 | 关路径 `process_events_emitted == 0`；订阅方在简化模式下应依赖 **终态/完成信号**（后续总线故事扩展）。 |
| **工具调用** | **是（直连桩）** — 允许工具/对话最小路径；与「开」差异为 **不经全蜂群编排层** | 与「开」相比：不开 `entered_multi_agent_handoff`。 |
| **默认与持久化** | 与 1.2 一致：默认 `enabled == true`（开），进程内内存 | 见 `cortex.rs` 与 README，不得与此文档矛盾。 |

---

## 3. 与 FR21 的边界

- **FR21（ForceLight / 强制轻量）** 与 OFF **合并或独立** 的最终 ADR 由 **Story 1.9** 封死。
- **当前实现：** 仅存在 **大脑皮层开/关** 门控；**未** 单独实现 ForceLight 标志位。在最小 turn 路由中，**ForceLight 行为视为与 OFF 暂同义**（即若将来仅有 OFF，则轻量诉求走同一简化路径；1.9 后可拆分为独立分支）。
- 若代码路径仅判断 `enabled`，不得静默假设 FR21 已合并；以本文与 1.9 ADR 为准迭代。

---

## 4. 与测试的契约

### 可执行断言（摘要）

- **A1（关 — 不进入全蜂群 handoff）：** `CortexState::with_enabled(false)` 时，`run_minimal_turn_headless(rt, text, false)` 返回的 `entered_multi_agent_handoff == false`。
- **A2（关 — 无过程事件计数）：** 同上，`process_events_emitted == 0`。
- **A3（开/关可区分）：** 使用 **非轻量** 长输入（超过 `SHORT_QA_MAX_SCALARS`）时，`enabled == true` 与 `false` 时 `CortexExecutionLayer` 不同（开 → FullSwarmOrchestration，关 → Simplified）。
- **A4（错误分支可检索）：** 若实现错误地在关路径进入 handoff，失败断言须含 **「大脑皮层」** 与 **「关」或「开」**（见测试名与 `assert!` 文案）。

### 测试对照表

| 文档章节 | 断言摘要 | 测试模块 / 函数名 |
|----------|----------|-------------------|
| §2 多参与者蜂群 | OFF → `!entered_multi_agent_handoff` | `cortex_off::cortex_off_minimal_turn_skips_full_swarm_handoff` |
| §2 过程事件 | OFF → `process_events_emitted == 0` | `cortex_off::cortex_off_minimal_turn_emits_no_process_events` |
| §2 工具/路径差异 | ON vs OFF → `layer` 不同 | `cortex_off::cortex_on_and_off_minimal_turn_observable_layers_differ` |
| §1 FR19 | 皮层 ON + 轻量输入 → `LightPath`、`!handoff` | `cortex_off::cortex_on_light_intent_minimal_turn_skips_full_swarm_handoff` |
| §4 A4 / AC#4 | 关路径误走 handoff 时 panic 消息可 grep | `cortex_off::cortex_off_wrong_branch_panics_with_cortex_keywords`（`#[should_panic]` + `buggy_always_swarm` 模拟错误实现） |

---

## 5. 与 `prd.md` 的指向关系

- **FR3：** 关模式下可测行为以本文 **§2 + §4** 为准；测试用 `// doc-ref:` 指向本文件。
- **FR17：** 用户可见文案与引导在 PRD/UX；实现断言以本文为准。
