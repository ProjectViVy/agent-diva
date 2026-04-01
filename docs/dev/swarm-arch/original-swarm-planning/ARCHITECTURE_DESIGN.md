# Agent-Diva — Rust 架构设计（v0）

本文档由 **架构向 subagent（architect）** 根据 `AGENT_DIVA_SWARM_RESEARCH.md`、`CAPABILITY_ARCHITECTURE_DEEP_DIVE.md` 中的共识产出，并与 **Clean / Hexagonal** 原则对齐：**依赖向内**、领域无 LLM SDK 泄漏、`LlmClient` / `MetaBus` 等以 **trait（端口）** 暴露。

**前置阅读：** [研究总览](./AGENT_DIVA_SWARM_RESEARCH.md) · [能力 + 对内 swarm](./CAPABILITY_ARCHITECTURE_DEEP_DIVE.md) · [**设计补充（可观测 / Meta 对齐 / 合成规格等）**](./DESIGN_SUPPLEMENT.md)

**日期：** 2026-03-30

---

## 1. 设计原则（摘要）

| 原则 | 说明 |
|------|------|
| **对外 Person** | 单一身份、单一用户可见流、单一问责链；成员边界不产品化。 |
| **对内 swarm** | 多成员、黑板/邮箱、handoff、并行探测、`request_help`、合成与收敛。 |
| **Capability** | swarm 成员的装备与契约；与 `SwarmMemberId` **正交**。 |
| **Meta** | 进程内 Hook 总线（OMC 语义），**不**用 shell 做控制流。 |
| **v0** | 单进程；不强制 Temporal / Redis；耐久与分布式为后续特性。 |

---

## 2. 上下文与信任边界

### 2.1 参与者

- **Person（产品面）**：对外逻辑身份；非独立 OS 用户，而是运行时的 **对外 façade**。
- **User**：驱动轮次；只接收 **经 Person 聚合** 的输出（无「每成员一个聊天窗」）。
- **Operator / 集成方**（可选）：配置能力 manifest、模型密钥、MCP；在 v0 中与 sysadmin 同级信任。

### 2.2 外部系统

- **LLM API**：不可信网络；密钥走 env/secret。提示与工具结果均经 **能力白名单 + enforcement**；Meta 用于观测与策略。
- **MCP**（可选）：独立信任域；能力声明允许哪些 MCP 工具；与原生工具 **同一套门禁**。
- **文件系统 / 工作区**：半可信数据面；沙箱策略由 **Capability + enforcement** 表达。

### 2.3 进程内

- **Swarm members**：多内部参与者；经黑板/邮箱协作；用户不视其为多个「人」。
- **Meta 层**：观察并可变 **策略输入**（如 `UserPromptSubmit` 注入提示片），**不**与 swarm 循环抢第二套编排权。

### 2.4 信任边界表

| 边界 | 内侧 | 外侧 | 规则 |
|------|------|------|------|
| B1 | Runtime（Rust） | User + 网络 | 所有 LLM/MCP I/O 经 `LlmTransport` + 按能力过滤的工具派发。 |
| B2 | Person façade | Swarm 内部 | 仅 Chair / synthesis 认可的增量进入用户可见流。 |
| B3 | Meta hooks | Swarm 引擎 | Hooks 作用于声明的事件；swarm **不**为控制流 shell out。 |
| B4 | Capability manifest | 成员身份 | 工具/提示/权限挂在 **capability id**，在调度时 **组合** 到成员上。 |

---

## 3. Cargo Workspace 划分

### 3.1 Crate 职责

