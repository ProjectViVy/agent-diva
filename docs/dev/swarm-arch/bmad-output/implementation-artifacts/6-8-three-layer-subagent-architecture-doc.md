---
story_key: 6-8-three-layer-subagent-architecture-doc
story_id: "6.8"
epic: 6
status: done
generated: "2026-03-31T20:30:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/subagent-to-swarm-migration-inventory.md
---

# Story 6.8：三层 Subagent 架构示意图与文档索引（MIG-03）

## Story

As a **维护者**,  
I want **在 `agent-diva` 开发者文档中增加「运行时 / IDE / BMAD 技能」三层 subagent 示意图并链接迁移清单**,  
So that **SWARM-MIG-03 落地，降低再混淆风险**。

## Acceptance Criteria

**Given** `docs` 或 `agent-diva-swarm/docs` 中单页  
**When** 新贡献者阅读  
**Then** 能区分 **A 层 Rust**、**B 层 .cursor agents**、**C 层 BMAD 技能**  
**And** 链接到 `subagent-to-swarm-migration-inventory.md`

## Tasks / Subtasks

- [x] 在 `agent-diva/docs/dev/architecture.md` 增加「三层 subagent」对照表与示意图（Mermaid）
- [x] 增加指向 `_bmad-output/planning-artifacts/subagent-to-swarm-migration-inventory.md` 的链接（SWARM-MIG-03）
- [x] 交叉引用 `agent-diva-swarm/docs/` 供深入设计

## Dev Notes

- 清单权威：`subagent-to-swarm-migration-inventory.md` §1、§7（SWARM-MIG-03）。
- 正文语言与既有 `architecture.md` 一致（英文），便于与 crate 说明混排。

## Dev Agent Record

### Debug Log

- 故事文件初稿仅有 frontmatter 与标题；AC 自 `epics.md` Story 6.8 补齐。

### Completion Notes

- ✅ 在 `architecture.md` 的 High-Level Architecture 与 Crate Responsibilities 之间插入 **Three layers of “subagent”** 节：表格区分 A/B/C、Mermaid 流程图、迁移清单相对链接、swarm 文档指针。
- ✅ 无代码变更；`cargo check -p agent-diva-core`（`agent-diva` 树内）通过作烟雾测试。

### Implementation Plan

- 单页落在 `agent-diva/docs/dev/architecture.md`，满足 AC「docs 中单页」；未另建 swarm 副本以免重复。

## File List

- `agent-diva/docs/dev/architecture.md`
- `_bmad-output/implementation-artifacts/6-8-three-layer-subagent-architecture-doc.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

## Change Log

- 2026-03-31：Story 6.8 — 三层 subagent 文档节（MIG-03）；sprint 状态 `in-progress` → `review`。
