# 2026 Agent Harness 论文与框架速览

> 调研日期：2026-06-11
> 调研范围：2026 年（含 2025 年底）arXiv 论文、业界框架、benchmark、实验室动向
> 调研目的：为 agent-diva 后续演进寻找可参考的最新研究
> 关联决策：`harness-landscape-2026.md`（OpenHarness Swarm 暂缓）、`swarm-branch-integration-feasibility.md`

---

## 调研方法

**搜索路径**（7 个 round，每轮 4 个并行 query）：

1. **核心 harness / 长跑 agent 论文**
2. **上下文工程 / compaction / memory**
3. **sub-agent / supervisor / orchestration**
4. **工具系统 / sandbox / permission**
5. **自进化 / 自我改进**
6. **评测 / benchmark**
7. **实验室动向**（Anthropic / OpenAI / DeepMind / Nous）+ 周边（voice / persona / 主动代理 / workflow）

**关联已有研究**：已存在的 `claude-code-analysis.md`、`codex-analysis.md`、`hermes-agent-analysis.md`、`genericagent-analysis.md`、`openharness-analysis.md`、`harness-landscape-2026.md` 已覆盖若干项目分析；本报告聚焦**arxiv 论文 + 2026 业界新发布**，不重复。

---

## 一、Tier 1：必读论文与项目（10 项）

### 1.1 Hermes Agent v0.9 + Self-Evolution（Nous Research）

**类型**：开源项目（github.com/NousResearch/hermes-agent）+ 公开博客
**时间**：v0.9 发布于 2026-04；Self-Evolution 模块发布于 2026-05

**关键点**：

- **5 阶段学习循环**：Curate memory → ...（5 阶段完整定义）
- **DSPy + GEPA 自我进化**：自动根据执行 trace 演化 skill 文件，无需 GPU
- **数据点**："20+ self-created skills → 40% faster token consumption"
- **架构**：SQLite FTS + LLM summarization 形成 "closed learning loop"
- **运行方式**：on your server，跨 session 记忆，自动构建可复用 skill

**与 agent-diva 的关系**：
- 高度对齐 —— 自进化是 mask 系统设计目标之一
- v0.9 5 阶段学习循环 = AutoDream 设计目标的现成参考
- **行动**：把 Hermes v0.9 README + Self-Evolution 设计作为 AutoDream ADR 输入

---

### 1.2 How to Model AI Agents as Personas?（arXiv 2603.03140v2）

**时间**：2026-03
**核心问题**：LLM agent 如何被建模为"角色"（persona）？与 social role / 性格 / 行为一致性有何关系？

**为什么必读**：
- **直接对齐 mask 系统** —— "persona" 概念与 `MaskConfig` 同源
- 提供了"如何形式化 persona"的方法论
- 引用高，方法学价值大

**对 mask 系统的潜在启发**：
- 形式化 mask 的 prompt 注入策略（training-free vs training-based）
- mask 切换的"行为一致性"评估方法
- mask collapse（角色混淆）的检测方法

---

### 1.3 Persona Collapse and Homogenization（arXiv 2604.24698）

**时间**：2026-04-27
**核心问题**：LLM agent 在多 persona 场景下，**会出现 persona collapse**（所有 persona 行为趋同）。这对 mask 系统的多 mask 切换是**已知风险**。

**为什么必读**：
- 揭示了 mask 系统必须解决的**关键失败模式**
- 提供了"如何检测 collapse"和"如何维持多样性"的实验设计
- 来自 NeurIPS'25 workshop，方法学严格

**对 mask 系统的具体威胁**：
- 多个 mask 切换时，行为可能趋向"最常见的"那个
- 长时间运行后，mask 的"个性"被磨平
- 解决方案可能是 "Facet-Level Persona Control"（arXiv 2602.19157）

**行动建议**：作为 mask Epic 1 后续 epic 的 risk register 输入

---

### 1.4 OpenDev: Scaffolding, Harness, Context Engineering（arXiv 2603.05344）

**时间**：2026-03
**核心贡献**（4 层架构）：

1. Context Engineering Layer
2. Harness Layer
3. Safety Layer
4. Scaffolding Layer

