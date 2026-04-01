---
story_key: 6-5-mig-capability-registry-subagent-tools
story_id: "6.5"
epic: 6
status: done
generated: "2026-03-31T20:30:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/subagent-to-swarm-migration-inventory.md
---

# Story 6.5：Subagent 工具集 → Capability 可引用条目（MIG-01）

## Story

As a **维护者**,  
I want **`SubagentManager` 使用的工具列表以数据驱动方式对齐 Capability/注册表条目（行为可先不变）**,  
So that **迁移清单 SWARM-MIG-01 落地，为工具门禁与 Shannon 式 enforcement 铺路**。

## Acceptance Criteria

（对齐 `epics.md` Story 6.5）

- **Given** 现有子代理固定工具集  
- **When** 查阅配置或注册表  
- **Then** 每项工具 **有** 稳定 id / 元数据槽位（危险度等可占位）  
- **And** `spawn` 与 FullSwarm 路径的边界在文档中 **交叉引用** PRD 1.0.0 Swarm P0  

## Tasks / Subtasks

- [x] 建立子代理工具 **单一数据源**（稳定 `tool.subagent.*` id、LLM `tool_name`、描述、`risk_tier` 占位、网络工具可用性说明）
- [x] `execute_subagent_task` 通过 **`build_subagent_tool_registry`** 注册工具，行为与原先一致（含网络开关）
- [x] **`SwarmCortexDoctorV1`** 暴露 `subagent_tools` 目录（`schema_version` 递增至 2）；CLI human/json 消费方同步
- [x] 单元测试：id 唯一、默认网络六项齐全、关闭 web 时两项缺失
- [x] `agent-diva/docs/dev/architecture.md`：`spawn` / FullSwarm / 工作区 manifest 边界 + **PRD / epics** 互链

## Dev Notes

- 清单：`subagent-to-swarm-migration-inventory.md` **SWARM-MIG-01**。  
- 工作区 `capability-manifest.json` 仍为 FR10/FR11 **包级** manifest；内置子代理工具表 **不自动合并** 进 `PlaceholderCapabilityRegistry`，避免 id 冲突与语义混淆。

## Dev Agent Record

### Implementation Plan

- 新增 `agent-diva-agent/src/subagent_tool_capabilities.rs`：`SUBAGENT_TOOL_CAPABILITY_SPECS` + `build_subagent_tool_registry`。  
- `swarm_doctor.rs`：`SubagentToolsDoctorSummary` / `SubagentToolDoctorEntry`，`schema_version: 2`。  
- 文档：`architecture.md` 小节 + `commands.md` 中 `schema_version` 说明一行。

### Debug Log

- （无）

### Completion Notes

- ✅ AC：每项子代理工具有稳定 capability id 与 `risk_tier`（low/medium/high）占位；诊断 API/CLI 可查阅。  
- ✅ AC：`architecture.md` 已写明 `spawn`/`SubagentManager` 与 `agent-diva-swarm` FullSwarm 差异，并链接 `prd.md`、`epics.md`。  
- ✅ `cargo test -p agent-diva-agent -p agent-diva-cli`：agent 单测与 `config_commands` 集成测试通过（含 `schema_version` 与 `subagent_tools.entries` 长度断言）。

## File List

- `agent-diva/agent-diva-agent/src/lib.rs`
- `agent-diva/agent-diva-agent/src/subagent.rs`
- `agent-diva/agent-diva-agent/src/subagent_tool_capabilities.rs`（新）
- `agent-diva/agent-diva-agent/src/swarm_doctor.rs`
- `agent-diva/agent-diva-cli/src/main.rs`
- `agent-diva/agent-diva-cli/tests/config_commands.rs`
- `agent-diva/docs/dev/architecture.md`
- `agent-diva/docs/user-guide/commands.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/6-5-mig-capability-registry-subagent-tools.md`

## Change Log

- 2026-04-01：实现 SWARM-MIG-01 数据驱动子代理工具表、`swarm-doctor` 目录输出、架构/PRD 交叉引用；故事置为 review。

### Review Findings

- [x] [Review][Defer] 子代理系统提示在 `build_subagent_prompt` 中无条件列出联网能力，但 `build_subagent_tool_registry` 可在网络关闭时不注册 `web_search` / `web_fetch` — [agent-diva/agent-diva-agent/src/subagent.rs:334-338] — deferred, pre-existing（与 Story 6.5「行为可先不变」一致，非本故事引入）
- [x] [Review][Defer] `execute_subagent_task` 在达到 `max_iterations` 且仍处于工具调用循环时回落为通用「无最终回复」文案 — [agent-diva/agent-diva-agent/src/subagent.rs:272-273] — deferred, pre-existing

## Status

done
