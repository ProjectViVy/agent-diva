---
stepsCompleted:
  - step-01-validate-prerequisites
  - step-02-design-epics
  - step-03-create-stories
  - step-04-final-validation
inputDocuments:
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
ux_wireframe_ref: _bmad-output/planning-artifacts/ux-design-directions.html
architecture_source: _bmad-output/planning-artifacts/architecture.md
note: >-
  PRD 已定稿：神经系统分期为 FR15–FR16；文档与诊断为 FR17–FR18；执行路径/收敛/用量为 FR19–FR22。
  UX 规格 **Decision Register** 中 UX-DR1–DR5 与下表一致；原「组件目录、BrainOverview 等」已重命名为 **UX-IMPL-***，避免与 DR1–DR5 冲突。
workflow_completed_at: '2026-03-30'
epics_amended_at: '2026-03-31'
epics_amendment_note: >-
  对齐 PRD FR19–FR22、NFR-P3 与 UX 规格；新增 Story 1.7–1.9、2.4；统一 UX-DR / UX-IMPL。
  2026-03-31：新增 **Epic 6**，收口 PRD「1.0.0 双轨 P0」与 deferred-work / 迁移清单缺口。
---

# newspace — Epic Breakdown

## Overview

本文件将 **PRD**、**architecture.md**（完整架构工作流产出）与 **UX Design Specification** 中的需求分解为 **史诗与用户故事**，供实现与排期使用。线框参考：`ux-design-directions.html`（非契约正文）。

## Requirements Inventory

### Functional Requirements

FR1: 用户可以在 **聊天主页** 通过 **大脑皮层** 控件 **启用或停用** 蜂群层。  
FR2: 系统在 **大脑皮层启用** 时，能够在用户任务 **进行过程中** 在 GUI 提供 **非仅最终文本** 的过程反馈（阶段/事件类，最小一种即可）。  
FR3: 系统在 **大脑皮层停用** 时，行为符合 **本文档或链接的实现说明** 中定义的 **简化模式**（须可测）。  
FR4: 用户可以在 **不切换应用** 的情况下，感知当前处于 **大脑皮层开或关** 状态。  
FR5: 用户可以从主导航进入 **神经系统** 视图（替换原「即将推出」占位）。  
FR6: 神经系统视图能够展示 **与当前会话或运行时相关的** 连接/活动或 **占位+说明**（v0 允许部分 stub，须标明数据阶段）。  
FR7: 用户可以在神经系统或关联界面获得 **排障线索**（例如空闲、错误、未推进），与聊天记录互补。  
FR15: 在 **MVP** 内，用户从主导航进入 **神经系统** 后，首屏须为 **DIVA 大脑** 的 **架构图式** 主视图，并包含 **左右两个可区分区域**（命名与语义见实现文案，须 **非空标签**）；**不得** 将「游戏化总控台」或「多角色忙碌场景」作为 MVP 必经首屏。  
FR16: **游戏化总控台优先入口**、**《头脑特工队》式多角色忙碌** 等体验 **不属于** MVP 验收范围；若实现中做占位，须 **明确标注为愿景/后续** 且 **不阻断** FR5–FR7、FR15 的 MVP 路径。  
FR8: 系统在用户可见渠道上维持 **单一 Person 叙事线**（不出现多个并列「机器人头像」式独立对话流）。  
FR9: 系统在 **大脑皮层启用** 时仍满足 FR8，对内协作 **不** 以多用户可见聊天室形式暴露。  
FR10: 用户可以通过 **既有或随 MVP 提供的入口** 管理 **skills 或能力包**（与 agent-diva 当前设置能力衔接）。  
FR11: 系统能够 **加载并校验** 用户提供的 **能力声明**（v0 字段子集，错误时可见反馈）。  
FR12: 系统能够在 **无 GUI** 场景下（测试或 headless）验证 **大脑皮层开/关** 与核心编排分支。  
FR13: GUI 仅通过 **已文档化的 API/事件** 获取蜂群状态与过程数据，**不** 将编排核心逻辑仅实现于前端。  
FR14: 系统能够将 **大脑皮层状态** 与 **gateway/后端** 状态保持 **单一真相源**（无长期双端不一致）。  
FR17: 维护者可以查阅 **与 MVP 对齐** 的说明，解释 **关大脑皮层** 的语义与边界。  
FR18: 系统或工具链支持 **基本的自检/诊断输出**（随 diva 主线 `doctor` 或等价物演进；**可扩展挂点** 而非一次做全）。  
FR19: 轻量类意图 **不得** 在无显式用户选择下启动 **完整多参与者蜂群编排**；须 **可完成** 轻量路径 + 文档化超时/步数上限内 **结果或显式失败原因**。  
FR20: 大脑皮层启用且走蜂群编排时须有 **内置收敛策略**（最大内部轮次或等价预算、**done**），禁止无终止「思考—推翻—再思考」为默认。  
FR21: 用户或配置可 **显式强制轻量路径**（可与皮层 OFF 合并或独立策略，实现二选一须在链接文档中冻结）。  
FR22: 提供 **成本/用量可观测性挂点**（内部步数/阶段计数、超建议预算的开发者向警告等）；MVP 不要求完整计费面板。

### NonFunctional Requirements

