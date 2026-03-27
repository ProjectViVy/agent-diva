# v0.0.6 Release

## Release Method

本轮没有执行提交、发布或部署动作。

## Delivery State

- `agent-diva-memory` 已具备 SQLite 主存储基础能力。
- `WorkspaceMemoryService` 已组合 SQLite store，但仍保持 file-based recall 为默认主链路。
- 本轮不涉及数据迁移，也不要求现有 workspace 做任何手工升级步骤。

## Rollout Notes

- 如果后续要继续做 `v0.0.7`，建议直接在当前基础上补：
  - SQLite recall
  - FTS5
  - `MEMORY.md` / diary backfill
- 当前 `brain.db` 只是主存储基础，不是最终 retrieval backend。

## Risk Notes

- 当前主要风险不在接口破坏，而在后续 recall 主链路切换时的排序与回填质量。
- `MemoryStore` 已经有 concrete implementation，但尚未被 agent runtime 大量消费；这符合本轮“先落 store，再切 recall”的目标。
