# 能力体系深度调研 — Person 中心蜂群（Rust）

本文档在 `AGENT_DIVA_SWARM_RESEARCH.md` 总览之上，汇总 **`.cursor/agents` 研究向 subagent** 对仓库内参考实现的深挖结论，并展开 **能力（Capability）优先** 的架构建议与可辩论方向。

**实现向架构（crate、ADR、阶段）：** [`ARCHITECTURE_DESIGN.md`](./ARCHITECTURE_DESIGN.md) · **实现补充（观测、Meta 对齐、合成规格、安全、测试）：** [`DESIGN_SUPPLEMENT.md`](./DESIGN_SUPPLEMENT.md)

**调研日期：** 2026-03-30  

**参考仓库：** `swarm/`、`openai-agents-python/`、`Shannon/`、`oh-my-claudecode/`、`claude-code/plugins/`、`crewAI-examples/`  

---

## 1. 执行摘要

你要的「史无先例」在于 **语义重定向**，**不是**拿「能力」换掉 **swarm**：

- **对外**：始终是一个 **Person**（单进程身份、单责任边界、单主记忆与 trace、对用户可见的叙事一条线）。
- **对内**：**明确保留 swarm** —— 多参与者协作、黑板/邮箱、handoff、并行分支、请求协助、合成与收敛，与 Shannon / Crew / 历史 Swarm 同族。**Swarm 是内建协作拓扑**。
- **与旧叙事的关系**：传统「子 Agent」在实现上 **再角色化** 为 **带 Capability 的 swarm 成员**（工具子集、提示片段、可选模型档、预算与门禁）+ **Voice**（审议声部）。**Capability 描述「是谁在协作」，不取消协作本身。**

各参考栈的 **可借 primitives** 高度互补：

| 来源 | 最该借的「能力层」思想 |
|------|------------------------|
| **openai-agents-python** | `as_tool` = 主会话保留方向盘；handoff = 换驾驶员；并行 = 代码层 `gather` |
| **历史 Swarm** | 极简 `run()` 循环 + `context_variables` 作 working memory + 显式 handoff |
| **Shannon** | `ToolCapability` 元数据、执行门禁、收敛/合成规则、分层历史防 token 爆炸 |
| **OMC + Claude plugins** | 跨轮 **hooks** 事件链、关键词/技能注入、Stop 延续、agent 表 + MCP 聚合 |
| **CrewAI Flows / LangGraph** | **Person 拥有状态**；crew/子图是「一轮能力协作」；图边界上挂统一身份策略 |

**v0 的一句话：** **对内 swarm**（多成员协作 + 收敛）+ **对外 Person**（单一出口）；其上叠 **CapabilityRegistry**、**SteeringLease**、**黑板/议会**、**Hook/Meta**。产品是「一个主体」，运行时仍是「蜂群在工作」—— 只是用户不面对多个聊天机器人头像。

---

## 2. 子代理调研要点（带路径）

### 2.1 Swarm 与 OpenAI Agents SDK（轻量多代理）

**Swarm**（`d:\newspace\swarm\swarm\core.py`, `types.py`）：

- 循环内 **任意时刻只有一个 `active_agent`**；工具返回 `Agent` 或 `Result(agent=...)` 完成换班；同轮多次 handoff 时 **最后一个** 生效。
- `context_variables` 贯穿 `instructions` 与工具，是 **共享工作记忆** 的自然载体。

**openai-agents-python**（`docs/multi_agent.md`, `docs/handoffs.md`）：

- **Handoff**：专精模型 **接管** 对话与工具面。
- **`Agent.as_tool()`**：经理 **保留** 最终叙述责任；专精是「被调用的能力」。
- **并行**：文档明确可由 **代码** 编排（如 `asyncio.gather`），与 LLM 路由解耦。

**Primitive 矩阵（ steering 视角）**

| 机制 | 谁握方向盘 | 适用 |
|------|------------|------|
| Handoff | 下一个 Voice / 专精 | 任务 **换角色**（不同工具面、不同系统提示） |
| as_tool | 始终 Person / Chair | 同一决策多透镜，**必须合一输出** |
| 并行 gather | 运行时（Rust） | 独立子问题并行探测，再合并 |

**对 Rust 运行时（Person 外壳 + 内群 swarm）的结论：**  
- **Swarm 保留**：内部仍可有多条 **成员轨迹**（mailbox、workspace topic、`request_help` 式动态增员），与 Shannon swarm 文档同构；差别只在 **对外折叠** 为 Person stream。  
- **角色切换** → **Steering 移交**（handoff 语义）：换 active 成员或换该成员的 `capability_set` / 模型档。  
- **同轮审议** → **并行成员 + 黑板**（可结合 as_tool 式子调用实现某一类成员）；**Chair** 或 synthesis 负责 **合并**，再经 **唯一对外出口** 写出用户可见增量。  
- **产品约束**：禁止「多线程同时对用户各说各话」；**不禁止** 对内多成员并发 —— 对齐「单 active **对外** 通道 + **对内** 代码层并行 / 多 station」。

