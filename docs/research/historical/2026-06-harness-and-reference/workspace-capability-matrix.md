# .workspace 参考项目 × agent-diva 综合能力矩阵

> **生成时间**: 2026-07-03  
> **调研范围**: `.workspace/` 下 13 个参考子项目 + agent-diva 主仓 baseline  
> **方法**: 只读源码与文档；14 个并行子任务分维度深读后合成；**analysis 项目为 docs-only，无运行时**  
> **关联研究**:  
> - [workspace-hooks-comparison.md](./workspace-hooks-comparison.md) — Hooks/Lifecycle 深读  
> - [workspace-subagent-comparison.md](./workspace-subagent-comparison.md) — Sub-agents 深读  
> - [harness-engineering-three-way-comparison.md](./harness-engineering-three-way-comparison.md) — Alife/Hermes/Diva 三方 Harness 对比  
> - [hermes-harness-overview.md](./hermes-harness-overview.md) — Hermes 生产级 Harness 能力盘点

---

## 1. Executive Summary

对 `.workspace` 13 个参考项目与 agent-diva baseline 在 15 个 Harness 维度横向对比后，对 **agent-diva 架构师** 最关键的五条结论：

1. **agent-diva 的定位是「平台 OS + 产品车道」，不是 coding-first harness**。Laputa 提案、AutoDream 人工监督运行、Mentle 记忆集成、Windows Service、Skill 安全审查等能力在参考项目中**无直接对标**；但在 Hooks、Sandbox 接线、OTEL、Swarm/DAG、Coding TUI 厚度上明显落后 pi/codex/claude-code 系。
2. **Hooks 是最大结构性缺口**。claude-code（27 事件）、hermes（`invoke_hook` 统一分发）、zeroclaw（typed `HookResult<T>`）、pi/oh-my-pi（ExtensionRunner typed reducer）已形成成熟范式；diva 仅有 `agent-diva-files` 域内 HookRegistry + `MemoryProvider` 四点 + planning HOOK-1，**无统一 agent lifecycle hooks 框架**。
3. **Sandbox 与 Security 存在「crate 有、路径未通」问题**。`agent-diva-sandbox` 已实现 execpolicy/rules/guardian，但 `ExecTool` 尚未接入；codex execpolicy、hermes 6 种 sandbox backend + 1751 行 approval、zeroclaw OS sandbox 均为可借鉴标杆。
4. **Channels 数量够用、厚度不足**。diva ~14 通道 vs openfang ~40、zeroclaw 30+、hermes 20+；diva 缺 hermes mirror 跨 gateway 协同与 openfang TriggerEngine 级触发编排。
5. **Sub-agent 停留在 bounded parallel，Swarm/DAG 为明确 gap**。diva `SubagentManager` + `DelegateTool`（JoinSet max 4, depth 3）对标 zeroclaw/hermes ③ Supervisor-Worker；claude-code/OpenHarness ④ Swarm+Mailbox、oh-my-pi/openfang ⑤ DAG/Wave 引擎均无对应实现（`feature-swarm-humanlike` 分支已 defer，仅作设计参考）。

---

## 2. 项目清单

