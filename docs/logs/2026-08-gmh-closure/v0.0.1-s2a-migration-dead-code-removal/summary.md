# Summary

GMH-50 兼容迁移清理的第一步：删除 `agent-diva-migration` crate 中三个未挂载的
废弃迁移模块（S2a）。

- 删除 `agent-diva-migration/src/config_migration.rs`（804 行）、
  `memory_migration.rs`（302 行）、`session_migration.rs`（280 行），共约 1386 行。
- `main.rs` 仅挂载 `experience` / `typed_memory` / `workspace_identity` 三个 mod，
  被删三文件从未被 `mod` 声明引用，全仓无引用（grep 仅命中文件内部同名测试函数
  与 `agent-diva-laputa/src/error.rs` 无关错误码字段名）。
- 保留 `typed_memory.rs` 的 `.mentle` 拒绝守卫与一次性 apply/rollback 迁移路径，
  不新增任何双写或 Mentle 逻辑。

## Impact

- 减少编译表面积约 1386 行；`agent-diva-migration` 编译与 7 测试全过。
- 无行为变更；`feature-gate-check.py` 不受影响。