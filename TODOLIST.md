# TODOLIST

项目级**活跃**待办与延期项。已完成历史见 Archive Index。

**严重度标签**使用 `sev-P0`（阻断）…`sev-P3`（琐碎），与已归档的
「Plan 阶段 P1/P2/P3」命名无关。

---

## Active Plan

当前主线：在 **typed Laputa clean-break（GMH-24）之后**，先完成
**AutoDream → 候选 → Laputa 提案 → typed Memory → Recall 反馈**的产品纵向闭环，
再执行最终真实桌面验收。GMH-00..24 架构底座已完成并归档，但 Evolution 当前仍是
占位/不可用产品，不能以已有页面或提案底座宣称可用。

全量执行顺序、依赖、人工暂停点和项目级完成定义见
[`docs/architecture/todolist-master-execution-plan.md`](docs/architecture/todolist-master-execution-plan.md)。
Codex“目标”功能必须按该蓝图逐切片推进，不得把清单机械并行执行。完整产品闭环计划见
[`docs/dev/autodream-laputa-product-closure/`](docs/dev/autodream-laputa-product-closure/)。

- [ ] **GA-MEM-PARITY：GenericAgent 功能对齐 × Memory/Laputa/AutoDream 完全可用** `sev-P0`
  2026-08-05 完成只读盘点：相对 GenericAgent，Agent 侧记忆管理工具面基本缺失
  （无 add/list/search/update/remove/distill），prompt 仍承诺 “memory tools”；
  Laputa/Typed 的 `sync_turn` 多为 pending proposal 而非即时权威；工作记忆、L0–L4
  分层纪律、主动蒸馏未产品化；AutoDream 有底座与手动 API，自动触发与审查闭合待证。
  权威入口：
  [`docs/logs/2026-08-05-memory-ga-parity-inventory/v0.0.1-gap-inventory/inventory.md`](docs/logs/2026-08-05-memory-ga-parity-inventory/v0.0.1-gap-inventory/inventory.md)。
  实施按 inventory Wave 0–5；**禁止**回退 Mentle 或 “file_write 即权威”。
  验收见同目录 `acceptance.md`。Open Questions（即时 durable vs proposal-first 等）
  须在 Wave 1 编码前冻结。

- [x] **E0–E7：AutoDream–Laputa 开箱可用纵向闭环** `sev-P0`
  按 Experience Journal、可恢复 Orchestrator、受限 Reflection、Candidate Gate、
  Laputa proposal/统一治理、typed apply、Recall feedback、一体化 GUI、恢复与发布门
  逐切片完成。普通 AutoDream trigger 必须启动真实 worker；禁止固定模板候选、
  直接写 Memory、自批准或静默降级。每切片四件套、独立提交、不 push。
  相关：`docs/dev/autodream-laputa-product-closure/09-project-management.md`。
  - [x] **E0：运行时表征与诚实产品状态**：普通手动触发会执行受限 worker 并返回
    终态；失败具有稳定的无载荷原因码；Evolution 页面明确提示当前仍是规则式候选、
    尚未形成可用闭环。
  - [x] **E1：Experience Journal**：把真实任务执行结果、工具结果与错误转成有界、
    可追溯、可脱敏的反思输入。
    - [x] **E1A 在线执行证据**：AgentLoop 唯一工具 seam 记录 payload-free 结果，
      具备确定性 ID、幂等、workspace 隔离、物理 retention 和损坏行拒绝；AutoDream
      优先收集该证据。
    - [x] **E1B 离线回填与回滚**：从已有 session tool-result 生成 dry-run manifest，
      显式 apply，按 manifest 回滚；不得读取或复制完整工具输出。
  - [x] **E2 可恢复 AutoDream 编排器**：手动与报告触发统一持久化
    `queued → gathering → reflecting → validating → publishing → completed` 阶段、
    attempt 和 deadline；Manager 异步派发并在启动时恢复中断任务；发布重放复用
    确定性提案，旧版不完整运行 fail closed。
  - [x] **E3 Reflection/Candidate Gate**：生产 Reflection 复用现有 provider resolver，
    使用有界、PII 脱敏输入与无工具 JSON schema；候选包含真实 proposal type、scope、
    sensitivity、confidence、value 与失效条件，并在发布前拒绝无证据、仅 compaction、
    forged evidence、重复、直接矛盾、低价值、越 workspace/容量、prompt injection
    和敏感内容。无 provider 明确失败，无合格候选诚实完成为 `no_candidates`。
  - [x] **E4 提案治理与 rejection suppression**：AutoDream 候选一对一进入
    PendingReview；Memory 决策继续使用 core Governance Ledger，编辑后旧 request/receipt
    失效；决策已提交但 proposal 状态未落盘时可重试补齐；拒绝内容以 90 天、1000 条、
    payload-free digest suppression 抑制原样重提，显著变化可重新进入 gate。
  - [x] **E5 typed apply、Recall 与反馈**：批准后的 AutoDream 提案映射为 canonical
    `MemoryRecord` 并保留 run/evidence/governance provenance；AgentLoop 在唯一 turn
    终态 seam 提交 payload-free Recall 成功/失败/纠错反馈；下一次 Reflection 可把
    纠错转为 governed deprecation，apply 时生成 content-free tombstone/supersedes，
    无效或缺失 target fail closed。
  - [x] **E6 一体化 Evolution Workspace**：单一页面汇总 typed authority/revision、
    AutoDream phase/attempt/输入覆盖、候选提案、治理动作、changelog/rollback 与
    payload-free Recall feedback；支持立即反思、取消、查看 run 提案、可视化编辑后
    生成新 proposal revision、批准并应用、拒绝、暂缓与回滚；typed degraded 原因
    显式展示。
  - [x] **E7 恢复与发布门**：完成 canonical workspace identity、单一副作用 seam、
    observability、恢复演练、全测试与发布候选自动化 E2E。
    E7 必须在关闭占用 debug 二进制的桌面进程后复跑 `just test`，并解决或规避当前
    Windows GUI lib test 的 MSVC `LNK1140` PDB 限制，不能以 `cargo check` 代替。