| Crate | 职责 |
|-------|------|
| `agent-diva-protocol` | Shannon 式 swarm **action JSON**（`tool_call`, `publish_data`, `send_message`, `request_help`, `done`）、工具结果信封、`version` 字段（向前兼容）。 |
| `agent-diva-domain` | `PersonSession`、`SwarmMember`、`CapabilityId`、`SteeringLease`、黑板键、邮箱消息、收敛/合成输入 —— **无** LLM SDK 类型。 |
| `agent-diva-capability` | `CapabilityRegistry`、manifest、工具白名单、提示片、`model_tier`、权限位；capability → 工具 schema 供 LLM 层使用。 |
| `agent-diva-enforcement` | 请求/token/时间预算、限流、危险工具门、熔断式降级（借鉴 Shannon `ToolCapability` / `RequestEnforcer`）。 |
| `agent-diva-meta` | `HookEvent`（`PreToolUse`, `PostToolUse`, `UserPromptSubmit`, `Stop`, `SessionStart`, `SessionEnd`, `PreCompact`）、`MetaBus` trait、有序派发、防循环的轻量 context trait。 |
| `agent-diva-llm` | `LlmClient` trait、消息/工具调用、**单路**外向 consumer 的流式块；Provider 适配（OpenAI-compatible 等）。 |
| `agent-diva-swarm` | 类历史 Swarm 的 **`run_loop`**、`context_variables` 等价物、handoff 解析、并行探测编排、黑板/邮箱 I/O、`request_help`、合成/收敛策略。 |
| `agent-diva-runtime` | **组合根**：`PersonSession`、meta → enforcement → swarm → llm 接线；单进程调度；`HookContext` 具体类型。 |
| `agent-diva-mcp`（可选） | MCP 客户端与能力面接线；MCP 类型不污染 `domain`。 |
| `agent-diva-cli`（binary） | 加载配置、启动 runtime、stdio / SSE 等薄入口。 |

### 3.2 依赖方向（DAG）

```
agent-diva-protocol          (serde, 日后可 schemars)
        ↑
agent-diva-domain  ─────────┘
        ↑
agent-diva-capability ──→ domain, protocol
agent-diva-meta       ──→ domain（仅 id + 小 trait）
agent-diva-llm        ──→ domain, protocol
        ↑
agent-diva-enforcement ──→ domain, capability
        ↑
agent-diva-swarm ──→ domain, protocol, capability, enforcement
        ↑
agent-diva-runtime ──→ meta, swarm, llm, enforcement, capability, domain, protocol
        ↑
agent-diva-cli / binaries
agent-diva-mcp（可选）──→ 经 trait 挂 llm 或 capability，避免与 runtime 成环
```

**硬规则：`agent-diva-swarm` 不得依赖 `agent-diva-meta`。** Runtime 订阅 hooks 并调用 swarm API；swarm 产出 **领域事件** 或 **`RuntimeEffect`**，由 runtime 在边界处触发 Meta（见 ADR-A）。

---

## 4. 核心领域类型（草案命名）

| 类型 | 一句职责 |
|------|----------|
| `PersonSession` | 用户可见 transcript 句柄、session id、对外暴露策略。 |
| `SwarmMember` | 内部参与者 id + 调试元数据；**不**内含能力集（另组合）。 |
| `CapabilitySpec` | 装备契约：工具白名单/tags、提示片、`model_tier`、权限、速率提示。 |
| `CapabilityBinding` | 某 tick/子阶段激活的 `(SwarmMemberId, CapabilityId)`。 |
| `SteeringLease` | **谁可写用户可见流**；handoff = 转让；`as_tool` = lease 不转让。 |
| `Blackboard` | 按 topic 的共享工作记忆（Swarm `context_variables` + Shannon workspace）。 |
| `Mailbox` | 成员间定向信封（每 lane 有序；支撑 `request_help` 路由）。 |
| `SwarmAction`（`protocol`） | 成员决策的 JSON 形态枚举。 |
| `ConvergencePolicy` | 迭代上限、无进展检测、强制 `done`、多 handoff **末者胜**。 |
| `SynthesisPolicy` | Chair 如何合并并行探测/竞争提案为单一 artifact + 可选用户可见块。 |
| `ParallelProbeSet` | N 路独立 LLM/工具子图及 join 语义（Rust 编排，类 `gather`）。 |
| `MetaBus` | 发布 hook 事件；runtime 注册处理器（OMC 链的进程内版）。 |
| `ToolInvocationContext` | 工具名、参数、成员、capability、lease 持有人 → 供 Pre/PostToolUse。 |
| `LlmClient` | chat + tools + stream 抽象。 |
| `RuntimeEffect` | 和类型：写用户、调度 handoff、入邮箱、写黑板、调工具 —— swarm 返回，runtime 执行并在边界打 hooks。 |

---

## 5. 运行时阶段

### 5.1 单次用户轮次（高层）