NFR-P1: 切换大脑皮层须在用户可感知范围内 **即时完成**（目标 **500ms 内** 状态切换与首帧反馈，网络与模型调用除外）；不长时间阻塞 UI 线程。  
NFR-P2: 过程可视更新 **不应** 单独拖垮主聊天流式；事件批处理或节流策略须在实现中说明。  
NFR-P3: 编排默认值须 **可接受延迟与调用次数**；**禁止** 将 **无上限内部多轮对话** 作为 **唯一** 完成手段（与 FR19–FR21 一致）。  
NFR-S1: API Key、令牌与本地配置遵循 **agent-diva 既有安全实践**；本增量不引入明文密钥新暴露面。  
NFR-S2: 黑板/内部事件等 **不可信输入** 进入模型或合成路径前须 **限长/白名单/校验**。  
NFR-I1: 新增事件与 DTO **不得破坏** 现有 LLM provider、MCP、channels 配置的 **向后兼容**（除非显式大版本）。  
NFR-I2: 对外集成面保持 **可列清单**（白名单事件/字段），避免「general」域无限膨胀。  
NFR-R1: **大脑皮层** 切换失败时须 **可恢复**（回滚到上一稳定状态或显式错误），**不** 静默进入未知模式。  
NFR-R2: 内部 trace 与用户 transcript **默认分轨**；调试内容 **不** 默认混入用户可见记录。  
NFR-A1: 大脑皮层控件与神经系统入口具备 **可命名、可键盘操作**（遵循现有 GUI 组件基线）。

### Additional Requirements

（来自 `architecture.md` 及棕地约束，影响故事切分与验收。）

- **棕地基线：** 不采用绿场 starter；在 **既有 agent-diva workspace** 内增量；新蜂群 crate **单向依赖** 现有 `agent-diva-*`。  
- **ADR-A：** 编排 **swarm** 实现 crate **不得**依赖 `agent-diva-meta`；Meta 仅在 runtime/gateway **组合层** 边界触发。  
- **真相源：** 「大脑皮层」状态、过程事件、神经系统快照的 **权威在 Rust**；GUI 仅 **Tauri command / 事件** 消费。  
- **MVP 数据面：** 黑板 **进程内内存**；不引入本特性专用新 DB。  
- **契约：** DTO **serde 可版本化**；字段/事件 **白名单**（NFR-I2）；变更同步 CHANGELOG/ADR。  
- **实现顺序建议（架构文档）：** (1) 版本化契约 + 无 GUI 测试 → (2) Rust 真相源 → (3) Tauri 桥接 → (4) Vue 最小 UI。  
- **待决（实现首迭代）：** 蜂群 crate 物理路径（workspace member vs 顶层 `path`）；过程事件 **推送 vs 轮询** 在实现 ADR 中二选一封死。  
- **PersonOutbox / SteeringLease：** 用户可见写路径须满足单一对外出口语义（与 swarm 设计文档一致）。  
- **CI 可选：** `cargo tree` 断言 swarm 不依赖 meta。

### UX Design Requirements

**与 `ux-design-specification.md`「UX Decision Register」对齐（产品决策）：**

UX-DR1: **大脑皮层** 在聊天主路径 **一眼可辨**（FR4）；图标 + 短标签或 `aria-label`（i18n）；`role="switch"` 或 `aria-pressed`；键盘与 loading/error（`CortexToggle`）。  
UX-DR2: **神经系统** 与 **中控台** **概念分坑**；侧栏 **不同图标 + 不同 i18n key**（`NervousSystemView`）。  
UX-DR3: **过程反馈** **不挡输入、不挡流式**；可折叠；节流/合并（NFR-P2）。  
UX-DR4: **轻量意图** 下界面传达 **会收敛**：**终态**（成功 / 明确失败或触顶说明）；禁止仅靠无尽阶段动画（FR19–FR20）；`ProcessFeedbackStrip` 的 `capped` / `lightweight` 态与后端事件一致。  
UX-DR5: **成本/用量** MVP 以 **开发者向挂点** 为主（设置、折叠调试区）；非阻断琥珀提示可表示超建议预算；完整计费 UI Post-MVP（FR22）。

**实现约定（避免与 DR 编号冲突，Epic AC 引用用 UX-IMPL-*）：**

UX-IMPL-1: 新蜂群相关 Vue 组件目录 **`src/components/swarm/`**（或项目既定 feature）；数据 **仅经 Tauri / gateway**，禁止组件内伪造长期皮层状态（对齐 FR13）。  
UX-IMPL-2: **BrainOverview（MVP）：** 左右分区 + 架构图式；非空语义标签；可选首入 hint；键盘切换分区（可选）。  
UX-IMPL-3: **NeuroDetailPanel：** 空/错/闲模板化文案 + 建议动作（FR7）。  
UX-IMPL-4: **Stub 诚实标注：** `DataPhaseBadge` 或等价（FR6）。  
UX-IMPL-5: **视觉 token：** `neuro-*`、`process-muted` 等 CSS/Tailwind 扩展。  
UX-IMPL-6: **动效：** MVP 少动画；`prefers-reduced-motion`（规格 Phase 3）。  
UX-IMPL-7: **设计系统：** Tailwind + Lucide + vue-i18n；MVP 不引入整套新 UI kit。

### FR Coverage Map

