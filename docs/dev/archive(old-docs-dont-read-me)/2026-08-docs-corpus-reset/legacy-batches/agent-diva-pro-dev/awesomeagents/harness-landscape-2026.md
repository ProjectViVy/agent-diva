# Agent Harness 方案调研 2026：超越 OpenHarness Swarm

> 调研日期：2026-06-11
> 调研目的：在决定 **暂缓 OpenHarness Swarm 路径** 后，识别其他值得参考的 agent harness 方案
> 关联决策：见 `openharness-analysis.md` 第 5.4 节、`openharness-vs-diva-pro-gap-analysis.md` 差距 1
> 关联原则：agent-diva 协作层的产品定位是 **"尽可能高效"** —— supervisor + 任务总线优先

---

## 一、调研背景

### 1.1 为什么做这次调研

之前 `openharness-vs-diva-pro-gap-analysis.md` 把 **Multi-Agent Swarm** 列为 agent-diva 最大的 P0 差距。但经过 2026-06-11 的产品讨论，确认：

- OpenHarness Swarm 的"头脑特工队"式平等协商，**不符合 agent-diva "协作层尽可能高效"的定位**
- 资源开销大（每个 agent 需要独立 Git worktree 隔离）
- 已有更现代的替代方案：supervisor + 任务总线

但 **"不走 OpenHarness Swarm"不等于"不参考其他 harness"**。本调研梳理 2026 年主流的 agent harness 方案，找出真正值得 agent-diva 借鉴的部分。

### 1.2 调研范围

本调研不关注**多 agent 协作拓扑**（已经决定走 supervisor 模式），而是关注 **harness 的其他关键能力**：

- 子 agent 调度与生命周期
- 上下文管理（compaction、memory）
- 工具系统（数量、隔离、权限）
- 安全/审批机制
- 测试方法论
- 持久化与恢复

---

## 二、主流 Agent Harness 方案

### 2.1 Claude Code（Anthropic）

