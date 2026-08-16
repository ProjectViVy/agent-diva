# TODOLIST

项目级**活跃**待办。这里只保留仍可执行、仍待决策或仍需验证的事项；完成、取消、
被取代和重复记录统一进入 [`docs/archive/todolist/`](docs/archive/todolist/README.md)。

严重度：`sev-P0` 阻断，`sev-P1` 高，`sev-P2` 中，`sev-P3` 低。

## 总 EPIC：Laputa 认知工作区 Clean Break

- [ ] **LAPUTA-COGNITIVE-WORKSPACE-RESET：完成研究、架构评审、破坏性重构与纵向验收** `sev-P0`
  当前状态：`D0–D4 Approved / Protect branch cut / S1 complete / S2–S4 implemented, desktop visual acceptance pending`。
  进化走 D6。保护分支已切（本地
  `protect/cognitive-pre-clean-break-20260815` @ `2aab18cc`，未 push）。
  **S2–S4 已领取并实现；真实桌面验收仍未完成。**
  编排：[`docs/research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md`](docs/research/cognitive-workspace-reset-epic-2026-08/epic-orchestration.md)。
  架构：[`docs/architecture/laputa/architecture.md`](docs/architecture/laputa/architecture.md)。

**已冻结产品边界（不是待重新决策项）：**

- Memory CRUD 与 STM 维护不走审批；BML 是普通长期 Memory 唯一权威；完整删除
  `MemoryMd` / `memory_md` 链路且不自动导入旧数据。概念上可称 STM/LTM；注入
  权威文件是全局一份 `{config_dir}/actmem/ACTMEM.MD`，禁止核心文件 `STM.MD` /
  `MEMORY.MD`。发言即写 Pulse；每轮助手回复立刻写 Recap；空闲 10 分钟只折叠胶囊。
  子代理不进。第一版不自动晋升
  BML。正文不装配。CORE 只有一个查询工具；管理工具必须 DEFER。

- Persona 权威为 `IDENTITY.MD`（含当前身体）/ `RELATIONSHIP.MD` / `REDLINE.MD` /
  `USER.MD` / `DREAM.MD` / `DARK.MD` / `WORLD.MD`，不等于 Memory。DREAM 与 DARK
  不进首次引导；DARK 为 FEAR/SHADOW 双展位。v1 种类闭集；架构可加、用户不可加。
  历史永久保留。**P22：整机一份伴侣**，不按 profile / git 再开一套。
- BML `memory.sqlite3` **跟人格走**（S1 修订），与七文件同一套家。工作区
  `.laputa/memory.sqlite3` 是旧落点。`memory_distill` 一律 Evolution 人审（D7）。
- Evolution 管理人审后的能力文件。Diva 特色（**D6**）：AutoDream 整理 ACTMEM，
  **诞生进化提案**，提炼结果为 **SOP/Skill 文件**；不可 apply。人格提案走 P5。
  禁止 BML/`MemoryPatch`。旧混域 Inbox 仍删。不再以 GA 为进化参考。
- **MEMRULES**（S9/P21）是写记忆手册，不是人格。路径 `{config_dir}/memory/MEMRULES.MD`。
  不进 Laputa / Frozen Core / Persona 左栏。Memory 设置可编；v1 只给人改。
  日常不进全文；常驻最多几行指针；写记忆（AutoDream / BML / 蒸馏）才注入全文。
  不要和操作避坑 `[RULES]` 并成一份。
- Chat Approval Center 只保留危险运行时操作等真正授权；旧领域治理不得回流。
- 全部重构采用 clean break；无任何迁移。保护分支已切，不作为 fallback。
- **ACTMEM Recap（S8 修订）：** Pulse = 用户短原话；Recap = 本轮助手最终回复结束立刻写的一句完成态（≤200 字，不另开大模型）；10 分钟空闲只把该 session 的 Pulse+Recap **折进胶囊并从头删掉**。不是「等 10 分钟才第一次归纳」。实施落在 D4 **S3**。

决策依据：
[Evolution](docs/research/evolution-genericagent-reset-2026-08/decision-record.md)、
[Persona](docs/research/persona-markdown-clean-break-2026-08/decision-record.md)、
[STM/Memory](docs/research/stm-cross-session-clean-break-2026-08/decision-record.md)。

### Research Gate：用户已过（2026-08-14）；下面条目保留为完成物指针