- [x] **Evolution 当前状态必须明确为不可用/建设中** `sev-P0`
  在 E0 characterization 前不得将现有页面标记为可用。允许且要求实施本闭环；
  不在闭环内的旧 AutoDream/Evolution 承诺继续冻结，完全无后端的入口必须隐藏、
  删除或明确 degraded。

- [ ] **G2D+：全流程完成后的真实桌面最终验收** `sev-P0`
  自动化纵向 E2E 与发布门通过后再由用户执行。保留原批准、拒绝、编辑后批准、
  重复点击、重启恢复、回滚六场景，并新增“真实任务 evidence → AutoDream →
  proposal → apply → 新会话 Recall → rollback 后消失”。不得以自动化冒充真机。
  保留脱敏 ID、revision、桌面版本、界面结果和 Manager/Tauri 日志。

完成索引（已归档）：Ask 只读边界、GMH-24A/B/C、G2D migration revision 修复等见
[`docs/archive/todolist/completed-through-2026-07-30.md`](docs/archive/todolist/completed-through-2026-07-30.md)。

---

## Operational Rules

standing policy（非功能债，执行相关验证时遵守）：

- [ ] **真机里程碑须先提醒用户** 需要真实设备/桌面/外部集成时，先列出环境、步骤、
  观察点与失败诊断物，获配合后再验收；禁止用模拟冒充真机。
- [ ] **真实 API 使用桌面 `keys.txt`** 不得把密钥或未脱敏内容写入仓库、日志、
  夹具或提交。

---

## Open Backlog

### Product / Architecture

- [ ] **RG-CODE-GOV 后续分期（可选实现入口）** `sev-P2`
  原位治理设计完成；**禁止**回迁 deep-governance 大爆炸。
  已完成并归档：G0 characterization、G1 AgentLoop 瘦身。
  未做：G2 Manager handler 变薄、G3–G5 GUI Host/state/DTO。
  设计：`docs/dev/agent-loop-manager-gui-governance/`。
  与 GMH-40 副作用 seam 有交集时优先走 GMH 故事，避免双轨。

- [ ] **待决策：全仓库代码清理提案集合审批与实施决策** `sev-P3`
  提案集合：[`docs/logs/2026-08-05-code-audit-proposal/v0.1.0-code-audit-proposal/`](docs/logs/2026-08-05-code-audit-proposal/v0.1.0-code-audit-proposal/)。
  经 4 路子代理全量深度审计完成：需用户决策是否批准并排期清理（包含 Migration 1389 行未挂载代码、Memory 影子比对逻辑、16 个未用 Tauri Command、10 个废弃 HTTP 路由、Vue 双通道 SSE 与通道死逻辑等 6-Wave 清理案）。