| # | 项目 | 语言/运行时 | 定位一句话 | 分析方式 |
|---|------|------------|-----------|---------|
| 1 | **claude-code** | TypeScript | Anthropic 官方 coding agent；27 事件 hooks + Swarm/Task DAG | 源码深读 |
| 2 | **codex** | Rust | OpenAI coding agent；TOML hooks + execpolicy + OTEL | 源码深读 |
| 3 | **GenericAgent** | Python | 轻量通用 agent；SOP 驱动 subagent + 最简 hooks | 源码深读 |
| 4 | **hermes-agent** | Python | 生产级 Harness；approval/toolset/budget/16+ hooks/20+ 通道 | 源码深读 |
| 5 | **learn-claude-code** | Python | 教学渐进式实现；hooks/subagent/swarm/MCP 章节演示 | 源码深读 |
| 6 | **MaiMBot** | Python | QQ 人格 bot（NoneBot）；**非 harness 对标** | 源码深读 |
| 7 | **memtle** | Rust | 记忆/MCP 供应商；diva 通过 Mentle 集成，**非竞品** | 源码深读 |
| 8 | **oh-my-pi** | TypeScript | pi fork；ExtensionRunner + swarm-extension DAG + robomp 队列 | 源码深读 |
| 9 | **openfang** | Rust | 全栈 Agent OS；~40 通道 + Workflow DAG + KG 记忆 | 源码深读 |
| 10 | **OpenHarness** | Python | Claude Code 风格 harness；Swarm + mailbox + config hooks | 源码深读 |
| 11 | **pi** | TypeScript | Coding harness 标杆；TUI/LSP/DAP + ExtensionRunner hooks | 源码深读 |
| 12 | **zeroclaw** | Rust | Rust Agent OS；typed hooks + OS sandbox + 30+ 通道 | 源码深读 |
| 13 | **analysis** | — | Compaction 模式研究笔记；**docs-only，无运行时** | 文档-only |
| — | **agent-diva** | Rust (15 crates) | 平台 OS + Laputa/AutoDream/Plan Mode 产品车道 | baseline |

> **列名缩写**（下文矩阵用）：CC=claude-code, CX=codex, GA=GenericAgent, HM=hermes, LC=learn-claude-code, MB=MaiMBot, MT=memtle, OP=oh-my-pi, OF=openfang, OH=OpenHarness, PI=pi, ZC=zeroclaw, AN=analysis, **DV=agent-diva**

**图例**: ✅ 领先/完整 · 🔶 部分具备/中等 · ❌ 缺失或薄弱 · ➖ 不适用或无运行时

---

## 3. Master Capability Matrix（15 维度 × 14 项目）