- [x] **COGNITIVE-R0-CURRENT-STATE：当前系统、数据与耦合全景盘点** `sev-P0`
  **研究包已交付（2026-08-13）**；用户 Research Gate 已过：
  [`docs/research/cognitive-r0-current-state-2026-08/README.md`](docs/research/cognitive-r0-current-state-2026-08/README.md)。
  三份完成物：当前状态图、依赖/数据清单（KEEP/RENAME/DELETE/DECIDE）、旧架构失败基线。
  核心事实：混域 Proposal 链、Persona JSON↔BML 双权威、`memory_md` 仍活、
  `governance.db`/`governance.sqlite3` 双账本、事件四轨。不沿旧模型打补丁。
  R1 Evolution 切片仍有效，由全量 R0 引用。

- [x] **COGNITIVE-R1-GENERICAGENT-EVOLUTION：更新并研究 GenericAgent Evolution** `sev-P0`
  **研究包已交付（2026-08-13）**；用户 Research Gate 已过：
  [`docs/research/cognitive-r1-genericagent-evolution-2026-08/README.md`](docs/research/cognitive-r1-genericagent-evolution-2026-08/README.md)。
  本地 GA 锁定 `ee5a474`；远程 tip（API）`f06d550`；触发/Action-Verified/L0–L4/SOP-Skill/
  实验/差距/建议齐全。P1 活体实验因无 `mykey.py` 阻断，记入开放项。研究完成前仍禁止
  实现晋升状态机。进化设计改走 D6，不再跟 GA。

- [x] **COGNITIVE-R1c-GA-AUTONOMY-ORIGIN：说清 GenericAgent 自主进化从哪来** `sev-P0`
  **因果页已交付（2026-08-14）**；用户已读并走 D6：
  [`docs/research/cognitive-r1-genericagent-evolution-2026-08/ga-autonomy-origin.md`](docs/research/cognitive-r1-genericagent-evolution-2026-08/ga-autonomy-origin.md)。
  结论：无独立进化引擎。用户已读论文并决策：进化走 D6，不再跟 GA。

- [ ] **CONTEXT-DENSITY-PRINCIPLE：把「有限窗口决策信息密度」收成设计原则** `sev-P1`
  笔记：[`docs/research/cognitive-r1-genericagent-evolution-2026-08/ga-context-density-vs-diva.md`](docs/research/cognitive-r1-genericagent-evolution-2026-08/ga-context-density-vs-diva.md)。
  Diva 有装配/预算/安全，常驻层偏肥、缺逐步活动锚、巩固没有为下一轮减负。
  不推翻已冻 WORLD/ACTMEM 工具车道。D0/C 系后续设计时对照，现在不施工。

- [ ] **COGNITIVE-R1b-GA-LIVE-EXPERIMENT：GenericAgent 活体结晶实验（可选补强）** `sev-P2`
  若跑：用桌面 `keys.txt`（不入库、不进 git）。R1c 机制说清优先。测量
  Action-Verified 遵守率、未验证写入率、L1 行数违规与 patch/overwrite 比。

- [x] **COGNITIVE-R2-STM-CONTEXT：STM 与上下文分层专项研究** `sev-P0`
  **研究包已交付（2026-08-13）**；用户 Research Gate 已过：
  [`docs/research/cognitive-r2-stm-context-2026-08/README.md`](docs/research/cognitive-r2-stm-context-2026-08/README.md)。
  三份完成物：装配约束、Hold 选项与静态实验、失败/并发矩阵。产品 STM 不存在；
  CanonicalCheckpoint / SessionCheckpoint / BmlStartupIndex 已拆名。
  物理权威后来冻为 `ACTMEM.MD`（S8）。分层旧提案部分作废（Recap 已冻）。
  [`docs/research/stm-cross-session-clean-break-2026-08/stm-layering-proposal.md`](docs/research/stm-cross-session-clean-break-2026-08/stm-layering-proposal.md)。

- [x] **COGNITIVE-R3-PERSONA-WORKSPACE：Persona 文档权威与工作区技术研究** `sev-P0`
  **研究包已交付（2026-08-13）**；用户 Research Gate 已过：
  [`docs/research/cognitive-r3-persona-workspace-2026-08/README.md`](docs/research/cognitive-r3-persona-workspace-2026-08/README.md)。
  三份完成物：权威盘点、revision/Diff/CAS 选项、Markdown 工作区技术评估。JSON
  section + Proposal 仍是权威；`content_version` 只展示；changelog ≠ 文档历史；
  WORLD `project()` 无生产 Prompt 调用者。未选目录 / Diff 引擎 / CM6 扩展。