### GMH 未完成（Phase 3–5）

> Phase 0–2（GMH-00..24）已完成，详见 archive。以下为仍开放 story。

- [x] **GMH-30：统一审批协调器** `sev-P1`
  Plan / Sandbox / Memory 经同一协调接口；执行器只消费有效 receipt。
  suspend/resume、重启恢复、取消、超时、重复响应、多客户端。
  - [x] **GMH-30A：领域协调器与状态机**：core 协调器统一组合纯策略与 append-only
    ledger；Plan/Sandbox/Memory envelope 共用 Pending 入口，安全允许和策略拒绝不制造
    审批记录，decision/state 保持 version CAS、幂等和 payload-free 约束。
  - [x] **GMH-30B：receipt 消费与恢复控制**：执行器只消费有效 receipt，并补齐
    - [x] **GMH-30B1：Sandbox durable receipt 与重启撤销**：命令审批使用统一
      `.laputa/governance.db`，Once 在执行前消费，session 五分钟到期，global rule
      仅在 Rule receipt 后持久化；启动分页撤销当前 workspace 的 Pending/Allowed，
      不保存或重放命令正文。
    - [x] **GMH-30B2：Plan/Memory receipt 最终统一**：收口 Plan 审批表与 Memory
      apply 的执行消费和恢复语义。
    suspend/resume、重启恢复、取消、超时、重复响应和多客户端协调。
- [x] **GMH-31：Manager API / SSE / Tauri 契约** `sev-P1`
  pending/详情/approve/edit/reject/cancel/审计；幂等键与版本前置条件；
  typed reason codes 与事件序列。统一 service 投影三域 authority，HTTP 提供稳定
  list/detail/decision/cancel 与 durable cursor SSE；Tauri 仅负责 transport，Rust fixture
  由 TypeScript guard 共用验证，旧 Command/Plan/Laputa wire contract 保留。
- [x] **GMH-32：GUI 决策中心与就地审批** `sev-P1`
  全局 pending badge/抽屉与 Chat 就地卡统一消费 Manager projection；支持三域筛选、
  风险/证据/diff/TTL、Command grant、Memory 源页面编辑/应用导航、事件去重、stale
  刷新和 outcome-unknown 禁止重发，并以文字状态、焦点圈与 44px 控件满足键盘边界。
- [x] **GMH-33：CLI/headless 行为** `sev-P2`
  `approvals review/list/decide/cancel` 统一消费 Manager projection，批量交互必须显式
  选择；`agent` 默认 fail-closed，只有 `--approval-mode queue` 且 Manager 可用时才返回
  可查询的 Plan/Memory Pending，Command queue 因原文不持久化而撤销并返回稳定失败码。
  shell / Memory 高风险 / Plan、JSON/exit code 与 unavailable 路径均有自动化覆盖。
- [x] **GMH-40：Agent Loop 单一副作用 seam** `sev-P1`
  组装/pre-call/执行同一治理快照；turn 分段可取消可度量；子代理/cron 禁止提权。
- [ ] **GMH-41：自治预算与熔断** `sev-P2`
  turn/session/day 限额；拒绝风暴熔断；离线高风险排队或拒绝。
- [x] **GMH-42：治理可观测性与审计** `sev-P2`
  decision latency、人工等待、deny/stale receipt、Memory apply/rollback 指标与
  correlation 证据链。
- [ ] **GMH-50：兼容迁移与 feature flags** `sev-P2`
  不得重新引入 Mentle；禁止长期双写。
- [x] **GMH-51：安全与数据恢复演练** `sev-P1`
- [ ] **GMH-52：全量验收** `sev-P1`
  `just fmt-check` / `check` / `test`、deletion-proof、GUI、真实 smoke。
- [ ] **GMH-53：灰度与清理** `sev-P2`

里程碑（更新后）：

- [x] M0 / M1 / M2 — 见 archive（含 GMH-24 clean-break）
- [ ] **M3** HITL 闭环（GMH-30..33）
  - [x] **M3-GOAL-PREP：长任务执行资料包**：冻结 Goal 边界、当前缺口、
    Plan/Memory/Command 消费与恢复矩阵、统一 API/SSE/Tauri、全局抽屉与 headless
    行为、自动化/人工验收和四个阶段停点。权威入口：
    `docs/dev/governance-m3-goal/README.md`。GMH-30/31/32/33、纳入的代码债和最终
    自动化矩阵均已完成；候选 `2fbb07d3` 等待一次集中人工 smoke。Windows release
    EXE 在当前自动化会话仍被 OS error 5 拒绝，作为人工矩阵首个硬门禁保留。
