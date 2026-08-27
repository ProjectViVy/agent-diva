# TODOLIST

项目级**活跃**待办。这里按 `L0 计划/大型 WBS → L1 工作流 → L2 事项` 分层，
只保留仍可执行、仍待决策或仍需验证的事项；完成、取消、被取代和重复记录统一进入归档目录
[`docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/README.md)。

严重度：`sev-P0` 阻断，`sev-P1` 高，`sev-P2` 中，`sev-P3` 低。

> **2026-08-23 收口**：总 EPIC「Laputa 认知工作区 Clean Break」关闭（S1–S6 与
> 真机桌面冒烟全部通过），积压的真机冒烟批次全部通过、修复已上主线。完成明细见
> [`completed-2026-08-23-real-device-smoke-batch.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-23-real-device-smoke-batch.md)。

## L0-WBS：WORKSPACE 系统收尾（已授权开工）

> **启动时间**：2026-08-26。**目标完成时间**：2026-09-07；2026-09-08 为风险缓冲，
> 不是默认扩展范围。实施必须使用隔离 worktree，并按每个 WS 切片单独认领锁、验证和提交。
> 本期核心对象是运行时 `WorkspaceContext`，不是散落在配置页中的路径字符串。

### L1-A：范围与冻结决策

- [ ] **WORKSPACE-SYSTEM-CLOSEOUT：统一工作区、AGENTS 合同与分层会话历史** `sev-P1`
  完成标志：运行时只有一个权威 `WorkspaceContext`；GUI 始终显示当前工作区及
  AGENTS.md 状态；切换遵循停止→保存→重建→恢复且失败不产生半切换；历史会话按
  `Workspace → Channel → Root Session → Branch/Subagent` 展示，并且层级来自持久化合同，
  不由标题或时间猜测。设计基线：
  [`gui-workspace-agents-design.md`](docs/research/workspace-agents-diva-adaptation-2026-08/gui-workspace-agents-design.md)。
  当前状态（2026-08-26）：WS-00～WS-05 的实现与自动化验证已完成；WS-06 仅剩真实桌面
  G2D+ smoke，故本总项暂不提前勾选完成。
  - [ ] 本期只查询当前活动 workspace 的 session authority；跨工作区全局历史索引另行立项。
  - [ ] 旧会话缺少 lineage 时按该频道的独立 root 展示，标记为 legacy，不伪造父子关系。
  - [ ] 禁止热换 `AppState` root、隐式迁移/复制会话、GUI 编辑 AGENTS.md、扫描未登记路径。
  - [ ] 路径优先级沿用已完成合同：CLI override → 显式配置 → 进程启动 CWD；旧私有默认值
    仅作为迁移来源。所有执行、Shell、Plan、Session 和 AGENTS 读取绑定同一 canonical root。

### L1-B：交付切片与依赖

#### WS-00：基线接管与后端合入（2026-08-27）

- [x] **WS-00-INTEGRATE-BACKEND：接管并合入 workspace-agents Wave A–D** `sev-P1` ✅ 2026-08-26
  从 `feat/workspace-agents-impl` 的 `8ccc82b2..d1264f3d` 做提交级审计，在基于最新 `dev`
  的新隔离 worktree 中重放/合并；不得直接在旧工作树续写。解决 TODOLIST 冲突后验证
  `WorkspaceContext`、Shell root 边界、`WorkspaceInstructions` digest/截断/安全包裹和
  `GET /api/workspace`。同时将 Manager endpoint 的 `source` 改为权威上下文投影，禁止
  根据 root 路径反推来源。已在 `feat/workspace-system-closeout` 新隔离 worktree 完成提交级
  重放；Manager/CLI/Tauri 的 Gateway bootstrap 统一传递 `WorkspaceContext`，`/api/workspace`
  直接投影 context source，并补充 process-cwd 反猜测测试。后续 WS-01 至 WS-06 依赖本切片。