1. **SessionStart / 连续性** — 新建或加载 `PersonSession`、默认 Chair、本会话黑板切片（v0 内存）。  
2. **`UserPromptSubmit`（meta）** — 关键词/技能注入（OMC）；能力包 **声明级** 热切换（非换 Person）。  
3. **Ingress** — 用户消息写入 Person 拥有的 transcript；快照黑板「公开」键供提示使用。  
4. **Steering** — 校验 `SteeringLease`；可抢占策略若启用则显式交回路径。  
5. **Swarm 调度** — 选活跃成员：顺序循环与/或有界并行 `ParallelProbeSet`。  
6. **Pre-LLM** — 组装消息：能力提示片 + 黑板 + 自上次 tick 的邮箱。  
7. **LLM** — 流式写入 **内部** 缓冲；仅 synthesis 认可段进入 Person 流（lease 感知）。  
8. **工具路径** — `PreToolUse` → enforcement → 执行 → `PostToolUse`；结果进入 `SwarmAction` / 工具结果，可触发 handoff 或黑板写。  
9. **Handoff vs as_tool** — handoff：转让 lease + 切换 `CapabilityBinding`；as_tool：lease 不变，结构化结果进 Chair 上下文。  
10. **`request_help`** — 唤醒 helper 成员与限定能力；经邮箱回复；Chair 收敛决定合并。  
11. **Convergence** — 应用 `ConvergencePolicy`；未结束则带新状态回到 6（有界迭代）。  
12. **Synthesis** — `SynthesisPolicy` 合并探测/邮箱为 **单一** 对外叙事块。  
13. **`Stop`（meta）** — 最小 checkpoint（v0 可选刷盘）；若配置则 PreCompact。  
14. **Turn end** — 提交黑板修订、清理瞬时探测状态、指标/日志。

### 5.2 单次 swarm tick（内层）

1. **Read** — 消费活跃成员邮箱；合并能力声明的只读黑板 topic。  
2. **Plan** — 模型产出 `SwarmAction` 或 tool calls；经 `protocol` 校验。  
3. **Act** — 执行工具或 `publish_data` / `send_message` 等。  
4. **Handoff** — 同 tick 多 handoff 时 **last wins**（历史 Swarm 语义）；更新 lease。  
5. **Parallel join** — 若有 probes，结果进入 `SynthesisPolicy` 输入队列。  
6. **Progress** — 收敛策略检查停滞计数、工具失败 streak、最大迭代。  
7. **Emit** — 返回 `Vec<RuntimeEffect>` 给 runtime，用于 Meta 边界与 Person I/O。

---

## 6. 架构决策记录（ADR）

### ADR-A — `agent-diva-meta` 与 `agent-diva-swarm` 分离

- **背景**：OMC 式 hooks 跨轮次与工具边界；swarm 循环是 handoff/探测的局部迭代。混装易导致循环依赖与「用脚本当编排」。  
- **决策**：**`MetaBus` 独立 crate**；**`SwarmEngine` 不直接调用 `MetaBus`**。`runtime` 在稳定生命周期点调用 hooks。  
- **取舍**：事件清单需文档化；runtime 新增边界时须记得挂 hook。  
- **状态**：v0 采纳。

### ADR-B — `SteeringLease` 与 handoff / `as_tool` 映射

- **背景**：openai-agents 区分 handoff（交出主导）与 as_tool（专精被调用、经理保留叙述）。  
- **决策**：**Lease 为对用户可见归因的单一真相**。Handoff = 原子转让 lease + 通常切换活跃 `CapabilityBinding`。as_tool = 子调用期间 **lease 钉在 Chair（或当前持有人）**；被调方默认只产出 **结构化结果**，除非策略显式提升引用到用户流。  
- **状态**：v0 采纳。

### ADR-C — 黑板存储：内存 vs SQLite

- **背景**：单进程仍需跨成员/tick 共享；后续可能耐久。  
- **决策**：**v0：`Arc` + `RwLock`/分片 map，按 `PersonSession` 隔离**。**Phase 2：可选 `sqlite` feature** 或 `BlackboardStore` trait，热路径仍以内存为主、异步刷盘。  
- **状态**：v0 采纳；多会话审计需求时再评估。

### ADR-D — Handoff 上下文继承与裁剪（草案）

