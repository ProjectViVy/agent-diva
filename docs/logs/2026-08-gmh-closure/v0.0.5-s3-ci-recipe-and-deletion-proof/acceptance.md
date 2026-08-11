# Acceptance

## 用户/产品视角验收步骤

1. CI pipeline 的 "Run Sprint 5 default-lane provider regressions" step 应成功
   （调用 `memory-provider-check`，不再因 `sprint5-default-check` 缺失而失败）。
2. 若有人误恢复 `config_migration` / `memory_migration` / `session_migration`，
   `just laputa-clean-break-check` / `just ci` 应报错并列出残留位置。

## 验收通过标准

- CI 断点修复（配方名存在且通过）。
- deletion-proof 扫描具备阳性/阴性 self-test，且真实树无假阳性（含 `.mentle`
  拒绝守卫与 `memory_migration_id` error-code 标识符）。