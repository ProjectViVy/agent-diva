# Summary — BML 逻辑层边界实施 v0.1.0

- **Date**: 2026-08-08
- **Scope**: `agent-diva-laputa` BML 模块边界 + 防回归门
- **Type**: 纯结构变更（零行为变化、零运行时风险）
- **决策链**: D3 三层模型冻结（AGENTS.md + TODOLIST.md）→ BML 抽层调研（`docs/research/bml-layer-extraction-2026-08/`）→ 决策 D（先 B 后 A）→ 本迭代实施 B

## What changed

1. **BML 命名空间**（`agent-diva-laputa/src/bml/mod.rs` 新建）：存储核心
   （`typed_store`）与记录适配层（`memory_records`）的公开面经 `bml` 模块
   re-export，模块 doc 声明层边界：治理层不得直写 BML 写接口
   （put/put_governed/import_records/rollback_governed/gc/backup/restore）。
   `lib.rs` 顶层 re-export 改经 `bml`（`pub mod typed_store` 保留），
   全部现有外部引用（agent/manager/migration/autodream）零改动编译。
2. **负向守卫测试**（`agent-diva-laputa/tests/bml_boundary_guard.rs` 新建）：
   复用 `tests/authority_boundary_guard.rs` 静态扫描器（`ForbiddenPattern`
   增加 `pub const fn new`），扫描 `agent-diva-laputa/src` 全目录，禁止
   治理模块出现 7 类 BML 写接口调用；豁免清单：存储核心自身
   （typed_store.rs）、组合门面（typed_provider.rs）、记录适配层
   （memory_records.rs）、迁移工具（migration.rs）、service.rs 测试辅助
   `write_supersedes_tombstone`（snippet 级豁免）。首跑零违规——验证了
   治理层生产代码无直写（调研 H1 结论的运行时守卫化）。
3. **文档与门禁同步**：AGENTS.md 三层模型段落追加 BML 逻辑层声明；
   `just bml-boundary-check` recipe 新增并纳入 `ci` 与
   `e7-automated-release-gate`；TODOLIST.md 记录 S9 完成。

## Impact range

- `agent-diva-laputa`：新增 `src/bml/mod.rs`、`tests/bml_boundary_guard.rs`；
  `lib.rs` re-export 路径重排（行为不变）；`tests/authority_boundary_guard.rs`
  增加 pub 构造器（既有守卫行为不变）
- 其他 crate：零代码改动（外部引用保持兼容）
- 文档/构建：AGENTS.md、justfile、TODOLIST.md

## Follow-ups

- 未来升 A（全抽 `agent-diva-bml` crate）：GMH-23A 条款修订 + §8 禁令
  amendment，时机 = Garden facade 落地（TODOLIST.md 已条目化）