- **背景**：多次 handoff 易撑爆上下文或泄漏无关工具史；OpenAI Agents SDK 提供 `input_filter` / `nest_handoff_history` 等思路。  
- **决策方向**：按 `CapabilityId`（或 handoff 边）配置 **继承条数 / 是否_strip 工具轨迹**；与历史 Swarm **同 tick last-wins** 联动的裁剪顺序在 [`DESIGN_SUPPLEMENT.md`](./DESIGN_SUPPLEMENT.md) §7 细化。  
- **状态**：草案；实现前将具体字段写入 `agent-diva-capability` 或 `protocol`。

---

## 7. 分阶段（按 crate）

| Crate | MVP | Phase 2 | Phase 3 |
|-------|-----|---------|---------|
| `protocol` | SwarmAction JSON + 工具信封 + version | Schema 迁移；可选 JSON Schema | 二进制/gRPC（远程 worker） |
| `domain` | Session、成员、lease、黑板/邮箱 | 审计 id、抢占队列 | 联邦标识（每实例仍单 Person） |
| `capability` | 静态 manifest、提示片、工具白名单 | 热重载、签名 manifest | 第三方能力包 |
| `enforcement` | 超时、token 顶、基础限流 | 按能力预算、自适应退避 | 可选 Redis 配额 |
| `meta` | Pre/PostTool、UserPromptSubmit、Stop、Session、PreCompact | 优先级、取消、tracing span | 遥测适配（派发仍在本地） |
| `llm` | 单一 provider、异步流 | 多档路由 `model_tier` | 回退链、缓存 |
| `swarm` | `run_loop`、handoff、黑板、邮箱、单 Chair 合成、有界并行 | 完整 `request_help`、更丰富 synthesis | 可插拔收敛仲裁 |
| `runtime` | 单进程组合、stdio 演示 | MCP feature | 可选 Temporal 承载长作业 **且不替代** 进程内 swarm |
| `mcp` | — | MCP 与能力映射 | 发现/OAuth 等 |
| `cli` | 本地 dev | 配置 profile | 打包分发 |

---

## 8. 主要风险与缓解

| 风险 | 缓解 |
|------|------|
| Lease 泄漏 / 多声部泄漏到 UX | 所有用户可见写经 runtime `PersonOutbox`；每次写断言 lease；集成测试覆盖 handoff + 并行探测。 |
| Crate 循环（meta ↔ swarm） | CI：`cargo tree` 或自定义 lint；仅 runtime 同时依赖二者；swarm 只返 `RuntimeEffect`。 |
| 并行探测打爆 token | `ParallelProbeSet` 硬顶；enforcement 汇总子调用；收敛强制单一 synthesis 输出。 |
| Manifest 供应链安全 | Schema 校验、危险标记、MCP 来源钉死；新工具 PreToolUse 默认拒绝直至显式允许。 |
| Synthesis 成为瓶颈 | 可插拔 `SynthesisPolicy`（先确定性合并，再可选小模型 Chair）；结构化记录合并决策；Shannon 式「卡住」检测避免议会死循环。 |

---

## 9. 实现顺序建议

1. `agent-diva-protocol` → `agent-diva-domain`  
2. `agent-diva-capability` → `agent-diva-enforcement`  
3. `agent-diva-meta` → `agent-diva-llm`（mock 实现优先）  
4. `agent-diva-swarm`（依赖上述；**不**依赖 meta）  
5. `agent-diva-runtime` + `agent-diva-cli`

**结构约束：** Person 对外、swarm 对内，由 **`SteeringLease` + runtime 独占 `PersonOutbox`** 强制执行。

---

## 10. 延伸阅读

实现阶段的 **可观测性字段、内外 transcript 分轨、OMC 事件对照表、`SynthesisPolicy` I/O、跨成员安全、file-as-memory、handoff 裁剪与测试契约** 见 [**`DESIGN_SUPPLEMENT.md`**](./DESIGN_SUPPLEMENT.md)。**Handoff 上下文策略** 见 **§6 ADR-D（草案）** 与 `DESIGN_SUPPLEMENT` §7。

---

## 11. 修订记录

| 日期 | 说明 |
|------|------|
| 2026-03-30 | 初版：architect subagent 产出 + 与现有调研文档对齐并中文化整理 |
| 2026-03-30 | 链至 `DESIGN_SUPPLEMENT.md`；原 §10 修订记录改为 §11；新增 ADR-D（草案） |