#### WS-01：唯一 WorkspaceContext 与只读 GUI（2026-08-28）

- [x] **WS-01-WORKSPACE-CONTEXT-GUI：建立 GUI 唯一工作区快照** `sev-P1` ✅ 2026-08-26
  建立 workspace feature 模块和唯一 store/composable；聊天底栏 `WorkspaceChip`、Popover、
  Workspace Settings 与 AGENTS 摘要抽屉只消费该快照。移除 `GeneralSettings.vue` 对
  workspace 的重复 `getConfigStatus()` 回显，不在组件内重新解析路径。已接入 Tauri
  `get_workspace_status`、启动刷新、refresh generation 保护、上下文预算右侧 chip、只读 Workspace
  Settings 和 AGENTS 状态展示；覆盖 loading/refreshing/ready/error/legacy-default，旧响应
  不得覆盖新 workspace generation。验证记录见 `v0.1.2-workspace-gui-readonly`，提交记录见本分支
  Git history。2026-08-27 根据真机验收反馈将入口从 Topbar 移到聊天输入框底栏，弹层向上展开；
  未显式指定时显示“默认工作区”，仅 `configured` / `explicit-cli` 显示具体目录名；记录见
  `v0.1.8-workspace-chat-footer-entry`、`v0.1.9-workspace-default-label` 与
  `v0.1.10-workspace-default-fallback`。状态未返回或旧 Gateway 404 时也保持默认标签，
  不再把错误态显示成“工作区加载中”。
  依赖：WS-00。

#### WS-02：选择、预览与持久化（2026-08-31）

- [x] **WS-02-WORKSPACE-INSPECT：目录选择与候选预检** `sev-P1` ✅ 2026-08-26
  接入原生目录选择、canonicalize/可读性/AGENTS 状态 inspect 和确认页。候选路径只属于
  switch draft；提交前不得覆盖已生效 WorkspaceContext。取消或预检失败时保留当前工作区，
  连续选择只接受最新响应。已接入 Tauri 原生目录选择、候选 workspace ID/readability/AGENTS
  预检、Settings draft 预览与最新 generation 响应保护；预检失败、取消和未确认时保留当前
  workspace。验证记录见 `v0.1.5-workspace-inspect`。依赖：WS-01。

#### WS-03：原子切换事务（2026-09-01 ～ 2026-09-02）

- [x] **WS-03-WORKSPACE-SWITCH：停止→保存→重建→恢复单一切换流程** `sev-P1` ✅ 2026-08-26
  流式输出、Plan 执行、审批/HITL 等不安全状态下拒绝切换并说明原因；允许时先保存旧会话，
  停止旧 runtime，持久化目标，重建完整 runtime/AppState，再读取新 workspace 的最新 GUI
  会话。失败时恢复旧 committed context 或进入明确的可重试错误态，不得出现“UI 已切、
  runtime 未切”。已接入串行切换锁、运行态 guard、旧会话刷新、嵌入式 gateway 停止/重建、
  `/api/workspace` 目标校验、旧配置与 runtime 回滚、GUI 快照应用和新 workspace 历史重载；
  debug 外部 gateway 明确拒绝原子切换。验证记录见 `v0.1.6-workspace-atomic-switch`。
  依赖：WS-02。

#### WS-04：会话层级持久化与 API（2026-09-03）

- [x] **WS-04-SESSION-HIERARCHY-CONTRACT：扩展会话摘要与 lineage 合同** `sev-P1` ✅ 2026-08-26
  在 session authority/摘要/API DTO 中增加稳定 `workspace_id`、`channel`、
  `kind(root|branch|subagent|ephemeral)`、`root_session_key`、`parent_session_key` 和可选
  `branch_label`；创建 root/branch/subagent 时写入真实关系。旧 JSONL 采用只读兼容投影：
  channel 从完整 key 解析、kind=root、lineage 为空、legacy=true。API 默认仅返回活动
  workspace 集合，不把 GUI 的 `extractChatId()` 当业务身份。依赖：WS-00；可与 WS-02
  并行，但必须在 WS-05 前完成。已完成 SessionInfo DTO、JSONL lineage metadata、root
  创建与显式 child(parent/kind/label) 创建 seam，并覆盖 legacy 不猜 lineage；验证记录见
  `v0.1.3-session-hierarchy-contract`。

