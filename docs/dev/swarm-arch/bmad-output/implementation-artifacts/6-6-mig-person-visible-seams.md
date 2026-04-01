---
story_key: 6-6-mig-person-visible-seams
story_id: "6.6"
epic: 6
status: done
generated: "2026-03-31T20:30:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/subagent-to-swarm-migration-inventory.md
---

# Story 6.6：子任务结果 internal vs person_visible 标注（MIG-02）

## Story

As a **产品**,  
I want **子代理/`spawn` 返回内容在管线中被显式标注为 internal 或 person_visible**,  
So that **单一 Person 叙事（FR8/FR9）可测，且与 PersonOutbox 长期设计一致（SWARM-MIG-02）**。

## Acceptance Criteria

（对齐 `epics.md` Story 6.6）

- **Given** 子任务完成并回传到主循环  
- **When** 合成对外回复  
- **Then** **仅** person_visible 内容进入用户可见 transcript；internal 默认 **不** 泄漏（NFR-R2）  
- **And** 至少 **一条** 测试或断言防止回归

## Tasks / Subtasks

- [x] `PersonSeamVisibility`（`internal` / `person_visible`）与 `InboundMessage::with_person_seam`（`agent-diva-core`）
- [x] 子代理结果回注：`announce_result` 对 `publish_inbound` 使用 `PersonSeamVisibility::Internal`
- [x] 主循环落盘：`save_turn` 将触发消息的 `person_seam` 写入 `ChatMessage`
- [x] 用户可见切片：`Session::get_history` 过滤 `Internal`；记忆合并 `consolidation` 跳过 `Internal`
- [x] 回归测试：`session::store::test_get_history_excludes_internal_person_seam` + `loop_turn::save_turn_subagent_trigger_internal_excluded_from_get_history`

## Dev Notes

- 清单：`subagent-to-swarm-migration-inventory.md` **SWARM-MIG-02**。  
- `person_seam: None` 视为与 **person_visible** 等价（对外默认可见）。

## Dev Agent Record

### Implementation Plan

- 核心类型与总线：`person_seam.rs`、`InboundMessage.person_seam`。  
- 子代理路径：`subagent.rs` → `with_person_seam(Internal)`。  
- 会话与叙事：`loop_turn::save_turn` + `Session::get_history` + `consolidation::consolidate` 排除 internal。

### Debug Log

- （无）

### Completion Notes

- ✅ AC：子代理回传在会话中标记为 Internal，持久化 transcript 经 `get_history` 仅暴露 person_visible 轮次；合并记忆不摄入 internal 原文。  
- ✅ AC：`cargo test -p agent-diva-agent -p agent-diva-core` 全绿；含两条针对 person seam 的单元测试。

## File List

- `agent-diva/agent-diva-core/src/person_seam.rs`
- `agent-diva/agent-diva-core/src/bus/events.rs`
- `agent-diva/agent-diva-core/src/session/store.rs`
- `agent-diva/agent-diva-core/src/lib.rs`
- `agent-diva/agent-diva-agent/src/subagent.rs`
- `agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva/agent-diva-agent/src/consolidation.rs`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/6-6-mig-person-visible-seams.md`

## Change Log

- 2026-04-01：确认 MIG-02 管线与测试覆盖；补全故事正文与台账；状态置为 review。

## Status

done
