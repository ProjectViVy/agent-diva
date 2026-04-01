# ADR：编排适配 SPI（Shannon / Agents 路线评估）

**状态：** 已采纳（Story 5.4）  
**日期：** 2026-04-01  
**范围：** `agent-diva-swarm` 与 **组合层**（runtime / gateway）；**不**在本 crate 内集成 Shannon 或 Python。

---

## 1. 背景与问题

全蜂群路径（[`ExecutionTier::FullSwarm`](../src/execution_tier.rs)）当前包含：

1. **序曲（多角色 LLM）**：`agent-diva-agent` 中 `run_swarm_deliberation_prelude`，读取工作区 `swarm-prelude.*`（[`SwarmPreludeConfig`](../src/prelude_config.rs)）。
2. **收敛 / 终局**：`agent-diva-swarm` 中 [`execute_full_swarm_convergence_loop`](../src/convergence.rs)（FR20 过程事件、`SwarmRunStopReason`）。

未来可能希望由 **外部编排器**（如 Shannon、openai-agents-python 宿主、独立进程）承担部分或全部「多步编排」职责。需要：

- 文档化 **何时** 调用外部编排、**输入/输出** 形态、与 FullSwarm 的 **边界**。
- 在 **不违反 ADR-A**（`agent-diva-swarm` **不得**依赖 `agent-diva-meta`）的前提下，提供 **可替换端口（trait stub）**。

---

## 2. 决策

1. **在 `agent-diva-swarm` 内** 定义端口 `SwarmOrchestrationPort` 及 v0 DTO：`SwarmOrchestrationInputV0`、`SwarmOrchestrationOutcome`（见 `src/orchestration_port.rs`）。  
   - 默认实现 `BuiltinSwarmOrchestrationPort` = 当前 Rust **收敛循环** + 默认 `is_done` 桩（与 headless 最小 turn 行为一致）。  
   - **本 ADR 冻结的 SPI 切片** 对应「收敛阶段」；序曲仍属 agent 层，避免在 swarm 内拉入 LLM / 异步运行时。

2. **外部编排的调用时机（规划）**  
   - **仅当** 已判定进入 `ExecutionTier::FullSwarm`（皮层 ON、非 Light、非 FR21 关路径压制）且组合层 **显式启用** 外部宿主时，由 **runtime / gateway** 在 agent 主循环 **之外或之内**（实现待定）将收敛委托给实现 `SwarmOrchestrationPort` 的适配器。  
   - **未启用** 或 **委托失败** 时：**失败降级** — 继续使用内置序曲 + 内置收敛；若产品策略要求更强隔离，可降级为 Light 路径（由组合层策略表决定，**不在** swarm crate 硬编码）。

3. **合规 ADR-A**  
   - `SwarmOrchestrationPort` 的实现类型（含 PyO3、IPC 客户端）**只能**出现在依赖 `agent-diva-meta` 或 **独立二进制** 的 crate 中；`agent-diva-swarm` **仅**保留 trait 与内置实现。  
   - CI：`cargo tree -p agent-diva-swarm` 输出 **不得** 出现 `agent-diva-meta`（见 [`scripts/ci/check_swarm_no_meta.py`](../../scripts/ci/check_swarm_no_meta.py)）。

---

## 3. 后果

- **优点：** 边界清晰；后续可并行评估 Shannon / Python 路线而不改 swarm 依赖图。  
- **代价：** 全功能「端到端外部编排」仍需在 agent + 组合层补全注入与错误策略；本 story **不**实现具体集成。

---

## 4. 决策备忘（≤2 页）：宿主形态推荐顺序

| 顺序 | 形态 | 理由（摘要） |
|------|------|----------------|
| **1** | **纯 Rust 扩展**（同进程，trait 实现） | 延迟最低、类型与 `ConvergencePolicy` / 过程事件天然对齐；最易满足 ADR-A 与测试。适合先验证语义与遥测。 |
| **2** | **侧车进程**（gRPC/stdio/本地 socket） | 与 Python 运行时隔离，崩溃不拖垮主进程；适合 openai-agents-python、重依赖栈。成本是序列化 DTO 与运维。 |
| **3** | **嵌入式 Python**（libpython / PyO3 同进程） | 集成密度高，但版本、GIL、与 Rust 异步互操作复杂；**建议在前两者验证 DTO 与降级策略后再评估**。 |

**推荐路径：** 先 **Rust 默认端口**（当前已落地）→ 侧车原型（若必须 Python）→ 再视稳定性需求考虑嵌入式 Python。

---

## 5. 相关文档

- ADR-E / FR19：`execution_tier.rs`；架构边界见产品侧 `architecture.md`（ADR-A）。  
- FR20 收敛与事件：`convergence.rs` 模块文档。  
- 序曲配置：`prelude_config.rs`、工作区 `swarm-prelude.*`。