| FR | Epic | 说明 |
|----|------|------|
| FR1 | E1 + E2 | 状态后端 + 聊天主页 UI |
| FR2 | E1 + E2 | 事件源 + 过程 UI |
| FR3 | E1 | 简化模式语义与无头测试 |
| FR4 | E2 | 状态可感知（UX-DR1） |
| FR5 | E3 | 路由与壳，替换占位 |
| FR6 | E3 | 数据/stub + UX-IMPL-4 |
| FR7 | E3 | 空错闲线索（UX-IMPL-3） |
| FR15 | E3 | 大脑架构图首屏左右分区 |
| FR16 | E3 | 非 MVP 愿景占位不得阻断 MVP 路径 |
| FR8 | E4 | 单一叙事验收 |
| FR9 | E4 | 皮层开时仍单一叙事 |
| FR10 | E4 | skills/能力入口 |
| FR11 | E1 + E4 | 校验核心 + UI 反馈 |
| FR12 | E1 | 无 GUI 测试 |
| FR13 | E1 + E2 | 契约 + 消费侧 |
| FR14 | E1 | 单一真相源 |
| FR17 | E4 | 关皮层语义文档 |
| FR18 | E4 | doctor 挂点 |
| FR19 | E1 | 意图/执行分层，轻量路径默认 |
| FR20 | E1 | 收敛策略、StopReason、触顶事件 |
| FR21 | E1 | 强制轻量路径（与 FR3 文档交叉引用） |
| FR22 | E1 + E2 + **E5** | 遥测 DTO/命令 + `RunTelemetryHint`；**E5** 与蜂群内部步语义统一 |
| NFR-P3 | E1 + **E5** + **E6** | 与 FR20 同源；**E5** 序曲预算；**E6** Timeout/Light enforcement 真实路径 |
| **1.0.0 双轨 P0** | **E5 + E6** | PRD 冻结：Swarm-类序曲/SPI + Shannon-类超时/诊断/一致性与 MIG；**E6.7** 发布勾选 |

## Epic List

### Epic 1：可控蜂群模式（大脑皮层语义与运行时契约）

用户在 **开/关大脑皮层** 时获得 **一致、可测、与 gateway 同步** 的行为；向 GUI 暴露 **稳定、已文档化** 的契约；满足 **轻量路径、收敛与用量挂点**（FR19–FR22、NFR-P3）及无头验证与架构边界。

**FRs covered:** FR3, FR11（校验核心）, FR12, FR13（契约定义）, FR14, FR1（状态侧）, FR2（事件发射侧）, **FR19, FR20, FR21, FR22（后端/契约侧）**, **NFR-P3**

### Epic 2：聊天主页 — 大脑皮层与过程可视

用户在 **聊天主页** 操作开关并 **感知状态**；任务进行中看到 **最小过程反馈**；**触顶/轻量** 态与 **开发者用量提示** 与后端一致（UX-DR4、UX-DR5）；满足 NFR-P1、P2、A1 与 UX-DR1、UX-DR3。

**FRs covered:** FR1（UI）, FR2（UI）, FR4, FR13（消费侧）, **FR22（UI 挂点）**, NFR-P1, NFR-P2, NFR-A1（皮层控件）

### Epic 3：神经系统观测视图（MVP 分期）

用户进入 **神经系统** 全屏视图：**MVP 首屏** 为 **架构图式大脑 + 左右分区**；可见 **真实或诚实 stub** 快照与 **排障线索**；**不得** 以游戏化总控台为必经首屏（FR5–FR7、FR15、FR16；UX-DR2、UX-IMPL-2–UX-IMPL-4）。

**FRs covered:** FR5, FR6, FR7, FR15, FR16

### Epic 4：Person 体验、能力与可维护性

维持 **单一 Person 叙事**；衔接 **skills/能力包** 与校验反馈；交付 **关皮层说明** 与 **诊断挂点**（FR8–FR11 用户侧、FR17、FR18；及需在界面/文档体现的 NFR）。

**FRs covered:** FR8, FR9, FR10, FR11（UI 反馈）, FR17, FR18, NFR-S1, NFR-I1, NFR-I2, NFR-R1, NFR-R2, NFR-A1（神经入口）, NFR-S2（若 UI 暴露内部摘要时的策略）

### Epic 5：蜂群编排加深（Correct Course 阶段 B）

在 **阶段 A**（AgentLoop 接入 `ExecutionTier`、固定双角色序曲、显式深度 API/GUI）之上，把蜂群做成 **可配置、可计量、可演进** 的产品能力：序曲 **角色/轮次/预算** 可配置；**FR22** 与过程事件对 **内部步数** 有统一语义；**handoff** 有明确检查点语义（为可恢复打基础）；**编排适配 SPI** 文档化，便于后续接 Shannon / openai-agents-python 式运行时 **而不破坏 ADR-A**。

**FRs covered:** FR19（显式深度与轻量边界延续）, FR20（收敛与预算在真实 turn 上可配置、可测）, FR22（用量/内部步数与 UI/ doctor 挂点一致）, NFR-P3, NFR-I2, NFR-S2  
**依赖：** Epic 1–2 已交付；与 `sprint-change-proposal-2026-03-31.md` 阶段 B 对齐。

### Epic 6：V1.0.0 双轨 P0 收口与发布就绪

收口 **PRD「Version roadmap & 对标边界」** 中 **1.0.0** 尚未被 Epic 1–5 **单独覆盖**的缺口：Shannon-类 **超时/轻量 enforcement、doctor 真数据、皮层双端一致**；Swarm-类 **spawn 路径与 Capability / Person 出口** 的工程化；以及 **可勾选发布清单** 文档。  
**不包含：** OMC 向、下一代自有栈（PRD 已冻结为 1.0.0 之后）。

**FRs covered:** FR12, FR14, FR18, FR19, FR20, FR21, FR22, FR8/FR9（MIG-02）, NFR-P3, NFR-I2, NFR-R2, NFR-S2  
**依赖：** Epic 5 中与 1.0.0 强相关的故事（5.1–5.4）**建议**先于或并行 6.x 中的编排项；6.x 可与 5.x **并行**由不同开发者负责时须约定集成顺序。

