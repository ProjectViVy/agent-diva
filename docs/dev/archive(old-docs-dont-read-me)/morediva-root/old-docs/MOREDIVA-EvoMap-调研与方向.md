# MOREDIVA — EvoMap 调研与接入方向

> **日期**: 2026-06-07
> **来源任务**: 用户提问「研究 EvoMap/evolver + 调研 Hermes 接入必要性」
> **约束**: 中文，紧凑表格，方案文档 ≤ 200 行

---

## 1. 调研背景

[EvoMap/evolver](https://github.com/EvoMap/evolver) 是当前公开领域最活跃的"AI agent 自进化"基础设施，核心引擎基于 **GEP（Genome Evolution Protocol）**。网站 [evomap.ai](https://evomap.ai)，理论依据论文 [arXiv:2604.15097](https://arxiv.org/abs/2604.15097)。已 8180+ stars、793+ forks。

**关键产品矩阵**:

| 项目 | 用途 | License | 状态 |
|---|---|---|---|
| `EvoMap/evolver` | 核心 CLI 引擎，扫描 memory/→选 Gene→输出 GEP prompt | GPL-3.0 | **已闭源**（commit 247 压到 3）|
| `EvoMap/gep-mcp-server` v1.7.0 | MCP 桥接包，暴露 `gep_evolve`/`gep_recall`/`gep_publish_bundle` 等 14 个工具 | **Apache-2.0** | 活跃维护 |
| `NousResearch/hermes-agent` | Hermes Agent 主仓（被指控抄袭 Evolver）| MIT | 3918 stars |
| `NousResearch/hermes-agent-self-evolution` | Hermes 自进化仓（创建 2026-03-09，比 Evolver 晚 5+ 周）| MIT | 449 forks |

---

## 2. Hermes ↔ Evolver 相似性分析（EvoMap 公开指控）

EvoMap 在 2026-04-11 发了详细分析文章，指控 Hermes "未引用即借鉴"。**逐概念映射**:

| 概念层 | Evolver（先发，2026-02-01）| Hermes（后发，2026-03-09）| 关系 |
|---|---|---|---|
| 经验策略 | **Gene** | **Skill** (SKILL.md) | 🔁 改名 |
| 验证过的修复 | **Capsule** | **Validated Skill** | 🔁 改名 |
| 审计事件 | **EvolutionEvent** | **trajectory_compressor.py** (65KB) + SessionDB | 🔁 改名 |
| 反思循环 | **reflection loop** | **reflection.js** | 🔁 同名 |
| 三层 memory | **Three-tier memory** | **narrativeMemory.js** + SessionDB + memory_manager | 🔁 同构 |
| 经验压缩 | **Gene compression** | **trajectory_compressor** | 🔁 同构 |
| 协作编辑 | **Solidify** | **Git PR via `pr_builder.py`** | 🔁 改名 |
| **底层优化器** | **自研 EVM（已闭源）** | **DSPy + GEPA**（Stanford 学术，ICLR 2026 Oral，MIT）| ✅ 不同 |

EvoMap 自己明确区分："**GEPA ≠ GEP**……本文分析的是 Hermes 在 GEPA 上层构建的架构——不是 GEPA 本身。"

**用户洞察的精准度**: 用户的类比——"**Hermes 改了个名字接入了原生的 mempalace**"——准确度约 85%。底层引擎确实换了（GEPA 是公开 MIT 库），但上层架构（Gene/Capsule/Reflection/3-tier memory）是逐项 1:1 改名。EvoMap 已从 MIT→GPL-3.0→闭源作为反击。

---

## 3. agent-diva 方向决策

### 3.1 ✅ 未来**原生支持 EvoMap**

不是浅层 MCP 桥接，而是 deeper integration：
- **目标**: agent-diva 节点可在 EvoMap Hub 上注册、发布 Gene/Capsule、接入 8180+ 用户网络
- **路径**:
  1. 装 `@evomap/gep-mcp-server` v1.7.0（Apache-2.0，合规）作为 MCP 桥
  2. 启动本地 Proxy（`127.0.0.1:19820`）
  3. `POST /a2a/hello` 注册节点 → 拿 `node_id` + `node_secret`
  4. 通过 `gep_publish_*` 把 diva 的进化资产发到 Hub（不绑定 Credits 经济，可选）
  5. 通过 `gep_recall` / `gep_search_community` 拉社区经验
- **可逆性**: Hub 注册是 opt-in，不注册完全不影响本地使用
- **许可证**: 我们的代码不引入 GPL（Evolver CLI 引擎避用，只用 Apache-2.0 的 MCP 包 + Hub HTTP API）

### 3.2 ✅ **保留自研 skill 进化系统**（核心护城河）

这是与 EvoMap 路线**根本性分叉**的地方。EvoMap 的 skill 进化是"LLM+GEPA 优化 SKILL.md 文本"；我们的进化是"**从工作过程中提取 skill**"。

**核心机制**: 周报 / 日报 / 月报 → 提取 skill
- 周期：日终（自动）/ 周终（半自动）/ 月终（人工审核）
- 提取源：session 日志、trajectory、memory_graph、user feedback
- 提取策略：跨周期 dedup + 置信度评分 + blast radius 计算（**思想借 GEP 论文，名字不撞**）
- 落地：写入 `skills/`, 注入 `MEMORY.md`/`HISTORY.md`, 更新 `agent-diva-core` 的 skill registry
- 优势：不依赖外部 Hub、不付 Credits、不卷优化器；走"**从实践中来**"路线

**为什么不直接走 GEP 路线**：
1. EvoMap 闭源 + 商业关系敏感（公开敌视 Hermes 生态）
2. 我们的 diva/diva-pro 已有的 `mempalace-rs` (BM25+SQLite) + Markdown memory 体系完整，迁移成本高
3. "周报提取 skill" 比"LLM 优化 prompt"更贴合大湿的工作流（嵌入式/自动化背景）
4. 命名上避开：不用 Gene/Capsule/EvolutionEvent/GDI/A2A 这些 EvoMap 商标相关词

### 3.3 设计边界白皮书（避嫌）

| 类别 | 我们用（合法）| 避开（避嫌）| 独创（护城河）|
|---|---|---|---|
| 范式思想 | 3-tier memory、反思循环、经验沉淀 | — | — |
| 资产命名 | Skill / MemoryEntry / HistoryEntry | Gene / Capsule / EvolutionEvent | — |
| 底层引擎 | 自研 / mempalace-rs | Evolver CLI | 周报提取 skill 流程 |
| 协作网络 | EvoMap Hub（opt-in, MCP 桥）| Evolution Circle / Guild / AI Council | diva-internal Kanban |
| 评分体系 | 自研 confidence + blast radius | GDI（35/30/20/15 权重）| 时序衰减 + 工作量归因 |
| 经验压缩 | 渐进式 trajectory distillation | — | "周报→skill" 半自动管线 |

---

## 4. 接入 PoC 计划（下一步）

如果大湿确认方向，建议：

1. **5 分钟连通性测试**: `npx @evomap/gep-mcp-server` 装好，`gep_recall` 跑通，不写任何资产到 Hub
2. **写一个 diva 适配层**: 把 EvoMap MCP 包挂到 diva 的 MCP 客户端，只暴露 read-only tools
3. **设计周报提取 skill 模块**: 在 `agent-diva-selfinprove` 下开新子 crate（参考其 `agent-diva-tooling` 模式）
4. **更新 MOREDIVA-待办汇总.md**: 把"原生支持 EvoMap"和"周报提取 skill"加入未来 roadmap

**前置澄清问题**（大湿拍板再动）：
- [ ] EvoMap Hub 注册要不要做？（涉及 node_secret 持久化、opt-in 流程）
- [ ] "原生支持" = MCP 桥接，还是想直接在 diva 内部实现 gep-a2a 协议？
- [ ] 周报提取 skill 的"周报"指 diva 自己生成的 LLM 总结，还是人工写的？
- [ ] 提取出的 skill 是仅本地使用，还是也想 publish 到 EvoMap Hub（两个系统的 skill 互通）？

---

## 5. 风险与备注

- **风险 1**: EvoMap 商业模式后续变动（已宣布转 source-available，未来可能更严）
- **风险 2**: 命名冲突——`mempalace` 在我们 diva 里是 Rust 实现（BM25+SQLite），在 EvoMap 文档里是 memory graph 的别称。两者**没关系**，但容易混淆
- **风险 3**: Hermes↔Evolver 的口水战可能波及"Hermes 生态"——我们用的是 NousResearch 的 Hermes，agent-diva 是独立项目，保持距离
- **后续**: 等大湿确认方向后，落到 `agent-diva-selfinprove/docs/` 或 `agent-diva-pro/CONTRIBUTING.md` 作为设计依据
