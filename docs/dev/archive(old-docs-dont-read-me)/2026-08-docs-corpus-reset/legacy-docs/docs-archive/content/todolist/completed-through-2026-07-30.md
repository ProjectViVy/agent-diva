# TODOLIST Completed Archive (through 2026-07-30)

归档时间：2026-07-30。

本文件承接根 `TODOLIST.md` 瘦身：将**已完成**或**已关闭的历史 program 证据**迁出主清单。
更早归档见 [`completed-through-2026-07-29.md`](./completed-through-2026-07-29.md)。
Plan/TODO 旧评审逐条处置见 [`plan-todo-p1-p3-disposition-2026-07-30.md`](./plan-todo-p1-p3-disposition-2026-07-30.md)。

详细验证仍以各 `docs/logs/**` 与 Git 历史为准。

---

## G0 Verified Backlog Reconciliation

- [x] **JsonlTodoStore concurrent rewrites** 共享文件锁；并发 create/update 与 create/archive 回归通过。
- [x] **Todo API/CLI status and error contracts** Core 解析状态；Manager 400/404/500 契约。
- [x] **Mentle prompt rebuild boundary** 活跃 provider 的 Startup Status 在 rebuild 后保留；遗留 L2 路由不再注入。

证据：`docs/logs/2026-07-governance-g0/v0.0.1-g0-baseline/verification.md`。

---

## Active Plan / operational closures (2026-07-30)

- [x] **Ask mode enforced read-only authority boundary** AgentLoop admission + 唯一执行 seam；五工具检查白名单；拒绝伪造 mutation。  
  证据：`docs/logs/2026-07-ask-mode-read-only/v0.0.1-authority-boundary/`；提交 `766279a7`。
- [x] **G2D preflight migration revision contract** 分离 expected/result revision，回放身份校验，可安全重放。  
  证据：`docs/logs/2026-07-g2d-migration-report/v0.0.1-revision-contract/`。

---

## Deferred items completed (migrated out of main list)

- [x] **Mentle: repair runtime prompt activation regressions**  
  日志：`docs/logs/2026-07-mentle-prompt-rebuild/v0.0.1-mentle-prompt-rebuild/`。
- [x] **Agent main-runtime prompt Englishization** 2026-07-30 完成。
- [x] **AgentLoop G1 coordinator slimming** `process_inbound_message_inner` ≈ 350 行；`turn/*` 分阶段。
- [x] **AgentLoop G1 full workspace test gate window** 最终 `just test` ≈ 350s 通过。
- [x] **Memory: current-baseline interfaces spec (GMH-20)**  
  `docs/architecture/memory-framework-interfaces.md`。
- [x] **GUI: migrate lucide-vue-next → @lucide/vue** 2026-07-29。

---

## Deferred Review Program — closed as abandoned/superseded

范围原为 `94baa4b..HEAD` 的多 Wave 并行审查。Wave A–G 执行勾选与 commit checklist 已完成，但下列交付物从未产出：

- Wave reports / Cross-wave matrix / Priority pool
- Day 1–4 timebox 收口

**处置（2026-07-30）：** program **abandoned / superseded**。后续治理与回归以 GMH 路线图、G0 characterization、以及各 story 的 `docs/logs` 为准，不再维持四日审查 timebox 为活跃 backlog。

执行证据（waves / commit checklist）原文曾在根 `TODOLIST.md`；需要时可从 Git 历史 `TODOLIST.md` 恢复。摘要：

- Wave A 基础设施与 Harness；B 安全/监督/预算；C 观测/Audit；D 压缩/限流；E Todo 数据面；F 后台/子代理/workspace；G 横切
- Priority 1–7 与 Wave 1 并行角色均已勾选

---

## GMH Phase 0–2 completed (architecture closed through GMH-24)

> 完整 story 正文原在根 `TODOLIST.md`。此处仅保留完成索引。

### 总体与 Phase 0–1

- [x] **GMH-00** 一条权威链架构约束冻结
- [x] **GMH-01** 现状行为刻画与权威清单
- [x] **GMH-02** 产品决策冻结
- [x] **GMH-03** characterization 与契约测试（Gate G0）
- [x] **GMH-10** 核心治理领域模型
- [x] **GMH-11** 纯函数策略评估器
- [x] **GMH-12** 持久化审批账本与状态机（Gate G1）

### Phase 2 — Embedded Laputa / Memory

- [x] **GMH-20** 当前基线 Memory Interfaces Spec
- [x] **GMH-21** 规范化 Memory 记录与 provenance
- [x] **GMH-22** Recall 与检索预备
- [x] **GMH-23** Memory 写入全提案化（阶段 1/2 + 恢复的 3）
- [x] **GMH-23A** Embedded Laputa / clean-break 架构冻结
- [x] **GMH-23B** typed SQLite/FTS5 store
- [x] **GMH-23C** Laputa recall 接入（shadow-only）
- [x] **GMH-23D** proposal apply/HITL + crash recovery
- [x] **GMH-24** cutover + Mentle/legacy clean-break  
  - 24A offline import `e0760897`  
  - 24B shadow/typed read `36009ede`  
  - 24C typed clean-break `af453d93`

### 里程碑

- [x] **M0** 基线冻结（GMH-01..03）
- [x] **M1** 治理内核可用（GMH-10..12）
- [x] **M2** Embedded Laputa 架构闭环（GMH-20..24；生产 typed authority + clean-break）

**仍开放（见主清单）：** G2D 真实桌面六场景验收；GMH-30+ HITL 统一与后续 phase；Evolution 功能冻结直至 G2D。

日志索引（节选）：

- `docs/logs/2026-07-governance-memory-hitl-roadmap/`
- `docs/logs/2026-07-gmh-23a` .. `gmh-23d` / `gmh-24a` .. `gmh-24c`
- `docs/logs/2026-07-ask-mode-read-only/`

---

## Active Governance Research — partial completion notes

- [x] **G0 characterization / capability ledger** 已落地（见 G0 日志）
- [x] **G1 AgentLoop turn pipeline 瘦身** 已完成（协调器 ~350 行）
- 研究设计文档：`docs/dev/agent-loop-manager-gui-governance/`
- **未授权大爆炸重写**；G2 Manager / G3–G5 GUI 分期仍为后续可选入口（主清单压缩描述）

---

## Plan/TODO P1–P3 package

- 整包作为活跃 backlog：**SUPERSEDED**
- 逐条处置表：`plan-todo-p1-p3-disposition-2026-07-30.md`
- 边界实现归档：`completed-through-2026-07-29.md` 中 “P2/P3 approval boundary…” 条目