**来源**：[Anthropic Harness Design](https://www.anthropic.com/engineering/harness-design-long-running-apps)、[AddyOsmani 解读](https://addyosmani.com/blog/agent-harness-engineering/)、[Decoding AI Harness System Design](https://www.decodingai.com/p/agentic-harness-system-design)

**架构核心**（基于 Fareed Khan 拆解）：

```
[Permission Gate] → [Tool Runtime (19-40 工具)] → [Subagent Context Firewall]
        ↓                    ↓                              ↓
[Hook 拦截层]          [Worktree Isolator]           [Memory Store]
        ↓                    ↓                              ↓
              [Agent Loop + Context Injection]
```

**关键设计点**：

1. **Subagent Context Firewall**（子 agent 上下文防火墙）—— 子 agent 有独立的上下文窗口，与主 agent 隔离。这是 agent-diva 当前 `SubagentManager` 已有的设计。
2. **Hook 强制执行** —— "hooks enforce behavior architecturally — a hook that blocks a tool call cannot be reasoned around"。这是 Claude Code 区别于纯提示词工程的关键。
3. **Workflows 一等公民**（CLAUDE_CODE_WORKFLOWS=1.52）—— "first-party orchestration primitive for flows that previously required custom dispatch scripts, mailbox state, and subagent coordination conventions"。
4. **工具分类**：file reads/writes、Bash、Git、web fetch、notebook、MCP。**每个工具独立 sandbox**。
5. **公开承认的限制**：Workflows "does not replace your safety model. Keep PreToolUse and PostToolUse hooks as the blocking layer, keep spawn budgets or workflow step budgets to prevent runaway width."

**对 agent-diva 的价值**：

| 可借鉴 | 优先级 | 原因 |
|--------|--------|------|
| Subagent Context Firewall 设计 | 中 | 当前 mask Epic 2 已在做工具限制运行时强制执行，是类似思路 |
| Hook 强制执行语义 | 高 | 当前只有 file/plan hook，缺通用 PreToolUse/PostToolUse |
| 每个工具独立 sandbox | 中 | 当前 sandbox 是粗粒度，可细化 |
| Workflow 一等公民 | 低 | 短期不必要，mask + plan 已覆盖 |

### 2.2 Hermes Kanban（Nous Research）

**来源**：[Hermes Architecture](https://hermes-agent.nousresearch.com/docs/developer-guide/architecture/)、[The Hermes Kanban Guide](https://magnus919.com/2026/05/the-hermes-kanban-a-complete-guide-to-multi-agent-task-orchestration/)、[Multi-Agent Setup Tutorial](https://hermes-tutorials.dev/blog/multi-agent-setup/)

**架构核心**：

```
              [Main Hermes Instance]
                       ↓ delegate_task
              [Subagent Pool (cheap tokio tasks)]
                       ↓
              [SQLite-backed Kanban Board]  ← 跨 profile 共享
              ├── pending tasks
              ├── running tasks
              ├── completed tasks
              └── dependency graph
```

**关键设计点**：

1. **任务总线而非 mailbox** —— 任务有显式的状态机（pending → running → completed/failed），持久化到 SQLite。
2. **跨 profile 共享** —— 多个 Hermes 实例可以共享同一个 Kanban，实现"多机器多 agent 协作"。
3. **Worktree mode** —— subagent 在独立 git worktree 中运行，避免文件系统冲突。
4. **delegate_task 工具** —— 主 agent 用显式工具调用 spawn subagent，不是隐式触发。
5. **依赖解析** —— Kanban 任务可以有依赖关系，自动等待上游完成。
6. **完整 run history** —— 失败可重放，crash 可恢复。

**对 agent-diva 的价值**：

| 可借鉴 | 优先级 | 原因 |
|--------|--------|------|
| 持久化任务总线 | **高** | 当前 `SubagentManager` 是 fire-and-forget，agent 失败后上下文丢失。Kanban 模式可解决。 |
| 任务状态机（pending/running/completed/failed） | **高** | 配合 mask 系统，给用户可见的并行任务进度 |
| 跨会话持久化 | 中 | agent-diva 已有 SQLite，扩展为任务表成本低 |
| delegate_task 显式工具 | 中 | 当前 `SubagentManager` 是隐式 API，改为显式工具更易观测 |
| Worktree 模式 | 低 | 暂缓（与 swarm 决策一致） |

### 2.3 LangGraph（LangChain）

**来源**：[LangGraph Multi-Agent](https://www.mager.co/blog/2026-03-12-langgraph-deep-dive/)、[LangGraph + Claude SDK](https://www.mager.co/blog/2026-03-07-langgraph-claude-agent-sdk-ultimate-guide/)

**架构核心**：

```
[Directed Graph with Conditional Edges]
   ├── 节点（agent/function）
   ├── 条件边（基于 state 决定下一步）
   └── State（持久化的图状态）
```

**关键设计点**：

1. **图而非对话** —— 显式定义"哪个 agent 在什么条件下被调用"，不是 GroupChat 风格的多 agent 自由对话。
2. **Conditional Edges** —— 路由可以是动态的（如：if score < 0.8 then retry else return）。
3. **树状 supervisor** —— supervisor 可以管理 sub-supervisor 再管理 worker，分层控制。
4. **状态持久化** —— 图状态可以序列化，支持中断恢复。

**对 agent-diva 的价值**：

| 可借鉴 | 优先级 | 原因 |
|--------|--------|------|
| 树状 supervisor 分层 | 低 | agent-diva 当前单层 supervisor 足够 |
| 条件边（基于 state 路由） | 中 | mask 的 `AgentMode` 切换可借鉴此模式 |
| 状态持久化 | 中 | 与 Hermes Kanban 价值重叠 |

### 2.4 AutoGen / AG2（Microsoft）

**来源**：[AutoGen v0.4 重写](https://alicelabs.ai/en/insights/best-ai-agent-frameworks-2026)、[Frameworks Comparison](https://medium.com/@atnoforgenai/10-ai-agent-frameworks-you-should-know-in-2026-langgraph-crewai-autogen-more-2e0be4055556)

**架构核心**：

```
[GroupChat] → [Speaker Selection] → [Next Agent]
       ↑                                ↓
   [User] ← [Termination Condition]
```

**关键设计点**：

1. **GroupChat** —— 多个 agent 共享一个对话频道，由 speaker selection 决定下一个发言者。
2. **Human-in-the-loop** —— 设计上就支持人在回路的 review。
3. **AG2 社区分支** —— v0.4 重写后，社区坚持 v0.2 风格形成 AG2，体现 "user-driven evolution"。

**对 agent-diva 的价值**：

| 可借鉴 | 优先级 | 原因 |
|--------|--------|------|
| Human-in-the-loop 一等公民 | **高** | 当前 `clarify` 工具是 ad-hoc，可考虑作为 AgentMode 之一 |
| User-driven evolution 模式 | 低 | 不直接借鉴，是项目治理经验 |

### 2.5 Kimi K2.6 Agent Swarm（Moonshot AI）

**来源**：[Kimi K2.6 Agent Swarm Guide](https://lushbinary.com/blog/kimi-k2-6-agent-swarm-300-sub-agents-guide/)、[MindStudio 解读](https://www.mindstudio.ai/blog/kimi-k2-300-sub-agents-4000-steps-4x-h100s-story-hermes-found)

**架构核心**：

```
[Orchestrator] → decompose task
       ↓
[300 Specialized Sub-Agents (parallel)]
       ↓
[4000 Coordinated Steps in single session]
```

**关键设计点**：

1. **极致规模** —— 300 子 agent，4000 步，4x H100。BrowseComp 上 86.3% vs GPT-5.4 78.4%。
2. **本质是 supervisor + 工作流** —— "Agent Swarm" 这个名字有误导性，**实际上是 orchestrator 调度 + 子 agent 并发**。
3. **零基础设施并行** —— 不需要外部数据库或 worktree，状态在内存。
4. **代价**：放弃审计能力 —— "External orchestration trades setup cost for auditability. Teams that need to explain their agent decisions — to auditors, to security reviewers — will find external orchestration's transparency worth the setup cost."

**对 agent-diva 的价值**：

| 可借鉴 | 优先级 | 原因 |
|--------|--------|------|
| Orchestrator + 大量子 agent 模式 | 中 | mask Epic 3 的 supervisor 模式与 Kimi 思路一致 |
| "Agent Swarm" 实质是 supervisor | 参考 | **进一步验证我们的决策**（OpenHarness Swarm 不适合 agent-diva） |
| 审计与可观测性权衡 | **高** | agent-diva 用户可能需要审计能力，不能完全追求零基础设施 |

### 2.6 OpenDev（arXiv 2603.05344）

**来源**：[OpenDev Paper](https://arxiv.org/abs/2603.05344)、[OpenDev HTML](https://arxiv.org/html/2603.05344v1)、[OpenDev Tutorial](https://co-r-e.com/method/opendev-terminal-coding-agent)

**架构核心**（4 层）：

```
[Layer 1: Context Engineering]
   - 5-stage Adaptive Context Compaction
[Layer 2: Harness]
   - Tool execution
   - Session persistence
[Layer 3: Safety]
   - Defense-in-depth
[Layer 4: Scaffolding]
   - Agent configuration
```

**关键设计点**：

1. **5 阶段 Adaptive Context Compaction**：
   - Stage 1: Message boundary detection
   - Stage 2: Priority scoring
   - Stage 3: Token budget allocation
   - Stage 4: Semantic compression
   - Stage 5: Loss validation
2. **Scaffolding vs Harness 区分**：
   - **Scaffolding** = agent 启动前的配置（instructions、tools、permissions）
   - **Harness** = 运行时编排层（tool execution、context management、safety、persistence）
3. **Defense-in-depth safety** —— 多层防护，不是单点。
4. **长期会话设计** —— 论文专注 "long-running application development"。

**对 agent-diva 的价值**：

| 可借鉴 | 优先级 | 原因 |
|--------|--------|------|
| 5 阶段 Adaptive Context Compaction | **高** | agent-diva 当前有 `feat(compaction): support multi-compaction chain`（CC-P5），可对齐此模型 |
| Scaffolding vs Harness 区分 | 中 | 概念上有用，但 agent-diva 当前未严格区分 |
| Defense-in-depth 安全 | **高** | 当前 sandbox 审计正在做，是同类思路 |
| 长期会话设计 | 中 | 当前已有 session 持久化，可强化 |

### 2.7 OpenAI Agents SDK / Swarm（参考但暂不深入）

**来源**：[Galileo OpenAI Swarm](https://galileo.ai/blog/openai-swarm-framework-multi-agents)

**核心**：handoffs（agent 之间的显式控制权转移）。OpenAI 已将 Swarm 概念合并到 Agents SDK。

**对 agent-diva 的价值**：

- 不直接借鉴（swarm 路径已暂缓）
- 但 handoffs 概念在 mask 的 `AgentMode` 切换中可借鉴

---

## 三、横向对比矩阵

| 维度 | Claude Code | Hermes Kanban | LangGraph | AutoGen | Kimi K2.6 | OpenDev | OpenHarness |
|------|-------------|---------------|-----------|---------|-----------|---------|-------------|
| **拓扑** | Supervisor+Worker | Supervisor+任务总线 | 图+Supervisor树 | GroupChat | Orchestrator+300子 | 单agent+scaffolding | Swarm对等 |
| **持久化** | 会话级 | SQLite Kanban | 图状态 | 会话级 | 内存（无） | 会话级 | 无（mailbox） |
| **可观测性** | 高（hooks） | 高（任务状态） | 高（图执行） | 中 | 低（"零基础设施"） | 高（4层） | 中 |
| **审计能力** | 高 | 高 | 中 | 中 | 低 | 中 | 低 |
| **规模** | 中（单会话） | 大（跨profile） | 中 | 中 | **极大（300+）** | 中 | 中 |
| **长期会话** | 强 | 强 | 中 | 中 | 弱（无持久化） | **极强（论文主题）** | 弱 |
| **生产成熟度** | **极高** | 高 | 高 | 中 | 中 | 低（论文） | 中 |
| **生态** | 闭源+API | 开源 | 开源 | 开源+闭源 | 闭源 | 开源 | 开源 |

---

## 四、关键洞察

### 4.1 "Agent Swarm" ≠ 对等协商

调研发现一个**反直觉的事实**：

> 业界所有号称"Agent Swarm"的方案，**实际上都是 supervisor + 工作流**，不是真正的对等协商。
> - Kimi K2.6 "Agent Swarm"：orchestrator 调度 300 子 agent
> - LangGraph "Multi-Agent"：图+条件边
> - Hermes "Subagent"：delegate_task 工具调用

**只有 OpenHarness 是真正的对等 mailbox 模式**，但代价是审计能力弱、资源开销大。

**对 agent-diva 的启示**：当前决策（supervisor 模式）符合 2026 年主流趋势，**不是降级**。

### 4.2 真正稀缺的能力

去掉多 agent 协作拓扑，2026 年 agent harness 真正稀缺的是：

1. **长期会话管理**（OpenDev 论文主题） —— 5 阶段 Adaptive Context Compaction
2. **审计与可观测性**（Kimi 的痛点） —— hooks、任务状态机、执行 trace
3. **持久化任务总线**（Hermes Kanban 风格） —— 跨会话、跨 profile 的状态共享
4. **Defense-in-depth 安全**（OpenDev + Claude Code 共同点） —— 多层防护
5. **Scaffolding vs Harness 分离**（OpenDev 概念） —— 配置与运行时解耦

### 4.3 与 mask 系统的对应

| mask Epic | 对应 harness 能力 | 状态 |
|-----------|------------------|------|
| Epic 1: mask 管理 | Scaffolding (OpenDev) | ✅ 已实现 |
| Epic 2: 安全能力模式 | Defense-in-depth | 🟡 部分实现 |
| Epic 3: 并行 sub-agent | Supervisor + 任务总线 (Hermes) | 🟡 实现中 |
| 待规划: Context Compaction | 5 阶段 Adaptive Compaction (OpenDev) | ✅ 已有 commit |
| 待规划: 任务持久化 | Kanban (Hermes) | ❌ 未开始 |
| 待规划: 通用 Hook 系统 | Hook 强制执行 (Claude Code) | ❌ P0 差距 |

---

## 五、推荐参考清单

按优先级排序，**推荐 agent-diva 重点参考**：

### 5.1 必读（高优先级）

1. **OpenDev 论文**（arXiv 2603.05344）
   - 价值：长期会话、5 阶段 compaction、defense-in-depth
   - 行动：将 5 阶段 compaction 模型对齐到当前 `feat(compaction)` 系列
   - 文档：`docs/dev/awesomeagents/opendev-paper-notes.md`（待创建）

2. **Hermes Kanban 源码**（github.com/NousResearch/hermes-agent）
   - 价值：持久化任务总线设计
   - 行动：mask Epic 3 之后，下一步实现任务持久化时参考
   - 文档：`docs/dev/awesomeagents/hermes-kanban-design-notes.md`（待创建）

3. **Claude Code Hook 语义**（Anthropic 公开文档）
   - 价值：PreToolUse/PostToolUse 强制拦截
   - 行动：填补 `openharness-vs-diva-pro-gap-analysis.md` 中 Plugin/Hook 的 P0 差距
   - 已有：当前只有 file/plan hook

### 5.2 可读（中优先级）

4. **Anthropic Harness Design for Long-Running Apps**（官方博客）
   - 价值：长期运行 agent 的设计原则
   - 行动：作为 mask 系统 + context compaction 的设计参考

5. **LangGraph Orchestrator-Worker 模式**
   - 价值：条件边、状态持久化
   - 行动：mask 的 `AgentMode` 切换可参考此模式

6. **Kimi K2.6 Agent Swarm 解读**
   - 价值：理解"零基础设施并行"的代价
   - 行动：确认"审计能力"是 agent-diva 不能放弃的差异化点

### 5.3 不必深入

- OpenAI Agents SDK / Swarm：与 OpenHarness Swarm 同思路，已暂缓
- AutoGen GroupChat：与 agent-diva 产品形态不符
- CrewAI 角色模型：与 mask 的 `MaskConfig` 设计重叠

---

## 六、对当前 P0 差距的修订

`openharness-vs-diva-pro-gap-analysis.md` 当前的 P0 缺口：

| ~~P0~~ Multi-Agent Swarm | ⏸️ **已暂缓** |
| **P0** Plugin/Hook 系统 | 🔴 仍是 P0，建议参考 **Claude Code Hook 语义** |
| **P1** Background Task Manager | 🟡 建议参考 **Hermes Kanban** 任务总线 |
| **P1** Coordinator 模式 | 🟡 建议参考 **LangGraph Orchestrator-Worker** |
| **P2** LSP 集成 | 🟡 暂不变 |
| **P2** Auth 框架 | 🟡 暂不变 |
| **P3** 其他 | 🟡 暂不变 |

**新增候选 P1**（基于本调研）：

| 候选 | 来源 | 理由 |
|------|------|------|
| Context Compaction 5 阶段对齐 | OpenDev 论文 | 当前 compaction 系列可升级 |
| 任务持久化总线 | Hermes Kanban | mask Epic 3 的下一步 |

---

## 七、行动建议

### 7.1 立即可做（1 周内）

1. 创建本调研的姊妹文档：
   - `docs/dev/awesomeagents/opendev-paper-notes.md` — 5 阶段 compaction 详细解读
   - `docs/dev/awesomeagents/hermes-kanban-design-notes.md` — Kanban 持久化设计

2. 与 mask Epic 3 团队对齐：
   - 是否将"任务状态机 + 持久化"纳入 Epic 3 范围？
   - 如果是，本调研的 Hermes Kanban 章节作为输入

### 7.2 短期（1-2 月）

3. 启动 OpenDev 5 阶段 compaction 的对齐工作：
   - 当前 `feat(compaction): support multi-compaction chain` (CC-P5) 已部分覆盖
   - 下一步可加：priority scoring、loss validation

4. 启动 Hook 系统 P0 工作：
   - 参考 Claude Code PreToolUse/PostToolUse 语义
   - 设计 mask-aware hook 拦截层

### 7.3 中期（3-6 月）

5. 实现 Hermes 风格任务总线：
   - 复用现有 SQLite 基础设施
   - 给用户可见的并行任务进度

---

## 八、结论

### 核心结论

1. **OpenHarness Swarm 暂缓是正确的** —— 2026 年主流"Agent Swarm"实质是 supervisor + 工作流，与 agent-diva 决策一致。
2. **更有价值的参考是 OpenDev + Hermes + Claude Code**：
   - OpenDev：长期会话、5 阶段 compaction
   - Hermes：任务总线、跨 profile 持久化
   - Claude Code：Hook 强制执行、子 agent firewall
3. **P0 缺口从 2 个收敛为 1 个** —— Multi-Agent Swarm 暂缓后，Plugin/Hook 是唯一 P0。
4. **新增候选 P1** —— 任务持久化总线、Context Compaction 升级。

### 一句话总结

> **不走 OpenHarness Swarm 的路，转向 OpenDev + Hermes + Claude Code 三件套。** 这三者覆盖了 agent-diva 真正需要补齐的能力：长期会话、任务持久化、Hook 强制执行。

---

## 九、参考文档

| 文档 | 路径 |
|------|------|
| OpenHarness 深度调研 | `docs/dev/awesomeagents/openharness-analysis.md` |
| OpenHarness vs agent-diva 差距分析 | `docs/dev/awesomeagents/openharness-vs-diva-pro-gap-analysis.md` |
| Swarm 分支接入可行性 | `docs/dev/awesomeagents/swarm-branch-integration-feasibility.md` |
| 能力清单 | `docs/dev/awesomeagents/diva-capability-checklist.md` |
| 演进路线 | `docs/dev/awesomeagents/evolution-roadmap.md` |
| 未知缺陷分析 | `docs/dev/awesomeagents/unknown-deficits.md` |
| 7 项目对比决策记录 | `docs/dev/awesomeagents/decisions.md` |

---

> 生成日期：2026-06-11
> 调研人：松本 (Hermes Agent)
> 状态：已完成，待用户确认后续行动
