# TODOLIST

项目级**活跃**待办。这里只保留仍可执行、仍待决策或仍需验证的事项；完成、取消、
被取代和重复记录统一进入归档目录
[`docs/dev/archive(old-docs-dont-read-me)/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/README.md)。

严重度：`sev-P0` 阻断，`sev-P1` 高，`sev-P2` 中，`sev-P3` 低。

> **2026-08-23 收口**：总 EPIC「Laputa 认知工作区 Clean Break」关闭（S1–S6 与
> 真机桌面冒烟全部通过），积压的真机冒烟批次全部通过、修复已上主线。完成明细见
> [`completed-2026-08-23-real-device-smoke-batch.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/completed-2026-08-23-real-device-smoke-batch.md)。

## EPIC 新启动（研究包已收敛、尚未开工的独立工作区）

> 以下条目各自构成新的 EPIC/工作区，均有独立研究包或既有表面做底座，
  尚未进入实施；与“产品与架构”中已在进行的工作区（workspace-agents、
  gui-style-phase2 等）相互独立，启动时按 LOCK.md 流程另行建锁。

- [ ] **EVENTBUS-TRAIT-HOOKS：EventBus Trait Hook 管道** `sev-P1`
  来源于 OpenHarness/ZeroClaw 的机制调研；保留为未来扩展点，当前延期。2026-08-22
  研究包已收敛为 Diva 化方案：[`harness-gap-diva-adaptation-2026-08/`](docs/research/harness-gap-diva-adaptation-2026-08/)，
  只允许显式 Rust handler、Observe/Guard 分流、超时和审计，不开放任意 shell/HTTP/LLM
  hook，也不得成为 BML/Persona/Evolution 写入口。

- [ ] **HARNESS-SESSION-ADMISSION-BOUNDED-QUEUE：每 session 有界串行准入** `sev-P1`
  当前 `turn/admission.rs` 只有熔断和小时速率拒绝，`MessageBus` inbound/outbound 是
  unbounded channel；缺少 per-session FIFO、队列深度上限、等待超时、取消和可观察的
  queue-full 语义。参考 ZeroClaw `SessionActorQueue`，但只接在 Agent Loop admission，
  不重写 MessageBus，不改变会话历史/Memory 权威。方案与验收见
  [`diva-adaptation-proposal.md`](docs/research/harness-gap-diva-adaptation-2026-08/diva-adaptation-proposal.md)。

- [ ] **CLARIFY-HITL Phase 3** `sev-P3`
  已有 `ask_user` 运行时、CLI/Tauri/GUI 表面，真机冒烟已过；剩余 Plan 矩阵、
  subagent 禁用断言与可选 messaging clarify。不得并入审批抽屉或 governance ledger。

- [ ] **NEURO-LINK-HEAVYWEIGHT-CHANNEL：neuro-link 重量级频道预留（待开工）** `sev-P3`
  用户确认：neuro-link 是特意保留的未来重量级 channel 设计（本地 WebSocket
  pipe / 第三方接入），不是漏加的 config-status 项。当前 `channel_statuses`
  省略它是有意的；不要按 telegram/qq 同款补一块 ready/missing_fields 交差。
  irc/mattermost/nextcloud_talk 已于 2026-08-18 从 GUI 下架。开工时按重量级
  频道合同设计 status/GUI/运行时，并保留 schema、handler、`channel_statuses`
  上的预留注释。关联 `NeuroLinkConfig`、`agent-diva-channels/src/neuro_link.rs`、
  `cli_runtime.rs`。原 `CHANNELS-STATUS-COVERAGE` 并入本条。

## 产品与架构（存量延续与已在进行的工作区）

- [ ] **RG-CODE-GOV 后续分期** `sev-P2`
  原位治理 G0/G1 已完成；剩余 G2 Manager handler 变薄、G3–G5 GUI Host/state/DTO。
  不回迁 deep-governance 大爆炸。设计：`docs/dev/agent-loop-manager-gui-governance/`。

- [ ] **GMH-53：灰度与清理** `sev-P2`
  只处理仍有效的通用治理/发布内容；Memory governance 与旧 Evolution 部分由
  clean-break 决策取代（EPIC 已关闭，本条只余通用部分）。

- [ ] **WORKSPACE-AGENTS-MD-INJECTION：项目 AGENTS.md 注入合同** `sev-P3`
  当前 `ContextBuilder` 已读取 `<workspace>/AGENTS.md`（4000 字符、按 session 缓存，
  缺失时不追加 Agent Rules）；仍需补来源/digest/截断可观测性、与统一 workspace root
  的绑定，以及“项目规则不能授予工具权限或写 BML/Persona”的安全合同。先做根文件 MVP，
  不直接复制 Codex 的层级扫描、override 和 fallback 生态。

- [ ] **WORKSPACE-GUI：工作区选择与 AGENTS 状态交互** `sev-P3`
  当前 GUI 只在 General 设置展示解析后的 workspace 路径，没有目录选择、切换阻塞态、
  Gateway 重建反馈或 AGENTS.md 状态。需要实现 WorkspaceChip、WorkspaceSettings、
  AGENTS 摘要抽屉，以及停止→保存→重建→恢复的单一切换流程；禁止热换 AppState root、
  GUI 内编辑 AGENTS.md、隐式迁移会话或外部 workspace 模板写入。设计：
  [`gui-workspace-agents-design.md`](docs/research/workspace-agents-diva-adaptation-2026-08/gui-workspace-agents-design.md)。
  Oil Frontend 细化还要求 `WorkspaceContext` 作为唯一快照来源，移除 `GeneralSettings.vue`
  的重复 `getConfigStatus()` 请求，采用候选预览→一次提交→失败保留上下文，并保护过期
  inspect/status 响应。