| 维度 | CC | CX | GA | HM | LC | MB | MT | OP | OF | OH | PI | ZC | AN | **DV** |
|------|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|:--:|
| **1. Runtime/Language** | ✅ TS 生产 | ✅ Rust 生产 | 🔶 Py 轻量 | ✅ Py 生产 | 🔶 Py 教学 | 🔶 Py bot | 🔶 Rust crate | ✅ TS fork | ✅ Rust OS | 🔶 Py harness | ✅ TS 生产 | ✅ Rust OS | ➖ | ✅ Rust 15 crate |
| **2. Agent Loop** | ✅ 完整 turn | ✅ Rust loop | 🔶 简单 loop | ✅ 5 阶段+bg review | 🔶 章节 demo | 🔶 消息队列 | ➖ | ✅ + extensions | ✅ kernel loop | ✅ query engine | ✅ harness | ✅ turn pipeline | ➖ | 🔶 loop_turn 无 iteration budget |
| **3. Tool System** | ✅ builtin+plugin | ✅ spec+handlers | 🔶 反射注册 | ✅ toolset+check_fn | 🔶 教学工具 | ❌ 无 tool 框架 | ➖ MCP tools | ✅ + Codex 兼容 | ✅ manifest | ✅ YAML defs | ✅ LSP/DAP/edit | ✅ risk profile | ➖ | 🔶 ToolRegistry 无 toolset/check_fn |
| **4. Hooks/Lifecycle** | ✅ 27 事件 | 🔶 TOML 6 事件 | 🔶 @register 8 点 | ✅ invoke_hook 16+ | 🔶 4 事件 demo | ❌ 无 | 🔶 stop hook | ✅ ExtensionRunner | 🔶 4 Rust 点 | 🔶 10 Claude 事件 | ✅ typed reducer | ✅ HookResult\<T\> | ➖ | ❌ 无统一框架 |
| **5. Sub-agents** | ✅ Swarm+Task DAG | 🔶 Collab spawn | 🔶 SOP shell | 🔶 delegate+Kanban | ✅ s06/s12/s15 | ➖ | ➖ | ✅ 6 范式全集 | ✅ workflow DAG | ✅ Swarm+mailbox | 🔶 ext subagent | 🔶 delegate/spawn | ➖ | 🔶 JoinSet max 4 |
| **6. Memory/Context** | 🔶 session | 🔶 session | 🔶 file IPC | ✅ FTS5+3 层 cache | 🔶 章节 demo | 🔶 简单 | ✅ 供应商 | 🔶 + extensions | ✅ KG+SQLite | 🔶 session | 🔶 context hook | 🔶 session | 🔶 compaction 笔记 | ✅ Laputa/Mentle |
| **7. Skills/Plugins** | ✅ marketplace | 🔶 features | 🔶 plugins/ | ✅ skills_guard | ❌ | ❌ | ➖ | ✅ .omp ext | 🔶 manifest | 🔶 config | ✅ extensions | 🔶 plugins | ➖ | 🔶 skill 安全审查 |
| **8. MCP** | ✅ | 🔶 | ❌ | ✅ | 🔶 s20 | ❌ | ✅ server | ✅ | ✅ + A2A | ✅ | 🔶 | ✅ | ➖ | 🔶 有集成 |
| **9. Channels/Gateway** | 🔶 IDE 为主 | 🔶 CLI | ❌ | ✅ 20+ | ❌ | 🔶 QQ only | ➖ | 🔶 + gateway | ✅ ~40 | 🔶 | 🔶 | ✅ 30+ | ➖ | 🔶 ~14 |
| **10. Cron/Triggers** | 🔶 | 🔶 | ❌ | ✅ 双池+croniter | ❌ | ❌ | ➖ | 🔶 robomp | ✅ TriggerEngine | 🔶 | 🔶 | ✅ | ➖ | 🔶 cron+heartbeat 无退避 |
| **11. Security/Sandbox** | 🔶 permission | ✅ execpolicy | ❌ | ✅ 6 backend+approval | 🔶 permission demo | ❌ | ➖ | 🔶 | 🔶 capability | 🔶 | 🔶 | ✅ OS sandbox | ➖ | 🔶 crate 有未接线 |
| **12. Provider** | ✅ Anthropic | ✅ OpenAI | 🔶 单 provider | ✅ 多 provider | 🔶 可配 | 🔶 LLM API | ➖ | ✅ 多+Codex 发现 | ✅ | 🔶 | ✅ 多 | ✅ | ➖ | ✅ providers crate |
| **13. Observability** | 🔶 | ✅ OTEL | ❌ | ✅ Langfuse+trajectory | ❌ | ❌ | ➖ | 🔶 | 🔶 | 🔶 | 🔶 | ✅ OTEL | ➖ | 🔶 audit/tracing |
| **14. UI/CLI** | ✅ TUI+IDE | ✅ TUI | 🔶 CLI | 🔶 CLI+desktop | ❌ REPL | 🔶 QQ | ➖ | ✅ TUI 厚 | 🔶 CLI | 🔶 CLI | ✅ TUI/LSP/DAP | 🔶 CLI | ➖ | 🔶 CLI+Tauri+WinSvc |
| **15. Config** | ✅ settings.json | ✅ config.toml | 🔶 memory/*.md | ✅ config.yaml | 🔶 章节 | 🔶 bot config | 🔶 hook settings | ✅ settings+.omp | ✅ TOML manifest | 🔶 YAML+env | ✅ ~/.pi | ✅ TOML | ➖ | 🔶 character+TOML |

---

## 4. Cross-cutting Themes

### 4.1 agent-diva 领先领域

| 能力 | 说明 | 参考项目对比 |
|------|------|-------------|
| **Laputa 提案/迁移** | 结构化 memory provider + proposal lifecycle + changelog | 无直接对标；hermes 有 trajectory 但无提案闸 |
| **AutoDream 监督运行** | 人工确认 input/output/proposal 生命周期 | 独有产品车道 |
| **Mentle 记忆集成** | 通过 `memtle` crate 嵌入，非自研竞品 | memtle 为供应商，diva 为集成方 |
| **Plan Mode** | planning hooks + HOOK-1 已接线 | 其他项目无等价「规划模式」产品层 |
| **Windows Service** | `agent-diva-service` 原生 Windows 服务 | 参考项目均无 |
| **Skill 安全审查** | Laputa proposal 间接 + skill install 拦截思路 | hermes `skills_guard` 最接近，diva 走提案流程更稳 |
| **Supervised bounded subagent** | JoinSet max 4 + depth 3 + tool gating | 比 GenericAgent SOP 更工程化；比 swarm 更简单可控 |
| **多 crate 类型安全** | Rust workspace 15 crate 职责分离 | zeroclaw/openfang 同类，但 diva 产品车道更宽 |

### 4.2 agent-diva 落后领域

| 能力 | Gap 描述 | 领先参考 |
|------|---------|---------|
| **统一 Hooks** | 无 `HookRegistry` agent 生命周期；planning HOOK-2~5 stub | claude-code, hermes, zeroclaw, pi |
| **ExecPolicy 深度** | sandbox crate 存在，`ExecTool` 未接入；缺 DANGEROUS_PATTERNS 库 | codex execpolicy, hermes approval.py |
| **OTEL / 可观测** | 仅 audit/tracing，无 Langfuse/OTEL export | zeroclaw, codex OTEL; hermes Langfuse |
| **Swarm / Teams** | 无 mailbox、SendMessage、Task DAG | claude-code, OpenHarness, oh-my-pi |
| **Coding Harness 厚度** | 非 coding-first；无 LSP/DAP/inline edit TUI | pi, oh-my-pi, codex, claude-code |
| **Harness 防护层** | 缺 prompt injection 防御、三层 token budget、PII redact | hermes threat_patterns + budget_config |
| **Channels 厚度** | ~14 够用但缺 mirror 跨 gateway | hermes mirror, openfang TriggerEngine |
| **Cron 工业度** | 无退避、无双池、无 job-level cooldown | hermes cron/jobs.py |
| **Tool 工业特性** | 无 toolset 组合、check_fn TTL、dynamic schema | hermes toolset + check_fn |
| **Subagent 可观测** | 无 SubagentStart/Stop hooks / ProcessEvent | claude-code, hermes subagent_stop |

### 4.3 Top 10 借鉴优先级（agent-diva）

| 优先级 | 借鉴项 | 来源项目 | 理由 |
|:------:|--------|---------|------|
| **P0** | 统一 agent lifecycle hooks（8–10 MVP 事件 + `HookResult<T>`） | zeroclaw, hermes, pi | 最大结构性缺口；阻塞 Plugin/Defense/Observability 统一扩展 |
| **P0** | ExecTool 接入 sandbox + execpolicy 规则库 | codex, hermes | crate 已有；接线即可显著提升安全 posture |
| **P0** | Prompt injection 防御 + 三层 token budget | hermes | harness-engineering 三方对比 P0 缺口；生产必备 |
| **P1** | OTEL export / structured trajectory | zeroclaw, codex, hermes | Wave 7 observability 目标 |
| **P1** | SubagentStart/Stop lifecycle hooks + ProcessEvent 分轨 | claude-code, feature-swarm-humanlike 设计 | 可观测 + GUI 分轨；不 merge 旧分支 |
| **P1** | `invoke_hook` 统一 dispatcher + `diva hooks list/test` CLI | hermes | 运维性与插件生态 |
| **P1** | toolset 组合 + check_fn TTL availability | hermes | Tool 工业特性；低侵入扩展 Tool trait |
| **P2** | Declarative DAG / Wave engine（YAML `waits_for`） | oh-my-pi swarm-extension, openfang workflow | Sub-agent 演进；吸收 swarm 分支序曲思想 |
| **P2** | Swarm mailbox / SendMessage IPC | claude-code, OpenHarness | 多 agent 协作；产品层需 ADR |
| **P2** | Coding TUI 厚度（LSP/DAP 可选 lane） | pi, codex | 非平台核心；可作为独立 product lane |

---

## 5. Per-Project One-liner（相对 agent-diva 定位）

| 项目 | 相对 agent-diva 的一句话定位 |
|------|------------------------------|
| **claude-code** | Coding + Swarm 标杆；hooks 事件面最广，diva 缺其 lifecycle 厚度与 Task DAG |
| **codex** | Rust coding agent；execpolicy + OTEL 最值得 diva 直接借鉴的安全/观测模板 |
| **GenericAgent** | 最简 Python 参考；SOP 文本 subagent 不如 diva 工程化，hooks 过于原始 |
| **hermes-agent** | 生产 Harness 全能王；diva 应在 defense/budget/approval/hooks 对齐其工业度 |
| **learn-claude-code** | 教学渐进实现；hooks/swarm/MCP 章节是 diva ADR 讨论的最小可读样例 |
| **MaiMBot** | QQ 人格 bot，非 harness；通道/人格层参考，不参与 Harness 对标 |
| **memtle** | 记忆供应商；diva 已通过 Mentle 集成，是依赖而非竞品 |
| **oh-my-pi** | pi 超集；swarm-extension DAG + robomp 队列是 diva subagent 演进首选参考 |
| **openfang** | Rust Agent OS 最宽；通道/Workflow/KG 记忆厚度超 diva，缺 Laputa 产品车道 |
| **OpenHarness** | Python Claude 兼容 harness；Swarm+mailbox 模式是 diva 多 agent 设计参考 |
| **pi** | Coding harness 纯度最高；ExtensionRunner typed hooks 是 diva hooks MVP 语义参考 |
| **zeroclaw** | Rust Agent OS 同族；typed hooks + OS sandbox + OTEL 与 diva 技术栈最近 |
| **analysis** | Compaction 研究笔记（docs-only）；context 压缩策略参考，非运行时 |
| **agent-diva** | 平台 OS + 产品车道（Laputa/AutoDream/Plan Mode）；Harness 厚度弱于 coding 系与 hermes |

---

## 6. 排除项与特殊案例

### 6.1 analysis（docs-only）

- **性质**: `.workspace/analysis/` 仅含 context compaction 模式研究文档，**无 agent 运行时、无 hooks、无 subagent**。
- **价值**: 为 diva compaction 策略提供模式对照（见 research 笔记），不参与 15 维矩阵能力评分（矩阵中记 ➖）。
- **处理**: 纳入项目清单供完整性，但不作为 Harness 竞品。

### 6.2 memtle（embedded supplier，非 competitor）

- **性质**: Rust 记忆/MCP 服务 crate；提供 stop-hook checkpoint、`memtle_hook_settings` 等工具。
- **与 diva 关系**: agent-diva 通过 **Mentle** 功能车道集成 published `memtle = 0.1.2` crate；memtle 是**上游供应商**，不是 diva 的 Harness 竞品。
- **矩阵处理**: Memory 维度记 ✅（供应商角色）；Hooks/Sub-agent 等 Harness 维度记 ➖ 或 🔶（仅 stop hook）。

### 6.3 MaiMBot（persona bot，非 harness）

- **性质**: NoneBot QQ 人格 bot；消息队列驱动，无 tool framework、无 hooks、无 subagent。
- **与 diva 关系**: 通道/人格/QQ 生态参考；**不参与 Harness 能力对标**。
- **矩阵处理**: Channels 记 🔶（QQ only）；其余 Harness 维度多为 ❌ 或 ➖。

---

## 7. 深度专题文档索引

| 文档 | 覆盖维度 | 要点 |
|------|---------|------|
| [workspace-hooks-comparison.md](./workspace-hooks-comparison.md) | §4 Hooks/Lifecycle | 5 种 hooks 范式；10 项目深读；diva MVP 8–10 事件建议 |
| [workspace-subagent-comparison.md](./workspace-subagent-comparison.md) | §5 Sub-agents | 6 种编排范式；13 项目矩阵；swarm 分支预览 |
| [harness-engineering-three-way-comparison.md](./harness-engineering-three-way-comparison.md) | §3 Tool, §11 Security, §8 Token | Alife/Hermes/Diva 13 维；defense/budget 缺口 |
| [hermes-harness-overview.md](./hermes-harness-overview.md) | 综合 Harness | Hermes 24 类能力；diva 41% vs hermes 91% 完成度 |

---

## 8. 验收对照

- [x] 13 个 `.workspace` 项目 + agent-diva baseline 项目清单
- [x] 15 维度 Master Capability Matrix（✅/🔶/❌/➖）
- [x] Cross-cutting：diva 领先 / 落后 / Top 10 借鉴优先级
- [x] Per-project one-liner 定位表
- [x] analysis / memtle / MaiMBot 特殊案例说明
- [x] 链接 hooks / subagent / harness 三方 deep-dive 文档
- [x] 零代码变更（仅本文档）