- [ ] **M4** Agent Loop 接入（GMH-40..42）
- [ ] **M5** 灰度发布（GMH-50..53）

统一 DoD：设计/威胁模型；成功/拒绝/超时/并发/重启测试；用户可见路径 smoke；
`docs/logs` 四件套；单 concern Conventional Commit；不擅自 push。

### Reliability / Test Debt

- [ ] **SANDBOX-SAVE-FIX-DESKTOP-SMOKE: manual GUI smoke for sandbox settings save** `sev-P2`
  2026-08-05 修复了 GUI 沙箱设置保存失败（kebab-case vs snake_case 枚举不匹配，
  `docs/logs/2026-08-sandbox-settings-save-fix/v0.1.0-sandbox-save-fix/`）。自动化门
  （vitest 451 用例 + vue-tsc/vite build）已通过，但真实 Tauri 桌面冒烟（切换模式保存、
  检查 `~/.agent-diva/config.json`、清空 timeout 边界）未在本会话执行，需按
  `acceptance.md` 步骤人工验收后勾选本项。

- [x] **GUI-RUST-1.94-ALL-TARGETS-CLIPPY: clean pre-existing Tauri test lint** `sev-P3`
  `cargo clippy -p agent-diva-gui --all-targets -- -D warnings` 在既有测试构造代码
  `agent-diva-gui/src-tauri/src/lib.rs` 命中 `field_reassign_with_default`。GMH-31 的
  Tauri `cargo check`、前端测试与生产构建均通过；在独立兼容性提交中机械修复，
  不与审批契约功能混合。

- [x] **WORKSPACE-TEST-UNUSED-FIXTURES: clean two test-only warnings** `sev-P3`
  `just test` 在既有 `agent-diva-channels/src/feishu.rs` fixture 的 `handler` 与
  `agent-diva-tools/src/filesystem.rs` fixture 的 `temp_dir` 报告未使用变量。全量测试
  与 `just check` 均通过；在独立测试清理提交中移除或以下划线明确保留。

- [x] **CORE-RUST-1.94-ALL-TARGETS-CLIPPY: clean pre-existing test lints** `sev-P2`
  `cargo clippy -p agent-diva-core --all-targets -- -D warnings` exposes 19
  pre-existing test-target findings across supervised/config/session/audit and
  related modules. The production library target and official `just check`
  pass; repair these warnings in a separate compatibility slice.

- [x] **MANAGER-LOG-RANGE-FULL-SUITE-FLAKE: isolate shared log state** `sev-P2`
  The first 2026-08-03 `just test` run failed
  `handlers::logs::tests::logs_filter_by_range` because an unexpected event
  entered the selected range; the focused rerun passed. Isolate its log source
  or clock/range fixture so workspace parallelism cannot contaminate it.

- [x] **MANAGER-RUST-1.94-ALL-TARGETS-CLIPPY: clean pre-existing test lints** `sev-P2`
  `cargo clippy -p agent-diva-manager --all-targets -- -D warnings` exposes six
  pre-existing test-target findings (`items_after_test_module`, needless borrow,
  `len_zero`, and two `single_match` cases). Production `cargo check` and the
  official `just check` remain the delivery gate; repair these in a separate
  compatibility slice without mixing them into GMH-30B1.

- [x] **CLIPPY-LINES-MAP-WHILE: update fallible line iteration** `sev-P2`
  The run-event reader now propagates line I/O failures explicitly while still
  skipping malformed JSON, with focused invalid-data coverage. Workspace gate
  restoration is verified with the GMH-30A batch.

- [x] **LAPUTA-RECOVERY-RECEIPT-FIXTURE: restore prepared journal recovery test** `sev-P1`
  The fixture now creates a non-expired approval revision and proves prepared
  recovery, one-time receipt consumption, and idempotent replay. Full workspace
  gate restoration is verified with the GMH-30A batch.

