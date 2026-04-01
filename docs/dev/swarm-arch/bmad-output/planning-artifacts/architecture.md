---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments:
  - _bmad-output/planning-artifacts/prd.md
  - agent-diva/project-context.md
  - _bmad-output/brainstorming/brainstorming-session-2026-03-30.md
  - agent-diva-swarm/docs/AGENT_DIVA_SWARM_RESEARCH.md
  - agent-diva-swarm/docs/CAPABILITY_ARCHITECTURE_DEEP_DIVE.md
  - agent-diva-swarm/docs/ARCHITECTURE_DESIGN.md
  - agent-diva-swarm/docs/DESIGN_SUPPLEMENT.md
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/release-checklist-v1.0.0.md
  - _bmad-output/planning-artifacts/subagent-to-swarm-migration-inventory.md
workflowType: architecture
lastStep: 8
status: complete
completedAt: '2026-03-30'
architecture_amended_at: '2026-03-31'
architecture_amendment_note: >-
  增补 FR19–FR22、NFR-P3 的执行分层、收敛与遥测决策；修正文档/诊断 FR 编号表述。
  2026-03-31：新增「跨仓库与具体参考文件索引」节——规划产物、根目录 agent-diva-swarm/docs 调研库、
  agent-diva workspace 实现路径、Shannon 对标、zeroclaw 排除说明；与 Epic 5/6 及 V1.0.0 清单互链。
project_name: newspace
user_name: Com01
date: '2026-03-30T12:00:00'
---

# Architecture Decision Document

_This document builds collaboratively through step-by-step discovery. Sections are appended as we work through each architectural decision together._

## Project Context Analysis

### Requirements Overview

**Functional Requirements:**

PRD 将能力合同化：未列能力默认不在本期范围。架构上可归纳为以下主题（与实现边界强相关）：

- **蜂群模式与对话（FR1–FR4）：** 「大脑皮层」开/关、启用时的**进行中过程反馈**、关闭时的**可测简化模式**、主界面可感知当前模式 —— 要求运行时与 GUI 之间就状态与事件有**稳定、版本化**的消费契约，且**单一真相源**（与 FR12–FR14 呼应）。
- **神经系统与观测（FR5–FR7 及 FR15–FR16）：** 主导航可达、连接/活动或分阶段 stub+说明、排障线索；MVP 首屏为**大脑架构图式 + 左右分区**，且明确**不把**游戏化总控台作为 MVP 必经首屏 —— 需要**独立的信息模型**（拓扑/活动/错误态）与**数据绑定策略**（真实数据 vs 约定替换路径），并与「过程可视」事件对齐或分轨清晰。
- **Person 与对外叙事（FR8–FR9）：** 单 Person 可见叙事线；蜂群开启时仍不得呈现多并列聊天机器人流 —— 架构上强化 **PersonOutbox / lease** 类不变式，与既有 swarm 设计文档一致。
- **能力与配置（FR10–FR11）：** skills/能力包入口与能力声明校验 —— 与 **CapabilityRegistry / manifest** 子集及错误反馈路径对齐。
- **运行时与集成（FR12–FR14）：** headless/测试可验证开闭分支；GUI 仅经文档化 API/事件消费；避免双端长期漂移 —— **Rust 为编排与状态真相源**，GUI/Tauri 为消费者。
- **执行路径、收敛与用量（FR19–FR22、NFR-P3）：** 轻量意图 **默认** 不启完整多代理编排；蜂群路径须有 **ConvergencePolicy**（最大内部轮次或等价预算、**done**、**StopReason**），**禁止** 无上限内部对话作为唯一完成手段；**RunTelemetrySnapshot** 等开发者向挂点 —— 见下文 **ADR-E**。
- **文档与诊断（FR17–FR18）：** 「关大脑皮层」语义可查；doctor 等自检**挂点**可扩展 —— 与仓库主线工具链对齐。**说明：** PRD 中神经系统分期单独编号为 **FR15–FR16**，勿与文档节混淆。

**Non-Functional Requirements:**

- **性能（NFR-P1–P3）：** 开关切换目标约 **500ms 内**完成状态与首帧反馈（不含模型/网络）；过程可视不得拖垮主流式 —— 节流/批处理策略与 UI 线程隔离（Tauri 侧约定）。**NFR-P3：** 编排默认值须 **可接受** 的延迟与调用次数；**禁止** 仅依赖无上限内部多轮对话完成用户任务（与 FR19–FR21、**ConvergencePolicy** 一致）。
- **安全（NFR-S1–S2）：** 密钥与配置沿用既有实践；黑板/内部事件进模型前**限长/白名单/校验** —— 与 `DESIGN_SUPPLEMENT` 中跨成员信任一致。
- **集成（NFR-I1–I2）：** 新事件/DTO **向后兼容**；集成面**可列清单** —— 控制「通用域」膨胀，与 PRD 范围策略一致。
- **可靠性与可观测（NFR-R1–R2）：** 切换失败可恢复或显错；内部 trace 与用户 transcript **默认分轨** —— 与架构补充中的调试流、审计分轨一致。
- **可访问性（NFR-A1）：** 开关与神经系统入口可命名、可键盘操作 —— 遵循现有 GUI 组件基线。

**Scale & Complexity:**

- **Primary domain:** 棕地 **开发者工具** — Rust workspace（核心运行时）+ **Tauri 2 + Vue 3** 桌面 GUI；本增量为**新蜂群 crate 骨架**接入与 GUI/网关消费面扩展，而非绿场全新应用。
- **Complexity level:** PRD 归类为 **medium**（范围失控时需 reassess 为 high）。
- **Estimated architectural components（逻辑上）：** 运行时状态与事件（大脑皮层 + 过程流）、神经系统视图与数据适配层、与现有 **gateway/agent/core** 的边界与 DTO、（与长期 swarm 文档一致的）Person/lease/outbox 不变式；**不**在 MVP 一次落地全套 meta/enforcement 全家桶。