---

## Epic 1: 可控蜂群模式（大脑皮层语义与运行时契约）

**目标：** 在后端/测试中确立 **大脑皮层开/关** 的 **单一真相源**、**简化模式语义**、**最小过程事件**、**执行分层与收敛**、**运行遥测挂点**，并向 GUI 暴露 **文档化契约**，且不破坏现有 gateway 配置。

### Story 1.1: 蜂群 crate 骨架接入 workspace

As a **维护者**,  
I want **在 agent-diva workspace 中新增蜂群相关 crate 并能被依赖编译**,  
So that **后续故事可在清晰边界内增量实现**。

**Acceptance Criteria:**

**Given** 当前 agent-diva 多 crate workspace  
**When** 添加新 crate（名称以实现为准）并声明 **单向依赖** 现有基础 crate  
**Then** `cargo build` / CI 通过且 **无循环依赖**  
**And** README 或 crate 级说明指向 PRD、`architecture.md` 与 ADR-A（swarm 实现 **不** 依赖 `agent-diva-meta`）

---

### Story 1.2: 大脑皮层状态模型与持久化边界

As a **运行时**,  
I want **在会话/进程边界内维护 Cortex（大脑皮层）开/关状态**,  
So that **gateway 与后续 GUI 能查询一致状态**。

**Acceptance Criteria:**

**Given** 一次用户会话或等价生命周期  
**When** 切换开/关（API 或测试钩子）  
**Then** 状态可查询且 **符合 PRD 定义的真值**（含默认值）  
**And** 持久化范围（仅内存 / 本地存储）在实现说明中写明并与 FR14 一致

---

### Story 1.3: Gateway 与蜂群状态同步契约

As a **集成方**,  
I want **GUI 与 gateway 通过同一契约读写大脑皮层状态**,  
So that **不存在长期双端漂移（FR14）**。

**Acceptance Criteria:**

**Given** 已定义 DTO 或 Tauri command / HTTP 接口（与现有 gateway 模式对齐）  
**When** 从 GUI 或测试客户端切换状态  
**Then** gateway 侧查询结果与请求一致  
**And** 契约写入 `docs` 或 crate `README` 片段，满足 FR13；DTO 含 **版本字段**（与 `architecture.md` 实现模式一致）

---

### Story 1.4: 「关大脑皮层」简化模式 — 行为与无头测试

As a **开发者**,  
I want **在无 GUI 的测试中验证关模式下行走路径**,  
So that **FR3 与 FR12 可回归**。

**Acceptance Criteria:**

**Given** 测试环境固定模型/桩  
**When** 大脑皮层为 **关** 并触发一轮标准对话/工具路径  
**Then** 行为符合 **实现说明中登记的简化语义**  
**And** 至少 **一条** 自动化测试失败时能指明「开/关」分支错误

---

### Story 1.5: 最小过程事件发射（供 UI 订阅）

As a **GUI（后续 Epic）**,  
I want **在后端发出可订阅的最小过程事件**（阶段或工具起止等）,  
So that **FR2 的数据源存在**。

**Acceptance Criteria:**

**Given** 大脑皮层为 **开** 且执行带工具或分阶段的请求  
**When** 运行时推进  
**Then** 事件总线或流上可出现 **至少一种** 非「仅最终文本」的条目  
**And** 事件 **白名单** 与 schema 稳定（NFR-I2）；过量事件可有 **节流** 钩子（NFR-P2）

---

### Story 1.6: 能力声明 v0 校验（核心）

As a **系统**,  
I want **解析并校验 v0 能力 manifest（最少字段）**,  
So that **FR11 在服务端成立且错误可向上返回**。

**Acceptance Criteria:**

**Given** 合法与非法 manifest 样例  
**When** 加载  
**Then** 非法样例返回 **可机读错误**；合法样例进入注册表或占位结构  
**And** 与 Epic 4 的 UI 反馈衔接；不阻塞 Epic 1 其他故事

---

### Story 1.7: 执行分层与轻量路径路由（FR19）

As a **运行时**,  
I want **在每次用户 turn 选择 Light 与 FullSwarm 等执行分层**，  
So that **轻量意图不会在无显式选择下启动完整多参与者蜂群（FR19）**。

**Acceptance Criteria:**

**Given** 实现说明中 **单文件维护** 的意图判定规则（显式 skill 调用、短问答清单等，可迭代）  
**When** 用户输入落在 **轻量类** 且 **未** 显式选择「深度/全蜂群」  
**Then** 编排 **不** 进入完整多代理 handoff/对弈图；走 **可完成** 的轻量路径  
**And** 在文档化 **超时或内部步数上限** 内交付 **结果或显式失败原因**  
**And** 至少 **一条** 无 GUI 测试：轻量输入 + 皮层 **开** 仍 **不** 启全图（与 PRD 旅程五 对齐）

---

### Story 1.8: 收敛策略与终局语义（FR20、NFR-P3）

As a **开发者**,  
I want **蜂群路径具备最大内部轮次（或等价预算）与完成定义**，  
So that **编排不会无限「思考—推翻」且满足 NFR-P3**。

**Acceptance Criteria:**