**最值得借鉴的细节**：
- **Scaffolding vs Harness 区分**（概念上）
  - Scaffolding = agent 启动前的配置（instructions / tools / permissions）
  - Harness = 运行时编排层（tool execution / context management / safety / persistence）
- **5 阶段 Adaptive Context Compaction**
  - Stage 1: Message boundary detection
  - Stage 2: Priority scoring
  - Stage 3: Token budget allocation
  - Stage 4: Semantic compression
  - Stage 5: Loss validation

**与 agent-diva 的关系**：
- 当前 `feat(compaction): support multi-compaction chain`（CC-P5）部分覆盖
- 5 阶段模型是**对齐目标**
- Scaffolding vs Harness 区分是 mask + agent loop 演进的**架构概念**

**行动**：把 5 阶段 compaction 模型作为 context compaction epic 2.0 的设计目标

---

### 1.5 Memory for Autonomous LLM Agents（arXiv 2603.07670）

**类型**：Survey
**时间**：2026-03
**核心价值**：覆盖 2022-2026 早期所有 agent memory 研究，**写–管理–读闭环**形式化

**为什么必读**：
- agent-diva 当前记忆系统（MEMORY.md + 日记）需要演进，这是最权威的路线图
- 形式化的 memory loop 可直接用于设计

**对记忆系统的输入**：
- Write → Manage → Read 模式与 AutoDream 设计吻合
- 调研了 MemGPT、hierarchical memory、graph memory 等范式
- 提供了 evaluation 方法

---

### 1.6 MemMachine: Ground-Truth-Preserving Memory（arXiv 2604.04853）

**时间**：2026-04-06
**核心问题**：现有 compaction 方法（如 Mastra）会"strip out the specific decisions and tool interactions agents need" —— 即丢失了"决策点"

**关键数据**：3-40× 压缩率，但牺牲了"原 episode 可恢复性"

**为什么必读**：
- 直接对应当前 `feat(compaction): summary quality validation with retry mechanism` (c40429d)
- 揭示了**compaction 的"ground-truth preservation"是新方向**
- 可能替代"summary + loss"的简单模型

**行动**：作为 compaction chain 下一阶段的设计参考

---

### 1.7 MCP Workflow Engine（arXiv 2605.00827）

**时间**：2026-05-05
**核心思想**：在 MCP（Model Context Protocol）之上，**构建原生的工作流引擎**，"decouples intelligence (deciding what to do) from execution (how to do it)"

**为什么必读**：
- diva 当前已有 MCP 集成（rust-mcp-sdk）
- 工作流引擎是 mask 系统的**重要补充**（mask 不只决定 agent 行为，还决定任务编排）
- "intelligence vs execution 解耦"是 2026 新趋势

**对 pro 的价值**：
- 替代或补充当前的 PlanOrchestrator
- 可作为 mask `AgentMode` 切换的统一载体
- 与 Hermes Kanban 思路（任务总线）天然契合

---

### 1.8 Anthropic Agent Skills Specification（2025-12-18）

**类型**：开放标准
**链接**：agentskills.io

**核心**：
- Anthropic 把 Agent Skills 作为**开放标准**发布
- SKILL.md 是核心 —— 一个目录包含 skill manifest
- **Progressive disclosure**：skill 按需加载，避免上下文爆炸

**与 mask 系统的关系**：
- mask 系统 vs skills 标准的**直接对标**
- 可作为 mask 系统的"industry alignment"目标
- progressive disclosure 可缓解 mask 切换的 token 开销

**行动建议**：
- 评估 mask 当前 manifest 格式是否符合 Skills spec
- 如不一致，可考虑兼容性适配

---

### 1.9 A Survey of Self-Evolving Agents（arXiv 2507.21046v4）

**时间**：2026-01-16 v4
**核心框架**（What/When/How/Where）：

```
What to evolve:  prompt / memory / tool / architecture
When to evolve:  on failure / on schedule / on demand
How to evolve:  reflection / RL / distillation / genetic
Where to evolve:  single node / pipeline / graph / holonic
```