- [x] **COGNITIVE-R4-CLEAN-BREAK-SAFETY：数据影响、保护分支与删除证明研究** `sev-P0`
  **研究包已交付（2026-08-13）**；用户 Research Gate 已过：
  [`docs/research/cognitive-r4-clean-break-safety-2026-08/README.md`](docs/research/cognitive-r4-clean-break-safety-2026-08/README.md)。
  三份完成物：删除影响、保护分支协议、零残留证明目录。未抽样生产 profile。
  保护分支已切（见 I0）。禁止导入/fallback。

### 独立诊断（不复活旧主链）

- [ ] **AUTODREAM-DIAGNOSTIC-LOGGING：AutoDream 大型排查、测试与完整日志** `sev-P1`
  决策：必须整理 STM；人格可按 P19 提案。2026-08-14 独立测试已跑：
  `cargo test -p agent-diva-autodream` 现有 56 绿 + 表征 3 条；
  `cargo test -p agent-diva-manager --test autodream_laputa_e2e` 6/6 绿。
  测到的是旧合同（生命周期 / MemoryPatch / 闸门 / 报告 / Manager 治理闭环）。
  S3 已接入 ACTMEM Work 整理、仅 Pulse/Recap 冲突重试、Work 冲突失败、零
  MemoryPatch/BML 写入；人格 Markdown 允许表仍不在本条诊断范围。
  表征钉在 `agent-diva-autodream/tests/current_contract.rs`。
  仍缺阶段级结构化日志（run_id / phase / 输入摘要 / gate 拒绝 / proposal_id /
  失败码）。禁止借机写 REDLINE/DREAM/用户偏好，或恢复 MemoryPatch/SopCreate/
  Governance，也不得把 STM 改成提案。今日生产路径几乎只有 `worker.rs` 两条
  `tracing::warn`。

### Architecture Gate：全部 Research 通过用户评审后才能开始

- [x] **COGNITIVE-D0-DOMAIN-AUTHORITY：总体领域、权威、生命周期与禁止依赖设计** `sev-P0`
  **已由后续批准生效**（A/B/C + D1–D4）。正文：
  [`docs/research/cognitive-d0-domain-authority-2026-08/domain-authority.md`](docs/research/cognitive-d0-domain-authority-2026-08/domain-authority.md)。
  通过 ≠ 改生产代码。

- [x] **COGNITIVE-D1-PERSONA：Persona/WORLD/首次初始化/历史架构设计** `sev-P0`
  **用户批准（2026-08-15）。** 稿：
  [`docs/research/cognitive-d1-persona-workspace-2026-08/persona-architecture.md`](docs/research/cognitive-d1-persona-workspace-2026-08/persona-architecture.md)。
  仍不改生产代码。

- [x] **COGNITIVE-D2-MEMORY-STM：Memory/BML/STM/Context 架构设计** `sev-P0`
  **用户批准（2026-08-15）。** 稿：
  [`docs/research/cognitive-d2-memory-stm-2026-08/memory-architecture.md`](docs/research/cognitive-d2-memory-stm-2026-08/memory-architecture.md)。
  仍不改生产代码。

- [x] **COGNITIVE-D3-EVOLUTION-SKILL：Evolution/SOP/Skill 架构设计** `sev-P0`
  **用户批准（2026-08-15）。** 稿：
  [`docs/research/cognitive-d3-evolution-skill-2026-08/evolution-architecture.md`](docs/research/cognitive-d3-evolution-skill-2026-08/evolution-architecture.md)。
  仍不改生产代码。

- [x] **COGNITIVE-D4-CLEAN-BREAK-DELIVERY：接口、删除、发布与验收设计** `sev-P0`
  **用户批准（2026-08-15）。** 稿：
  [`docs/research/cognitive-d4-clean-break-delivery-2026-08/delivery.md`](docs/research/cognitive-d4-clean-break-delivery-2026-08/delivery.md)。
  **已切** `protect/cognitive-pre-clean-break-20260815` @ `2aab18cc`（本地，未 push）。
  领 S1 另说。批准 ≠ 已改生产。