**Given** `ConvergencePolicy`（或等价）在代码或配置中 **有默认值** 并写入维护者文档  
**When** 皮层 **开** 且走 **FullSwarm**（或等价命名）路径  
**Then** 每步检查预算；触顶时产生 **`StopReason`**（如 `Done` | `BudgetExceeded` | `Timeout` | `Error`）并 **emit 白名单事件**（如 `swarm_run_capped` / `swarm_run_finished`）  
**And** **禁止** 仅依赖无上限内部对话作为唯一完成手段  
**And** 无 GUI 测试覆盖 **触顶路径** 至少一种

---

### Story 1.9: 强制轻量路径与 FR21 冻结文档

As a **维护者**,  
I want **在实现说明中冻结「强制轻量」与「大脑皮层 OFF」是合并还是独立策略**，  
So that **FR21 可测且与 FR3 不打架**。

**Acceptance Criteria:**

**Given** ADR 或 `docs` 片段 **二选一并写明**（合并 vs 独立配置位）  
**When** 策略为 **ForceLight**（命名以实现准）  
**Then** 编排 **不** 为多视角对弈链预留额外模型回合，直至用户显式升级本次任务  
**And** Story 1.4 / 4.3 **交叉引用** 该文档；至少 **一条** 测试或清单项验证 ForceLight（若独立于 OFF）

---

## Epic 2: 聊天主页 — 大脑皮层与过程可视

**目标：** 用户在主聊天界面 **操作并理解** 大脑皮层开关，并在启用时看到 **过程反馈**；**轻量/触顶** 与 **用量提示** 符合 UX-DR4、UX-DR5；满足 NFR-P1、P2、A1 与 UX-DR1、UX-DR3。

### Story 2.1: 聊天主页大脑皮层开关 UI（CortexToggle）

As a **用户**,  
I want **在聊天主页用带大脑图标的开关控制蜂群层**,  
So that **我能按任务深浅切换模式（FR1、FR4）**。

**Acceptance Criteria:**

**Given** 主聊天界面已加载  
**When** 点击或键盘切换  
**Then** 调用 Story 1.3 契约且 UI **立即反映** 新状态（目标满足 NFR-P1 量级）  
**And** 满足 UX-DR1：`aria`/i18n、**focus-visible**；符合 NFR-A1

---

### Story 2.2: 开关与 gateway 错误处理

As a **用户**,  
I want **同步失败时看到明确错误而非静默错误状态**,  
So that **符合 NFR-R1**。

**Acceptance Criteria:**

**Given** 模拟 gateway 拒绝或超时  
**When** 用户切换大脑皮层  
**Then** 显示错误提示且 **回滚或保持上一稳定状态**  
**And** 不进入「未知开/关」中间态

---

### Story 2.3: 过程反馈最小 UI（ProcessFeedbackStrip）

As a **用户**,  
I want **在大脑皮层开启时看到进行中的过程反馈**,  
So that **我不必只盯着最终回复（FR2）**。

**Acceptance Criteria:**

**Given** 大脑皮层 **开** 且后端发出 Story 1.5 事件  
**When** 任务进行中  
**Then** 展示 **至少一种** 过程反馈（时间线、步骤条等择一），且 **可折叠**  
**And** **不遮挡** 主输入与流式（UX-DR3、NFR-P2）；可选用 UX-IMPL-5 token  
**And** 订阅 Story 1.8 的终局/触顶事件：展示 **`lightweight`**（极简阶段）或 **`capped`** 态，且 **主对话区** 须有 **可读系统说明**（UX-DR4）

---

### Story 2.4: 运行用量提示与开发者挂点（FR22、UX-DR5）

As a **进阶用户 / 开发者**,  
I want **在可选位置看到本次运行的内部步数或预算提示**,  
So that **诊断「为何烧 token」不全黑盒（FR22）**。

**Acceptance Criteria:**

**Given** Story 1.x 暴露的 **`RunTelemetrySnapshot`**（或等价 DTO）经 Tauri **dev 命令、设置子区或折叠诊断抽屉** 之一（实现冻结一种）  
**When** 用户打开该挂点（默认 **不** 打扰主聊天）  
**Then** 可见 **内部步数/阶段计数** 或 **超建议预算** 的 **琥珀非阻断** 提示  
**And** 实现可选用独立组件 **`RunTelemetryHint`**（见 UX 规格）；feature-flag 控制可接受  
**And** 不将内部 trace **默认写入** 用户 transcript（NFR-R2）

---

## Epic 3: 神经系统观测视图（MVP 分期）

**目标：** 替换 `NormalMode` 占位；**MVP 首屏** 满足 **大脑架构图 + 左右分区**；**真实或诚实 stub**；排障线索；**FR16** 愿景不得阻断 MVP（UX-DR2、UX-IMPL-2–UX-IMPL-4）。

### Story 3.1: 神经系统路由与壳页面（NervousSystemView）

As a **用户**,  
I want **从侧栏进入神经系统全屏视图**,  
So that **不再看到永久「即将推出」空壳（FR5）**。

**Acceptance Criteria:**

**Given** 侧栏 `neuro`（或最终命名）入口  
**When** 点击进入  
**Then** 渲染专用 Vue 组件占满主内容区，含 **返回聊天**  
**And** i18n 与 **中控台** 入口 **图标+key 区分**（UX-DR2）  
**And** **首屏子视图** 为 **BrainOverview**：**左右两分区**、**架构图式**、分区 **非空产品语义标签**（FR15、UX-IMPL-2）；**不得** 将游戏化总控台或多角色忙碌作为 **进入神经后的必经首屏**（FR16）

---

### Story 3.2: BrainOverview + 分区详情与数据阶段