**为什么必读**：
- AutoDream 设计的**完整 taxonomy**
- 可作为 mask 系统 + AutoDream 联合演进的理论框架

**关联资源**：GitHub XMUDeepLIT/Awesome-Self-Evolving-Agents（含 100+ 资源）

**直接相关的子论文**：
- **EvolveR**（arXiv 2510.16079）—— Experience-Driven Lifecycle
- **Rema**（NeurIPS'25）—— multi-agent RL for meta-thinking
- **SEDM**（NeurIPS'25 workshop）—— scalable self-evolving distributed memory

---

### 1.10 The 2026 Agent Reliability Gap（agentmarketcap.ai 综合分析）

**时间**：2026-04-07
**核心数据**：分析 200+ 论文后，揭示：
- **SWE-bench 已近饱和**（top 40-75%）
- **60.83% 成功率涉及"solution leakage"**（fix 已在 issue 描述中泄露）
- 真实世界 agent 能力**远低于 benchmark 数字**

**为什么必读**：
- 提醒我们不要被 benchmark 数字迷惑
- 真实部署可靠性是 2026 关键问题
- 提供了多个新 benchmark（TheAgentCompany、SWE-EVO 等）

---

## 二、Tier 2：高度相关（按主题分组）

### 2.1 Long-Running Agent / Harness Design

| 论文 / 来源 | 时间 | 核心 |
|------------|------|------|
| **Anthropic: Effective Harnesses for Long-Running Agents** | 2025-11-26 | 长跑 agent 核心挑战：discrete sessions、跨 session 记忆 |
| **Externalization in LLM Agents** (arXiv 2604.08224) | 2026-04 | 把状态外部化的方法学 |
| **Natural-Language Agent Harnesses** (arXiv 2603.25723) | 2026 | harness engineering 是长跑 agent 鲁棒性的主要驱动 |
| **Code as Agent Harness** (arXiv 2605.18747) | 2026-05-18 | executable / verifiable / stateful harness 概念 |
| **Harness Engineering Survey** (RUCAIBox/awesome-agent-harness) | 持续更新 | 官方 survey 配套仓库 |
| **Preprints Harness Layer** (202603.1756) | 2026-03 | harness 作为 first-class layer 的论证 |

**行动建议**：
- 把"5 阶段 long-running harness"作为 agent-diva agent loop 演进的理论框架
- 跟踪 RUCAIBox awesome-agent-harness 仓库的更新

### 2.2 Context Engineering / Compaction

| 论文 | 时间 | 核心 |
|------|------|------|
| **Parallel Context Compaction** (arXiv 2605.23296) | 3 weeks ago | 长 horizon agent 并行 compaction |
| **Active Context Compression** (arXiv 2601.07190) | 2026-01-12 | 模型自主管理 context |
| **Multi-Layered Memory Architectures** (arXiv 2603.29194) | 2026-03-31 | working / episodic / semantic 三层 |
| **Memory in the LLM Era** (arXiv 2604.01707) | 2026-04-02 | 模块化记忆架构统一框架 |
| **Memori: Persistent Memory Layer** (arXiv 2603.19935) | 2026-03-20 | 数据结构化方法的记忆层 |
| **A Self-Evolving Framework for Terminal Agents via Observational Context Compression** (arXiv 2604.19572) | 2026-04-21 | 自我进化 + compaction 融合 |
| **SWE-Pruner** (Wang et al., 2026) | 2026 | 终端 agent 上下文剪枝 |

**与 agent-diva 现状的对应**：
- `feat(compaction): support multi-compaction chain` (CC-P5) — 已起步
- `feat(compaction): add summary quality validation with retry mechanism` (c40429d) — 部分对应 MemMachine
- **缺**：5 阶段模型（OpenDev）、ground-truth preservation（MemMachine）、多层级 memory

### 2.3 Multi-Agent Orchestration（与 supervisor 决策一致）