### Implementation Gate：架构批准后再展开

- [x] **COGNITIVE-I0-PROTECTION-BASELINE：备份分支** `sev-P0`
  **已切（2026-08-15）。** 本地 `protect/cognitive-pre-clean-break-20260815` =
  `2aab18cc5d89769876542262c77d693dfadb7822`。不追旧 SHA。未 push。无迁移。
  不是 runtime fallback。

- [ ] **COGNITIVE-I1-CLEAN-BREAK-IMPLEMENTATION：按批准设计分切片实施并验证** `sev-P0`
  I0 已切，S1–S5 已完成；S2 Persona、S3 Memory/ACTMEM/Recap 与 S4 Evolution/Skill 的内核、运行时/API/工具与 GUI 已实现，自动化门通过，待真实桌面视觉验收。
  S5 卸旧已落地（2026-08-16，`710e7684`..`1ea54d08` 五笔功能提交：旧 GUI 治理面、Manager/Tauri 旧治理 API、AutoDream 旧提案路径、Laputa section/proposal 内核与混域 ProposalType、MEMORY.md 文件链、WorldGovernance 队列、persona-retire、旧 distill 路径物理删除；全量门通过，仅余 6 例既有 `CLI-WIREMOCK-502-PREEXISTING`）。
  S5 复核修复已落地（2026-08-16，`0c1f0dfd`..`c011e680`）：WORLD 巩固丢弃、Prompt 去掉审批假合同、ContextBuilder 测试构造器隔离、Approval/capability/CLI 去掉 memory 域。见 `docs/logs/2026-08-s5-review-repairs/`。
  S6 **机器证明门**已落地（2026-08-16，`2e58a022`）：`just cognitive-clean-break-check` 已进 `just ci`。见 `docs/logs/2026-08-cognitive-s6-proof/`。
  **S6 真机未勾**（D4 §7 八条 + §8 恢复演练）。I1 整项保持未完成。按 D4：**S1 停种子 -> S2 Persona -> S3 Memory/ACTMEM/Recap -> S4 Skill -> S5 卸旧 -> S6 证明**。
  先领须另说。每片独立验证、独立 Conventional Commit。S2–S4 **必须含该域 GUI**，不能只交 API。
  S3 必须含每轮 Recap。S6 含桌面 UI smoke（`gui-changes-need-gui-smoke`）。

### GUI 排期（跟切片走，不是另开一条无限期 UI 债）

S1 无用户可见面。S5 拆旧 UI（JSON 编辑器、Persona 右栏治理、混域 Inbox）——已落地（2026-08-16，见 `710e7684` 与 `docs/logs/2026-08-cognitive-s5-legacy-removal/`）。下面三片缺 GUI 不得标完成。

- [ ] **UI-S2-PERSONA：Persona 文档工作区** `sev-P0` `acceptance:desktop-smoke`
  左栏只七份；中央三态（当前文档 / 待审 / 历史）；最小 CM6 + Markdown 预览；
  五文件引导与 incomplete 修复。删 JSON 门、永久右栏、MEMRULES/`memory_md` 左栏。
  依据 D1。实现已落在 `93f4b554`：GUI 466 tests、生产构建、Tauri cargo check、
  本地 Vite HTTP smoke 通过；当前环境无浏览器控制执行器，仍需真实桌面视觉/交互 smoke 后勾选。

- [ ] **PERSONA-S2-DESKTOP-SMOKE：真实桌面 Persona 验收** `sev-P1`
  启动 Tauri + Manager，分别验证 uninitialized 五文件引导、incomplete 只修坏件、
  七文件 CM6 编辑/预览、CAS 冲突刷新、pending 接受/拒绝、历史 Diff/重新保存；
  同时确认未 ready 时 Chat 被独立 Persona 状态门挡住。自动化证据见
  `docs/logs/2026-08-cognitive-workspace-reset-implementation/v0.0.2-s2-persona-home/verification.md`。
  2026-08-17：首次「建立 Persona」打开即 `unknown Laputa API error` 已修（Manager
  信封 `status: "ok"` + Tauri 解码旧 `{ status: PersonaStatusView }`）；仍须重启
  桌面端后做视觉验收。