As a **用户**,  
I want **点击分区看到连接/活动或诚实占位**,  
So that **我了解系统内在干什么（FR6）**。

**Acceptance Criteria:**

**Given** 后端快照 API 或 stub 标志  
**When** 选中左/右分区  
**Then** **NeuroDetailPanel** 展示列表或 **带数据阶段说明的占位**（UX-IMPL-4）  
**And** 与 Story 1.5 / gateway **同源**，不与聊天区矛盾

---

### Story 3.3: 排障线索与空/错态

As a **用户**,  
I want **在异常或空闲时获得可行动提示**,  
So that **我能决定重试或关大脑皮层（FR7）**。

**Acceptance Criteria:**

**Given** 无活动、工具错误或阶段卡住  
**When** 视图刷新  
**Then** 显示 **模板化空/错/闲文案** 与建议下一步（UX-IMPL-3）  
**And** 不重复堆砌整段原始日志

---

### Story 3.4: 愿景占位（可选）与 MVP 路径隔离

As a **产品**,  
I want **若存在游戏化/总控台占位，明确标注「后续」且不阻断 MVP**,  
So that **满足 FR16**。

**Acceptance Criteria:**

**Given** 实现中若加入非 MVP 入口或占位卡片  
**When** 用户走 MVP 路径（侧栏 → 神经 → BrainOverview）  
**Then** **无需** 经过该占位即可完成 FR5–FR7、FR15  
**And** 占位文案/i18n **标明愿景/非验收**

---

## Epic 4: Person 体验、能力与可维护性

**目标：** **单一 Person 叙事**（FR8–FR9）；**skills/能力包** 与校验反馈（FR10–FR11）；**文档与诊断**（FR17–FR18）。

### Story 4.1: Person 单一叙事回归测试 / 走查清单

As a **产品/QA**,  
I want **可重复的检查单或自动化断言**,  
So that **用户可见界面不出现多机器人并列流（FR8、FR9）**。

**Acceptance Criteria:**

**Given** 大脑皮层 **开/关** 各至少一条典型路径  
**When** 执行检查  
**Then** **无** 多个并列「独立 agent 聊天头像」式通道  
**And** 结果记录为测试或手动清单链接

---

### Story 4.2: Skills/能力包管理与校验反馈（UI）

As a **进阶用户**,  
I want **在设置或既定入口管理 skills/能力并看到校验错误**,  
So that **FR10、FR11 在用户侧闭环**。

**Acceptance Criteria:**

**Given** 用户提交或编辑能力声明（与现有 Settings 对齐）  
**When** 校验失败  
**Then** 展示 **明确错误**（字段级或文件级）  
**When** 成功  
**Then** 列表或状态更新，且与 Story 1.6 一致

---

### Story 4.3: 「关大脑皮层」语义说明文档

As a **维护者**,  
I want **仓库内有一份与 MVP 对齐的说明**,  
So that **FR17 满足**。

**Acceptance Criteria:**

**Given** `docs` 或 crate 文档目录  
**When** 阅读该文档  
**Then** 明确 **关** 模式下行走路径、限制及与 **开** 模式差异  
**And** 与 Story 1.4 测试语义 **交叉引用**

---

### Story 4.4: 诊断（doctor）扩展挂点

As a **开发者**,  
I want **doctor 或等价命令能打印蜂群相关诊断块**,  
So that **FR18 与 NFR-R2 精神落地**。

**Acceptance Criteria:**

**Given** 现有或规划中的 `doctor` 入口  
**When** 运行带蜂群标志或子命令  
**Then** 输出 **大脑皮层状态、注册能力数或错误摘要** 中至少一项  
**And** 内部 trace **不** 默认写入用户 transcript（NFR-R2）

---

## Epic 5: 蜂群编排加深（阶段 B）

**目标：** 将 **FullSwarm 序曲** 从硬编码双代理升级为 **可声明配置**；将 **蜂群内部轮次/阶段** 与 **RunTelemetry / FR22** 对齐；定义 **handoff 检查点** 语义（MVP 可先 turn 内）；产出 **外部编排器 SPI（文档 + Rust trait 边界）**，为 Shannon / Agents SDK 留出接入点。

### Story 5.1: 蜂群序曲角色与预算可配置

As a **维护者 / 进阶用户**,  
I want **用配置文件（或 workspace 清单）声明 FullSwarm 序曲的角色列表、系统提示模板、最大序曲轮次与每轮 token/调用预算**,  
So that **成本与行为可预测（FR20、NFR-P3），且不必改代码即可调参**。

**Acceptance Criteria:**

**Given** 一份版本化的配置（路径与格式在实现中冻结并写入 README）  
**When** 启动 gateway / 桌面壳并触发 FullSwarm turn  
**Then** 序曲按配置执行（至少支持 **禁用序曲**、**单角色**、**多角色链** 三档之一；默认与当前阶段 A 行为 **向后兼容**）  
**And** 超预算时产生 **可观测终局或显式失败原因**（与 FR20 一致），且 **不** 无限重试

---

### Story 5.2: 蜂群内部步与 RunTelemetry / FR22 统一

As a **开发者 / 排障用户**,  
I want **一次 FullSwarm turn 的「内部 LLM 步数 / 阶段计数」在 RunTelemetry、过程事件与 doctor 挂点中语义一致**,  
So that **FR22 可观测；用户不会在「界面显示」与「日志」之间看到矛盾数字**。

**Acceptance Criteria:**

