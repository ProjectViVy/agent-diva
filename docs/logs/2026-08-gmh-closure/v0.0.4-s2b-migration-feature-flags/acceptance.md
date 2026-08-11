# Acceptance

## 用户/产品视角验收步骤

1. `just migrate -- --features experience experience apply --workspace <ws>` 应被放行。
2. 省略 `--features` 时对 `experience apply` 应报错
   `Apply for `experience` is disabled until the `--features experience` flag is passed`。
3. `DryRun` / `Rollback` 不受 `--features` 影响，始终可用。
4. 迁移仍为一次性 apply/rollback，manifest 记录；`.mentle` 不再出现。

## 验收通过标准

- 测试与 clippy 全绿（本 slice 已确认）。
- 确认未引入 crate feature、未恢复 Mentle、未引入长期双写。