- [ ] **UI-S3-MEMORY-ACTMEM：Memory / ACTMEM / MEMRULES 工作区** `sev-P0` `acceptance:desktop-smoke`
  BML 列表/详情直改（无审批）；ACTMEM 入口展示 Pulse / Recap / Work / 胶囊；
  MEMRULES 设置可编。依据 D2 + S8 Recap。实现已落地：GUI 467 tests、生产构建、
  Tauri cargo check 通过；仍须真实桌面完成 BML CRUD、ACTMEM CAS、胶囊、MEMRULES
  与零审批路径后勾选。自动化证据见 S3 `verification.md`。

- [ ] **MEMORY-S3-DESKTOP-SMOKE：真实桌面 Memory 验收** `sev-P1`
  启动 Tauri + Manager，完成 BML 新增/编辑/软删、ACTMEM 三节编辑与冲突保稿、
  胶囊查看/单次确认删除、MEMRULES 默认/文件切换，并确认 Approval Center 零新增。
  当前执行环境可完成编译与自动化测试，但没有原生 WebView 控制器代替人工交互。

- [ ] **UI-S4-EVOLUTION-SKILL：Evolution Skill 工作区** `sev-P0` `acceptance:desktop-smoke`
  Skill 列表（搜索/启用/编辑/历史）；待审只接受/拒绝；无 Memory/人格混箱。
  依据 D3。实现已落地：GUI 446 tests、生产构建、Tauri cargo check 通过；
  Skill/CAS/历史/待审状态与异步详情防串位有自动化覆盖。当前环境的浏览器控制器
  无可用 backend，仍须完成真实桌面 smoke 后勾选。

- [ ] **EVOLUTION-S4-DESKTOP-SMOKE：真实桌面 Skill Evolution 验收** `sev-P1`
  启动 Tauri + Manager，验证 Skill 编辑/CAS 冲突保稿、停用、历史、硬删与内置回显，
  Settings ZIP/Marketplace 只新装，用户请求、AutoDream/distill 待审与接受，接受后新
  Session 可发现，以及 Evolution 无 Memory/Persona 混箱。自动化证据见
  `docs/logs/2026-08-cognitive-s4-skill-evolution/v0.5.0-skill-evolution/verification.md`。

- [x] **ACTMEM-S3-RECAP：S3 接线每轮 Recap** `sev-P1`
  助手最终回复结束立刻写 `## Recap`；Pulse 仍只收用户短原话；10 分钟只折叠。
  依据 S8 修订 / D2。已实现机械首个非代码结论段、≤200 字即时写入；暂停时间测试
  覆盖 Pulse、Recap、10 分钟折叠、代次取消、cron/子代理排除及 reset 清理。

## 产品与架构

- [ ] **PLAN-MODE-PHYSICAL-STATE-MACHINE：Plan Mode 物理限制状态机** `sev-P1`
  从已完成的 Context 主线分轨，需另行冻结状态、能力矩阵和非法迁移验收。

- [ ] **EVENTBUS-TRAIT-HOOKS：EventBus Trait Hook 管道** `sev-P1`
  来源于 OpenHarness 调研；保留为未来扩展点，当前延期。

- [x] **WORLD-MEMRULES-GATE：WORLD 写核 R6 拦截** `sev-P2`
  D1 已删 WorldGovernance 队列。R6 进 WORLD **写核**（用户直存 / 接受 P5）。
  不得恢复 Memory 审批。已在 `352cd57f` 实现：提案新增/修改必须是带 status/source
  的有界 claim，新增散文拒绝；AutoDream 不能改既有 claim；用户直存仍保持直存。

- [x] **MEMRULES-DEFAULT-SEED-ALIGNMENT：默认 R1–R7 与生产策略对齐** `sev-P3`
  D2 已定并随 S3 落地：R4 为 revision/CAS 直写，证据只作 advisory；缺文件使用
  内置 R1–R7，只有用户首次保存才创建 `{config_dir}/memory/MEMRULES.MD`。

- [ ] **RG-CODE-GOV 后续分期** `sev-P2`
  原位治理 G0/G1 已完成；剩余 G2 Manager handler 变薄、G3–G5 GUI Host/state/DTO。
  不回迁 deep-governance 大爆炸。设计：`docs/dev/agent-loop-manager-gui-governance/`。

