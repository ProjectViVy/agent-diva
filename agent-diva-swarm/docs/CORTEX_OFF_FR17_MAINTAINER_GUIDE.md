# 「关大脑皮层」语义说明（FR17 · 维护者向）

**读者：** 维护者、贡献者（无需先读 Rust 实现即可理解 **关/开** 产品语义与边界）。  
**技术真值（断言与测试）：** [`CORTEX_OFF_SIMPLIFIED_MODE.md`](./CORTEX_OFF_SIMPLIFIED_MODE.md)（Story **1.4** 登记文档）。

---

## 1. 术语：什么是「关大脑皮层」

- **开（ON）：** 大脑皮层启用（`CortexState.enabled == true`）。在 **MVP 最小 turn** 中，系统可按输入与策略走 **轻量路径** 或 **完整蜂群编排**（多代理 handoff/对弈链等），详见实现登记文档中的 FR19 分层说明。
- **关（OFF）：** 大脑皮层关闭。此时采用 **简化模式**：为单次用户 turn 走 **简化编排**，**不** 进入完整多代理蜂群对弈链；过程类中间事件的计数在登记语义下为 **零**（订阅方应依赖终态/完成类信号，见实现文档）。

**默认：** 与 Story **1.2** 一致，默认 **开**；持久化边界见蜂群 [`README.md`](../README.md) 与 `cortex.rs`。

---

## 2. 行走路径与限制（关 vs 开）

| 维度 | 关（OFF / 简化模式） | 开（ON） |
|------|----------------------|----------|
| **多代理蜂群对弈链** | **不进入** | 非轻量长输入等条件下可进入 **FullSwarmOrchestration**（见 FR19） |
| **显式「全蜂群 / 深度编排」** | **仍不进入** FullSwarm：**FR3（皮层关）优先于** 单次 turn 上的显式深度请求；网关/UI 可通过可观测标志得知「请求被关路径抑制」（详见登记文档 §2） | 满足条件时可按策略进入完整编排 |
| **过程事件（中间计数）** | 登记语义下 **为 0** | 可走带过程事件的完整路径 |
| **工具 / 对话最小路径** | **允许**（直连桩式最小路径），但 **不经** 全蜂群编排层 | 与关相比可进入更高编排层；具体分层见实现登记 |
| **用户可感知** | 偏「单路径、轻量、无多代理对弈链」 | 可按输入启用完整蜂群能力 |

以上与 [`CORTEX_OFF_SIMPLIFIED_MODE.md`](./CORTEX_OFF_SIMPLIFIED_MODE.md) **§2** 一致；若实现变更，须 **先** 更新该登记文档与测试，再更新本 FR17 摘要。

---

## 3. 测试与追溯（与 Story 1.4 交叉引用）

**规划/故事工件（评审检索）：**

- [`_bmad-output/implementation-artifacts/1-4-cortex-off-headless-tests.md`](../../../_bmad-output/implementation-artifacts/1-4-cortex-off-headless-tests.md) — Story **1.4** 的 AC、任务与交付说明。

**实现登记 + 测试对照表（技术真值）：**

- [`CORTEX_OFF_SIMPLIFIED_MODE.md`](./CORTEX_OFF_SIMPLIFIED_MODE.md) — 文末 **「测试对照表」**：`文档章节 / 断言摘要 / 测试模块或函数名`。

**维护者说明与可验证主张的对应关系：**  
本文件 **§1–§2** 的每一条结论，均可在上述对照表或 **1.4** AC 中找到可执行断言或验收条目；请勿在本指南中写入与对照表 **矛盾** 的表述。

**开/关负向用例（AC #4 意图，非实现细节）：**  
CI 中需要能够区分 **「代码走错了开/关分支」** 与单纯的 **数据/期望值错误**。因此至少有一条自动化用例在 **故意走错分支** 时会失败，且失败信息中带有 **「大脑皮层」** 与 **「开」或「关」** 等可检索关键词，便于从日志直接定位到 **门控逻辑** 而非无关断言。实现与用例名见对照表中 **§4 A4 / AC#4** 一行。

---

## 4. 与强制轻量路径（FR21）的关系

**见 FR21 冻结：** [`ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`](./ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md)（仓库根相对路径：`agent-diva/agent-diva-swarm/docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md`）

当前产品语义下，**「关大脑皮层」** 与 **ForceLight（强制轻量）** 在 **MVP** 中按 **合并选型** 理解：**无独立 ForceLight 位时，强制轻量与皮层 OFF 走同一简化路径**。阅读 **1.4** 无头测试时：**关路径** 的断言即代表当前实现下 **「强制轻量等价路径」**；若将来引入 **皮层开 + ForceLight**，须按 Story **1.9** 与上述 ADR 修订增补测试矩阵。

**Story 1.9 工件（矩阵与任务）：** [`_bmad-output/implementation-artifacts/1-9-force-light-path-fr21.md`](../../../_bmad-output/implementation-artifacts/1-9-force-light-path-fr21.md)

---

## 5. 相关链接

| 说明 | 路径 |
|------|------|
| Gateway / 皮层契约 v0 | [`docs/swarm-cortex-contract-v0.md`](../../../docs/swarm-cortex-contract-v0.md)（仓库根 `docs`） |
| FR19 执行分层 ADR | [`docs/adr-e-fr19-execution-tier.md`](../../../docs/adr-e-fr19-execution-tier.md) |
| PRD FR17 | [`_bmad-output/planning-artifacts/prd.md`](../../../_bmad-output/planning-artifacts/prd.md) |
