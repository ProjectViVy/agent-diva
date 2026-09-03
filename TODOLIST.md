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

> **2026-08-30 正式关闭 HARNESS Session Admission Epic**：HQ-00～HQ-05 的合同、
> 有界准入内核、per-session worker、runtime control、跨入口故障注入和发布门禁全部完成。
> 关闭记录见
> [`completed-2026-08-30-harness-session-admission.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-30-harness-session-admission.md)。

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

#### WBS-02：CHANNEL-EPIC / Super Channel Fabric（已启动）

- [ ] **CHANNEL-EPIC：超级通道与外部频道统一 Fabric** `sev-P1`
  2026-08-30 正式启动。Neuro-Link v1 是能够完整承载新前端的 Owner Frontend
  **超级通道**；Telegram、Discord、Feishu、DingTalk、Email、QQ 是低信任外部
  `ChannelAdapter`。两者共享 typed envelope、capability、bounded queue、receipt、
  supervisor、State Sync 和 TCK，但绝不共享 Owner 信任语义。完整冻结架构：
  [`docs/dev/channel-epic/architecture.md`](docs/dev/channel-epic/architecture.md)。

  硬约束：采用 Octos `5ea9878` 的 Channel/Manager/bounded-bus 形状作为适配主参考；
  Neuro-Link 测试期无身份限制且固定复用 Manager loopback；WS 实时流 + 原位 HTTP
  Service Bindings；版本化 JSON Schema 为 wire 权威；混合事件日志；不新增频道配置；
  C1～C6 在隔离 `feat/channel-epic` worktree 施工，C6 完成后一次性原子合入。最终产品树
  不允许旧 pipe、旧 DTO、旧 bus、双轨、shim、兼容别名或六个退役频道源码。

  - [x] **C0：完整架构冻结与 Epic 启动**（2026-08-30）
    已冻结术语、模块边界、消息/能力合同、Neuro-Link v1、Service Binding、混合 journal、
    背压、Clean Break、TCK、迁移 WBS 和十条不变量；本批只改文档。
  - [x] **C1：JSON Schema、核心 typed contracts 与表征测试**（2026-08-30）
    已建立版本化 `neuro-link/v1` JSON Schema、`ChannelEnvelopeV1`、typed parts、
    capability/command/event/receipt/error 的协议基础，Rust/TypeScript 共用正反 fixture，
    并以旧 pipe、MessageBus、allowlist 和 loopback guard characterization 锁定旧路径边界。
    交付记录：[`v0.1.1-neuro-link-contract`](docs/logs/2026-08-channel-epic/v0.1.1-neuro-link-contract/)。
  - [x] **C2：bounded Fabric Kernel、Adapter Registry 与 supervisor**（2026-08-30）
    替代无界 ingress/egress，建立 control/durable/transient/adapter lane、pacing、health、
    panic/exit recovery 和 fault-injection TCK。
    - [x] **C2a：bounded Fabric Kernel 与队列 TCK**（2026-08-30）
      已建立固定容量 control/ingress/durable/transient lane、deadline/cancel admission、
      transient coalesce/Gap、request fence、同 session FIFO/跨 session 并行调度和 drain
      shutdown；未接入旧 MessageBus。交付记录：
      [`v0.1.2-bounded-fabric-kernel`](docs/logs/2026-08-channel-epic/v0.1.2-bounded-fabric-kernel/)。
    - [x] **C2b：Adapter Registry、supervisor、pacing 与故障 TCK**（2026-08-30）
      已建立新 `ChannelAdapter`、capability/command runtime、独立 adapter egress、
      Retry-After、panic/exit recovery、部分分片 receipt 和 fake-adapter smoke；未迁移真实
      平台或切换生产路径。交付记录：
      [`v0.1.3-adapter-runtime`](docs/logs/2026-08-channel-epic/v0.1.3-adapter-runtime/)。
  - [x] **C3：Neuro-Link v1 Gateway、Projection Journal 与 Service Catalog**（2026-08-31）
    在 Manager loopback 提供 JSON-RPC WebSocket、ACK/resume/snapshot 和原位 HTTP
    Service Binding 登记；不增加 host/port/auth 配置。
    - [x] **C3a：协议结果类型与 Service Catalog**（2026-08-31）
      Rust/Schema/GUI 类型已同步 catalog revision、state-sync、admission、ACK 和稳定错误码；
      现有领域 HTTP handler 以 Service Binding 登记，不复制路由。
    - [x] **C3b：Projection Journal**（2026-08-31）
      Manager data root 的 `super-channel-events.db` 已支持 cursor、ACK、replay、retention、
      session 清理和幂等冲突保护。
    - [x] **C3d：loopback Gateway 与 TCK smoke**（2026-08-31）
      `/api/neuro-link/v1/ws` 已完成 hello、session/open、turn/cancel、event/ack、state/resume、
      frame/part/attachment bounds 及真实 WebSocket 冒烟。
    - [x] **C3c：AgentLoop typed admission wiring**（2026-08-31）
      `RuntimeControlCommand::StartChannelTurn` 已通过 bounded per-session dispatcher 接入
      AgentLoop，Manager production bootstrap 安装 `AgentLoopNeuroLinkRuntime`，并保留
      session/request/trace correlation；`turn/cancel` 复用 typed control lane。
    - [x] **C3e：AgentEvent → ProjectionEvent stream hub**（2026-08-31）
      已建立进程级 Neuro-Link projection hub：消费带 session/request/trace 关联的
      AgentBusEvent，映射为受限 Fabric envelope，先写入 `super-channel-events.db` 再广播到
      所有匹配会话的 WebSocket；conversation、reasoning、tool、planning、provider、compaction
      与 terminal error/final 事件分别标记 durable/transient，瞬时增量不进入断线 replay。
      `turn/start` 的首个 queued/running admission 仍由 Gateway 响应路径原子写入，避免重复行；
      C6 仍负责旧 bus/DTO 的最终 clean break。
  - [x] **C4：桌面 GUI 首个 Neuro-Link 客户端与 Presentation 迁移**（2026-08-31）
    GUI 实时链路已迁移到 typed Neuro-Link v1；Mate 删除 avatar chat ID/`speak` 特例，改用
    semantic Presentation；GUI tests/build、Rust workspace 门禁和协议/TCK 回归均通过。详见
    [`v0.1.7-desktop-neuro-link-client`](docs/logs/2026-08-channel-epic/v0.1.7-desktop-neuro-link-client/)。
    - [x] 发布工作站真实 Tauri/桌面断线恢复冒烟（2026-09-03 用户确认此前冒烟无异常）。
      本确认覆盖 C4 桌面路径；C6 新原生频道生产切换后的真实平台收发仍单列在 C6-E，不能用
      本次确认替代。
  - [ ] **C5：六个现役 ChannelAdapter 与 capability TCK**
    迁移 Telegram、Discord、Feishu、DingTalk、Email、QQ；每个 `true` capability 都有
    离线 fixture/mock 证明，至少一个真实平台纵向 smoke。
    - [x] **C5-P：Octos 能力迁移方案与 agent 交接冻结**（2026-08-31）
      已固定 Octos `5ea987813de4fd2afdd1d78f2106ad2868f0d923`，完成扫描 playbook、
      provenance ledger、28×6 capability matrix、共享 ADR、六平台规格、并行 agent WBS、
      capability TCK/fixture 方案、QQ 真机 runbook 和 C6 边界。详见
      [`c5-octos-migration`](docs/dev/channel-epic/c5-octos-migration/)。
    - [x] **C5-P2：六频道 Octos 端点深扫与实施交接补强**（2026-08-31）
      已由 Telegram、Discord、Feishu、DingTalk、Email、QQ 六个独立频道任务完成深扫，
      新增 endpoint ledger、跨频道 gap matrix、decision log、agent task cards、evidence
      manifest 和六份端点级报告。QQ intents/media 仍按 Blocked 交接，未伪造完成能力。
    - [x] **C5-I：六个原生 ChannelAdapter 与 Octos 能力增强**（2026-09-03）
      先冻结共享 adapter/services seam，再按“一频道一 agent、资源分波”的隔离 worktree
      分工实施；禁止 wrapper、双发 bus、默认成功和生产双轨切换。实现 ownership 与顺序见
      [`agent-task-cards.md`](docs/dev/channel-epic/c5-octos-migration/agent-task-cards.md)。
      - [x] **C5-I-G1：共享 AdapterServices/AttachmentStore seam 与 TCK 基座**（2026-08-31）
        已完成 typed attachment、内容寻址 digest 校验、allowlist helper、外部 envelope/
        receipt/error builders、Gate 1 factory 失败语义及共享 TCK；Manager 生产装配仍留给
        C6。
      - [x] **C5-I-G2：六个 native adapter 与 C5 factory 接线**（2026-09-01）
        已在隔离 `feat/channel-epic` worktree 实现 Telegram、Discord、Feishu、DingTalk、
        Email、QQ 六个 `ChannelAdapter`，各自保留 Octos/DIVA wire 行为并统一 typed envelope、
        AttachmentStore、dedup、health、receipt、unsupported 零副作用；factory 现在只负责
        构造，不启动 listener，也未切 Manager 生产路径。QQ Identify intents/media 仍按 C5-Q
        保持 `partial/blocked`，不把未验证能力伪装为完成。实现证据见
        [`platforms/*-gate2.md`](docs/dev/channel-epic/c5-octos-migration/platforms/README.md)
        与 [`evidence-manifest.md`](docs/dev/channel-epic/c5-octos-migration/evidence-manifest.md)。
    - [ ] **C5-V：capability evidence、全量门禁与 QQ 真实纵向 smoke**
      六频道 target-true 能力的本地 wire/fixture/TCK 已补齐并逐行审计；QQ 已有 C2C/群 @、
      admission-before-dedup、final-only、真实 message ID、recovery/stop 的离线证据，但
      凭据只从仓库外注入，未完成 QQ 真机、D-013/D-014 与其余 partial 的外部证据前不得勾选 C5。
      - [ ] **C5-V-QQ-LIVE-BLOCKED** `sev-P1`
        已加入显式 `#[ignore]` 的 `qq_live_harness`，覆盖 C2C、群 @、重放、final-only、
        receipt、resume/reconnect 和 stop；真实运行命令为
        `cargo test -p agent-diva-channels --test qq_live_harness -- --ignored --nocapture`。
        当前 worktree 没有仓库外 QQ 凭据或平台权限，故 live smoke 未执行，C5-V 不能关闭。
      - [x] **C5-V-FULL-GATE-DEBT** `sev-P1`
        频道 all-target clippy 已在 `c6b0a756` 清理并验证；channel-scoped Rust 1.80 probe
        经 `348d42a1` 的 `Cargo.lock` 兼容 pin 通过，最终 `just fmt-check`、`just check`、
        `just test` 也通过。Manager loopback flake、未声明完成的 broader workspace MSRV
        audit 和 QQ live/官方 wire blockers 仍按独立 TODO 保持开放；记录见本轮
        [`verification.md`](docs/logs/2026-09-channel-epic/v0.2.2-c5-capability-evidence/verification.md)。
    - [ ] **C5-Q：QQ intents、group send 与 media 官方 wire 证据** `sev-P1`
      当前 DIVA 的 C2C/group outbound 与本地 recovery 已有 wire-shaped fixture，但 DIVA 使用
      `(1<<25)|(1<<12)`、Octos 使用 `(1<<25)|(1<<30)`，官方事件投递仍未证实；D-014 也没有
      官方 media endpoint+wire proof。必须补官方证据后才能调整状态，不能以离线 fixture 关闭。
    - [ ] **C5-V-PARTIAL-AUDIT-FINDINGS：21 条 partial 的实现与证据缺口** `sev-P1`
      2026-09-02 已逐项对照固定 Octos SHA 完成修复审计；本地状态为 `11 verified / 17
      partial / 1 blocked/unsupported`，不是所有 partial 都能凭本地 mock 升级。剩余缺口是：
      Telegram TG-02～TG-06 的 live 群权限/CDN/送达/C6 supervisor 与 keyboard 边界；Discord
      DC-01/02/03/05 的 live Gateway、guild/CDN/permission/retry/supervisor；DingTalk DT-01/
      03/04 的 live Stream、媒体权限/receipt、官方签名 callback；Email EM-02/03/04 的真实
      IMAP/SMTP TLS/auth/delivery/UID Seen；QQ QQ-01/02 的 live Gateway 与 D-013。Feishu
      FS-01/03/04/05 仅在本地 deterministic wire/store 范围升级 verified，远端真实性与长时
      soak 仍不等同于 live proof。逐行 evidence、receipt/error 和生命周期结果见
      [`evidence-manifest.md`](docs/dev/channel-epic/c5-octos-migration/evidence-manifest.md)
      及六份 `platforms/*-gate3.md`；在实现或证据闭合前不得关闭 C5-V。
    - [x] **C5-DOC：修正 BaseChannel allow_from 语义说明** `sev-P2`（2026-08-31）
      已更新 `agent-diva-channels/AGENTS.md`：空 `allow_from` 是 allow-all，并明确 C5
      使用原生 `ChannelAdapter`、共享 Fabric 与 C6-only Manager 装配边界。
  - [ ] **C6：Clean Break 删除、全量门禁与原子合并**
    删除旧 Neuro-Link、`ChannelHandler`、旧消息 DTO、旧 SSE、无界频道 bus、配置别名和
    Slack/WhatsApp/Matrix/IRC/Mattermost/Nextcloud Talk 源码/feature；通过 workspace、
    GUI、MSRV、TCK、clean-break 和 release acceptance 后一次性合入 `dev`。
    - [x] **C6-A：Manager 原生频道生产装配与运行时事实源**（2026-09-03）
      六个 adapter 已由 Manager 构造 `AdapterServices`、Registry、pacing 与 supervisor；外部
      ingress 先经 bounded Fabric 再进入 AgentLoop；GUI/API 读取真实注册、健康与诊断状态；
      配置热更新在候选构造失败时恢复旧配置和旧 runtime，凭据不完整则由 runtime health
      明确报告，不阻断整个 gateway 启动。
    - [x] **C6-B：退休频道与旧桌面/HTTP 实时入口删除**（2026-09-03）
      已删除六个退休频道源码、配置、feature 和旧 ChannelHandler 树；旧 `/api/chat`、
      `/api/chat/stop`、`/api/events` 及 Tauri SSE/background 注册均已移除；上下文压缩改用
      独立 typed command。
    - [x] **C6-C：频道队列有界化与 callback subscriber 删除**（2026-09-03）
      Agent 消息队列改为固定容量 256，满载显式失败；生产 egress 由单一 receiver 顺序转入
      adapter pacing lane，不再使用 callback subscriber 或无界 ingress/egress。
    - [x] **C6-D：物理删除 AgentLoop 旧 InboundMessage/OutboundMessage DTO** `sev-P1`（2026-09-04）
      已按架构 §15.2/§19.10 完成 strict Clean Break：AgentLoop 直接接收
      `ChannelEnvelopeV1` 并返回 `Option<ChannelCommand>`，旧 DTO、旧 inbound/outbound queue、
      publish/take receiver API、callback subscriber 和 metadata shim 均已从 active source
      删除。OwnerFrontend 只产生 AgentEvent projection；ExternalUser/Runtime 严格要求
      context 为 None 并走 typed adapter egress；Manager、六个 external adapter、Cron、
      Subagent、Presence、message tool 和 CLI SSE 均已迁移。trust matrix、correlation/
      reply_to、typed content/media、attachment admission、stop/reset/backpressure/shutdown
      与 worker recovery 均有回归证据；clean-break gate 和全 workspace Rust gates 通过。
      证据：[C6-D iteration log](docs/logs/2026-09-channel-epic/v0.3.1-c6-d-typed-agent-loop/)。
    - [ ] **C6-E：切换后真实平台/桌面验收与全工作区 MSRV** `sev-P1`
      C6 已按用户明确指令于 2026-09-03 通过 `8cc6580b` 本地合入 `dev`，且合并后 workspace、
      TCK 与 clean-break 门禁通过；C6-D 已在隔离分支完成，但仍需至少一个真实平台完成入站、
      最终 receipt，并在新生产路径复测桌面断线恢复。Rust 1.80 channel-scoped probe 仍被
      缓存的 `getrandom 0.4.3` Edition2024 manifest 要求阻塞；待真实平台/桌面和全工作区
      MSRV 条件满足后关闭。本地合入不等于完整验收或已推送。

  - [ ] **GUI 依赖安全审计基线** `sev-P2`
    C1 同步 GUI npm lock 时，npm 报告依赖图存在 11 个 audit vulnerabilities（2 moderate、9 high）。
    本项不属于 Neuro-Link 合同实现，且没有运行 `audit fix`；需单独评估升级、兼容性和 pnpm/npm
    lock 策略后处理。相关文件：`agent-diva-gui/package.json`、`agent-diva-gui/package-lock.json`。

### L1-B：频道能力与 Agent 互操作

> 原 `CHANNEL-CAPABILITY-CONTRACT-EPIC` 已并入 WBS-02 的 CHANNEL-EPIC C1/C2/C5；
> 研究包继续作为实现依据：
> [`channel-capability-reference-2026-08/`](docs/research/channel-capability-reference-2026-08/)。

#### WBS-03：既有频道页面能力补齐

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

- [ ] **MANAGER-NEURO-LINK-LOOPBACK-FLAKE** `sev-P2`
  2026-09-01 C5-I Gate 2 最终全量 `just test` 在既有
  `agent-diva-manager::neuro_link::tests::loopback_websocket_handshake_session_and_turn_smoke`
  偶发失败（断言 `left: Null, right: 1`），随后定向复跑通过。需隔离 loopback websocket
  测试的并发/通知顺序竞态，连续多次全量与定向通过后再关闭；相关代码：
  `agent-diva-manager/src/neuro_link.rs:1358`。

- [ ] **AGENT-ALL-TARGETS-CLIPPY-AWAIT-HOLDING-LOCK** `sev-P3`
  `cargo clippy -p agent-diva-agent --all-targets -- -D warnings` 在既有测试
  `agent-diva-agent/src/agent_loop.rs` 的规则加载场景报告 `await_holding_lock`（约 3537～3557
  行）。工作区标准 `just check` 不含 `--all-targets`，本轮未扩张修复；应缩短 guard 生命周期，
  并把 all-targets lint 纳入对应 crate 的稳定门禁。

 - [x] **CHANNEL-ALL-TARGETS-CLIPPY-LEGACY-TESTS** `sev-P3`
  已在 C5-V 频道 lint 清理批次完成：`c6b0a756` 修复 QQ integration 的冗余转换/条件，
  并对 DingTalk/Email/Feishu/QQ legacy test-only fixtures 增加局部 lint 例外；
  `cargo clippy -p agent-diva-channels --all-targets -- -D warnings` 已通过。

- [ ] **AGENT-RETRY-CORRELATION-FLAKE** `sev-P2`
  本轮隔离 worktree 的 `just test` 首次运行在
  `agent_loop::tests::concurrent_sessions_keep_provider_retry_correlation_isolated` 超时，
  其余 432 个测试通过；该测试与 C5 文档变更无直接关系。需单独复现并修复并发 retry event
  的等待/相关性竞态，连续通过后再关闭；相关代码：`agent-diva-agent/src/agent_loop.rs`。

- [ ] **WORKSPACE-MSRS-1.80-DEPENDENCY-CONFLICTS** `sev-P2`
  工作区声明 Rust 1.80，但 ICU/Darling/Pest/CRC/Tauri 等依赖的全 workspace MSRV 尚未
  取得独立、完整的兼容性证明；本轮仅以 `Cargo.lock` pin 使
  `just msrv-probe check -p agent-diva-channels` 在 Rust 1.80.1 通过，不关闭本 broad
  audit，后续仍需不削弱现有 gate 的全量方案。

## Archive Index

归档目录：
[`docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/README.md)

- 最新：[`completed-2026-08-30-harness-session-admission.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-30-harness-session-admission.md)
  （HARNESS Session Admission Epic 正式关闭，HQ-00～HQ-05）
- [`completed-2026-08-28-workspace-system-closeout.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-28-workspace-system-closeout.md)
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
