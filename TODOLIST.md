# TODOLIST

项目级**活跃**待办与延期项。已完成历史见 Archive Index。

**严重度标签**使用 `sev-P0`（阻断）…`sev-P3`（琐碎），与已归档的
「Plan 阶段 P1/P2/P3」命名无关。

---

## Active Plan

当前主线：在 **typed Laputa clean-break（GMH-24）之后**，完成 **G2D 真实桌面验收**，
然后才能解冻 Evolution / 进入发布验收。GMH-00..24 架构工作已完成并归档。

- [ ] **G2D：typed authority 真实桌面六场景验收** `sev-P0`
  必须在**重启并加载新二进制**后的真实桌面环境执行，不得以自动化测试冒充。
  场景：批准、拒绝、编辑后批准、重复点击、重启恢复、回滚。
  保留 request/proposal/audit/rollback ID、界面结果、Manager/Tauri 日志。
  相关：`docs/logs/2026-07-gmh-24c/`、GMH-23D HITL 路径。

- [ ] **Evolution 功能开发冻结（直至 G2D）** `sev-P0`
  在六场景通过前，不修复/扩展 AutoDream/Evolution 产品功能；仅允许安全与
  数据完整性修复，以及明确的不可用/降级 UX。通过后基于 typed Laputa 再定基线。

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

- [ ] **Skill / SOP 统一为兼容 Skills** `sev-P1`
  不建第二套 SOP 格式/目录/运行时。扩展 `SKILL.md` 可选 `kind: sop`（默认
  `skill`），复用完整 Skill 生命周期；GUI/摘要显示 `AGENT-DIVA SOP` 标签且不授
  权。Notebook「固化为 SOP」应生成 SOP Skill 候选。
  契约：`docs/architecture/skill-sop-unification.md`。

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

- [ ] **QQ invalid-resume 集成测试 load-sensitive** `sev-P2`
  全量 gate 偶发 opcode 乱序；隔离重跑通过。
  `agent-diva-channels/tests/qq_reconnect_integration.rs`。
- [ ] **Memory authority provider 选择缺 focused characterization** `sev-P2`
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
