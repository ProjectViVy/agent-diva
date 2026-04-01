# Agent-Diva / agent-diva-swarm — 研究汇总与架构备忘

**延伸阅读：** [能力 + 对内 swarm 深挖](./CAPABILITY_ARCHITECTURE_DEEP_DIVE.md) · [**Rust 架构设计（workspace / ADR / 阶段）**](./ARCHITECTURE_DESIGN.md) · [**设计补充（可观测、Meta 对齐、合成与安全等）**](./DESIGN_SUPPLEMENT.md)

本文档汇总对工作区内多个参考项目的研究结论，服务于目标：**以 Rust 实现的、类 nanobot 的单一「人」运行时**，其内部为 **能力（Capability）与声音（Voice）** 构成的 **蜂群式审议与抢方向盘**，而非传统「多个对用户可见的子智能体产品」。

> **子智能体说明**：Cursor 的 *subagent* 指 `.cursor/agents/*.md` 定义文件，供 IDE 按需委派；本仓库已创建多名 **研究向 subagent**，见 `.cursor/agents/`。它们不会自动并行执行，需你在对话中点名使用或由 Cursor 根据 `description` 路由。

---

## 1. 愿景对齐（来自对话共识）

| 概念 | 含义 |
|------|------|
| **Person** | 单进程 / 单会话身份：对外责任、主记忆、主 trace |
| **Voice** | 内部审议者：可并行提案、可争 **Steering（方向盘）** |
| **Capability** | 工具包 + 提示片段 + 可选模型档；先能力后性格 |
| **内群 swarm** | 《头脑特攻队》式：控制室多声部 + 仲裁；不是必然多邮箱「外群同事」 |
| **Meta 层** | 跨轮策略：类似 Claude Code **Stop / PreToolUse** hook，而非仅 LLM turn 内推理 |

---

## 2. 各参考栈的可借与勿抄

### 2.1 OpenAI `swarm`（历史教学库）

- **核心**：`Agent` + **handoff**（函数返回下一个 `Agent`）；**任意时刻只有一个 active agent**；`context_variables` 可累积。
- **借**：极简 `run()` 循环；handoff = **显式交方向盘**；共享 dict 作 working memory。
- **勿抄为唯一模型**：无法表达 **同时多声部并行审议**；需在其上叠 **黑板 / 仲裁**。

*路径：`swarm/swarm/core.py`, `types.py`*

### 2.2 `openai-agents-python`

- **核心**：**handoffs** vs **Agent.as_tool()** —— 前者换主导，后者主会话保留方向盘。
- **借**：**对外** 叙事可收敛为 **单 Person**；**对内** 仍用 handoff / 并行 / as_tool 组合实现 **swarm 式协作**（多专精、多轮、合成），而非「为了单主体就砍掉内群」。
- **文档**：`docs/multi_agent.md`。

### 2.3 Shannon

- **核心**：Go **Temporal** 编排 + **Rust agent-core**（enforcement、tool registry、WASI）+ Python LLM；**Swarm** = 多 **AgentLoop**、Redis **邮箱 + workspace**、结构化 action、收敛与 synthesis。
- **借**：**ToolCapability 元数据**、执行门禁、**共享 workspace 注入 prompt**、**多结果合成**、失败与迭代上限。
- **勿抄整栈**：若目标是单进程 Person，不必默认 Temporal + Redis；可 **降级协议** 到进程内队列与黑板。

*路径：`docs/shannon-agent-architecture.md`, `docs/swarm-agents.md`, `rust/agent-core/src/tool_registry.rs`*

### 2.4 `claude-code`（官方仓库可见部分）

- **核心**：插件 = **commands / agents / skills / hooks / MCP**；**hooks** 事件驱动（PreToolUse、Stop、UserPromptSubmit、PreCompact、SubagentStop 等）。
- **借**：**Stop hook 可 block 并回灌 prompt**（`ralph-wiggum`）；**feature-dev / code-review** 的 **多 agent 并行 → 过滤 → 单一输出** 配方。
- **路径**：`plugins/README.md`, `plugins/plugin-dev/skills/hook-development/SKILL.md`, `plugins/ralph-wiggum/hooks/stop-hook.sh`。

### 2.5 `oh-my-claudecode`（OMC）

