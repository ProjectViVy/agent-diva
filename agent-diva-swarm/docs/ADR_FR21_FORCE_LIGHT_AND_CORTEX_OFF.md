# ADR：FR21 强制轻量（ForceLight）与大脑皮层关（Cortex OFF）的冻结选型

**状态：** MVP 冻结（与 Story **1.9**、**4.3** 交叉引用；若后续独立实现 ForceLight 标志位，须以本 ADR 的「取代/细化」段落更新，而非静默改语义。）

**对齐：** 规划文档 [`architecture.md`](../../../_bmad-output/planning-artifacts/architecture.md) — **ADR-E**（FR19–FR22）：FR21 与 `CortexState::Off`（FR3）须在 ADR 中交叉说明。

### 与 `architecture.md` ADR-E 的关系（取代 / 细化）

- 本文件 **不取代** ADR-E 总则；是对其中 **「FR21 / ForceLight」** 条目的 **首版实现细化**：将 **二选一** 落实为 **选项 A（合并）**，并固定 **可测追溯**（见 §4）。
- 若产品后续引入 **选项 B（独立 ForceLight）**，须修订本 ADR，并在 `architecture.md` ADR-E 对应句或本文件抬头 **显式** 说明变更，避免静默漂移。

---

## 1. 选型（二选一结果）

**选项 A — 合并（当前实现）：**

- 产品中 **尚未** 暴露独立的 **ForceLight** 配置位；最小 turn 路由里，**「强制走轻量/简化编排」** 的用户可感知效果与 **大脑皮层关（Cortex OFF）** 下的简化路径 **一致**。
- 即：**关大脑皮层** 即表示本轮不走完整多代理对弈链、不预留多视角蜂群编排的额外模型回合（与 PRD **FR21** 强制期间语义一致，直至用户显式升级任务 —— 升级入口由产品/UI 故事演进）。

**未选：** 选项 B（皮层 **开** 时仍可单独 ForceLight）— 待产品引入独立标志位与单点设置后，再发 ADR 修订本文件。

---

## 2. 与 FR3 / Story 1.4 测试语义的关系

- **关模式可测行为** 以 [`CORTEX_OFF_SIMPLIFIED_MODE.md`](./CORTEX_OFF_SIMPLIFIED_MODE.md)（实现登记 + 测试对照表）为技术真值。
- 在 **合并** 选型下，**1.4** 无头测试对 **皮层 OFF** 的断言 **同时覆盖** 当前产品下「强制轻量」等价路径；若未来 **拆分** ForceLight，须按 Story **1.9** AC #3 增补 **皮层开 + ForceLight** 用例。

**规划/评审追溯：**

- Story **1.4** 工件：[`_bmad-output/implementation-artifacts/1-4-cortex-off-headless-tests.md`](../../../_bmad-output/implementation-artifacts/1-4-cortex-off-headless-tests.md)

---

## 3. 与 Story 4.3（FR17）的关系

- 维护者向说明（产品语言、路径与边界）：[`CORTEX_OFF_FR17_MAINTAINER_GUIDE.md`](./CORTEX_OFF_FR17_MAINTAINER_GUIDE.md)

---

## 4. 验收附录（可执行覆盖）

| 覆盖项 | 位置 |
|--------|------|
| 合并语义下 OFF 路径与「不进入全蜂群 handoff」 | `CORTEX_OFF_SIMPLIFIED_MODE.md` §2、§4 测试对照表；`cargo test -p agent-diva-swarm` 过滤器可用 `minimal_turn::cortex_off_tests::`（与对照表完整模块路径一致） |
| **FR21（合并）：** 关皮层 ≡ 强制轻量等价 — **不** 为多视角全蜂群 handoff 预留额外内部回合 | `minimal_turn::cortex_off_tests::fr21_merge_off_path_no_full_swarm_extra_rounds`（显式命名，供 AC #3 / IR 门禁检索） |
| 开/关分支错误可检索（AC #4） | `minimal_turn::cortex_off_tests::cortex_off_wrong_branch_panics_with_cortex_keywords`（见 1.4 File List） |