- [ ] **全仓库代码清理提案审批与排期决策** `sev-P3`
  对 `docs/logs/2026-08-05-code-audit-proposal/v0.1.0-code-audit-proposal/` 的清理集合
  做取舍；不得把已被 Evolution/Memory 新决策覆盖的旧方案重新实现。

- [ ] **GMH-53：灰度与清理** `sev-P2`
  只处理仍有效的通用治理/发布内容；Memory governance 与旧 Evolution 部分由上述
  clean-break 决策取代。

- [ ] **day/hour token 死循环熔断安全阀决策** `sev-P3`
  定位为极高默认阈值的异常循环保护，而不是常规预算管理；决定全局或会话窗口，以及
  与 `session_token_budget_limit`、`RejectionCircuitBreaker` 的交互后再实施。

- [ ] **Workspace CLI managed-path 与 runtime 任意路径契约** `sev-P3`
  路径穿越已修；仍需统一 `config_dir/workspaces/*` 与 runtime 任意路径模型。

- [ ] **CLARIFY-HITL Phase 3** `sev-P3`
  已有 `ask_user` 运行时、CLI/Tauri/GUI 表面；剩余 Plan 矩阵、subagent 禁用断言与
  可选 messaging clarify。不得并入审批抽屉或 governance ledger。

- [ ] **UX-DR-3/4/7** `sev-P3`
  Sprint 评审遗留 UX 缺口；恢复前先重新确认原问题仍存在并补专项设计。

## 自动化与生产路径证明

- [ ] **BACKGROUND-TASK-PRODUCTION-E2E：后台任务生产路径纵向证明** `sev-P2`
  覆盖 `enqueue_background_task` 的 assembly/agent loop 接线、supervised worker
  启动/排空/取消/重启、subagent 终态，以及上下文与预算继承。

- [ ] **STEPFUN-REAL-ENDPOINT-E2E：StepFun model pass-through** `sev-P3`
  单测已覆盖 model 透传；仍需使用桌面 `keys.txt` 做真实 endpoint E2E，密钥和未脱敏
  响应不得进入仓库、日志、夹具或提交。

## 人工与真实桌面验收

- [ ] **M3-HITL-DESKTOP-SMOKE：审批三模式集中验收** `sev-P2`
  验证谨慎/智能/信任三模式、trusted 规则学习、重启保持和危险 shell 仍受 Guardian
  限制。审批只从聊天页统一 Approval Center 出现；不包含 Memory CRUD 或旧 Evolution。
  参考：`docs/logs/2026-08-m3-hitl-closure/`。

- [ ] **WINDOWS-RELEASE-EXEC-ACCESS：恢复本地 release 可执行启动** `sev-P1`
  当前普通用户启动 release EXE 返回 Windows OS error 5；需明确的人工/系统策略授权，
  解决后再执行 M3 与其他真实桌面 smoke。

- [ ] **SANDBOX-SAVE-FIX-DESKTOP-SMOKE** `sev-P2`
  切换沙箱模式并保存，检查配置落盘及清空 timeout 边界。步骤：
  `docs/logs/2026-08-sandbox-settings-save-fix/v0.1.0-sandbox-save-fix/acceptance.md`。

- [ ] **CLARIFY-HITL-DESKTOP-SMOKE** `sev-P2`
  用真实 LLM 触发 `ask_user`，分别验证 CLI/GUI 提问、回答、超时和恢复。步骤：
  `docs/logs/2026-08-ask-user-hitl-research/v0.2.0-ask-user-surface/acceptance.md`。

- [ ] **GATEWAY-PORT-DESKTOP-SMOKE** `sev-P3`
  配置 `gateway.port` 后确认 gateway 监听配置端口；未配置时仍使用 3000。步骤：
  `docs/logs/2026-08-small-fixes-batch/v0.5.1-small-fixes-batch/acceptance.md`。

- [ ] **GUI-PROVIDER-ERROR-RETRY-DESKTOP-SMOKE** `sev-P2`
  用真实失败/重试场景确认 GUI 显示重试进度、stall 和最终错误，不因 request ID 或
  SSE 断流永久挂起。参考：`docs/logs/2026-08-gui-provider-error-visibility/`。

- [ ] **MASK-FEATURE-ACCEPTANCE** `sev-P2`
  恢复 `.sisyphus/plans/mask-feature-implementation.md` 前先复核现状，再执行剩余验收。

## Reliability / Test Debt