#### WS-05：历史会话分层 GUI（2026-09-04）

- [x] **WS-05-SESSION-HISTORY-TREE：按工作区、频道和 lineage 展示历史** `sev-P1` ✅ 2026-08-26
  `ConversationSidebar` 从平铺/仅置顶分组改为层级投影：当前 workspace header → channel
  group → root session → branch/subagent。列表只展示身份、标题、最后活动与必要状态；完整
  消息仍由详情区负责。搜索结果保留祖先路径，折叠/展开不改变服务端集合，active session
  在刷新和切换后仍可定位；pinned 是会话属性，不另造第二套 session 集合。已完成 workspace
  header、channel group、lineage indentation、本地折叠/展开、搜索祖先保留和 legacy root
  标识；依赖：WS-01、WS-04。验证记录见 `v0.1.4-session-history-tree`。

#### WS-06：纵向验收与收口（2026-09-07，缓冲 2026-09-08）

- [ ] **WS-06-E2E-CLOSEOUT：自动化、真机 smoke、文档与归档** `sev-P1`
  完成 Rust focused/full gates、GUI vitest/typecheck/build、Tauri/Manager 纵向测试和桌面真机
  smoke。至少覆盖：无 AGENTS/正常/截断、候选预检竞态、切换阻塞、切换成功、重建失败回滚、
  workspace 会话隔离、legacy root、真实 branch/subagent、搜索祖先路径。验收通过后将本 WBS
  及原 `WORKSPACE-AGENTS-MD-INJECTION` / `WORKSPACE-GUI` / managed-path 条目一起归档。
  当前自动化出口已完成：`just ci`、`just gui-automated-check`、Tauri workspace guard、
  Manager `/api/workspace`、CLI effective workspace、Gateway lifecycle focused tests 均通过；
  验证与接受记录见 `v0.1.7-workspace-closeout`。真实桌面 G2D+ 仍待人工执行，完成前不归档本
  WBS 和其合并的旧条目。
  依赖：WS-03、WS-05。

### L1-C：里程碑排期

| 里程碑 | 日期 | 出口条件 |
| --- | --- | --- |
| M0 开工冻结 | 2026-08-26 | 本 WBS、范围、依赖、验收门槛入库 |
| M1 后端基线 | 2026-08-27 | Wave A–D 并入最新基线，focused gates 通过 |
| M2 工作区可见 | 2026-08-28 | GUI 唯一快照、Chip/Settings/AGENTS 只读状态可用 |
| M3 可控切换 | 2026-09-02 | inspect + 原子切换 + 失败恢复闭环 |
| M4 分层历史 | 2026-09-04 | lineage DTO 与会话树闭环，legacy 行为明确 |
| M5 交付收口 | 2026-09-07 | 全门禁、桌面 smoke、文档和 TODO 归档完成 |
| 风险缓冲 | 2026-09-08 | 只处理 M1–M5 暴露的阻断，不新增功能 |

## L0-WBS：工作台系统与频道全面增强（大型 WBS）

> 这是上一轮决策收敛出的总工作分解。Workbench、PEN、Mirror、Neuro-Link、A2A
> 和频道能力合同不是互相孤立的 EPIC，而是同一大型 WBS 下的工作流；正式实施前仍需
> 分别立项、冻结边界和按 LOCK.md 建立独立工作区。

### L1-A：工作台系统与外部形态

#### WBS-01：Workbench / PEN / Mirror / Companion Node