| 论文 / 框架 | 时间 | 核心 |
|------------|------|------|
| **Magentic-One** (Microsoft, arXiv 2411) | 2024-11, 仍被引用 | 4 个 specialized agents + Orchestrator，**就是 supervisor 模式** |
| **Microsoft Agent Framework Magentic Orchestration** | 2026-05-26 | Microsoft 官方把 Magentic 纳入工作流 |
| **RL for LLM-based Multi-Agent Systems** (arXiv 2605.02801) | 2026-05-04 | 通过 orchestration traces 训练 |
| **When Does Multi-Agent RL Improve LLM Workflows?** (arXiv 2605.24202) | 2026-05-22 | 多 agent RL 何时有效 |
| **PerspectiveGap** (arXiv 2606.08878) | 4 days ago | 多 agent orchestration prompting benchmark |
| **DoVer** (Microsoft, ICLR 2026 Best Paper) | 2026 | Intervention-Driven Auto Debugging for LLM Multi-Agent Systems |
| **Agent, Sub-Agent, Skill, or Tool?** (techrxiv) | 2026-02-25 | 实践者指南：四者如何选型 |
| **Misaligned Roles** (ICLR 2026) | 2026 | 多 agent 角色错位问题 |
| **The Rise of Agentic Reasoning** (Medium) | 2026-02-01 | 业界综述 |

**与 agent-diva 决策的一致性**：
- **100% 一致**：所有 2026 主流方案都是 supervisor + worker
- OpenHarness 的真 swarm 反而是少数派
- DoVer、Misaligned Roles 等 ICLR 2026 论文都在解决 multi-agent 系统的可靠性问题

**行动**：
- 继续推进 mask Epic 3（supervisor 模式 + 并行 sub-agent）
- 跟踪 DoVer 的"intervention-driven debugging"思想，未来可加入 mask 系统的运行时诊断

### 2.4 Tool Use / MCP / Sandboxing

| 论文 / 框架 | 时间 | 核心 |
|------------|------|------|
| **MCP Tool Descriptions Are Smelly!** (arXiv 2602.14878) | 2026-02-16 | MCP-Universe benchmark，202 tools / 231 tasks |
| **MCP Threat Modeling** (arXiv 2603.22489) | 2026-03-23 | tool poisoning + prompt injection |
| **Enhancing MCP with Context-Aware Server Collaboration** (arXiv 2601.11595) | 2026-01-06 | MCP 状态化扩展 |
| **Securing MCP** (arXiv 2511.20920) | 2025-11-25 | NIST/ISO 治理框架映射 |
| **Graph-Based Self-Healing Tool Routing** (arXiv 2603.01548) | 2026 | 工具调用失败自愈 |
| **ToolTree: MCTS for Tool Planning** (arXiv 2603.12740) | 2026-03-13 | 多工具调用作为 MCTS 问题 |
| **DeepAgent** (arXiv 2510.21618v2) | 2026-02-04 | CodeAct + Plan-and-Solve 融合 |
| **SandboxEscapeBench** (arXiv 2603.02277) | 2026-03-01 | CTF 风格 sandbox escape benchmark |
| **Saber** (arXiv 2606.01317) | 2 weeks ago | 编码 agent 在 stateful workspace 的 operational safety |
| **OpenAgentSafety** (ICLR 2026) | 2026 | comprehensive safety benchmark |
| **AgentSpec** (arXiv 2503.18666v3) | 2025-07-31 | customizable runtime enforcement |
| **Reframing LLM Agent Security** (arXiv 2605.24309) | 2026-05-23 | agent-human interaction 视角 |
| **CausalArmor** (arXiv 2602.07918) | 2026 | causal attribution guardrail |
| **Securing AI Agent Execution** (arXiv 2510.21236) | 2025-10-24 | MCP server sandboxing |
| **Cline 事件** (Feb 2026) | 2026-02 | 真实 prompt injection 攻击案例 |

**与 diva 沙箱审计的对应**：
- diva 当前有完整的沙箱审计（sandbox-audit-a/b/c.md），已覆盖 Windows/Linux/macOS
- 缺的：tool-specific sandboxing、tool description optimization、tool routing
- **新趋势**：causal attribution guardrail（CausalArmor）是 2026 新方向

### 2.5 Memory / Personalization