### Technical Constraints & Dependencies

- **情境：** brownfield，基于 **agent-diva** 现有 crate 图；新蜂群相关 crate 与现有 crate **单向依赖**，默认可 `cargo` 构建。
- **栈（以 `agent-diva/project-context.md` 为准）：** Rust 2021、workspace MSRV **1.80.0**、Tokio、Tauri **2**、Vue **3**、Vite **6**、TS ~**5.6**、Tailwind **3**。
- **产品约束：** 首要用户触点为桌面 GUI；**神经系统**占位在 `NormalMode` 等既有结构，**中控台**与神经系统**分坑**（PRD + 头脑风暴勘误）。
- **上游设计资产：** `agent-diva-swarm/docs` 中 crate DAG、ADR-A（swarm 不依赖 meta）、ADR-B/C/D、RuntimeEffect、分轨 transcript 等 —— 实施时需与 **本期 MVP 裁剪**（头脑风暴 v0 收敛）对齐，避免一次实现 OMC 全量 parity。

### Cross-Cutting Concerns Identified

- **单一真相源：** 大脑皮层状态、过程事件源、神经系统快照 —— 前后端/多窗口间不得长期双写双读。
- **契约与版本化：** IPC/HTTP/事件 DTO 的演进策略（与 FR13、NFR-I1 一致）。
- **可测性：** 开/关与编排分支必须**无 GUI** 可自动化验证（FR12）。
- **范围控制：** 白名单事件与字段、集成面可列清单（NFR-I2、PRD 域风险）。
- **对内协作 vs 对外呈现：** Person、lease、Chair/合成与 GUI「一个人」体验一致（FR8–FR9 与 swarm 文档）。
- **MVP vs 愿景：** 神经系统前期「架构图式大脑」与后期总控台/游戏化 **分期**，架构决策须可挂到后续阶段而不绑架 MVP。

## Starter Template Evaluation

### Primary Technology Domain

**棕地开发者工具：** 既有 **Rust workspace（agent-diva-*）** + **Tauri 2 + Vue 3** 桌面壳；本 PRD 增量为运行时与 GUI 的受控扩展，而非新建独立仓库。

### Starter Options Considered

- **绿场 CLI starter（如全新 Tauri/Vite 模板）：** 不适用 — 会重复已有 `agent-diva-gui`、gateway 与配置模型。
- **选定基线：** **当前 agent-diva monorepo/workspace** 作为唯一「模板」；新蜂群 crate 以 **path 依赖** 接入，遵循根 `Cargo.toml` 的 `[workspace.dependencies]` 与 `project-context.md` 中的 crate 边界。

### Selected Starter: Existing agent-diva workspace

**Rationale for Selection:**

- PRD 明确 **brownfield** 与 **单向依赖**；设计与实现必须与现有会话、channel、agent 循环一致。
- `project-context.md` 已冻结栈与质量条（clippy `-D warnings`、`cargo test --all` 等），优于任何外部生成器另起炉灶。

**Initialization Command:**

```bash
# 无 — 使用既有仓库。新成员从 workspace 根目录：
git clone <agent-diva-repo> && cd agent-diva
# 随后按实现故事在 workspace 中 cargo new / 添加 crate 并编辑根 Cargo.toml members。
```

**Architectural Decisions Provided by Starter:**

**Language & Runtime:** Rust Edition 2021；MSRV **1.80.0**（以根 `Cargo.toml`/`clippy.toml` 为准）；异步 **Tokio** 与 workspace 对齐。

**Styling Solution:** 前端 **Tailwind 3** + Vue 3 组件体系（`agent-diva-gui`）。