- [ ] **DIVA-WORKBENCH-EXTERNAL-EMBODIMENT-EPIC：工作台、PEN、Mirror 与伴生形态** `sev-P1`
  2026-08-23 综合调研已收敛，用户认可总体设计，**但尚未授权生产实现**。研究包：
  [`diva-workbench-pen-mirror-neurolink-2026-08/`](docs/research/diva-workbench-pen-mirror-neurolink-2026-08/)。
  冻结方向：Workbench 是模块化第一方前端和控制面，不吞并 Persona/Memory/Evolution
  等领域权威；PEN 是进程外优先的外部能力单元；Mirror 是无默认通用执行权的具身/
  呈现单元；手机优先作为首个 Companion Node，感知严格区分 Observation、Moment 与
  经治理的 BML Memory。立项至少拆分 Module Package/Supervisor、PEN Tool Projection、
  Browser PEN、Mirror 合同与 Mate 样板迁移、手机伴生节点与 Experience/Moment 研究。
  **待决策**：第三方模块是否 v1 全部进程外；MVP transport；WorkbenchModule kind；
  Browser/Mate 首个样板；Experience Store 是否成为独立短期权威；专用硬件继续 defer。

#### WBS-02：Neuro-Link 前端超级通道

- [ ] **NEURO-LINK-FRONTEND-FABRIC：完整前端超级通道（待开工）** `sev-P1`
  用户确认：Neuro-Link 是理论上的超级通道，可彻底作为 agent-diva 的新前端；它不是
  普通 telegram/qq Channel，也不是 PEN 的专用传输。当前本地 WebSocket pipe 仅是
  概念胚胎，`channel_statuses` 省略它是有意的，不得补一块 ready/missing_fields 交差。
  正式 Epic 需要 Frontend/Device Identity、Capability Negotiation、Conversation、
  Presentation、Control、State Sync、Versioned Service Bindings、有界队列、ACK/取消、
  重连/快照和 TCK；Mate 的 avatar chat ID/`speak` 特例最终迁移为 Presentation Event。
  研究与安全门禁：
  [`neurolink-front-end-fabric.md`](docs/research/diva-workbench-pen-mirror-neurolink-2026-08/neurolink-front-end-fabric.md)。
  关联 `NeuroLinkConfig`、`agent-diva-channels/src/neuro_link.rs`、`cli_runtime.rs`；
  原 `NEURO-LINK-HEAVYWEIGHT-CHANNEL` / `CHANNELS-STATUS-COVERAGE` 并入本条。

### L1-B：频道能力与 Agent 互操作

#### WBS-03：外部频道能力合同与可靠性基线

- [ ] **CHANNEL-CAPABILITY-CONTRACT-EPIC：外部频道能力合同与可靠性基线** `sev-P1`
  第二批对照研究确认：当前 agent-diva 的 ChannelHandler 与消息 envelope 缺少统一的
  `message_id/thread_id`、typed attachments、typing/edit/delete/reaction/health、流式
  finalize、pacing/backpressure 和 supervisor/reconnect 合同；QQ 的群/Guild/媒体能力
  尤其不足。方案不能直接照搬单一项目：以 Octos 作为结构对照、ZeroClaw 作为能力标杆、
  OpenFang 作为 Bridge/TCK 参考，并保留 agent-diva 的 Manager/Sandbox/Approval/Laputa/BML
  治理边界。研究包：[`channel-capability-reference-2026-08/`](docs/research/channel-capability-reference-2026-08/)。
  **待决策**：是否先冻结统一 envelope/capability matrix/TCK，再按 QQ、Feishu、DingTalk
  分阶段施工；不得因 Octos 的 Rust 2024/MSRV 1.85 直接升高 agent-diva 当前 MSRV 1.80。

##### WBS-03-L2：既有频道页面能力补齐

- [ ] **CHANNELS-WIZARD-TEST-DELETE：向导连接测试与卡片删除接入** `sev-P2`
  `ChannelsSettings.vue` `handleWizardTest` 恒失败、`handleCardDelete` 只弹窗
  （两处既有 TODO）；需后端真实连接测试 API 与通道删除 API。2026-08-17 频道页
  修复时确认仍缺；真机冒烟已过，本条另行迭代。

