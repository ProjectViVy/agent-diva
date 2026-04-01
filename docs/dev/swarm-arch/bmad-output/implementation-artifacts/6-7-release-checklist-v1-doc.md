---
story_key: 6-7-release-checklist-v1-doc
story_id: "6.7"
epic: 6
status: done
generated: "2026-03-31T20:30:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
---

# Story 6.7：V1.0.0 发布清单附件（可勾选表）

## Story

As a **发布负责人**,  
I want **仓库内有一份可勾选清单，将 PRD 1.0.0 双轨 P0 映射到具体测试/文档/命令**,  
So that **标 1.0.0 前可机械核对，不得靠记忆**。

## Acceptance Criteria

（对齐 `epics.md` Story 6.7）

**Given** `_bmad-output/planning-artifacts/release-checklist-v1.0.0.md`（单点冻结）  
**When** 逐项勾选  
**Then** 覆盖 **Swarm-类 P0** 与 **Shannon-类 P0** 摘要条目，并链接到 **Epic 5/6 故事或 `cargo test`/文档**  
**And** PRD 正文「细则以发布清单附件为准」与本文件 **互链**

## Tasks / Subtasks

- [x] 定稿 `release-checklist-v1.0.0.md`：状态由草案改为定稿，表格含可点击故事路径与 `cargo test`/源码/文档引用
- [x] 增加「与 PRD / 史诗互链」表，满足 R1（清单 ↔ prd.md；PRD 已有附件路径）
- [x] `agent-diva/CHANGELOG.md` [Unreleased] 增加 Documentation 条，指向清单路径与版本对齐说明（R2）

## Dev Notes

- 工作副本：`_bmad-output/planning-artifacts/release-checklist-v1.0.0.md`
- 故事文件相对清单位于同级上级下的 `implementation-artifacts/`；清单内链接使用 `../implementation-artifacts/*.md`
- 本故事为 **文档交付**，无 Rust 代码变更；回归验证对 `agent-diva` 运行 `cargo test -p agent-diva-swarm` 确认未受影响

## Dev Agent Record

### Implementation Plan

- 扩展发布清单：双轨表 + 发布卫生 + 互链区；PRD 锚点因渲染器差异改为显式章节名 + `prd.md` 相对链接
- CHANGELOG 从 `agent-diva/` 用 `../_bmad-output/...` 指向清单，便于发版责任人查找

### Debug Log

- 无

### Completion Notes

- ✅ 清单定稿并映射至具体实现工件路径与 `cargo test -p agent-diva-swarm` / `agent-diva-agent` 等命令
- ✅ PRD 互链：清单多处指向 `prd.md`；PRD 原有「发布勾选附件」路径保持不变
- ✅ R2：`CHANGELOG.md` 增加 Unreleased 文档条

## File List

- `_bmad-output/planning-artifacts/release-checklist-v1.0.0.md`（修改）
- `_bmad-output/implementation-artifacts/6-7-release-checklist-v1-doc.md`（修改）
- `_bmad-output/implementation-artifacts/sprint-status.yaml`（修改）
- `agent-diva/CHANGELOG.md`（修改）

## Change Log

- 2026-03-31：Story 6.7 — 发布清单 v1 定稿、PRD/史诗互链、CHANGELOG 门禁说明

### Review Findings

（BMad code review，`story_key`：`6-7-release-checklist-v1-doc`；2026-03-31；无 Git diff，按交付文件对照 AC；并行层：Blind Hunter / Edge Case Hunter / Acceptance Auditor；`failed_layers`：无。）

- [x] [Review][Patch] H1 行 `cargo test` 在 Cargo CLI 下非法（仅能有一个 `TESTNAME` 过滤参数） [`release-checklist-v1.0.0.md`:32] — 已改为两条独立命令：`cargo test -p agent-diva-swarm convergence`、`cargo test -p agent-diva-swarm minimal_turn`（选项 0 批量修复）。

## Status

done
