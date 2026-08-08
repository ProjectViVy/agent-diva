# Verification — BML 逻辑层边界实施 v0.1.0

> 本迭代为纯结构变更，验证重点是：守卫测试零违规、全 workspace
> 编译无破坏、门禁 recipe 可用。

## 1. 守卫测试（核心验收）

```bash
cargo test -p agent-diva-laputa --test bml_boundary_guard
```

结果：`1 passed; 0 failed`，且**零违规**（`governance_modules_must_not_write_bml_directly` 通过）。
首跑零违规即证明治理层生产代码无 BML 直写（调研 §2.1 H1 的运行时守卫化）。

豁免清单（写入测试文件，含理由注释）：
- `typed_store.rs`：存储核心自身（内部自调用，如 open 时 backup）
- `typed_provider.rs`：组合门面（crud_store.put / session GC 合法调用者）
- `memory_records.rs`：BML 记录适配层
- `migration.rs`：迁移工具（合法写路径）
- `service.rs` `write_supersedes_tombstone`：测试辅助（写 TempDir，snippet 级豁免）

## 2. 编译与静态检查

```bash
cargo check --workspace
just fmt-check
just check   # cargo clippy --all -- -D warnings
```

预期：全通过（结果记录于本迭代 commit body / 最终回复）。

## 3. 回归

```bash
just test   # 全量；CLI wiremock 502 为既有失败 CLI-WIREMOCK-502-PREEXISTING
```

预期：除既有 CLI wiremock 失败外全绿——lib.rs re-export 路径重排不破坏
agent/manager/migration/autodream/cli 任一 crate。

## 4. 门禁接线

- `just bml-boundary-check` 可独立运行
- `just ci` 与 `just e7-automated-release-gate` 已含 `bml-boundary-check`
