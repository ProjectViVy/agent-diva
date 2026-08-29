# TODOLIST

项目级**活跃**待办。这里按 `L0 计划/大型 WBS → L1 工作流 → L2 事项` 分层，
只保留仍可执行、仍待决策或仍需验证的事项；完成、取消、被取代和重复记录统一进入归档目录
[`docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/README.md)。

严重度：`sev-P0` 阻断，`sev-P1` 高，`sev-P2` 中，`sev-P3` 低。

> **2026-08-23 收口**：总 EPIC「Laputa 认知工作区 Clean Break」关闭（S1–S6 与
> 真机桌面冒烟全部通过），积压的真机冒烟批次全部通过、修复已上主线。完成明细见
> [`completed-2026-08-23-real-device-smoke-batch.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-23-real-device-smoke-batch.md)。

> **2026-08-28 正式关闭 WORKSPACE 系统收尾**：WS-00～WS-06 的实现、自动化验证、
> 真实桌面验收、分支/worktree 退休和成果归档均已完成。关闭记录见
> [`completed-2026-08-28-workspace-system-closeout.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-28-workspace-system-closeout.md)。

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

  **立项状态与排期（2026-08-29）**：已进入开发准备，按 1 名主开发、工作日连续投入估算，
  基线工期 **13 个工程日**，计划窗口 **2026-08-31 ～ 2026-09-16**；预留 2 个工程日风险
  缓冲，最晚目标 **2026-09-18**。若与其他改动同时触碰 `AgentLoop`、runtime control、
  Manager chat API 或 `TODOLIST.md`，须使用独立 worktree 并在合并前重跑全套门禁。

  **关键架构闸门**：当前 `AgentLoop::run(&mut self)` 全局串行消费 inbound，turn 期间还持有
  loop 级可变状态。首阶段必须证明并冻结“dispatcher + per-session worker/lease”的所有权
  模型；不得用一把全局 mutex 冒充 per-session 并发。若无法在不复制 AgentLoop、重写
  `MessageBus` 或破坏 Plan/Sandbox/Memory 边界的前提下实现不同 session 并行，停止施工并
  重新评审，不进入后续接线。

  **开发分解与里程碑**：

  - [x] **HQ-00 合同冻结与并发所有权表征**（08-31 ～ 09-01，2d）：已补齐当前全局串行、
    session identity、Stop/Reset、直接调用与 Bus 调用的表征测试；冻结 typed outcome/error
    code、配置位置与默认值（`max_queue_depth=2`、`wait_timeout=30s`、
    `idle_ttl=10m`），并输出 dispatcher/worker 生命周期与 drop/abort 行为。冻结合同见
    [`hq00-contract.md`](docs/dev/harness-session-admission/hq00-contract.md)；HQ-01/02 须按该
    所有权模型证明不改 MessageBus 权威即可让不同 session 独立推进、同 session 严格 FIFO。
  - [x] **HQ-01 Core admission kernel**（原排期 09-02 ～ 09-04；08-29 提前完成）：已在
    `agent-diva-core/src/session/admission.rs` 实现有界 FIFO、RAII lease、等待超时、队满、
    显式取消、关闭排空和 idle slot 回收；时钟可注入、测试可暂停，且所有状态锁均在 await
    前释放。12 个定向单测覆盖 FIFO、容量边界、跨 session 独立、timeout/cancel race、future/
    lease drop、任务 abort、idle eviction 与 close drain。实现提交 `5ff59b5c`，验证记录见
    [`v0.2.0-hq01-core-kernel`](docs/logs/2026-08-harness-session-admission/v0.2.0-hq01-core-kernel/verification.md)。
  - [x] **HQ-02 Agent dispatcher 与 turn 接线**（原排期 09-07 ～ 09-09；08-29 提前完成）：
    Bus 与 `process_direct(_stream)` 已统一在 circuit/rate/provider 前经过 HQ-01 lease seam；
    session 可变字段已收敛进显式 `SessionWorkerState`，approval/tool surface 与 subagent mask
    改为 turn 快照。6 个 dispatcher 测试证明同 session FIFO/不重入、跨 session 并行、
    queue-full/timeout 零执行副作用，以及 Stop 与 Reset 的 running/waiter 边界。按 HQ-00 闸门，
    生产 Bus 继续保持兼容串行，待 HQ-03 完成 request/trace 投影后再开放并发。实现提交
    `9302f8e7`、`b53c619f`；验证记录见
    [`v0.3.0-hq02-agent-dispatcher`](docs/logs/2026-08-harness-session-admission/v0.3.0-hq02-agent-dispatcher/verification.md)。
  - [x] **HQ-03 Runtime control、配置与可观察合同**（原排期 09-10 ～ 09-11；08-29 提前完成）：
    生产 Bus 已切换为持久 per-session actor，同 session 保持有界 FIFO，不同 session 可并发；
    Stop 可按 request 精确取消 running turn 且保留 queued turn，Reset/Delete 在 turn 静默后清理。
    `agents.defaults.session_admission` 已提供 `max_queue_depth=2`、`wait_timeout=30s`、
    `idle_ttl=600s` 的兼容默认值；稳定 outcome code、queue depth、wait latency 与
    request/trace/session correlation 已投影到 Manager、CLI、GUI 和通用 SSE。实现提交
    `011db1f3`；验证记录见
    [`v0.4.0-hq03-runtime-contract`](docs/logs/2026-08-harness-session-admission/v0.4.0-hq03-runtime-contract/verification.md)。
    MessageBus 仍只承担 transport，小时/天 token 熔断未改造成 queue。
  - [ ] **HQ-04 跨入口验证与故障注入**（09-14 ～ 09-15，2d）：覆盖 Manager API、GUI
    stream、CLI/direct、至少一个 Channel/Bus 入口；注入 provider stall/retry、Stop、Reset、
    timeout 和 queue-full，证明无丢消息、无串 session、无幽灵 lease，且错误对用户可解释。
  - [ ] **HQ-05 收口与发布门禁**（09-16，1d）：运行 `just fmt-check`、`just check`、
    `just test`、相关 crate 定向测试及 CLI/GUI 最小真实路径 smoke；补齐迭代日志、配置迁移/
    默认值说明、回滚说明和人工验收步骤。全部通过后才关闭本 Epic。

  **验收门**：同 session 最大并发 turn=1 且 FIFO 可重复证明；不同 session 在阻塞 provider
  fixture 下确实并行；深度上限不含 running turn 并有边界测试；queue full、wait timeout、
  cancelled、evicted 使用稳定机器码；Stop/Reset 的 running 与 queued 语义无歧义；所有拒绝
  均发生在 provider/tool/BML 副作用之前；PlanMode、Sandbox、Approval、会话历史、BML/
  Persona/Evolution 权威保持不变；MessageBus 继续作为 transport，不承担 session 调度。

  **依赖与风险**：HQ-01 依赖 HQ-00 所有权方案通过；HQ-02 依赖 HQ-01；HQ-03 可在 HQ-02
  后半段并行准备但须串行合并；HQ-04/05 依赖前述全部完成。主要风险是 AgentLoop 共享
  `ToolRegistry`、session/context cache、deferred tools、ACTMEM timer 与 cancellation state 的
  并发拆分，风险缓冲优先用于 race/取消语义，不用于扩张到 EventBus Hook、A2A 或
  Neuro-Link。计划记录见
  [`2026-08-harness-session-admission-planning`](docs/logs/2026-08-harness-session-admission-planning/v0.1.0-development-plan/summary.md)。

#### WBS-06：可审计 EventBus Hook 扩展点

- [ ] **EVENTBUS-TRAIT-HOOKS：EventBus Trait Hook 管道** `sev-P1`
  来源于 OpenHarness/ZeroClaw 的机制调研；保留为未来扩展点，当前延期。2026-08-22
  研究包已收敛为 Diva 化方案：[`harness-gap-diva-adaptation-2026-08/`](docs/research/harness-gap-diva-adaptation-2026-08/)，
  只允许显式 Rust handler、Observe/Guard 分流、超时和审计，不开放任意 shell/HTTP/LLM
  hook，也不得成为 BML/Persona/Evolution 写入口。

## L0-产品与架构交付

### L1：治理与既有工作区

- [ ] **WS-CLI-LEGACY-DEFAULT-MIGRATION：评估 CLI legacy 默认值的 CWD 兼容策略** `sev-P3`
  Workspace 收口后仍保留的独立兼容性缺口：GUI legacy 默认值已在
  `v0.1.14-default-workspace-reset` 中稳定投影到 profile-local Diva workspace，设置页也已
  支持独立配置/重置，运行时 workspace 选择不会反写默认值。CLI 仍按既有合同把 legacy
  `~/.agent-diva/workspace` 解析为进程 CWD；若未来要让 CLI 与 GUI 使用同一默认语义，需要
  单独设计迁移、doctor 提示和向后兼容策略。相关：`agent-diva-core/src/workspace.rs`、
  `agent-diva-cli/tests/effective_workspace.rs`。

- [ ] **RG-CODE-GOV 后续分期** `sev-P2`
  原位治理 G0/G1 已完成；剩余 G2 Manager handler 变薄、G3–G5 GUI Host/state/DTO。
  不回迁 deep-governance 大爆炸。设计：`docs/dev/agent-loop-manager-gui-governance/`。

- [ ] **GMH-53：灰度与清理** `sev-P2`
  只处理仍有效的通用治理/发布内容；Memory governance 与旧 Evolution 部分由
  clean-break 决策取代（EPIC 已关闭，本条只余通用部分）。

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

- [ ] **LAPUTA-STORAGE-STALE-LOCK-FLAKE-RECURRENCE** `sev-P2`
  2026-08-29 HQ-00 全量 `just test` 再次在
  `agent-diva-laputa::lock::tests::stale_lock_file_with_pid_is_recovered` 出现 `LockTimeout`；
  随后定向连续 3 次均通过，确认是已关闭问题的偶发回归。应隔离 PID/时间/文件系统竞态，
  使陈旧锁恢复测试可重复且不依赖宿主时序；相关代码：`agent-diva-laputa/src/lock.rs`。

- [ ] **AGENT-ALL-TARGETS-CLIPPY-AWAIT-HOLDING-LOCK** `sev-P3`
  `cargo clippy -p agent-diva-agent --all-targets -- -D warnings` 在既有测试
  `agent-diva-agent/src/agent_loop.rs` 的规则加载场景报告 `await_holding_lock`（约 3537～3557
  行）。工作区标准 `just check` 不含 `--all-targets`，本轮未扩张修复；应缩短 guard 生命周期，
  并把 all-targets lint 纳入对应 crate 的稳定门禁。

- [ ] **WORKSPACE-MSRS-1.80-DEPENDENCY-CONFLICTS** `sev-P2`
  工作区声明 Rust 1.80，但 ICU/Darling/Pest/CRC/Tauri 等依赖存在更高 MSRV；需要独立
  pin/升级方案，不削弱现有 gate。

## Archive Index

归档目录：
[`docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/README.md)

- 最新：[`completed-2026-08-28-workspace-system-closeout.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-28-workspace-system-closeout.md)
  （WORKSPACE 系统收尾正式关闭，WS-00～WS-06 及已合并的 Workspace 旧条目）
- [`completed-2026-08-23-autodream-diagnostic-logging.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-23-autodream-diagnostic-logging.md)
  （S3 worker 阶段级结构化日志，1 条）
- [`completed-2026-08-23-todolist-auto-close.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-23-todolist-auto-close.md)
  （机械收尾 + 合同冻结 + 用户确认真机/不可复现关闭，12 条）
- [`completed-2026-08-23-real-device-smoke-batch.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-23-real-device-smoke-batch.md)
  （EPIC 收口 + 真机冒烟批次 + 历史完成项指针）
- [`completed-2026-08-26-todolist-hierarchy.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-26-todolist-hierarchy.md)
  （GUI Phase 2 完成记录、过时决策点和重复条目的归档指针）
- 清理前完整快照：
  [`snapshot-before-2026-08-13-cleanup.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/snapshot-before-2026-08-13-cleanup.md)
