# Verification

## 命令与结果

- `python scripts/ci/check_laputa_clean_break.py --self-test`
  → `self-test passed`（`config_migration::`/`session_migration::` 命中；
  `memory_migration_id` 不误报）。
- `python scripts/ci/check_laputa_clean_break.py`
  → `Embedded Laputa / migration module clean-break verified`（无假阳性）。
- `just laputa-clean-break-check` → 绿色。
- `just memory-provider-check` → 绿色（4 个命名 provider 回归测试通过）。
- `grep -n "sprint5-default-check" justfile .github/workflows/ci.yml`
  → 无残留引用。

## 覆盖点

| 场景 | 结果 |
|---|---|
| CI 中 `sprint5-default-check` 引用移除 | ok |
| `memory-provider-check` recipe 可运行且绿色 | ok |
| 阳性用例：注入 `config_migration::` 被检测 | ok |
| 阴性用例：`memory_migration_id` error-code 不误报 | ok |
| 真实树扫描无假阳性（含 `.mentle` 拒绝守卫） | ok |
| 完整 `laputa-clean-break-check` 通过 | ok |