**Given** 一次含序曲 + 主循环的 FullSwarm turn  
**When** 收集 `RunTelemetrySnapshot`、白名单过程事件与（若启用）doctor 输出  
**Then** **内部步数或等价计数** 字段 **同源**（单一累加器或明确换算表写在文档）  
**And** 轻量路径 **不** 因本故事引入额外噪声计数（FR19 不变）

---

### Story 5.3: Handoff 检查点语义（turn 内可恢复基线）

As a **运行时**,  
I want **在 FullSwarm 序曲链中定义可序列化的「检查点」边界（例如每角色输出摘要 + 轮次 id）**,  
So that **在可恢复失败时可解释进度（用户取消的 v0 边界见验收标准），并为跨 turn 恢复留扩展点**。

**Acceptance Criteria:**

**Given** 序曲中某一角色发生 **可恢复失败**（超时、模型/API 错误）  
**When** 运行时中止序曲并在日志中暴露 **最后成功检查点**（若此前至少一步成功；字段见 `agent-diva-swarm/docs/handoff-checkpoint-v0.md`）  
**Then** 文档与实现一致说明：**MVP 是否支持自动续跑** 或 **仅报告状态**（二选一封冻）  
**And** 检查点载荷 **限长、可审计**（NFR-S2），**不** 默认进入用户 transcript（NFR-R2）  
**And（v0 边界）** **用户取消** 在序曲串行 `chat` 尚未返回时的检查点日志 **不** 纳入书面保证；取消轮询在主 ReAct 循环（见该文档「用户取消」节）

---

### Story 5.4: 编排适配 SPI 与 Shannon / Agents 路线评估

As a **架构师 / 维护者**,  
I want **一份 ADR + 最小 Rust trait（或端口）定义「外部编排入口」**，  
So that **未来可接 Shannon、Python Agents 或独立进程，而不让 swarm crate 违反 ADR-A（不依赖 meta）**。

**Acceptance Criteria:**

**Given** 仓库内 ADR（路径实现定）  
**When** 阅读 SPI 文档  
**Then** 明确：**输入**（用户 turn 上下文句柄）、**输出**（结构化阶段/完成信号）、**与 `ExecutionTier::FullSwarm` 的调用边界**  
**And** 附 **一页** 对比：嵌入式 Python vs 独立编排进程 vs 纯 Rust —— **推荐默认与取舍理由**（不要求本故事完成任一外部集成）

---

## Epic 6: V1.0.0 双轨 P0 收口与发布就绪

**目标：** 使 **1.0.0** 可按 PRD **双轨 P0** 勾选验收：补齐 **deferred-work.md** 与 **subagent-to-swarm-migration-inventory** 中已登记的缺口；交付 **发布清单附件**（PRD ↔ 实现/测试引用）。

### Story 6.1: 收敛循环墙钟/外部超时与 Timeout 终局可观测

As a **开发者**,  
I want **在真实编排路径上能为 `execute_full_swarm_convergence_loop`（或等价）注入墙钟/外部超时并产生 `SwarmRunStopReason::Timeout` + `swarm_run_finished`**,  
So that **FR20 / ADR-E 的 Timeout 语义不是「仅有枚举无路径」（关闭 Story 1.8 评审 deferred）**。

**Acceptance Criteria:**

**Given** 可配置或测试注入的超时（具体挂点以实现 + ADR 为准）  
**When** 超时在收敛或序曲执行中触发  
**Then** 可观测 **Timeout** 终局（过程事件或等价）且 **有** 自动化测试覆盖  
**And** 与 `BudgetExceeded` / `Done` 的优先级在文档中单点说明

---

### Story 6.2: Light 路径在真实 AgentLoop 上的步数/墙钟 enforcement

As a **用户**,  
I want **轻量路径在真实对话循环中遵守已文档化的内部步数与/或墙钟上限**,  
So that **FR19「可完成或显式失败」在主线成立，而非仅 headless 桩（关闭 Story 1.7 评审 deferred）**。

**Acceptance Criteria:**

**Given** `ExecutionTier::Light` 且皮层状态按 FR19 路由  
**When** 主循环超过 `LIGHT_PATH_MAX_INTERNAL_STEPS` 或等价墙钟  
**Then** turn **终止**并返回 **可对用户说明的失败/触顶原因**（不必完整 GUI，契约清晰）  
**And** 至少 **一条** 集成或单元测试覆盖「Light 触顶」

---

### Story 6.3: Doctor 与真实 Capability 注册表接线

As a **开发者**,  
I want **`doctor`（或等价）在 gateway/宿主上下文中可接收真实 `CapabilityRegistry`（或等价）并打印非占位计数/错误摘要**,  
So that **FR18 与 Shannon-类「可诊断」P0 成立（关闭 Story 4.4 评审 deferred）**。

**Acceptance Criteria:**

**Given** 运行中的 manager 或 CLI 子命令可访问与 GUI 提交 **同源** 的 registry  
**When** 执行带蜂群/能力标志的 doctor  
**Then** 输出 **注册能力数或校验错误摘要** 中至少一项为 **真实值**  
**And** NFR-R2：内部 trace **不** 默认混入用户 transcript

---

### Story 6.4: 皮层状态 GUI 与 Gateway 权威源一致性

As a **用户**,  
I want **聊天顶栏皮层开关所反映的状态与 gateway 内 `CortexRuntime` 不产生长期错觉性漂移**,  
So that **FR14 在「可见 UI」维度也成立（关闭 Story 2.3 评审 deferred）**。

**Acceptance Criteria:**

