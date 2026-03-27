# v0.0.7 Release

## Release Method

本轮没有执行提交、发布或部署动作。

## Delivery State

- `brain.db` 已具备 FTS recall 基础能力
- 现有 diary 与 `MEMORY.md` 可通过 backfill/sync 进入 SQLite
- `WorkspaceMemoryService` 已切到混合 recall 过渡模式
- 现有 agent/tool 对外接口保持稳定

## Rollout Notes

- 当前模式仍是“SQLite recall 为主，file recall 补漏”，不需要一次性迁移所有运行时调用。
- 后续 `v0.0.8` 可以直接在此基础上补：
  - embedding provider abstraction
  - hybrid merge
  - recall stage 组合

## Risk Notes

- 当前主要风险在 recall 去重和排序质量，而不是接口破坏。
- `MEMORY.md` 仍为 compatibility source，本轮没有把它从运行链路里移除。
