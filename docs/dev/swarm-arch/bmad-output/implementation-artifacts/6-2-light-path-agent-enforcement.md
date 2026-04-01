---
story_key: 6-2-light-path-agent-enforcement
story_id: "6.2"
epic: 6
status: done
generated: "2026-03-31T20:30:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/implementation-artifacts/deferred-work.md
---

# Story 6.2：Light 路径在真实 AgentLoop 上的步数/墙钟 enforcement

## 故事陈述

作为 **用户**，  
我希望 **轻量路径在真实对话循环中遵守已文档化的内部步数与/或墙钟上限**，  
以便 **FR19「可完成或显式失败」在主线成立，而非仅 headless 桩**（关闭 Story 1.7 评审 deferred）。

## 验收标准（epics.md）

1. **Given** `ExecutionTier::Light` 且皮层状态按 FR19 路由  
   **When** 主循环超过 `LIGHT_PATH_MAX_INTERNAL_STEPS` 或等价墙钟  
   **Then** turn **终止**并返回 **可对用户说明的失败/触顶原因**（契约清晰）  
2. **And** 至少 **一条** 集成或单元测试覆盖「Light 触顶」

## 任务 / 子任务

- [x] 在 `AgentLoop` 主循环对 Light 路径按 `LIGHT_PATH_MAX_INTERNAL_STEPS`、`LIGHT_PATH_MAX_WALL_MS` 检查（每轮迭代起点 + 流式内轮询墙钟）  
- [x] 触顶文案经 `format_light_path_stop_for_user`（含稳定 `machine_code`）  
- [x] `agent-diva-swarm`：`format_light_path_stop_for_user` 单测；`agent-diva-agent`：`light_path_internal_step_cap_returns_user_message` 集成测  
- [x] 更新 `deferred-work.md` 1-7 挂起项为已收口  

## 开发说明

- headless `run_minimal_turn_headless` 的 Light 仍为单步桩；本故事范围是 **真实多步** `process_inbound_message_inner`。  
- FullSwarm 路径 **不** 套用 Light 上限（沿用既有 `max_iterations` 与收敛策略）。

## Dev Agent Record

### Implementation Plan

- `light_path_limits.rs`：新增 `format_light_path_stop_for_user`，并导出；文档注明由 AgentLoop enforcement。  
- `loop_turn.rs`：`Instant::now()` 起算墙钟；`'agent_turn` 标签跳出；Light 下步数 > `LIGHT_PATH_MAX_INTERNAL_STEPS` 或墙钟超限时设 `final_content` 后 `break`。  
- 测试：无限 `read_file` 的 stub provider + 轻量中文问句 + 皮层 ON + `max_iterations=500`，断言回复含 `light_path.internal_step_limit`。

### Debug Log

- （无）

### Completion Notes

- ✅ AC 满足：`cargo test -p agent-diva-swarm -p agent-diva-agent` 全绿。  
- ✅ 关闭 `deferred-work.md` 中 1-7 关于「真实墙钟/步数 enforcement」的评审挂起项。  

## File List

- `agent-diva/agent-diva-swarm/src/light_path_limits.rs`  
- `agent-diva/agent-diva-swarm/src/lib.rs`  
- `agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs`  
- `agent-diva/agent-diva-agent/src/agent_loop.rs`  
- `_bmad-output/implementation-artifacts/deferred-work.md`  
- `_bmad-output/implementation-artifacts/sprint-status.yaml`  
- `_bmad-output/implementation-artifacts/6-2-light-path-agent-enforcement.md`  

## Change Log

- 2026-03-31：Story 6.2 实现 Light 路径 AgentLoop 步数/墙钟 enforcement、用户可见触顶文案与测试；故事状态 → review。
- 2026-03-31：bmad-code-review（无 Git diff，基于 File List 全文件审阅 + `cargo test`）；零待办项；状态 → done。  

## Status

done