**Given** 用户仅通过 GUI 或仅通过 API 切换皮层  
**When** 在另一侧查询状态  
**Then** 在文档定义的同步窗口内 **一致**；若不同步须有 **可感知错误/recover**（对齐 Story 2.2 精神）  
**And** 验收用例写入 UX 或实现说明

---

### Story 6.5: Subagent 工具集 → Capability 可引用条目（MIG-01）

As a **维护者**,  
I want **`SubagentManager` 使用的工具列表以数据驱动方式对齐 Capability/注册表条目（行为可先不变）**,  
So that **迁移清单 SWARM-MIG-01 落地，为工具门禁与 Shannon 式 enforcement 铺路**。

**Acceptance Criteria:**

**Given** 现有子代理固定工具集  
**When** 查阅配置或注册表  
**Then** 每项工具 **有** 稳定 id / 元数据槽位（危险度等可占位）  
**And** `spawn` 与 FullSwarm 路径的边界在文档中 **交叉引用** PRD 1.0.0 Swarm P0

---

### Story 6.6: 子任务结果 internal vs person_visible 标注（MIG-02）

As a **产品**,  
I want **子代理/`spawn` 返回内容在管线中被显式标注为 internal 或 person_visible**,  
So that **单一 Person 叙事（FR8/FR9）可测，且与 PersonOutbox 长期设计一致（SWARM-MIG-02）**。

**Acceptance Criteria:**

**Given** 子任务完成并回传到主循环  
**When** 合成对外回复  
**Then** **仅** person_visible 内容进入用户可见 transcript；internal 默认 **不** 泄漏（NFR-R2）  
**And** 至少 **一条** 测试或断言防止回归

---

### Story 6.7: V1.0.0 发布清单附件（可勾选表）

As a **发布负责人**,  
I want **仓库内有一份可勾选清单，将 PRD 1.0.0 双轨 P0 映射到具体测试/文档/命令**,  
So that **标 1.0.0 前可机械核对，不得靠记忆**。

**Acceptance Criteria:**

**Given** `_bmad-output/planning-artifacts/release-checklist-v1.0.0.md`（或等价路径，单点冻结）  
**When** 逐项勾选  
**Then** 覆盖 **Swarm-类 P0** 与 **Shannon-类 P0** 摘要条目，并链接到 **Epic 5/6 故事或 `cargo test`/E2E**  
**And** PRD 正文「细则以发布清单附件为准」与本文件 **互链**

---

### Story 6.8: 三层 Subagent 架构示意图与文档索引（MIG-03）

As a **维护者**,  
I want **在 `agent-diva` 开发者文档中增加「运行时 / IDE / BMAD 技能」三层 subagent 示意图并链接迁移清单**,  
So that **SWARM-MIG-03 落地，降低再混淆风险**。

**Acceptance Criteria:**

**Given** `docs` 或 `agent-diva-swarm/docs` 中单页  
**When** 新贡献者阅读  
**Then** 能区分 **A 层 Rust**、**B 层 .cursor agents**、**C 层 BMAD 技能**  
**And** 链接到 `subagent-to-swarm-migration-inventory.md`

---

## Validation Summary（Step 4）

- **FR1–FR22** 均在 **Coverage Map** 与故事中体现；**FR19–FR22** 由 **1.7–1.9、2.4（及 1.5/1.8 事件衔接）** 覆盖；神经系统 **FR15/FR16** 由 Epic 3 与 Story 3.1、3.4 覆盖；**FR17/FR18** 由 Epic 4 文档/医生故事覆盖。  
- **NFR：** P1/P2 见 Epic 2；**P3** 见 Story 1.8 与架构 `ConvergencePolicy`；S1/S2 见 Epic 1.6、4 与架构约束；I1/I2 见契约与白名单；R1/R2 见 2.2、4.4；A1 见 2.1 与神经入口。  
- **UX-DR1–DR5：** 见 Epic 2（含 2.3、2.4）及 UX 规格 Register；**UX-IMPL-1–7** 在故事 AC 或组件目录约定中体现。  
- **故事依赖：** 同 Epic 内仅依赖 **序号更小** 的故事；**2.3、2.4** 依赖 **1.5、1.8** 及契约；Epic 2 依赖 Epic 1；Epic 3 依赖 Epic 1 数据/事件；Epic 4 的 **4.2** 依赖 **1.6**。  
- **反旅程（PRD 五、六）：** 以 **1.7 + 1.8 + 2.3** 与 **2.4** 的 AC 做走查/测试验收。  
- **Epic 5（阶段 B）：** 在 **Correct Course** 提案批准前提下，补齐 **可配置序曲、统一遥测、handoff 检查点、编排 SPI**；与 **FR19–FR22、NFR-P3** 交叉验收。  
- **Epic 6（1.0.0）：** 收口 **PRD 双轨 P0** 与 **deferred-work / MIG** 缺口；**Story 6.7** 为标 1.0.0 前的 **机械核对入口**；完成度与 **OMC / 下一代栈** 无关。  
- **Starter：** 不适用（棕地）；与 `architecture.md` **Starter Template Evaluation** 一致。  
- **架构合规：** ADR-A、单一真相源、DTO 版本字段、禁止 swarm→meta 依赖须在实现与 CI 中落实。

---

**工作流状态：** Epics & Stories 已含 **Epic 5（阶段 B）** 与 **Epic 6（V1.0.0 收口）**。建议：**5.1** 与 **6.1–6.4** 按依赖并行规划；**6.7** 在 5.x/6.x 接近完成时定稿勾选。**[IR] Check Implementation Readiness** 前优先跑 **6.7** 草案。