#### WBS-04：A2A Agent-to-Agent 互操作

- [ ] **A2A-INTEROPERABILITY-EPIC：Agent-to-Agent 协议与多智能体互操作实现** `sev-P1`
  当前已完成 `.workspace` 参考项目调研和多方案初步收敛，**待正式立项，不得据此自动
  开始生产实现**。研究包：[`a2a-interoperability-2026-08/`](docs/research/a2a-interoperability-2026-08/)。
  建议主路线为“内部统一 AgentRun/Task/Policy 模型 + agent-diva-manager 原生 A2A
  adapter”，短期可用 Sidecar 做 OpenFang/ZeroClaw 互操作验证；不整体引入任一参考项目
  作为核心运行时。立项后至少拆分：单 Agent 入站 A2A、持久化 Task/取消/鉴权、出站 Agent
  Card/远程委托、多 Agent Alias/技能白名单、流式/推送和生产 TCK/安全验证。
  **待决策**：A2A v1.0 HTTP+JSON 与 JSON-RPC 兼容范围、TaskStore 复用 RunStore 还是独立
  SQLite 表、默认技能/工具权限、远程出站范围、SSE/Webhook 及多 Agent 是否拆分后续阶段。

### L1-C：WBS 共用运行时底座

#### WBS-05：每 session 有界串行准入

- [ ] **HARNESS-SESSION-ADMISSION-BOUNDED-QUEUE：每 session 有界串行准入** `sev-P1`
  当前 `turn/admission.rs` 只有熔断和小时速率拒绝，`MessageBus` inbound/outbound 是
  unbounded channel；缺少 per-session FIFO、队列深度上限、等待超时、取消和可观察的
  queue-full 语义。参考 ZeroClaw `SessionActorQueue`，但只接在 Agent Loop admission，
  不重写 MessageBus，不改变会话历史/Memory 权威。方案与验收见
  [`diva-adaptation-proposal.md`](docs/research/harness-gap-diva-adaptation-2026-08/diva-adaptation-proposal.md)。

#### WBS-06：可审计 EventBus Hook 扩展点

- [ ] **EVENTBUS-TRAIT-HOOKS：EventBus Trait Hook 管道** `sev-P1`
  来源于 OpenHarness/ZeroClaw 的机制调研；保留为未来扩展点，当前延期。2026-08-22
  研究包已收敛为 Diva 化方案：[`harness-gap-diva-adaptation-2026-08/`](docs/research/harness-gap-diva-adaptation-2026-08/)，
  只允许显式 Rust handler、Observe/Guard 分流、超时和审计，不开放任意 shell/HTTP/LLM
  hook，也不得成为 BML/Persona/Evolution 写入口。

## L0-产品与架构交付

### L1：治理与既有工作区

- [ ] **RG-CODE-GOV 后续分期** `sev-P2`
  原位治理 G0/G1 已完成；剩余 G2 Manager handler 变薄、G3–G5 GUI Host/state/DTO。
  不回迁 deep-governance 大爆炸。设计：`docs/dev/agent-loop-manager-gui-governance/`。

- [ ] **GMH-53：灰度与清理** `sev-P2`
  只处理仍有效的通用治理/发布内容；Memory governance 与旧 Evolution 部分由
  clean-break 决策取代（EPIC 已关闭，本条只余通用部分）。

- [ ] **WORKSPACE-AGENTS-MD-INJECTION / WORKSPACE-GUI：已并入 WORKSPACE 系统收尾 WBS** `sev-P1`
  后端隔离分支、GUI 当前工作区展示、受控切换和分层会话历史统一由 WS-00～WS-06 跟踪，
  不再以两个互相割裂的待办重复排期；待 WS-06 真机 smoke 后随 WBS 一并归档。