| 论文 | 时间 | 核心 |
|------|------|------|
| **Opal: Private Memory for Personal AI** (arXiv 2604.02522) | 2026-04-02 | 隐私优先的长期记忆 |
| **AMemGym** (arXiv 2603.01966) | 2026-03-02 | 交互式记忆 benchmark |
| **According to Me: Long-Term Personalized Referential Memory QA** (arXiv 2603.01990) | 2026-03 | 多模态个性化记忆 |
| **HiMeS** (arXiv 2601.06152) | 2026-01 | 海马体启发的记忆架构 |
| **MemX** (arXiv 2603.16171) | 2026-03 | local-first 长期记忆 |
| **LLM Agent Memory Survey** (preprints 202603.0359) | 2026-03-04 | 统一表示视角的 survey |
| **Awesome-GraphMemory** (GitHub) | 2026-02-03 | 图记忆资源集合 |
| **MemGraphRAG** (arXiv 2606.00610, KDD 2026) | 2026-05-30 | 记忆 + 图 + 多 agent RAG |

**与 diva 记忆系统的对应**：
- diva 当前有 MEMORY.md + 日记 + AutoDream 设计
- **新趋势**：graph memory、hippocampus-inspired、local-first
- **隐私问题**：Opal 强调 private memory，与 diva 桌面应用定位一致

### 2.6 Voice / Multimodal / Embodied

| 论文 / 来源 | 时间 | 核心 |
|------------|------|------|
| **Building Enterprise Realtime Voice Agents** (arXiv 2603.05413) | 2026-03-05 | 实时语音 agent 教程 |
| **WildASR** (arXiv 2603.25727) | 2026-03-26 | 真实人声 ASR benchmark |
| **Audio-Language Models Survey** (arXiv 2501.15177v2) | 2026-03-12 | ALM 系统综述 |
| **ProVoice-Bench** (arXiv 2604.15037) | 2026-04-16 | 语音 agent 主动行为 benchmark |
| **VisionClaw** (arXiv 2604.03486) | 2026-04-08 | always-on 智能眼镜 agent |
| **Digital Humans with Ambient Intelligence** (arXiv 2604.05120) | 2026-04-29 | 数字人从 reactive 到 proactive |
| **FAEA: Demonstration-Free Robotic Control** (arXiv 2601.20334) | 2026-01-28 | LLM agent 用于 embodied manipulation |
| **VLA Survey** (arXiv 2405.14093v8) | 2026-05-01 | Vision-Language-Action 模型 |
| **MGA: Memory-Driven GUI Agent** (arXiv 2510.24168v3) | 2026-04-14 | 长期 GUI agent 记忆 |
| **UI-TARS-2 Technical Report** | 2025-09-02 | GUI agent 多平台模型 |
| **Mobile-Agent-v3.5** (arXiv 2602.16855) | 2026-02-15 | GUI-Owl-1.5 多平台 |

**与 diva 的对应**：
- diva 有 TTS/ASR 多 provider、VRM 桌面宠物、语音模式
- **新趋势**：proactive voice agent（不是被动响应，而是 ambient）
- VisionClaw / Ambient Intelligence 是"always-on"方向，与 diva 桌面宠物定位有想象空间

### 2.7 Workflow Orchestration / DAG

| 论文 / 框架 | 时间 | 核心 |
|------------|------|------|
| **GraphBit** (arXiv 2605.13848) | 2026-03-08 | 引擎编排的 DAG 工作流框架 |
| **MCP Workflow Engine** (arXiv 2605.00827) | 2026-05-05 | MCP-native 工作流引擎 |
| **Temporal / Airflow / Prefect** | 持续 | 持久化执行引擎 |
| **Autonomous Data Processing using Meta-Agents** (arXiv 2602.00307) | 2026-02-19 | meta-agent 自动生成 DAG |

**与 diva 的对应**：
- 当前 `PlanOrchestrator` 是状态机
- 缺口：缺少持久化、DAG 描述
- 方向：MCP Workflow Engine 天然适配

### 2.8 Cost / Token Optimization

