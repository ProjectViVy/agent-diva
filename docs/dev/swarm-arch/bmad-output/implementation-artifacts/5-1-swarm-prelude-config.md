---
story_key: 5-1-swarm-prelude-config
story_id: "5.1"
epic: 5
status: done
generated: "2026-03-31T18:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/sprint-change-proposal-2026-03-31.md
---

# Story 5.1：蜂群序曲角色与预算可配置

状态：**done**（评审裁定选项 1 已文档化；Pipeline 触顶过程事件单测已补）

## 故事陈述

作为 **维护者 / 进阶用户**，  
我希望 **用配置声明 FullSwarm 序曲的角色链、提示模板、最大轮次与调用/token 预算**，  
以便 **成本可控、行为可预测**，且 **不必为调参改 Rust 硬编码**（对齐 FR20、NFR-P3）。

## 背景（Why）

阶段 A 在 `loop_turn.rs` 中 **硬编码** 规划/批评两角色。产品上要回答：**谁有权改蜂群性格？改完如何回滚？多租户/多 workspace 怎么办？** —— 最小可验证增量是 **文件级配置 + 默认值与当前行为一致**。

## 验收标准

1. **Given** 一份 **版本化** 的配置源（例如 workspace 下 `swarm-prelude.toml` / `.yaml` 或并入现有 `config`，路径在 README/维护者文档 **单点冻结**）  
   **When** FullSwarm turn 执行  
   **Then** 序曲角色顺序与系统提示来自配置；**未提供文件时** 行为与阶段 A **逐字等价**（向后兼容）

2. **And** 支持至少一种 **预算**：`max_prelude_rounds` 和/或每轮 `max_tokens`；触顶时 **可观测**（过程事件或明确错误），**不** 死循环

3. **And** 支持 **关闭序曲**（`enabled: false`）：直接进入主 ReAct，仍遵守 FR19/FR20

4. **And** 新增或更新 **无头/单元测试**：至少覆盖「默认等价阶段 A」「序曲关闭」「超轮次触顶」之一

## 任务分解（Dev）

- [x] 选定配置位置与 schema 版本字段（NFR-I2）
- [x] 在 `agent-diva-agent`（或 `agent-diva-swarm`）解析配置；失败时 **明确日志 + 安全回退默认**
- [x] 将 `run_swarm_deliberation_prelude` 改为 **数据驱动循环**
- [x] 文档：`agent-diva-swarm` README 或 `docs/` 片段 + 示例配置
- [x] 测试：`cargo test` 覆盖上述 AC

### Review Findings

- [x] [Review][Decision] 序曲触顶后是否仍发射 merge 阶段过程事件 — **已裁定：选项 1**（维持现状）。维护者在 `agent-diva-swarm` README「FullSwarm 序曲配置」节已补充：`swarm_prelude_round_cap` 后 **仍** 发射 `[merge_phase]` 事件；观测方需结合 cap 与摘要区分完整/截断序曲。

- [x] [Review][Patch] 缺省带 Pipeline 的触顶可观测性测试 — **已补** `swarm_prelude_round_cap_emits_phases_when_pipeline_attached`（`loop_turn.rs`）：断言 `swarm_peer_planner` → `swarm_prelude_round_cap` → `swarm_peer_merge`。

- [x] [Review][Defer] 同分支 diff 与 5.1 边界 — `loop_turn.rs`、`lib.rs` 等与 Story 5.1 File List 重叠的变更中混有执行层、轻量路径、收敛等非序曲能力；严格按 5.1 验收时应以 File List 与序曲/配置逻辑为准；后续可考虑拆 PR 便于回滚与评审。 — deferred, pre-existing

## 依赖与边界

- **依赖：** 阶段 A 已合并（`ExecutionTier::FullSwarm` + 序曲存在）  
- **不包含：** UI 配置编辑器（可 Post-MVP）；**5.2** 会接遥测统一

## 参考实现位置（阶段 A）

- `agent-diva-agent/src/agent_loop/loop_turn.rs`：`run_swarm_deliberation_prelude`

## Dev Agent Record

### Debug Log

- 无阻塞项；`LLMProvider` 单测需 `Arc<dyn LLMProvider>` 显式类型以满足 `run_swarm_deliberation_prelude` 签名。

### Completion Notes

- 工作区根 **冻结路径**：`swarm-prelude.toml` → `swarm-prelude.yaml` → `swarm-prelude.yml`；`schema_version` 默认 1。
- `agent-diva-swarm` 新增 `prelude_config`：`load_swarm_prelude_config_from_workspace`、默认与阶段 A 等价；解析失败 `warn!` 回退默认。
- 序曲按 `roles` 与 `max_prelude_rounds`（LLM 步数上限）循环；触顶发射 `swarm_phase_changed`：`swarm_prelude_round_cap`；`max_prelude_rounds = 0` 时发射说明文案并跳过序曲。
- `enabled = false` 时返回 `Ok(None)`，主循环不注入序曲 system 消息。
- README 增补配置说明与完整 TOML 示例。

### Implementation Plan

1. Workspace 增加 `toml` 依赖；swarm 增加 `toml` + `serde_yaml` + `prelude_config` 模块与导出。  
2. Agent `loop_turn` 传入 `workspace`，序曲内加载配置并数据驱动 `chat`。  
3. 单元测试：swarm 解析/默认；agent 三用例（关闭、无文件两跳、max=1 单跳）。

## File List

- `agent-diva/Cargo.toml`
- `agent-diva/agent-diva-swarm/Cargo.toml`
- `agent-diva/agent-diva-swarm/src/prelude_config.rs`（新）
- `agent-diva/agent-diva-swarm/src/lib.rs`
- `agent-diva/agent-diva-swarm/README.md`
- `agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs`

## Change Log

- 2026-03-31：Story 5.1 — 工作区序曲配置（TOML/YAML）、数据驱动序曲、`max_prelude_rounds` / `max_tokens`、可观测触顶、README 与测试。