- [ ] **WINDOWS-RELEASE-EXEC-ACCESS: restore local release executable launch** `sev-P1`
  The 2026-08-02 Tauri rebuild produced updated EXE/NSIS/MSI artifacts, but
  Windows rejected `Start-Process target/release/agent-diva-gui.exe` with OS
  error 5 (`Access denied`). Diagnose endpoint protection/file policy or the
  post-link hard-link state, then repeat the desktop health smoke.
  2026-08-03 final-candidate audit rebuilt the complete Tauri release successfully;
  source and isolated copies have matching SHA-256, permissive ACLs, no Zone.Identifier,
  and no matching AppLocker/Code Integrity/Defender event, but all ordinary-user launch
  attempts still return OS error 5. Resolving unsigned-binary trust/endpoint policy needs
  explicit human/system-policy authorization; the final manual smoke starts with this gate.

- [x] **跨平台 canonical workspace identity 与 identity-only migration** `sev-P1`
  Windows 路径分隔符差异曾使 Migration 与 Manager 对同一 workspace 计算出不同
  identity，并由 typed fail-closed 检出。统一 CLI/Manager/GUI/Migration 的 canonical
  规则，提供只迁 identity、不复制或改写 Memory 内容的可验证迁移与回滚。
  相关：执行蓝图 B2、GMH-24 Migration/typed authority。

- [x] **QQ invalid-resume 集成测试 load-sensitive** `sev-P2`
  全量 gate 偶发 opcode 乱序；隔离重跑通过。
  `agent-diva-channels/tests/qq_reconnect_integration.rs`。
- [x] **Memory authority provider 选择缺 focused characterization** `sev-P2`
  `.laputa/` 打开失败应选 `DegradedMemoryProvider`；
  `cargo test -p agent-diva-agent memory_boundary` 当前 0 测。
  `agent-diva-agent/src/memory_boundary.rs`。
- [x] **Manager library suite 在 workspace gate 下 load-sensitive** `sev-P2`
  隔离 `cargo test -p agent-diva-manager --lib` 可通过。
  需定位并同步不稳定用例。
- [ ] **Workspace Rust 1.80 MSRV 与无关新依赖冲突** `sev-P2`
  GMH-24 已移除 Mentle 链；仍有 ICU/Darling/Pest/CRC/Tauri 等声明 MSRV >1.80。
  独立 pin/升级切片；勿削弱 clean-break gate。
- [ ] **MSRV 探测污染默认 target cache** `sev-P2`
  未来 `cargo +1.80` 须用独立 `CARGO_TARGET_DIR`。
- [ ] **Laputa service 预存 clippy `int_plus_one`** `sev-P3`
  `agent-diva-laputa/tests/service.rs` 四处断言风格问题。
- [ ] **StepFun 真实 endpoint E2E（model pass-through）** `sev-P3`
  单测已覆盖透传；缺真实 key 时的 E2E。使用桌面 `keys.txt`，勿入库。

### Plan Residual（2026-07-30 重评后仍 open）

对照当前代码保留；完整历史处置表见
[`docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md`](docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md)。

- [x] **扩展 phase×capability / transition 矩阵与 denial 副作用测试** `sev-P2`
  policy 已 fail-closed，但缺完整非法迁移笛卡尔与 denial 前后 store 快照断言；
  assembly 仅子集工具。
  相关：`agent-diva-core/src/planning/policy.rs`、
  `agent-diva-agent/src/tool_assembly.rs`、agent-loop 集成夹具。
- [x] **runtime 配置热更新后按 phase 重建工具表** `sev-P2`
  `rebuild_tools_for_active_phase` 当前 `rebuild_tools_for_turn(..., None, ...)`，
  网络/MCP 更新可能短暂丢掉 phase 边界。
  `agent-diva-agent/src/agent_loop/loop_runtime_control.rs`。
- [x] **空 execution TODO 列表的 Verify 门闩** `sev-P2`
  `PlanVerifier::verify` 在 `total == 0` 时直接 Pass，可能让无步骤计划误完成。
  `agent-diva-agent/src/planning/verifier.rs`。
- [x] **预存在 TODO 与 materialize 策略文档化/修复** `sev-P2`
  `TodoAlreadyMaterialized` 仍可永久卡住 Always 路径；需产品决策：视为已物化
  成功、禁止预批准写入、或提供清理 API。
  `agent-diva-core/src/planning/store.rs`。
- [x] **删除 orchestrator 内注释掉的死迁移矩阵** `sev-P3`
  `agent-diva-agent/src/planning/orchestrator.rs` 大段注释旧表；core 已是唯一真相。

### Deferred Product