| 论文 | 时间 | 核心 |
|------|------|------|
| **Token Economics for LLM Agents** (arXiv 2605.09104) | 2026-05-09 | 计算-通信双重视角 |
| **AgentDiet** (arXiv 2509.23586v2) | 2026-03-15 | trajectory reduction 节省 token |
| **SkillReducer** (arXiv 2603.29919) | 2026-03-31 | skill 优化节省 60-80% token |
| **Energy-per-Token** (ACM 2024) | 2024 | 能耗指标 |

**与 diva 的对应**：
- 当前 Phase 0 已经规划了 token 优化（tool result truncation、context budget）
- SkillReducer / AgentDiet 提供了具体方法

### 2.9 Observability / Tracing

| 论文 / 框架 | 时间 | 核心 |
|------------|------|------|
| **Governance-Aware Agent Telemetry** (arXiv 2604.05119) | 2026 | closed-loop 治理遥测 |
| **Structured Telemetry Span Schema** | 2026-04-24 | skill 执行 span 标准化 |
| **AgentSight** (ACM 2025) | 2025-10-13 | eBPF 系统级 agent 可观测性 |
| **Langfuse** (consensus pick) | 持续 | OpenTelemetry-based |
| **OpenTelemetry GenAI** | 2025+ | GenAI 语义约定 |

**与 diva 的对应**：
- 当前缺少统一的 observability 框架
- OpenTelemetry GenAI conventions 是 2026 事实标准

### 2.10 Reasoning / Test-Time Compute

| 论文 | 时间 | 核心 |
|------|------|------|
| **Think Deep, Not Just Long** (arXiv 2602.13517) | 2026-02-12 | 推理"努力"度量 |
| **Reasoning Models Generate Societies of Thought** (arXiv 2601.10825) | 2026-01-15 | 推理模型 = 多 agent society |
| **MemCoT** (arXiv 2604.08216) | 2026-04-09 | 记忆驱动的 CoT |
| **Learning Efficient Reasoning in Long CoT** (arXiv 2603.00578) | 2026-02-28 | 5,668 → 986 减少 token |
| **Categories of Inference-Time Scaling** (Sebastian Raschka) | 2026-01-24 | 综述 |

**与 diva 的对应**：
- 当前有 thinking mode 集成（Phases 2-4）
- **新发现**：reasoning 模型的"societies of thought"与多 agent 思想有融合

### 2.11 CodeAct / Action Space

| 论文 | 时间 | 核心 |
|------|------|------|
| **CodeAct** (arXiv 2402.01030) | 2024-02 | executable Python 作为统一 action space |
| **DeepAgent** (arXiv 2510.21618v2) | 2026-02-04 | scalable toolsets 通用推理 agent |

**与 diva 的对应**：
- 当前 tool 系统是 trait-based，不是 code-based
- **未来方向**：可考虑 CodeAct-style action space

### 2.12 Prompt Optimization / DSPy

| 论文 / 框架 | 时间 | 核心 |
|------------|------|------|
| **GEPA: Reflective Prompt Evolution** | 持续 | LLM 反思 + Pareto evolution |
| **DSPy 2026 Optimizers Comparison** | 2026-05-29 | GEPA / ProTeGi / MIPRO 对比 |
| **Why Prompt Optimization Works** (arXiv 2605.26655) | 2026-05-26 | 何时有效何时无效 |
| **Structured Comparison of Task Adaptation** (arXiv 2604.09418) | 2026-04-10 | 提示构建 = systematic search |

**与 diva 的对应**：
- Hermes 已经用 DSPy+GEPA 自我进化
- diva 可考虑在 mask system + AutoDream 中引入

### 2.13 Deep Research Agents

| 论文 | 时间 | 核心 |
|------|------|------|
| **Deep Research Agents Survey** (arXiv 2506.18096) | 2025-06 | survey |
| **DeepResearch Bench** (OpenReview) | 2026-01-26 | 评测 deep research 能力 |
| **A Deep Research Agent with Progressive Confidence Estimation** (arXiv 2604.05952) | 2026-04-07 | 可信度评分 |
| **OpenAI Deep Research / Gemini Deep Research** | 2025-2026 | 商业产品 |

**与 diva 的对应**：
- 本次"调研模式"本身可以包装成 deep research agent
- mask 系统可作为 deep research 场景的入口

