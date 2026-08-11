# Verification

## 命令与结果

- `cargo test -p agent-diva-migration` → 10 passed（新增 3 个 `require_feature`
  单测 + 既有 7 个迁移测试全绿）。
- `cargo clippy -p agent-diva-migration --all-targets -- -D warnings` → 绿色。
- CLI smoke：
  - `cargo run -p agent-diva-migration -- experience apply --workspace /tmp/fake`
    → `Error: Apply for `experience` is disabled until the `--features experience` flag is passed`
  - `cargo run -p agent-diva-migration -- --features experience experience apply --workspace /tmp/fake`
    → 继续执行并输出去重报告（不再被 gate 拦截）。

## 覆盖点

| 场景 | 结果 |
|---|---|
| 无 `--features` 时 Apply 被拒 | ok |
| 开启对应 `--features` 后 Apply 放行 | ok |
| 未列举 feature 保持禁用 | ok |
| feature 名大小写敏感（精确匹配） | ok |
| `--help` 展示 features 参数 | ok |
| 既有 dry_run/apply/rollback 幂等测试不受影响 | ok |