- [ ] **GUI-STYLE-UNIFICATION-PHASE-2：样式令牌化二期** `sev-P3`
  其中伙伴装饰层主题方案与 WelcomeWizard 粉色身份色板两处属待决策项，
  已单列在下方“待决策事项”分区；其余批次为实施工作。
  一期（`docs/logs/2026-08-gui-style-unification/v0.1.0-design-tokens/`）已完成令牌基建、
  useTheme 治理、`.theme-*` 覆盖层退役与头部 3 组件语义色令牌化。剩余：
  ① 其余约 31 个组件/scoped 样式的硬编码 `#hex/rgba()` 迁移到 `var(--token, fallback)`；
  ② 伙伴装饰层（DivaMateView / DesktopMateOverlay）rgba 白色系色板的主题方案；
  ③ 组件内 scoped 的按主题覆盖块（如 ConversationSidebar 尾部 `.theme-*` 段）收敛到全局令牌；
  ④ WelcomeWizard 粉色身份色板（#be185d/#9d174d/#6b2737）的跨主题适配决策；
  ⑤ 深色对比色阶语义令牌扩展（如 `--danger-strong` #dc2626、`--warning-strong` #d97706）后替换字面量；
  ⑥ `tk-*` 字号/间距 scale 渐进迁移（本期只建基建未动现有字号）。
  迁移顺序建议：先 SettingsView 子树 → Mask/Persona 子树 → 其余；每批附 vitest + 四主题冒烟。

## 待决策事项（先拍板、后实施）

- [ ] **全仓库代码清理提案审批与排期决策** `sev-P3`
  对 `docs/logs/2026-08-05-code-audit-proposal/v0.1.0-code-audit-proposal/` 的清理集合
  做取舍；不得把已被 Evolution/Memory 新决策覆盖的旧方案重新实现。
  **待决策**：清理项取舍与排期。

- [ ] **day/hour token 死循环熔断安全阀决策** `sev-P3`
  定位为极高默认阈值的异常循环保护，而不是常规预算管理。
  **待决策**：熔断挂在全局还是会话窗口；与 `session_token_budget_limit`、
  `RejectionCircuitBreaker` 的交互语义。拍板后再实施。

- [ ] **Workspace CLI managed-path 与 runtime 任意路径契约** `sev-P3`
  路径穿越已修；仍需统一 `config_dir/workspaces/*` 与 runtime 任意路径模型。2026-08-22
  Codex 对照研究补充：现有 `--workspace` 已能覆盖当前 CLI，但默认仍是
  `~/.agent-diva/workspace`。**待决策**：“进程当前目录 vs 私有默认工作区”默认语义、
  canonical root、Shell `working_dir` 越界策略和外部工作区模板写入契约。
  研究包：[`workspace-agents-diva-adaptation-2026-08/`](docs/research/workspace-agents-diva-adaptation-2026-08/)。

- [ ] **GUI-STYLE-UNIFICATION-PHASE-2（决策点）** `sev-P3`
  二期整体属实施项（见上方“产品与架构（存量延续与已在进行的工作区）”分区），
  但有两处先决决策未拍板：
  ① 伙伴装饰层（DivaMateView / DesktopMateOverlay）rgba 白色系色板的主题方案；
  ② WelcomeWizard 粉色身份色板（#be185d/#9d174d/#6b2737）的跨主题适配方向。
  决策落定前不启动对应批次迁移。

## 频道遗留

- [ ] **CHANNELS-WIZARD-TEST-DELETE：向导连接测试与卡片删除接入** `sev-P2`
  `ChannelsSettings.vue` `handleWizardTest` 恒失败、`handleCardDelete` 只弹窗
  （两处既有 TODO）；需后端真实连接测试 API 与通道删除 API。2026-08-17 频道页
  修复时确认仍缺；真机冒烟已过，本条另行迭代。

## 自动化与生产路径证明

- [ ] **BACKGROUND-TASK-PRODUCTION-E2E：后台任务生产路径纵向证明** `sev-P2`
  覆盖 `enqueue_background_task` 的 assembly/agent loop 接线、supervised worker
  启动/排空/取消/重启、subagent 终态，以及上下文与预算继承。

## 人工验收遗留

- [ ] **MASK-FEATURE-ACCEPTANCE** `sev-P2`
  恢复 `.sisyphus/plans/mask-feature-implementation.md` 前先复核现状，再执行剩余验收。

- [ ] **PET-TO-MATE-DESKTOP-SMOKE** `sev-P3`
  pet→mate 全面改名后的桌面级人工冒烟：侧边栏"伙伴"入口、设置页"启用伙伴"开关、
  桌面伙伴弹窗（`desktop-mate` 窗口）、`config.json` 出现 `"mate"` 节且旧配置不丢。
  步骤见 `docs/logs/2026-08-pet-to-mate-rename/v0.1.0-pet-to-mate-rename/acceptance.md`。

## Reliability / Test Debt

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
- 清理前完整快照：
  [`snapshot-before-2026-08-13-cleanup.md`](docs/dev/archive%28old-docs-dont-read-me%29/2026-08-docs-corpus-reset/legacy-docs/docs-archive/content/todolist/snapshot-before-2026-08-13-cleanup.md)
