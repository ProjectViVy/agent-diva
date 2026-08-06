# Verification — GA-MEM-PARITY Wave 1（Agent 记忆工具 CRUD）

## 方法

每切片独立：`cargo fmt --all -- --check`、`cargo clippy -p <crate> --all-targets -- -D warnings`、
`cargo test -p <crate>`；S4 后全 workspace `cargo test --workspace`。

## 结果

| 切片 | 提交 | 验证 |
|------|------|------|
| S1 | `1e40bf16` | core 682 全绿（含默认方法 Failed + serde roundtrip 新测试）；clippy 干净 |
| S2 | `5a742c82` | laputa 20+ 全绿（tempdir + 真实 store：add→list、tombstone 过滤、proposal 映射、distill 两路、启动一致性）；clippy 干净 |
| S3 | `522df4d0` | agent 378+ 全绿（含 legacy add → proposal 落 `.laputa`）；clippy 干净 |
| S4 | `bd04cfc5` | tools 12+ / agent / core 全绿（六工具单测 + ToolAssembly 注册 + mask deny）；clippy 干净（含 tools 3 个既有 lint 修复） |
| S5 | 本次 | docs only，`git diff --check` |

全 workspace：除 CLI 6 个既有 wiremock 502 失败（`CLI-WIREMOCK-502-PREEXISTING`，
TODOLIST 已记录，2026-08-05 干净树复现，与本迭代无关）外全部通过。

## 验收测试落点（G1/G3）

- G3 tombstone：S2 测试「put tombstone 后 search_visible/list 为空」——
  工具路径删除后不再出现（复用既有注入过滤）。
- G1 distill 最小版：S2/S3 测试覆盖「新建 skill 即时写 SKILL.md」与
  「覆盖走 SopCreate proposal」。

## 遗留

- CLI wiremock 502 排查（独立 TODO，`CLI-WIREMOCK-502-PREEXISTING`）。
- `memory_update` apply 时 `adapt_governed_proposal` 不设 supersedes
  （memory_records.rs:36）→ TODO（Wave 3/5 条目化跟进，不阻塞）。
- W1-4 结果不自动注入：热注入归 Wave 3（TODOLIST WAVE3 已含）。
