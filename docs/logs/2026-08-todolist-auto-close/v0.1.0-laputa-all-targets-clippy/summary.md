# Summary — LAPUTA-TESTS-1.94-ALL-TARGETS-CLIPPY

- 版本：`v0.1.0-laputa-all-targets-clippy`
- 日期：2026-08-23
- 类型：测试目标 clippy 机械修复

## 背景

`cargo clippy -p agent-diva-laputa --all-targets -- -D warnings` 在 Rust 1.94+
对集成测试 helper 报 `dead_code`。生产库目标与 `just check` 不受影响。S5 后
`governance_proof_loop` / `context_plane_invariants` 已不在树内；剩余问题是
`tests/authority_boundary_guard.rs` 作为顶层 `tests/*.rs` 被多个 integration
crate `mod` 进来，未使用的常量/函数在各自 binary 里变成 dead code。

## 做了什么

- 扫描器搬到 `tests/common/mod.rs`（Cargo 不把 `tests/common/` 当成独立测试 crate）。
- 权威边界断言搬到 `tests/common/assert.rs`，仅由 `authority_boundaries` /
  `direct_write_guard` 通过 `#[path]` 引入。
- `bml_boundary_guard` 只使用 `common::{scan_forbidden_access, ForbiddenPattern}`。
- 删除顶层 `tests/authority_boundary_guard.rs`。
- 未改 BML schema、未改 `put` / `import_records` / `gc`、未拿掉
  `bml_boundary_guard` 对已退役 `put_governed` / `rollback_governed` 的扫描针。

## 影响范围

- `agent-diva-laputa/tests/**` 测试 helper 布局
- `TODOLIST.md` 本条 Done
- 本日志目录