---

### 2.2 Shannon（生产级编排与 Rust agent-core）

**文档：** `Shannon/docs/shannon-agent-architecture.md`、`multi-agent-workflow-architecture.md`、`swarm-agents.md`、`extending-shannon.md`  

**Rust 侧：** `rust/agent-core/src/tool_registry.rs`（`ToolCapability` 等）、`enforcement.rs`、`tools.rs`

**可直接借到「能力系统」的 primitives：**

- **ToolCapability**：id、schema、`required_permissions`、`is_dangerous`、`rate_limit`、`cache_ttl_ms`、tags、examples —— 这就是 **能力声明** 的工业级形状。
- **RequestEnforcer**：超时、token 上限、限流、熔断 —— 可降级为 **进程内** 实现，无需 Redis/Temporal。
- **Swarm 文档中的动作协议**（`tool_call`, `publish_data`, `send_message`, `request_help`, `done`）与 **收敛条件**（无进展、永久工具错误、最后一轮强制 done）—— 映射为 **内群迭代预算** 与 **议会结束条件**。
- **Synthesis**：多结果合并模板 —— 映射为 **Chair 合成** 或 **单独 synthesis 能力**。

**与 Person 愿景的 gap（刻意利用而非照搬）：**

- Shannon swarm **多 station** 与对外 SSE，偏「用户可见多代理」；你要的是 **对内仍多 station（swarm）**，仅在 **产品层** 折叠为 **单 Person 对外叙事** —— **协作模型照抄，呈现层收束**。
- **Synthesis** 在 Shannon 是 **合并多代理输出**；在你这里是 **合并 swarm 成员 / 多 Voice 提案**，对象一致（都是内群产物），机制可直接复用。

---

### 2.3 oh-my-claudecode 与 Claude Code 插件（Meta 层）

**OMC** `hooks/hooks.json` + `scripts/*.mjs` + `src/index.ts`（`createOmcSession`）：

- **UserPromptSubmit**：关键词、技能注入 —— 映射为 **入口能力路由**（仍是一个 Person，换的是「加载的能力包」）。
- **PreToolUse / PostToolUse**：映射为 **工具执行门禁与验真**（与 Shannon enforcement 同层不同面）。
- **Stop / PreCompact / SessionStart|End**：映射为 **跨轮记忆策略与续跑** —— 「史无先例」里 **厌倦/中断** 将来可挂这里；能力阶段可先实现 **预算与压缩**。
- **SubagentStart/Stop**：映射为 **外群 worker** 或 IDE Task，与 **内群 Voice** 区分。

**两层模型（务必在 Rust 里分开 crate/模块边界）：**

1. **Turn 内**：模型 + 工具 + 能力子集（类似 SDK run loop）。
2. **Turn 间**：hooks 式 **事件总线**（持久化策略、续跑、注入）。

**风险：** Node/bash/python 脚本依赖、Windows 路径、`persistent-mode` 类状态单源；Rust 宿主应 **内置同类事件**，避免绑定 shell 生态。

---

### 2.4 CrewAI-examples（阶段化 DAG 与状态所有者）

**Flow：** `flows/README.md`、`write_a_book_with_flows`（大纲 → 并行章节 crew → 合并文件）、`email_auto_responder_flow`（状态 + 条件分支）。  

**LangGraph 集成：** `integrations/CrewAI-LangGraph` —— **图 = 控制流与等待**；**crew = 单节点内的多代理黑箱**。

**映射：**

- **Person ≈ Flow state / graph state 的 owner**：`topic`、`goal`、队列、去重 id 等 **不得** 散落在某个 crew 私有上下文。
- **Crew ≈ 一轮 Voice 协作或一个 capability 子图**：对外只产出 **合并后的 artifact**。
- **不要抄：** 无界并行（如每章一个 crew）且 **无统一 voice merge** —— 会破坏「一个人」的连贯口吻；需在能力层加 **style/merge 能力** 或固定 Chair 规则。

---

## 3. 能力体系设计建议（畅所欲言）

### 3.1 能力的定义（v0）

**Capability** 是 **swarm 成员的装备与契约**（声明 + 注册），不是「用能力代替 swarm」。成员仍是 swarm 里的行动者；Capability 说明该成员能调什么工具、带什么提示、受什么门禁。