**Build Tooling:** **Vite 6** 前端构建；Rust **cargo** + 工作区；桌面 **Tauri 2**（版本以锁文件为准；生态核心近期多为 **2.10.x** 量级，见 [Tauri v2 releases](https://v2.tauri.app/release)）。

**Testing Framework:** **`cargo test --all`**；HTTP 侧 **mockito/wiremock** 等沿用各 crate。

**Code Organization:** 按 `project-context.md` 的 crate 职责表放置新代码；**Gateway** 仍为消息与会话中心之一。

**Development Experience:** **`just`** 推荐入口（`check`/`test`/`run`）；提交前理想顺序见 project-context。

**Version note (2026-03-30)：** Rust **latest stable ≈ 1.94.1**（[Rust blog](https://blog.rust-lang.org/)）；与仓库 **MSRV 1.80.0** 并存 — 新代码须至少在 MSRV 上通过 CI。

**Note:** 首个实现故事应是 **在既有 workspace 内** 增加蜂群骨架与 GUI/gateway 契约，而非执行新的 starter CLI。

## Core Architectural Decisions

### Decision Priority Analysis

**Critical Decisions（阻塞实现若不拍板）：**

- **运行时真相源：** 「大脑皮层」状态、过程事件、神经系统快照的 **权威状态在 Rust（gateway/新蜂群适配层）**；GUI 仅通过 **已文档化的 Tauri command / 事件** 读取与触发，禁止长期双端各写一份（对齐 FR12–FR14、NFR-R1）。
- **Swarm 与 Meta 边界：** 编排循环 **不直接依赖** Meta crate；由 **runtime（或 gateway 组合层）** 在生命周期边界派发 hook，swarm 返回 **RuntimeEffect**（ADR-A）。
- **MVP 数据面：** 黑板 **v0 进程内内存**（ADR-C）；会话与用户可见 transcript 策略与 **DESIGN_SUPPLEMENT** 分轨一致；**不**在 MVP 引入新的独立「产品数据库」作为蜂群前提。

**Important Decisions（显著塑形）：**

- **API 与通信：** 与现有 **Gateway 中心** 模式对齐；新增 DTO **serde 可版本化**、字段 **白名单**（NFR-I2）；流式/过程事件与现有 channel 流式约定协调（节流 NFR-P2）。
- **前端架构：** `agent-diva-gui` 内 **神经系统视图** 替换 `NormalMode` 占位；**大脑皮层** 控件与 **过程反馈** 组件绑定后端契约；路由与 i18n 沿用现有体系。
- **安全：** 沿用仓库 **密钥与配置** 实践（NFR-S1）；黑板/内部 payload **限长、topic/来源白名单** 再进模型（NFR-S2、DESIGN_SUPPLEMENT §5）。

- **ADR-E — 执行分层、收敛与运行遥测（FR19–FR22、NFR-P3）：**
  - **ExecutionTier（或等价枚举）：** 至少区分 **Light**（可完成短路径：显式 skill、短问答等，**判定规则在实现说明中单文件维护**）与 **FullSwarm**（多参与者编排）。**FR19：** 在无 **显式用户选择**「深度/全蜂群」时，**不得** 仅因大脑皮层 ON 就进入 FullSwarm；轻量类须走 **可完成** 路径并在文档化 **超时/内部步数上限** 内给出 **结果或显式失败原因**。
  - **FR21 / ForceLight：** 用户或配置可 **强制轻量路径**；实现须 **二选一封冻** 并与 **CortexState::Off**（FR3）语义在 ADR 中 **交叉说明**（合并为同一语义 **或** 独立会话标志）。**首版实现冻结（Story 1.9）：** 选型 **选项 A — 合并**（无独立 ForceLight 位时与皮层 OFF 同路径），落文于仓库 [`agent-diva/agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`](../../agent-diva/agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md)；若后续改为独立标志位，须在该 ADR 发修订版并同步本段或显式标注「取代」。
  - **ConvergencePolicy：** `max_internal_rounds`（或等价预算）、**完成定义（done）**；编排循环每步检查；触顶 emit **白名单事件**（如 `swarm_run_finished`、`swarm_run_capped`）并携带 **`StopReason`**（`Done` | `BudgetExceeded` | `Timeout` | `Error` 等）。**FR20 + NFR-P3：** 禁止无终止「思考—推翻—再思考」为默认。
  - **RunTelemetrySnapshot（FR22）：** `internal_step_count`、`phase_count`、可选 `over_suggested_budget`；经 **Tauri 开发者命令**、**设置子区** 或 **poll** 暴露；**不** 默认写入用户 transcript（NFR-R2）；字段 **版本化 + 白名单**（NFR-I2）。
  - **可测性：** 扩展 FR12：无 GUI 用例覆盖 **轻量 + 皮层 ON 不启全图**、**触顶 StopReason**、**ForceLight**（若独立于 OFF）。

**Deferred Decisions（MVP 后）：**

- 黑板 **SQLite / BlackboardStore trait**（ADR-C Phase 2）；Meta 事件全集与插件扩展（CoreEvent vs ExtensionEvent）；分布式多进程 swarm；完整 request_help / 高阶 synthesis；游戏化总控台首屏（PRD 后期）。

### Data Architecture

- **持久化：** 不新增本特性的专用 DB；会话与配置延续 **agent-diva 既有存储与加载**（`project-context`）。
- **黑板：** MVP **内存 + 按 PersonSession 隔离**；长期耐久与审计策略 **后置**。
- **验证：** 能力 manifest **v0 子集** + 校验错误对用户/日志可见（FR11）；协议 JSON **golden files** 方向与 `DESIGN_SUPPLEMENT` §8 一致（实现阶段落地）。

### Authentication & Security

- **身份：** 本增量 **不引入** 新终端用户认证体系；沿用应用与渠道既有模型。
- **授权与工具：** 能力白名单 + **is_dangerous / PreToolUse** 门禁思想与 Shannon 对齐；hook **不得提升权限**（头脑风暴/补充文档共识，实现阶段细化）。

### API & Communication Patterns

- **GUI ↔ Rust：** **Tauri 2** `invoke` / 事件；命名与版本与现有 `src-tauri` 桥接一致（`project-context`）。
- **内部：** Gateway 与 agent 循环间消息 **沿用总线/现有模式**；新增 **过程事件**、**终局/触顶事件（StopReason）**、**皮层开关**、**可选 RunTelemetry 查询** 为 **可列清单** 的契约类型（ADR-E）。
- **错误：** 切换失败 **可恢复或显式错误**（NFR-R1），不静默未知模式。

### Frontend Architecture

- **状态：** 以 **服务端真相源 + 前端订阅/拉取** 为主；避免在 Vue 中复制编排状态为第二真相源。
- **组件：** 神经系统 **大脑架构图式 + 左右分区**（FR15）；与「过程可视」共享或分轨事件由实现 ADR 明确。
- **性能：** 过程更新 **批处理/节流**，不阻塞主流式（NFR-P2）。

### Infrastructure & Deployment

- **构建/CI：** 延续 **`just` / `cargo clippy -D warnings` / `cargo test --all`**（`project-context`）。
- **可观测：** **tracing** 结构化字段建议与 `DESIGN_SUPPLEMENT` §1 对齐（session、lease、capability、tick）。
- **分发：** Tauri 打包路径沿用仓库 **scripts / 既有渠道**。

### Decision Impact Analysis

**Implementation Sequence（建议）：**

1. 在 workspace 中定义 **版本化契约**（开关状态、过程事件、神经系统 DTO）与 **无 GUI 测试** 钩子（FR12）。
2. **Rust 侧** 接入真相源（新 crate 或 gateway 适配层，与 PRD「新蜂群 crate 骨架」一致）。
3. **Tauri 桥接** 暴露查询/订阅与最小变更命令。
4. **Vue**：大脑皮层开关 + 最小过程反馈 + 神经系统首屏（替换占位）。

**Cross-Component Dependencies：**

- GUI 依赖 **(2)(3)** 的契约稳定性；**(2)** 依赖 **ADR-A** 不在 swarm 内拉 meta；**(1)** 约束 **(2)(3)(4)** 的字段演进（NFR-I1）。

## Implementation Patterns & Consistency Rules

### Pattern Categories Defined

**Critical Conflict Points Identified:** 8 类 —— Rust/TS 命名与序列化、Tauri 命令与事件、DTO 版本、错误形态、测试位置、日志与 span、过程事件节流、GUI 状态单一真相源。

### Naming Patterns

**Database Naming Conventions:**

- 本特性 MVP **不新增**业务表；若后续为黑板/审计引入存储，表/列名遵循 **snake_case**（与 Rust/SQL 惯例一致），在 ADR 中命名后再实现。

**API Naming Conventions（Tauri / 内部 JSON）：**

- **Tauri command：** 与现有 `src-tauri` 一致，采用 **snake_case** 或项目既有 camelCase —— **以现有 invoke 注册名为准**，新命令 **先搜一遍同目录命名再添加**。
- **Serde JSON（Rust → 前端）：** 字段默认 **snake_case**（`#[serde(rename_all = "camelCase")]`）仅在 **全链路已统一** 时引入；**禁止** 同一概念在 Rust 与 TS 侧用不同拼写且无 serde 映射。
- **过程/蜂群事件名：** **snake_case + 过去式或名词短语**，例如 `cortex_toggled`、`swarm_phase_changed`；避免 `UserDidSomething` 与 `userDidSomething` 混用。

**Code Naming Conventions:**

- **Rust：** 类型 `PascalCase`，函数/模块 `snake_case`，常量 `SCREAMING_SNAKE`；与 `rustfmt.toml` / clippy 一致。
- **Vue 组件：** 与 `agent-diva-gui` 现有 **PascalCase 单文件组件** 一致；新神经系统子组件以 **`Nervous*` / `Brain*`** 等可检索前缀为宜（与 PRD 隐喻一致即可）。
- **TypeScript：** 变量/函数 **camelCase**；与前端 ESLint 配置一致。

### Structure Patterns

**Project Organization:**

- **Rust 新代码：** 放入 PRD/决策约定的 **新 crate 或现有 gateway/agent/core**（见 `project-context` crate 表）；**禁止** 在 GUI crate 中实现编排核心逻辑。
- **测试：** Rust 用各 crate 既有 **`tests/` 与 `#[cfg(test)]`** 布局；**无 GUI 的编排/开关测试** 放在 **Rust 集成测试**，不依赖 Tauri WebView。
- **Vue：** 神经系统与大脑皮层相关组件放在 **与 `NormalMode` 同层或专用 `components/neuro/`（若新建目录须在本文档或后续 ADR 一记）**，避免散落在无关页面。

**File Structure Patterns:**

- 契约类型：**优先** 放在可被 **Rust 核心 + Tauri** 共享的 crate（或 `shared` 模块），避免在 `.vue` 中手写与 Rust 不一致的重复类型定义；若用 TS 类型，**与 serde 输出对齐并注明版本**。

### Format Patterns

**API Response Formats（invoke / 事件 payload）：**

- **成功：** 直接返回强类型结构体序列化结果；**避免** 再包一层无约定的 `{ ok, data }` 除非全应用已统一。
- **错误：** Tauri 侧 **`Result<T, String>` 或项目既有错误类型** —— **与现有 command 错误形态一致**；用户可见文案走 i18n key，不把内部堆栈直接当 toast。

**Data Exchange Formats:**

- **JSON：** 日期/时间 **ISO 8601 字符串**（若需）；布尔 **true/false**；可选字段用 **`Option` / `null`**，不用 `1/0`。
- **版本字段：** 对外 DTO 含 **`schema_version` 或 `api_version`**（小整数或 semver 字符串二选一，**全项目统一一种**）。

### Communication Patterns

**Event System Patterns:**

- **过程可视事件：** payload **小而稳定**（阶段 id、简短 message、可选结构化 id）；大块内容用 **引用**（artifact id、path）而非塞满事件体（对齐 file-as-memory 思想）。
- **订阅：** 前端 **监听 Tauri 事件** 时，事件名 **常量集中定义**（单文件或 enum），禁止魔法字符串散落。

**State Management Patterns:**

- **大脑皮层开关：** **以后端状态为准**；前端可乐观更新 **仅当** 已有统一回滚/错误处理；否则 **以后端确认为准**。
- **禁止** 在 Pinia/组件树中维护第二份「权威」编排状态与后端长期分叉。

### Process Patterns

**Error Handling Patterns:**

- **Rust：** 边界 `anyhow::Context`；库 `thiserror`；**生产路径** 不对用户输入/网络 `unwrap`（`project-context`）。
- **切换失败（NFR-R1）：** **显式错误 + 回滚到上一稳定态**；记录 **tracing** error span。

**Loading State Patterns:**

- 过程反馈使用 **局部 loading / skeleton**，不阻塞整个聊天窗口；**节流** 更新（例如每 100–250ms 合并一次），由实现选定并在代码中写清。

### Enforcement Guidelines

**All AI Agents MUST:**

- 新增 Tauri command / 事件前 **对照现有命名与错误类型**，保持一致。
- 用户可见写路径 **仅经** PersonOutbox/lease 语义（与 ADR-B）；不在 Vue 中「直接拼接最终用户消息」冒充后端合成。
- **cargo clippy -D warnings** 与 **`cargo test --all`** 对改动 crate 可运行范围内通过后再合入。
- 内部调试数据 **不** 默认写入用户 transcript（DESIGN_SUPPLEMENT 分轨）。

**Pattern Enforcement：**

- PR 审查对照本文 **Naming / Communication** 两节；契约变更同步 **CHANGELOG 或 ADR**。
- 违反 **ADR-A**（swarm 依赖 meta）的依赖关系 **CI 可用 `cargo tree` 脚本拦截**（DESIGN_SUPPLEMENT §8）。

### Pattern Examples

**Good Examples：**

- 在 `protocol` 或共享模块定义 `CortexState { enabled: bool, schema_version: u32 }`，前后端共用 serde 形状。
- 过程事件：`{ "event": "swarm_phase", "phase": "synthesis", "session_id": "…" }` 字段稳定、可扩展。

**Anti-Patterns：**

- 在 `.vue` 内手写字符串命令名拼错且与 Rust 注册名不一致。
- 前端维护 `cortexOn` 而后端 `cortex_enabled` 长期不同步且无同步协议。
- 为省事在 swarm crate 内 `use agent_diva_meta`。

## Project Structure & Boundaries

### Complete Project Directory Structure

> 根目录：`newspace/`。以下为与本 PRD 相关的 **现有** 与 **规划** 路径；`（规划）` 表示实现时尚未存在的目录/包。

```
newspace/
├── _bmad-output/
│   └── planning-artifacts/
│       └── architecture.md          # 本文档
├── agent-diva-swarm/
│   └── docs/                          # 设计/调研（已存在）
│       ├── AGENT_DIVA_SWARM_RESEARCH.md
│       ├── CAPABILITY_ARCHITECTURE_DEEP_DIVE.md
│       ├── ARCHITECTURE_DESIGN.md
│       └── DESIGN_SUPPLEMENT.md
│   └── （规划）src/                   # 未来 Rust crate 根，或迁移至 agent-diva 下为 workspace member
│   └── （规划）Cargo.toml
├── agent-diva/                        # Rust workspace 根（已存在）
│   ├── Cargo.toml                     # workspace members、workspace.dependencies
│   ├── rustfmt.toml
│   ├── clippy.toml
│   ├── justfile
│   ├── agent-diva-core/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   ├── tests/                     # 若存在
│   │   └── project-context.md
│   ├── agent-diva-agent/
│   │   ├── src/
│   │   └── project-context.md
│   ├── agent-diva-channels/
│   ├── agent-diva-cli/
│   ├── agent-diva-manager/
│   ├── agent-diva-migration/
│   ├── agent-diva-neuron/
│   ├── agent-diva-providers/
│   ├── agent-diva-service/
│   ├── agent-diva-tools/
│   └── agent-diva-gui/                # Tauri 2 + Vue 3
│       ├── package.json
│       ├── vite.config.ts
│       ├── src/
│       │   ├── main.ts
│       │   ├── App.vue
│       │   ├── api/                   # desktop.ts、providers.ts 等
│       │   ├── components/
│       │   │   ├── NormalMode.vue     # 神经系统占位替换点
│       │   │   ├── ChatView.vue       # 大脑皮层开关等（规划挂载点）
│       │   │   ├── GatewayControlPanel.vue
│       │   │   └── （规划）neuro/ 或 Nervous*.vue
│       │   ├── locales/               # zh.ts、en.ts
│       │   └── utils/
│       └── src-tauri/
│           ├── Cargo.toml
│           ├── tauri.conf.json
│           └── src/
│               ├── main.rs
│               ├── lib.rs
│               ├── commands.rs        # Tauri 命令扩展点
│               └── app_state.rs
└── docs/                              # 项目知识（可为空或后续文档）
```

**实现阶段须决议：** 新蜂群 crate 物理位置 —— **(A)** 作为 `agent-diva/` workspace 新 member（例如 `agent-diva/agent-diva-swarm`），或 **(B)** 保留 `newspace/agent-diva-swarm` 为独立 crate 并以 `path` 加入 workspace。无论哪种，须满足 **单向依赖** 与 **ADR-A**。

### Architectural Boundaries

**API Boundaries:**

- **外部：** LLM/MCP/渠道 —— 仍经 `agent-diva-providers`、`agent-diva-channels`、配置模型；本特性 **不** 新建平行配置体系。
- **GUI ↔ 本地进程：** Tauri **`commands.rs`（及 `lib` 注册）** 为边界；**禁止** 在 Vue 中直接启动/替代 gateway 编排核心。
- **内部：** Gateway ↔ Agent 循环 ↔ 新蜂群适配层 —— 消息与 DTO 在 **Rust** 内定义；DTO 变更走版本字段（见实现模式节）。

**Component Boundaries:**

- **Vue：** 展示与交互；订阅 Tauri 事件 / invoke；**不** 持有编排真相源。
- **Rust swarm 运行时：** handoff、黑板、合成等 **不** 依赖 `agent-diva-meta`；Meta 仅在 **runtime/gateway 组合层** 触发。

**Service Boundaries:**

- **Gateway：** 会话与消息路由中心之一；蜂群状态注入点应 **单点** 文档化（避免多处各写各的）。
- **新 crate：** 承担「皮层开关 / 过程事件 / 神经系统快照」中与 PRD 对齐的 **领域逻辑与类型**，通过清晰 `pub` 面暴露给 gateway/gui 侧薄适配层。

**Data Boundaries:**

- **MVP：** 黑板与过程状态 **进程内**；用户 transcript 与内部 trace **分轨**（DESIGN_SUPPLEMENT）；**无** 本特性专用新 DB 表。

### Requirements to Structure Mapping

**FR 类别 → 主要落点：**

| 类别 | 主要目录 / 边界 |
|------|-----------------|
| 大脑皮层 / 过程可视（FR1–FR4） | 新蜂群 crate + gateway/agent 接线；`commands.rs` + 前端 `ChatView` / 聊天相关组件 |
| 神经系统（FR5–7、FR15–16） | `NormalMode.vue` 替换；`components/neuro/` 或等价；DTO 来自 Rust |
| Person 叙事（FR8–9） | 编排层（agent / 新 crate）与 **单一对外写出口** |
| 能力与 manifest（FR10–11） | `agent-diva-agent` skills 路径 + 新 registry 子模块（若拆出） |
| 无 GUI 可测（FR12） | 新 crate `tests/` 或 workspace 集成测试 |
| 契约（FR13–14） | 共享类型 crate 或 `agent-diva-core` 扩展 + Tauri 序列化 |
| 执行路径 / 收敛 / 遥测（FR19–22、NFR-P3） | 新蜂群 crate 编排入口 + gateway；`ExecutionTier`、`ConvergencePolicy`、`StopReason`；`swarm_run_*` 事件；`RunTelemetrySnapshot` DTO / dev 命令 |

**Cross-Cutting Concerns：**

- **tracing / 可观测：** 各 Rust 层 span 属性与 `DESIGN_SUPPLEMENT` §1 对齐。
- **i18n：** `agent-diva-gui/src/locales/*` 新增键；**禁止** 硬编码用户可见英文/中文混用无 key。

### Integration Points

**Internal Communication：**

- Rust：**函数调用 + 既有总线/通道模式**；新增 **过程事件** 可为 channel 或回调，须在 ADR 指定一种，避免双栈并行。
- **Tauri：** `invoke` 查询状态；`listen` 订阅流式/过程更新（若采用事件推送）。

**External Integrations：**

- 无新增第三方 SaaS **作为本 MVP 必要条件**；沿用 LLM/MCP/skills。

**Data Flow（简图）：**

用户消息 → Gateway → Agent/蜂群运行时 →（过程事件）→ Tauri emit → Vue 更新；**最终用户文本** 仅经 lease/outbox 路径 → 既有聊天 UI。

### File Organization Patterns

**Configuration Files：**

- Rust：**根 `Cargo.toml` workspace 依赖**；各 crate `Cargo.toml` 仅 `path` / `workspace = true`。
- GUI：**`agent-diva-gui` 根目录** `package.json`、`vite.config.ts`；Tauri **`tauri.conf.json`**。

**Source Organization：**

- **按 crate 职责**（`project-context` 表）放置；新代码 **不** 放入错误 crate「图省事」。

**Test Organization：**

- **Rust：** `#[cfg(test)]` 与 crate 级 `tests/`；蜂群开关/状态机 **优先纯 Rust 测**。
- **GUI：** 随现有 Storybook/组件测试惯例（若有）；E2E 非 MVP 阻塞项除非 PRD 追加。

**Asset Organization：**

- 静态资源 `agent-diva-gui/public/`、`src/assets/`；神经系统图示若用 SVG **放 `assets` 或组件就近**。

### Development Workflow Integration

**Development Server Structure：**

- 前端：`agent-diva-gui` 下 `pnpm`/`npm` dev；Rust：`cargo run` / Tauri dev（以 README 为准）。

**Build Process Structure：**

- `vue-tsc --noEmit && vite build`；Tauri bundle；workspace `cargo build --all`。

**Deployment Structure：**

- 沿用 **Tauri 打包与 `scripts/`**；本特性 **不** 改变发布渠道 unless 产品另定。

## 跨仓库与具体参考文件索引

> **路径基准：** 下文凡写相对路径，均以 monorepo 根目录 `newspace/` 为起点（与 `_bmad-output/planning-artifacts/architecture.md` 的兄弟关系：`../../` 到达 `newspace/`）。
>
> **重要：`agent-diva-swarm` 双轨位置**
>
> | 位置 | 性质 | 架构上怎么用 |
> |------|------|----------------|
> | `agent-diva-swarm/docs/`（**仓库根下**） | 调研与概念设计主库（ARCHITECTURE_DESIGN、CAPABILITY 深潜等） | 读 **ADR 概念、Person/黑板/分轨**；与 PRD/epics 交叉引用 |
> | `agent-diva/agent-diva-swarm/` | **Cargo workspace member**；含 `src/` 与 **实现侧** `docs/` | **代码与运行期契约的真相源**；Tauri/Gateway 接线以此为准 |

### A. 规划与发布（`newspace/_bmad-output/planning-artifacts/`）

| 文件 | 内容摘要 | 与架构的关联 |
|------|----------|----------------|
| `prd.md` | 产品需求全文；**1.0.0 双轨 P0**、FR1–FR22、NFR、神经系统分期 | 决策与验收的 **权威需求** |
| `epics.md` | 史诗/故事、FR 覆盖表；**Epic 5（编排加深）**、**Epic 6（V1.0.0 收口）** | 将 ADR-E、MIG 落到 **可交付故事** |
| `release-checklist-v1.0.0.md` | Swarm-类 / Shannon-类 P0 **可勾选表** + 发布卫生 | 标 1.0.0 前 **机械核对**；链到 epics 与测试 |
| `subagent-to-swarm-migration-inventory.md` | **三层 subagent**（Rust / Cursor / BMAD）、**SWARM-MIG-01~03** | Epic **6.5–6.8**、Person 叙事与工具注册表演进 |
| `ux-design-specification.md` | UX Decision Register（UX-DR1–5）与实现约定 | GUI 契约与 **NFR-P2** 节流、可访问性对齐 |

### B. 根目录调研文档库（`newspace/agent-diva-swarm/docs/`）

| 文件 | 内容摘要 |
|------|----------|
| `AGENT_DIVA_SWARM_RESEARCH.md` | 愿景、模块草图、**非目标**；与「Person + 内化蜂群」叙事一致 |
| `CAPABILITY_ARCHITECTURE_DEEP_DIVE.md` | 能力×swarm、handoff/as_tool、Chair、**参考栈矩阵**（含 Shannon/OMC 思想摘录） |
| `ARCHITECTURE_DESIGN.md` | **ADR-A**（swarm 不依赖 meta）、ADR-B/C/D、RuntimeEffect、Mailbox、crate 关系 |
| `DESIGN_SUPPLEMENT.md` | 合成与 I/O、**内部 trace vs 用户 transcript 分轨**、审计、注入/限长（**NFR-S2、NFR-R2**） |

### C. 主工程实现（`newspace/agent-diva/`）

| 路径 | 内容摘要 |
|------|----------|
| `Cargo.toml` | Workspace **members**、**rust-version 1.80.0**、`[workspace.dependencies]` |
| `project-context.md` | 栈（Tauri 2、Vue 3、MSRV）、**clippy/test/just** 质量闸、crate 职责表 |
| `agent-diva-swarm/README.md` | swarm crate **职责与文档入口** |
| `agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md` | **关大脑皮层**简化路径行为（**FR3**） |
| `agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md` | **FR21** 与皮层 OFF **合并策略**冻结；与本文 **ADR-E** 一致 |
| `agent-diva-swarm/docs/CORTEX_OFF_FR17_MAINTAINER_GUIDE.md` | **FR17** 维护者向说明 |
| `agent-diva-swarm/docs/process-events-v0.md` | 过程事件 **v0** 约定与白名单思路（**NFR-I2**） |
| `agent-diva-swarm/docs/PROCESS_EVENTS_CORTEX_OFF.md` | 皮层 OFF 时过程事件 **是否发射/如何降级** |
| `agent-diva-swarm/src/process_events.rs` | 过程事件 **实现**（发射、过滤、与 GUI 契约对齐时的改动的首选文件之一） |
| `agent-diva-swarm/src/minimal_turn.rs` | **轻量 / 最小 turn** 与执行分层相关逻辑 |
| `agent-diva-gui/src-tauri/src/capability_commands.rs` | Tauri **能力侧**命令示例；**GUI ↔ Rust** 边界参考 |
| `agent-diva-agent/src/subagent.rs` | 运行时 **SubagentManager**（与迁移清单 **A 层**、MIG-01/02 相关） |
| `agent-diva-tools/src/spawn.rs` | **Spawn** 工具与后台子任务（与 Swarm-类「spawn vs 全图编排」边界对照） |

### D. 对标参考（`newspace/Shannon/`，非 V1.0.0 运行时依赖）

| 路径 | 内容摘要 | 使用方式 |
|------|----------|----------|
| `Shannon/rust/agent-core/README.md` | **工具编排、强制执行网关、可观测**（tracing/metrics）、WASI 等高层能力描述 | **对标 Shannon-类 P0**（有界、失败可分类、门禁思想）；**不**默认作为 Cargo 依赖接入 unless 另立 ADR 与 Epic 5.4 SPI 评估一致 |
| `Shannon/rust/agent-core/WASI_SETUP.md` | WASI 沙箱环境 setup | Vision+ 或 **独立编排进程** 路线时的技术参考 |

### E. 明确排除（与本 PRD 架构依赖图无关）

| 路径 | 说明 |
|------|------|
| `zeroclaw-0.1.7/` | 嵌入式/固件/机器人相关 workspace；**不**纳入 agent-diva 蜂群、1.0.0 或 Epic 5/6 **依赖边界** |

### F. 与 Epic 5 / Epic 6 / V1.0.0 的快速对照

| 主题 | 优先打开的参考 |
|------|----------------|
| 序曲可配置、遥测同源、handoff 检查点、编排 SPI | `epics.md` Epic 5；`ARCHITECTURE_DESIGN.md`；`release-checklist-v1.0.0.md` S1–S4、H1 |
| Timeout / Light enforcement / doctor 真数据 / GUI↔gateway 一致 | `epics.md` Epic 6；`release-checklist-v1.0.0.md` H2–H7；`process-events-v0.md` + `process_events.rs` |
| Subagent → Capability / Person 可见性 | `subagent-to-swarm-migration-inventory.md`；`subagent.rs`；`spawn.rs` |

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility:**

- 技术栈（Rust workspace + Tauri 2 + Vue 3）与棕地基线、MSRV、质量闸（clippy/test）一致；GUI 为消费者、swarm 不依赖 meta、lease/outbox 语义与 `ARCHITECTURE_DESIGN` 一致。
- 版本策略上「最新 stable Rust」与「MSRV 1.80.0」并存已在 Starter 节说明，不冲突。

**Pattern Consistency:**

- 实现模式中的 serde/Tauri 命名、事件节流、错误与 i18n 规则支撑核心决策中的契约稳定与 NFR-P2/R1。
- 反模式列表直接对应 ADR-A/B 与 FR13。

**Structure Alignment:**

- 目录树将 `NormalMode.vue`、`commands.rs`、各 `agent-diva-*` crate 与规划中的蜂群代码位置对齐；边界表覆盖 Gateway、Tauri、数据分轨。

### Requirements Coverage Validation ✅

**Epic/Feature Coverage:**

- 与 `_bmad-output/planning-artifacts/epics.md` 对齐；覆盖以 PRD **FR/NFR** + **ADR-E** 映射完成（见「Requirements to Structure Mapping」表）。

**Functional Requirements Coverage:**

- FR1–FR4、FR5–7、FR8–9、FR10–11、FR12–14、**FR17–FR22** 及神经系统分期（**FR15–FR16**）均可在「Core Decisions（含 ADR-E）」「Structure」「Patterns」中找到落点或约束。

**Non-Functional Requirements Coverage:**

- NFR-P（节流、非阻塞流式、**P3 与收敛**）、NFR-S（密钥实践、黑板输入校验）、NFR-I（兼容与白名单）、NFR-R（切换恢复、分轨）、NFR-A（轻量 a11y）均有对应架构或模式条目。

### Implementation Readiness Validation ✅

**Decision Completeness:**

- 关键决策（真相源、ADR-A、MVP 数据面、API/GUI 边界）已文档化；开放项（crate 物理路径、过程事件派发机制）已显式标为 **实现前决议**。

**Structure Completeness:**

- 现有路径与规划路径已区分；集成点与数据流已文字化，足以指导 AI 代理不随意放文件。

**Pattern Completeness:**

- 已覆盖命名、结构、格式、通信、错误、加载态与强制条款；并含正反例。

### Gap Analysis Results

**Critical Gaps：** 无 —— 无「缺了则无法开工」且未提及的硬决策；**crate 落位** 为 **高优先级待决**，已在结构章节列出选项。

**Important Gaps：**

- 首个 **实现 ADR** 建议冻结：`CortexState` / 过程事件 **JSON 或类型形状**、**`StopReason` / `swarm_run_*` 载荷**、Tauri 命令清单 v0、过程事件 **推送 vs 轮询**、**Light vs FullSwarm 判定清单 v0**。
- **CI：** `cargo tree` 校验 swarm 不依赖 meta 可后置到首 PR 合入前。

**Nice-to-Have Gaps：**

- 神经系统 **具体 SVG/布局组件** 命名规范可在 UX 迭代时补；E2E 策略非 MVP 阻塞。

### Validation Issues Addressed

- **crate 位置二选一：** 保留在结构节作为 **实现阶段决议**，验证不予强行闭合，避免与仓库实际迁移不同步。
- **FR19–FR22 / NFR-P3：** 已通过 **2026-03-30 修订** 纳入 ADR-E 与映射表；实现前须在首 ADR 中冻结具体类型名与事件名。

### Architecture Completeness Checklist

**✅ Requirements Analysis**

- [x] Project context thoroughly analyzed
- [x] Scale and complexity assessed
- [x] Technical constraints identified
- [x] Cross-cutting concerns mapped

**✅ Architectural Decisions**

- [x] Critical decisions documented with versions
- [x] Technology stack fully specified
- [x] Integration patterns defined
- [x] Performance considerations addressed

**✅ Implementation Patterns**

- [x] Naming conventions established
- [x] Structure patterns defined
- [x] Communication patterns specified
- [x] Process patterns documented

**✅ Project Structure**

- [x] Complete directory structure defined
- [x] Component boundaries established
- [x] Integration points mapped
- [x] Requirements to structure mapping complete

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION（以 **先决决议**：蜂群 crate 路径 + 事件机制 ADR 为首个迭代任务）

**Confidence Level:** **中高** —— 棕地路径与 PRD 对齐充分；开放项明确且局部。

**Key Strengths:**

- Person/lease、ADR-A、分轨观测与 MVP 裁剪在同一文档内可执行。
- 与 `agent-diva-swarm/docs` 及 `project-context` 交叉引用清晰。

**Areas for Future Enhancement:**

- Meta 全量、黑板耐久、分布式 swarm —— 已标 Deferred。

### Implementation Handoff

**AI Agent Guidelines:**

- 严格遵循本文 **Core Architectural Decisions** 与 **Implementation Patterns**；放代码前对照 **Project Structure** 边界。
- 契约问题以 **本文 + 后续 ADR** 为准；与 PRD 冲突时 **先对齐 PRD** 再改架构文档。

**First Implementation Priority:**

1. **决议** 蜂群 crate 落位（workspace member 路径）并 `cargo new`/members 更新。
2. 定义 **v0 DTO + 无 GUI 测试**（皮层状态、最小过程事件、**ExecutionTier + ConvergencePolicy 默认值**）。
3. **Gateway / Tauri** 薄适配 + `NormalMode` / `ChatView` 最小 UI 接线（含 **触顶/轻量** 与 **可选遥测** 挂点）。

## Workflow Completion

**架构工作流已完成。** 单一事实来源：`_bmad-output/planning-artifacts/architecture.md`（frontmatter：`status: complete`，`completedAt: 2026-03-30`）。

**与 Com01 一起完成的内容：** 从输入发现到项目上下文、棕地基线、核心决策、实现模式、目录与边界、验证与交付优先级，已形成可给多代理共用的架构说明。