- [ ] **LAPUTA-STORAGE-STALE-LOCK-FLAKE** `sev-P2`
  Windows 全工作区负载下 stale lock 回收偶发 `LockTimeout`，定向重跑与后续 CI 通过；
  需隔离临时目录锁回收时序。关联 `agent-diva-laputa/src/lock.rs`。

- [ ] **WORKSPACE-GUI-TOOLING-LOAD-FLAKES** `sev-P2`
  GUI embedded gateway 启动和 tooling registry timeout 测试曾在 full suite 偶发失败、
  focused 重跑通过；隔离共享资源和时序依赖。

- [ ] **CLI-WIREMOCK-502-PREEXISTING** `sev-P2`
  CLI approval wiremock 用例在 Windows 环境偶发/持续返回 502；排查系统代理绕过与 mock
  服务器隔离，关闭标准是 `just test` 全绿。

- [ ] **GUI-TAURI-PLAN-STREAM-DISCONNECT** `sev-P3`
  两条 Plan SSE/Tauri 循环仍可能在无终止事件断流时静默返回；统一为明确错误或恢复事件。

- [ ] **AGENT-DIVA-FILES-CLIPPY** `sev-P3`
  修复 `agent-diva-files/src/s3.rs` 在 Rust 1.94 下的
  `empty_line_after_doc_comments`，使用独立机械提交。

- [ ] **WORKSPACE-MSRS-1.80-DEPENDENCY-CONFLICTS** `sev-P2`
  工作区声明 Rust 1.80，但 ICU/Darling/Pest/CRC/Tauri 等依赖存在更高 MSRV；需要独立
  pin/升级方案，不削弱现有 gate。

- [ ] **MSRV-ISOLATED-TARGET-CACHE** `sev-P2`
  所有 `cargo +1.80` 探测必须使用独立 `CARGO_TARGET_DIR`，避免污染默认 target cache；
  将此约束固化进验证命令或脚本。

- [ ] **LAPUTA-TESTS-1.94-ALL-TARGETS-CLIPPY** `sev-P3`
  `cargo clippy -p agent-diva-laputa --all-targets -- -D warnings` 仍有测试目标 dead code、
  `cmp_owned` 等 lint；生产库目标和 `just check` 不受影响，独立机械修复。

- [ ] **BML-GOVERNED-SEAM-DEAD-CODE** `sev-P3`
  S5 移除治理协调器（`governed_apply`/`MemoryGovernanceCoordinator`）后，
  `TypedMemoryStore::put_governed` / `rollback_governed` 成为无调用方的存储内部接缝。
  按 D4 §3.2 必须保留 BML 表结构，清理时只删代码路径与 `bml/mod.rs` 边界说明同步更新，
  不动 schema。关联 `agent-diva-laputa/src/bml/`。等 S6 扫描门先落地。

- [ ] **MEMORY-CRUD-PROPOSAL-CREATED-DEAD-ENUM** `sev-P3`
  `MemoryCrudOutcome::ProposalCreated` 与 `SyncTurnStatus::ProposalCreated` 仍是公开枚举，
  巩固路径还有死分支匹配。生产 MemoryHome 不再产出该结果。独立删除枚举与匹配臂，
  不要和 BML schema 清理绑在一起。关联 `agent-diva-core/src/memory/`。

- [ ] **GOVERNANCE-LEDGER-SINGLE-RECORD-BRICK** `sev-P2`
  `governance.db` 中任何一条带未知 wire 值（如已删除的 `memory_apply` capability）的
  历史记录都会在启动全量重放（`states_page` → `replay` → `request.validate()`）时
  fail-closed，整条网关引导被单条死数据阻断（2026-08-17 实际发生，已手工清理 16 条
  `memory-apply:*` 事件，备份 `governance.db.bak-memory-apply-20260817`）。需要独立
  决策：启动恢复路径对无法重放的孤立聚合按条隔离/跳过并告警，还是增加一次性启动
  迁移。保持授权路径 fail-closed 不变。关联 `agent-diva-core/src/governance/ledger.rs`、
  `agent-diva-sandbox/src/approval_coordinator.rs`。

## Archive Index

- [`docs/archive/todolist/README.md`](docs/archive/todolist/README.md)
- 清理前完整快照：
  [`snapshot-before-2026-08-13-cleanup.md`](docs/archive/todolist/snapshot-before-2026-08-13-cleanup.md)