- [ ] **Skill 可视化全生命周期编辑器（延期）** `sev-P3`
  产品对象只有 Skill；SOP 不建立独立类型、标签、入口、DTO、存储或运行时，
  “SOP”仅是用户对某类 Skill 内容的称呼。未来若启动，统一为全部 Skill 提供
  可视化创建、读取、编辑、删除、校验、预览与安全审查，而不是只做 SOP 编辑器。
  当前不在预期范围，须由用户明确恢复后再设计/实现。
  决策：`docs/architecture/skill-sop-unification.md`。

- [ ] **Mask feature 验收** `sev-P2`
  `.sisyphus/plans/mask-feature-implementation.md` 剩余项，待独立恢复。
- [ ] **Wave 3 residual：enqueue_background_task 生产路径 E2E 证明** `sev-P2`
  `agent-diva-tools/src/enqueue_background_task.rs`、assembly / agent_loop。
- [ ] **Wave 3 residual：supervised subagent worker bootstrap 刻画** `sev-P2`
  启动/排空/取消/重启缺生产路径测试。
- [ ] **Wave 3 residual：subagent 终态生命周期 E2E** `sev-P2`
- [ ] **Wave 3 residual：background task 上下文与预算继承 E2E** `sev-P2`
- [ ] **Wave 3 residual：workspace CLI managed-path 与任意路径产品契约** `sev-P3`
  路径穿越已修；managed `config_dir/workspaces/*` 与 runtime 任意路径模型仍分歧。
- [ ] **Approval dual-channel unification + 超时 UX** `sev-P3`
  Legacy `command-approval-requested` SSE（`App.vue:2003`）与统一 `approval-event`
  （`App.vue:2006`）两套通道并存，GUI 同时维护 `commandApprovals` 与 `unifiedApprovals`，
  容易丢事件。同时 `ApprovalCoordinator` 默认超时后直接 `Expired`，前端无倒计时，
  用户无法感知剩余审批窗口。2026-08 cautious-mode 修复时仅修了后端透传+escalation，
  通道合并与倒计时待独立迭代。
  关联：`docs/logs/2026-08-cautious-approval-fix/v0.5.0-cautious-approval/`.
- [ ] **审批 UI 三重显示去重** `sev-P2`
  同一 ExecTool 审批请求在 GUI 同时出现三种视觉形态：
  (1) Drawer 内的 `ApprovalCenterCard`（完整样式，`ApprovalCenterDrawer.vue:155`）；
  (2) Chat 消息流底部的内联 `ApprovalCenterCard`（compact，`ChatView.vue:1051`）；
  (3) 偶尔闪现的 legacy `ApprovalBanner`（`ChatView.vue:1068`）。
  根因：后端 `approval_coordinator` 同时广播 legacy `command_approval_requested`
  与写入 governance ledger，前端互斥守卫只盖 legacy Banner。
  修复方案：单通道 + 单渲染点（保留 Drawer 为唯一全量入口，删除 Chat 内联两份重复渲染），
  详见 `~/.qoder-cn/plans/daring-dune-thrush.md`（2026-08 修订版）。
  关联：`docs/logs/2026-08-cautious-approval-fix/v0.5.0-cautious-approval/`.
- [ ] **UX-DR-3/4/7** `sev-P3`
  Sprint 评审 UX 缺口，待专项设计。

---

## Archive Index

| 文档 | 内容 |
|------|------|
| [`docs/archive/todolist/completed-through-2026-07-29.md`](docs/archive/todolist/completed-through-2026-07-29.md) | 早期完成项与 P2/P3 boundary 实现索引 |
| [`docs/archive/todolist/completed-through-2026-07-30.md`](docs/archive/todolist/completed-through-2026-07-30.md) | G0、GMH-00..24、Deferred Review Program 关闭、G1 等 |
| [`docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md`](docs/archive/todolist/plan-todo-p1-p3-disposition-2026-07-30.md) | 2026-07-11 Plan P1–P3 评审逐条 CLOSED/SUPERSEDED/OBSOLETE/OPEN-RESIDUAL |
| [`docs/logs/2026-07-todolist-triage/v0.0.1-archive-and-residual/`](docs/logs/2026-07-todolist-triage/v0.0.1-archive-and-residual/) | 本次瘦身迭代日志 |

**原则：** 主清单只保留本阶段 Active Plan 与真实未完成项；完成项迁 archive，不静默删除。