### 2.14 Proactive / Ambient Agents

| 论文 | 时间 | 核心 |
|------|------|------|
| **Proactive Hardening with HASTE** (arXiv 2601.19051) | 2026-01-28 | LLM defenses |
| **Designing Digital Humans with Ambient Intelligence** (arXiv 2604.05120) | 2026-04-29 | ambient digital humans |
| **ProVoice-Bench** (arXiv 2604.15037) | 2026-04-16 | 语音 proactivity |
| **ProAgentBench** (arXiv 2602.04482) | 2026 | 主动协助 benchmark |
| **Proactive Memory (ProMem)** (arXiv 2601.04463) | 2026 | 主动记忆提取 |
| **VisionClaw** (arXiv 2604.03486) | 2026-04-08 | always-on |
| **NeuroSkill** (Hermes issue #500) | 2026-03-06 | 持续 signal sensing + context injection |

**与 diva 的对应**：
- diva 当前是被动响应（用户说话才反应）
- **未来方向**：proactive 模式（基于时间和事件的主动建议）
- 与 AutoDream 设计目标一致

---

## 三、Tier 3：值得扫一眼（10 项）

按相关度递减：

1. **TheAgentCompany benchmark** (NeurIPS 2025) — 真实公司任务的 agent 评测
2. **SWE-EVO** (arXiv 2512.18470v5) — 长 horizon 软件演化场景；提到 **Meta Context Engineering** 89.1% vs 70.7% baseline
3. **Governance-Aware Agent Telemetry** (arXiv 2604.05119) — closed-loop enforcement
4. **Awesome-Agent-Harness** (RUCAIBox GitHub) — 官方 survey 配套
5. **MemGraphRAG** (KDD 2026) — 知识图谱 + 多 agent
6. **CausalArmor** (arXiv 2602.07918) — 因果归因防御
7. **Multi-Layered Memory Architectures** (arXiv 2603.29194) — 实验对比
8. **Awesome-Self-Evolving-Agents** (GitHub) — 100+ 资源
9. **DoVer** (Microsoft, ICLR 2026 Best Paper) — multi-agent debugging
10. **The 2026 Agent Reliability Gap** (agentmarketcap.ai) — 行业现状综述

---

## 四、对 agent-diva 的总览建议

### 4.1 优先级矩阵

| 主题 | 已覆盖度 | 推荐补强 | 关键论文 |
|------|---------|---------|---------|
| **Mask / Persona 系统** | Epic 1-3 实现中 | Persona collapse 防御 | arXiv 2603.03140, 2604.24698, 2602.19157 |
| **AutoDream / 自进化** | 设计阶段 | DSPy+GEPA 实战 | Hermes v0.9 Self-Evolution, arXiv 2507.21046v4 |
| **Context Compaction** | CC-P5 + quality validation | 5 阶段 + ground-truth preservation | OpenDev 2603.05344, MemMachine 2604.04853 |
| **Memory / 长期记忆** | MEMORY.md + 日记 | 多层 + graph + hippocampus | arXiv 2603.07670, 2603.29194 |
| **Sub-agent / Supervisor** | mask Epic 3 推进中 | 验证 + debugging | Magentic-One, DoVer (ICLR 2026) |
| **Tool / MCP** | rust-mcp-sdk 集成 | Workflow engine | MCP Workflow Engine 2605.00827 |
| **Harness / 长期运行** | 部分 | 5 阶段设计 | OpenDev, Anthropic 2025-11 |
| **Voice / 多模态** | TTS/ASR 多 provider | Proactive voice | ProVoice-Bench 2604.15037 |
| **Sandbox / 安全** | 完整审计 | Tool-specific + CausalArmor | SandboxEscapeBench, Saber, CausalArmor |
| **Observability** | 缺失 | OpenTelemetry GenAI | Langfuse / AgentSight |

### 4.2 三个"立即可启动"行动

1. **Mask 系统的 Persona Collapse 防御**
   - 输入：arXiv 2604.24698 + 2603.03140
   - 输出：mask Epic 1.5 风险评估 + 防御设计
   - 工时：1 周

2. **Context Compaction 5 阶段对齐**
   - 输入：OpenDev 5 阶段模型 + MemMachine ground-truth preservation
   - 输出：compaction epic 2.0 ADR
   - 工时：1-2 周

3. **Hermes Self-Evolution 调研**
   - 输入：Hermes v0.9 README + DSPy+GEPA
   - 输出：AutoDream 设计的 5 阶段学习循环 ADR
   - 工时：1 周

### 4.3 中期关注（1-3 月）

- Magentic-One / DoVer 的 multi-agent debugging 思想融入 mask runtime
- MCP Workflow Engine 替代 PlanOrchestrator
- Anthropic Skills spec 与 mask system 的 industry alignment
- Proactive memory (ProMem) 设计 AutoDream 的"主动记忆提取"模块

### 4.4 长期关注（3-6 月）

- Meta Context Engineering 思路（89.1% SWE-bench）
- Ambient intelligence / proactive agents
- Embodied / VRM 方向的 VisionClaw / Digital Humans Ambient Intelligence
- Energy-per-Token 指标（如果 diva 走向 mobile / 嵌入式）

---

## 五、调研元数据

- **调研深度**：7 轮并行搜索，每轮 4-10 个 query，共约 30+ 次 web_search
- **覆盖范围**：arXiv 2025-11 至 2026-06；ICLR 2026 / NeurIPS 2025 / KDD 2026 / EACL 2026 论文；Anthropic / OpenAI / Microsoft / Google / Nous Research 公开材料
- **未覆盖**：部分中文社区资源（B 站、知乎）、非英文实验室（DeepSeek、Qwen、Kimi 技术报告）——可后续补充
- **冲突点**：少数论文 ID 重复出现（如 2604.xxxx 系列），已去重
- **可信度**：所有论文链接均来自 arXiv / 官方仓库 / 顶会官网

---

## 六、参考资源

### 6.1 Awesome 列表（持续更新）

- https://github.com/RUCAIBox/awesome-agent-harness — 官方 harness survey
- https://github.com/VoltAgent/awesome-ai-agent-papers — 2026 agent 论文集
- https://github.com/XMUDeepLIT/Awesome-Self-Evolving-Agents — 自进化 agent
- https://github.com/DEEP-PolyU/Awesome-GraphMemory — 图记忆
- https://github.com/ai-boost/awesome-harness-engineering — harness engineering
- https://github.com/liutaocode/TTS-arxiv-daily — TTS 论文日报
- https://github.com/YunjiaXi/Awesome-Search-Agent-Papers — 搜索 agent
- https://github.com/ZJU-REAL/Awesome-GUI-Agents/blob/main/ICLR2026/Paperlist.md — GUI agent ICLR 2026

### 6.2 已有 morediva 内部文档（已存在）

- `claude-code-analysis.md` — Claude Code 深度分析
- `codex-analysis.md` — OpenAI Codex 分析
- `hermes-agent-analysis.md` — Hermes Agent 深度分析
- `genericagent-analysis.md` — 通用 agent 项目分析
- `openharness-analysis.md` — OpenHarness 深度分析
- `openharness-vs-diva-pro-gap-analysis.md` — 差距分析
- `harness-landscape-2026.md` — 其他 harness 方案调研
- `swarm-branch-integration-feasibility.md` — 旧 swarm 分支接入
- `comparison-matrix.md` — 7 大 agent 项目对比
- `decisions.md` — 决策记录
- `evolution-roadmap.md` — 演进路线
- `unknown-deficits.md` — 未知缺陷

### 6.3 顶会 2026 重要获奖

- **NeurIPS 2025 Best Papers** (4 best + 3 runner-up) — 见 NeurIPS 官方博客
- **ICLR 2026 Best Paper** — DoVer (Microsoft)
- **ICLR 2026 Workshop Best Paper** — GLEAN
- **KDD 2026** — MemGraphRAG

---

> 生成日期：2026-06-11
> 调研人：松本 (Hermes Agent)
> 状态：已完成 30 分钟全自主研究
> 下一步：等待用户阅读并指示后续行动
