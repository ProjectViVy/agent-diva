# Story 5.2 Mentle Governance Exclusion Summary

## 变更

- 补齐 AutoDream 守护测试，确认输出产物、提案创建和节律报告写入不会生成 `memory/palace.db` 或 `.mentle`。
- 补齐 Laputa 守护测试，确认提案创建、apply、audit、rollback 与 changelog 路径保持 file-first，不依赖 Mentle 运行态。
- 保留并验证现有 feature-gated Mentle 运行时兼容行为，但默认治理上下文不暴露 Mentle recall 或 `memtle_*` 路由。
- 重新执行 Story 5.2 要求的整组验证，并在隔离 worktree `story-5-2-compaction-summary` 中通过。

## 影响

- EVO-DIVA governance 路径对 Mentle 排除边界具备可回归的负向覆盖。
- Story 5.2 已从 `in-progress` 推进到 `review`，评审可以明确区分兼容层 Mentle 行为与治理层文件优先边界。