- **核心**：`createOmcSession()` 聚合 **orchestrator system prompt + agents 表 + MCP**；**hooks/hooks.json** 全链路（UserPromptSubmit 魔法词 + skill 注入、Subagent 跟踪、Stop 延续、PreCompact 等）；**skills/** 内多阶段状态机 + `.omc` 产物路径。
- **借**：**事件总线式 meta 层**；**关键词 → 模式**；**Skill 文档即工作流**；**MCP 单 server 聚合工具**（`t`）。
- **路径**：`hooks/hooks.json`, `src/index.ts`, `scripts/keyword-detector.mjs`, `skills/autopilot/SKILL.md`。

### 2.6 `crewAI-examples`

- **核心**：**Flows** = 跨 crew 状态 + 路由 + 人机回路；与 LangGraph 集成示例展示 **图编排 + crew 执行** 分界。
- **借**：**阶段化 DAG** 表达「先探索再合成」；状态机所有者应是 **Person**。
- **路径**：`flows/README.md`，各 `flows/*` 子项目。

### 2.7 `nanobot`（工作区 Python 版）

- **借**：个人助理 + **skills 文件** 加载思路，与 OMC / Claude **skills** 同族。

---

## 3. 推荐参考架构（Rust 侧逻辑模块，非最终实现）

**已展开为正式设计文档：** [`ARCHITECTURE_DESIGN.md`](./ARCHITECTURE_DESIGN.md)（Cargo workspace、crate DAG、ADR、运行时阶段、风险）。

以下为早期逻辑模块草图（与文档中 crate 名 roughly 对应：`runtime` ≈ person-runtime + hooks 接线；`swarm` ≈ council + run_loop；`capability` ≈ registry）。

```
person-runtime/          # 会话、steering、合并对外回复
  ├── steering/          # SteeringLease、抢占策略
  ├── council/           # 黑板、Voice 提案、轮次预算
  ├── hooks/             # 对齐 CC 事件语义：PreToolUse、Stop、…
capability-registry/     # 声明、bundle、工具子集、schema（借鉴 Shannon + OMC agent 表）
invocation/              # 单次 Voice 调用、结构化返回、归因 voice_id
plugins/                 # manifest、skill 包、可选 WASM/脚本 hook
workers/（可选）         # 外群：tmux/子进程，对齐 OMC team 思路
```

---

## 4. 分阶段路线（建议）

1. **MVP**：单 Person + **CapabilityRegistry** + 类 Swarm 的 **handoff 或 as_tool 语义**（二选一清晰）；进程内 **黑板** + **steering**。
2. **内群 swarm**：多 Voice **并行提案**（async），**Chair** 规则或小型合成 LLM；**tiered history** 防 token 爆炸（借鉴 Shannon agent loop 文档）。
3. **Meta 层**：**Stop / PreTool** 等价钩子 + 文件或 sqlite 状态（借鉴 Ralph + OMC persistent-mode）。
4. **外群可选**：任务路由到外部 worker（Shannon/OMC team 思路）。
5. **性格**：在 **Voice 权重 / steering 偏好 / 记忆归因** 上挂钩子，不改核心协议。

---

## 5. 非目标（v0 明确不写）

- 完整复刻 Shannon 多服务 + Temporal。
- 用户可见的「十几个聊天机器人头像」。
- 无预算、无归因、无收敛判据的「全员每轮发言」。

---

## 6. 本仓库 Subagent 索引

| 文件 | 用途 |
|------|------|
| `shannon-orchestration-researcher.md` | Shannon / Rust core / swarm 协议 |
| `claude-omc-hooks-researcher.md` | Claude Code 插件 + OMC hooks/skills |
| `lightweight-swarm-handoff-researcher.md` | OpenAI Swarm + Agents SDK |
| `crew-flow-researcher.md` | CrewAI Flows |
| `agent-diva-synthesizer.md` | 更新本文档与总览 |
| （Task）`architect` | 产出/迭代 `ARCHITECTURE_DESIGN.md` 级 workspace 与 ADR |

使用示例（在 Cursor 中）：「用 `agent-diva-synthesizer` 根据当前代码库更新 `docs/AGENT_DIVA_SWARM_RESEARCH.md`」。

---

## 7. 修订记录

| 日期 | 说明 |
|------|------|
| 2026-03-30 | 初稿：综合对话研究与仓库阅读；创建 5 个子智能体定义 |
| 2026-03-30 | 链至 `CAPABILITY_ARCHITECTURE_DEEP_DIVE.md`（四路 research subagent 并行调研 + 能力体系展开） |
| 2026-03-30 | 2.2 补充：**对内保留 swarm 协作**，单 Person 指对外呈现与责任边界 |
| 2026-03-30 | 新增 `ARCHITECTURE_DESIGN.md`（architect subagent + 与 §3 交叉引用） |
| 2026-03-30 | 新增 `DESIGN_SUPPLEMENT.md`（审查补充：观测、transcript、Meta 表、Synthesis、安全、测试等） |
