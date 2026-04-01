# V1.0.0 发布清单（可勾选）

**状态：** **定稿**（Epic 6 / [Story 6.7](../implementation-artifacts/6-7-release-checklist-v1-doc.md)；维护入口单点为本文件）。  
**权威原则：** [prd.md](prd.md) 章节 **「Version roadmap & 对标边界（1.0.0 发布清单 — 2026-03-31 冻结）」**（正文冻结双轨 P0；**细则以本附件为准**）。  
**说明：** 本表为 **机械核对**，不替代架构 ADR；标 **1.0.0** 前须两套 P0 均勾选完毕（见 PRD 同节说明）。

## 与 PRD / 史诗互链

| 链接 | 用途 |
|------|------|
| [prd.md](prd.md)（§ Version roadmap & 对标边界 … 1.0.0） | 产品原则与「未勾满不得标 1.0.0」 |
| [epics.md — Epic 6](epics.md) | 故事边界与 Story 6.7 AC |
| 本文件 | PRD 中「发布勾选附件」指向路径：`_bmad-output/planning-artifacts/release-checklist-v1.0.0.md` |

---

## Swarm-类 P0（handoff / 多角色）

| # | 项 | 验证方式（链接 / 命令） | 完成 |
|---|----|------------------------|------|
| S1 | 皮层 ON 时存在 **非仅 spawn** 的可观测多步编排 | [5-3-handoff-state-checkpoint](../implementation-artifacts/5-3-handoff-state-checkpoint.md) + [5-4-orchestration-spi-shannon-bridge](../implementation-artifacts/5-4-orchestration-spi-shannon-bridge.md)；手动：皮层开 + 长输入 / 显式深度 → 过程条 / 阶段事件 | [ ] |
| S2 | 轻量 vs 全量 **显式** 分层（FR19） | 实现：`agent-diva-agent` `resolve_execution_tier` / `ExecutionTier`（`agent_loop/loop_turn.rs`）；测试：`cargo test -p agent-diva-swarm minimal_turn`、`cargo test -p agent-diva-agent`（与 tier 相关用例） | [ ] |
| S3 | 序曲 / handoff **可配置或可扩展** | [5-1-swarm-prelude-config](../implementation-artifacts/5-1-swarm-prelude-config.md) AC + 仓库内 `swarm_prelude` 示例配置 | [ ] |
| S4 | 编排 SPI 文档 + 边界 | [5-4-orchestration-spi-shannon-bridge](../implementation-artifacts/5-4-orchestration-spi-shannon-bridge.md)（交付 ADR / 设计说明以该故事为准） | [ ] |
| S5 | 单一 Person 叙事（FR8/FR9） | [4-1-person-narrative-regression](../implementation-artifacts/4-1-person-narrative-regression.md) + [6-6-mig-person-visible-seams](../implementation-artifacts/6-6-mig-person-visible-seams.md) | [ ] |
| S6 | Subagent 工具 → Capability 条目（MIG-01） | [6-5-mig-capability-registry-subagent-tools](../implementation-artifacts/6-5-mig-capability-registry-subagent-tools.md) | [ ] |

## Shannon-类 P0（生产向纪律）

| # | 项 | 验证方式 | 完成 |
|---|----|-----------|------|
| H1 | 有界编排 + 收敛（FR20、NFR-P3） | [1-8](../implementation-artifacts/1-8-convergence-policy-fr20.md)（实现基线）+ [5-1](../implementation-artifacts/5-1-swarm-prelude-config.md) + [6-1](../implementation-artifacts/6-1-convergence-timeout-observable.md)；`cargo test -p agent-diva-swarm convergence`、`cargo test -p agent-diva-swarm minimal_turn` | [ ] |
| H2 | **Timeout** 终局可观测（非仅有枚举） | [6-1-convergence-timeout-observable](../implementation-artifacts/6-1-convergence-timeout-observable.md)；`agent-diva-swarm` `minimal_turn` / `convergence` 中与 `Timeout`、`swarm_run_finished` 相关测试 | [ ] |
| H3 | **Light** 路径真实循环 enforcement | [6-2-light-path-agent-enforcement](../implementation-artifacts/6-2-light-path-agent-enforcement.md)；`cargo test -p agent-diva-swarm`、`cargo test -p agent-diva-agent`（Light / FR19 门禁） | [ ] |
| H4 | 失败 / 触顶 **可分类** | `agent-diva-swarm` 过程事件白名单 + `swarm_run_capped` / `swarm_run_finished`（见 `convergence.rs`、`process_events`）；文档：`agent-diva-swarm/docs/process-events-v0.md` | [ ] |
| H5 | 能力可声明 + **doctor 真数据** | [6-3-doctor-capability-registry-wiring](../implementation-artifacts/6-3-doctor-capability-registry-wiring.md) | [ ] |
| H6 | 无 GUI 可测核心分支（FR12） | `cargo test -p agent-diva-swarm`（`minimal_turn` 皮层开/关、FullSwarm / Light）；`agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md` | [ ] |
| H7 | 皮层 GUI ↔ gateway **一致** | [6-4-cortex-gui-gateway-parity](../implementation-artifacts/6-4-cortex-gui-gateway-parity.md)；契约：`agent-diva/docs/swarm-cortex-contract-v0.md` | [ ] |

## 发布卫生

| # | 项 | 验证方式 | 完成 |
|---|----|-----------|------|
| R1 | 本表与 `prd.md` 1.0.0 节 **互链** | 本文件页眉与上表 → [prd.md](prd.md)；PRD「发布勾选附件」→ 本路径 `_bmad-output/planning-artifacts/release-checklist-v1.0.0.md`（已存在） | [ ] |
| R2 | `CHANGELOG` / 版本号策略 | `agent-diva/CHANGELOG.md` [Unreleased] 已注明本清单路径与 **1.0.0** 勾选门禁；发版时将条目并入版本节并对齐 `Cargo.toml` `workspace.package.version` | [ ] |

---

*Story **6.8**：三层 subagent 文档 — 贡献者 onboarding，建议 1.0.0 前完成但不阻塞二进制发布（产品可标为「文档 P0」）。参见 [6-8-three-layer-subagent-architecture-doc](../implementation-artifacts/6-8-three-layer-subagent-architecture-doc.md)。*
