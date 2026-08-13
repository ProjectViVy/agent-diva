# Verification — COGNITIVE-R0 v0.0.1

## 命令与检查

| 检查 | 结果 |
| --- | --- |
| EPIC 三完成物落盘 | Pass：`current-state-map.md`、`dependency-and-data-inventory.md`、`legacy-failure-baseline.md` |
| 源码交叉：`LaputaSectionName` / `ProposalType::target_section` | Pass（`types.rs:73-93,220`） |
| 源码交叉：`governance.db` vs `governance.sqlite3` | Pass（`bootstrap.rs` / `layout.rs:109-111`） |
| 源码交叉：`finalize_typed_proposal` 不写 section JSON | Pass（Persona 探索 + `service.rs` write_authority=false） |
| 源码交叉：Persona `JSON.stringify` / SectionEditor JSON 门 | Pass |
| 源码交叉：错误串组合 | Pass（`ledger.rs:193` + `governed_apply.rs:35`） |
| 子代理 3/3 完成 | Pass |
| R1 Evolution 切片被引用 | Pass |
| `just fmt-check/check/test` | **未跑**（纯 docs，无 Rust 变更） |
| 真机桌面三症状复测 | **未跑**（固化已有人工记录；不修旧链） |

## 证据分级遵守

各文档标注 源码事实 / 提交事实 / 实验观察 / 推断 / 建议。

## 已知限制

1. 未抽样真实用户 profile 的 `.laputa` 体积与内容（交给 R4）。
2. 未做新的桌面复测；三条症状依据 EPIC 人工记录 + 源码触发链。
3. `run_startup_gc` 无生产调用、SelfEvolution 未接 cron 仅静态确认。
