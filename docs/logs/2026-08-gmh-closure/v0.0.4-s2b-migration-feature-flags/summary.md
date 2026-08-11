# Summary

GMH-50 兼容迁移清理的第二步（S2b）：为 `agent-diva-migration` 增加配置驱动的
迁移 feature flag（非 crate feature，避免污染 workspace）。

## 变更

- `agent-diva-migration/src/main.rs`：
  - `Cli` 新增全局 `--features`（逗号分隔）参数。
  - 定义三个规范 feature 名：`experience` / `memory` / `workspace_identity`，
    与已挂载的模块名一致。
  - 新增 `require_feature(enabled, name)`：仅当开关开启才允许对应 `Apply`；
    `DryRun` / `Rollback` 始终允许（对齐计划"仅在对应 Apply 时校验"）。
  - 三个 `Apply` 分支（Memory apply、Identity apply、Experience apply）接入校验。

## Impact

- 运维需显式 `--features <name>` 才能执行对应 `Apply`，降低误触迁移风险。
- 未恢复 Mentle（`typed_memory.rs` 的 `.mentle` 拒绝守卫保留）；未引入长期双写
  （迁移仍一次性 apply/rollback，manifest 记录）。
- 不引入 crate feature，关键路径编译表层面积不变。