- [ ] **CLARIFY-HITL Phase 3** `sev-P3`
  已有 `ask_user` 运行时、CLI/Tauri/GUI 表面，真机冒烟已过；剩余 Plan 矩阵、
  subagent 禁用断言与可选 messaging clarify。不得并入审批抽屉或 governance ledger。

### L1：GUI 样式后续

- [ ] **GUI-STYLE-UNIFICATION-PHASE-3：gray-utility 桥退役 + 剩余字号** `sev-P4`
  前提：模板侧 `text-gray-*/bg-white` 工具类全部迁移到语义令牌后，
  删除 styles.css 兼容桥段 + NormalMode/ChatView/AppDialogLayer 的
  `theme-${themeMode}` DOM 类绑定。另处理非精确匹配字号（10/11/15/17/22px 等）。

## L0-决策闸门（先拍板、后实施）

- [ ] **全仓库代码清理提案审批与排期决策** `sev-P3`
  对 `docs/logs/2026-08-05-code-audit-proposal/v0.1.0-code-audit-proposal/` 的清理集合
  做取舍；不得把已被 Evolution/Memory 新决策覆盖的旧方案重新实现。
  **待决策**：清理项取舍与排期。

- [ ] **day/hour token 死循环熔断安全阀决策** `sev-P3`
  定位为极高默认阈值的异常循环保护，而不是常规预算管理。
  **待决策**：熔断挂在全局还是会话窗口；与 `session_token_budget_limit`、
  `RejectionCircuitBreaker` 的交互语义。拍板后再实施。

## L0-生产路径证明

### L1：自动化与纵向 E2E

- [ ] **BACKGROUND-TASK-PRODUCTION-E2E：后台任务生产路径纵向证明** `sev-P2`
  覆盖 `enqueue_background_task` 的 assembly/agent loop 接线、supervised worker
  启动/排空/取消/重启、subagent 终态，以及上下文与预算继承。

## L0-人工验收遗留

- [ ] **MASK-FEATURE-ACCEPTANCE** `sev-P2`
  恢复 `.sisyphus/plans/mask-feature-implementation.md` 前先复核现状，再执行剩余验收。

- [ ] **PET-TO-MATE-DESKTOP-SMOKE** `sev-P3`
  pet→mate 全面改名后的桌面级人工冒烟：侧边栏"伙伴"入口、设置页"启用伙伴"开关、
  桌面伙伴弹窗（`desktop-mate` 窗口）、`config.json` 出现 `"mate"` 节且旧配置不丢。
  步骤见 `docs/logs/2026-08-pet-to-mate-rename/v0.1.0-pet-to-mate-rename/acceptance.md`。

## L0-可靠性与测试债务

- [ ] **WORKSPACE-MSRS-1.80-DEPENDENCY-CONFLICTS** `sev-P2`
  工作区声明 Rust 1.80，但 ICU/Darling/Pest/CRC/Tauri 等依赖存在更高 MSRV；需要独立
  pin/升级方案，不削弱现有 gate。

## Archive Index

归档目录：
[`docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/README.md)

- 最新：[`completed-2026-08-23-autodream-diagnostic-logging.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-23-autodream-diagnostic-logging.md)
  （S3 worker 阶段级结构化日志，1 条）
- [`completed-2026-08-23-todolist-auto-close.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-23-todolist-auto-close.md)
  （机械收尾 + 合同冻结 + 用户确认真机/不可复现关闭，12 条）
- [`completed-2026-08-23-real-device-smoke-batch.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-23-real-device-smoke-batch.md)
  （EPIC 收口 + 真机冒烟批次 + 历史完成项指针）
- [`completed-2026-08-26-todolist-hierarchy.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-26-todolist-hierarchy.md)
  （GUI Phase 2 完成记录、过时决策点和重复条目的归档指针）
- 清理前完整快照：
  [`snapshot-before-2026-08-13-cleanup.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/snapshot-before-2026-08-13-cleanup.md)
