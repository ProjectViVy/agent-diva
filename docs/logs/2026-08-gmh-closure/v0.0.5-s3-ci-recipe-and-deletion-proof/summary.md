# Summary

GMH-52 的 CI 断点修复与 deletion-proof 补强（S3a + S3b）。

## S3a — CI 断点修复

- `.github/workflows/ci.yml:96`：`just sprint5-default-check`（justfile 中不存在的
  recipe，CI 断点）→ `just memory-provider-check`（历史语义已并入）。未降 gate。

## S3b — deletion-proof 补强

- `scripts/ci/check_laputa_clean_break.py` 新增对 GMH-50（S2a）删除的
  `config_migration` / `memory_migration` / `session_migration` 死代码残留扫描，
  防 1386 行被误恢复：
  - 仅匹配模块用法（`mod config_migration;` 或 `config_migration::`），
    避免 `agent-diva-laputa/src/error.rs` 的 `memory_migration_id` 等
    error-code 标识符误报。
  - `agent-diva-migration` 单独扫描模块残留（不扫 mentle 模式，因其
    `typed_memory.rs` 的 `.mentle` 拒绝守卫是正确行为）。
  - 新增 `--self-test` 模式：阳性用例（`config_migration::` 应命中）与阴性用例
    （`memory_migration_id` 不应误报）。

## Impact

- CI 断点消除，`sprint5-default-check` 引用全部移除。
- 死代码删除具备防回归扫描，后续误恢复会被 `just laputa-clean-break-check` 拦截。