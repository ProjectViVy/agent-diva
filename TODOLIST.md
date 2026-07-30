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

- [ ] **E0–E7：AutoDream–Laputa 开箱可用纵向闭环** `sev-P0`
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
  - [ ] **E7 恢复与发布门**：完成 canonical workspace identity、单一副作用 seam、
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

### GMH 未完成（Phase 3–5）

> Phase 0–2（GMH-00..24）已完成，详见 archive。以下为仍开放 story。

- [ ] **GMH-30：统一审批协调器** `sev-P1`
  Plan / Sandbox / Memory 经同一协调接口；执行器只消费有效 receipt。
  suspend/resume、重启恢复、取消、超时、重复响应、多客户端。
- [ ] **GMH-31：Manager API / SSE / Tauri 契约** `sev-P1`
  pending/详情/approve/edit/reject/cancel/审计；幂等键与版本前置条件；
  typed reason codes 与事件序列。
- [ ] **GMH-32：GUI 决策中心与就地审批** `sev-P1`
  风险/证据/diff/授权时长；Memory edit-and-approve；badge 与重连去重。
- [ ] **GMH-33：CLI/headless 行为** `sev-P2`
  交互可批；非交互 fail/queue，绝不默认放行。Gate G3：shell / Memory 高风险 /
  Plan 各一条 E2E。
- [ ] **GMH-40：Agent Loop 单一副作用 seam** `sev-P1`
  组装/pre-call/执行同一治理快照；turn 分段可取消可度量；子代理/cron 禁止提权。
- [ ] **GMH-41：自治预算与熔断** `sev-P2`
  turn/session/day 限额；拒绝风暴熔断；离线高风险排队或拒绝。
- [ ] **GMH-42：治理可观测性与审计** `sev-P2`
  decision latency、人工等待、deny/stale receipt、Memory apply/rollback 指标与
  correlation 证据链。
- [ ] **GMH-50：兼容迁移与 feature flags** `sev-P2`
  不得重新引入 Mentle；禁止长期双写。
- [ ] **GMH-51：安全与数据恢复演练** `sev-P1`
- [ ] **GMH-52：全量验收** `sev-P1`
  `just fmt-check` / `check` / `test`、deletion-proof、GUI、真实 smoke。
- [ ] **GMH-53：灰度与清理** `sev-P2`

里程碑（更新后）：

- [x] M0 / M1 / M2 — 见 archive（含 GMH-24 clean-break）
- [ ] **M3** HITL 闭环（GMH-30..33）
- [ ] **M4** Agent Loop 接入（GMH-40..42）
- [ ] **M5** 灰度发布（GMH-50..53）

统一 DoD：设计/威胁模型；成功/拒绝/超时/并发/重启测试；用户可见路径 smoke；
`docs/logs` 四件套；单 concern Conventional Commit；不擅自 push。

### Reliability / Test Debt

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
- [ ] **Manager library suite 在 workspace gate 下 load-sensitive** `sev-P2`
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

- [ ] **扩展 phase×capability / transition 矩阵与 denial 副作用测试** `sev-P2`
  policy 已 fail-closed，但缺完整非法迁移笛卡尔与 denial 前后 store 快照断言；
  assembly 仅子集工具。
  相关：`agent-diva-core/src/planning/policy.rs`、
  `agent-diva-agent/src/tool_assembly.rs`、agent-loop 集成夹具。
- [ ] **runtime 配置热更新后按 phase 重建工具表** `sev-P2`
  `rebuild_tools_for_active_phase` 当前 `rebuild_tools_for_turn(..., None, ...)`，
  网络/MCP 更新可能短暂丢掉 phase 边界。
  `agent-diva-agent/src/agent_loop/loop_runtime_control.rs`。
- [ ] **空 execution TODO 列表的 Verify 门闩** `sev-P2`
  `PlanVerifier::verify` 在 `total == 0` 时直接 Pass，可能让无步骤计划误完成。
  `agent-diva-agent/src/planning/verifier.rs`。
- [ ] **预存在 TODO 与 materialize 策略文档化/修复** `sev-P2`
  `TodoAlreadyMaterialized` 仍可永久卡住 Always 路径；需产品决策：视为已物化
  成功、禁止预批准写入、或提供清理 API。
  `agent-diva-core/src/planning/store.rs`。
- [ ] **删除 orchestrator 内注释掉的死迁移矩阵** `sev-P3`
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