建议把 **Capability** 定为 **一等声明**（数据 + 注册），与「swarm 成员 id / loop」正交组合，而非混成一个含糊的 Agent 类名：

- **id**、**human_name**（内部调试名，可不展示给用户）
- **tool_allowlist**（或 Shannon 式 tags → 解析为工具集）
- **prompt_injections**（system 片段顺序与优先级）
- **model_tier**（可选：快/中/大）
- **permissions**、`is_dangerous`、**rate_limit**（直接借 Shannon `ToolCapability` 字段思想）
- **dependencies / produces**（弱依赖 Shannon workspace 的 `produces` 思想，进程内可用 **黑板 topic** 实现）

**Voice（下一阶段）** 可以是 **Capability 实例 + 审议策略**（权重、发言顺序），而不是第二套类型系统。

### 3.2 运行时三块硬骨头

1. **CapabilityRegistry**：解析 manifest（可对标 OMC agent 表 + Shannon 工具发现），支持 **按场景激活子集**（避免一次塞满全工具）。
2. **SteeringLease**：谁持有「当前对用户回复的责任」；handoff = lease 转让；as_tool = lease 不转让，只追加 **结构化子结果**。
3. **Council / 黑板**：多 Voice **并行提案**（Rust async + 有界队列），**Chair** 规则（确定性优先级 / 小模型仲裁 / 单轮投票）+ **收敛条件**（借鉴 Shannon swarm 的迭代上限与「无进展」检测）。

### 3.3 Meta 层优先顺序（借鉴 OMC 借款列表）

1. PreToolUse / PostToolUse（门禁与验真）  
2. UserPromptSubmit（关键词 → 能力包热加载）  
3. Stop（续跑与压缩前策略）  
4. SessionStart/End、PreCompact  
5. Subagent 语义（外群）

### 3.4 可辩论的「史无先例」方向

- **能力作为预算单位**：不仅为工具限流，还为 **每轮议会总 token**、**每 Voice 最大发言次数** 计费，天然接 Shannon enforcement。
- **Steering 可抢占**：高优先级能力（安全、合规）可 **短暂抢方向盘**，事后把 lease 还回 Chair（类似 handoff + 强制交回）。
- **双环记忆**：**Person 主记忆**（用户可见因果）与 **能力草稿黑板**（可丢弃中间态），压缩时只保留前者 + 已提交 artifact。
- **与 nanobot 对齐**：skills 文件 ≈ capability bundle 的 **用户可编辑层**；Rust 内核只做 **加载、校验、版本、哈希**。

---

## 4. 与现有路线图的关系

本文件 **不替代** `AGENT_DIVA_SWARM_RESEARCH.md` 中的模块草图与分阶段路线，而是把 **能力** 与 **对内 swarm** 的并列关系写透：

- **MVP**：**内群 swarm 骨架**（至少：双成员 handoff 或 一成员 + 一顾问式子调用 + 黑板）+ **CapabilityRegistry** + **SteeringLease** + 进程内黑板。  
- **内群 swarm 强化**：并行成员 / 多 Voice + Chair + Shannon 式动作协议与收敛（与「仅能力、无协作」路线 **不兼容** —— 协作是默认）。  
- **Meta**：hooks 等价物。  
- **性格**：接在 Voice 权重与 Steering 偏好上，**不改** swarm 拓扑与能力协议。

---

## 5. Subagent 与文档维护

| Subagent 定义 | 本次覆盖主题 |
|---------------|----------------|
| `lightweight-swarm-handoff-researcher.md` | Swarm 循环、Agents SDK handoff / as_tool / 并行 |
| `shannon-orchestration-researcher.md` | ToolCapability、enforcement、swarm 动作与合成 |
| `claude-omc-hooks-researcher.md` | OMC hooks 清单、两层模型、Rust 宿主借款顺序 |
| `crew-flow-researcher.md` | Flow/LangGraph 状态归属、crew 边界、反模式 |

后续更新总览可点名：`agent-diva-synthesizer` 合并各研究结论进 `AGENT_DIVA_SWARM_RESEARCH.md`。

---

## 6. 修订记录

| 日期 | 说明 |
|------|------|
| 2026-03-30 | 初版：四路 subagent 并行调研 + 能力体系展开建议 |
| 2026-03-30 | 澄清：**对外 Person、对内保留 swarm 协作**；Capability 为成员装备，不取代蜂群 |
| 2026-03-30 | 链至 `ARCHITECTURE_DESIGN.md` |
| 2026-03-30 | 链至 `DESIGN_SUPPLEMENT.md` |
