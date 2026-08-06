# Verification — GA-MEM-PARITY Wave 2（分层与工作记忆）

## 方法

每切片独立：`cargo fmt --all -- --check`、`cargo clippy -p <crate> --all-targets -- -D warnings`、
`cargo test -p <crate>`；S4 后相关 crate 全量回归。

## 结果

| 切片 | 提交 | 验证 |
|------|------|------|
| S1 | `7b30dc72` | core 692 全绿（新增 working 契约 + 配置测试）；clippy 干净 |
| S2 | `2e70d92c` | laputa 27+ 全绿（tempdir + 真实 store：L1 有界、0 预算、checkpoint 三路、supersedes 清理、evidence）；clippy 干净 |
| S3 | `b1c101f9` | agent 383 全绿（policy 段、注入位置、空块跳过）；clippy 干净 |
| S4 | `c8111c22` | tools 109 / agent 383 / core 692 / laputa / manager 全绿；clippy 干净 |
| S5 | 本次 | docs only，`git diff --check` |

全 workspace 相关 crate 回归：core 692 / laputa 27+（lib+集成）/ agent 383 /
tools 109 / manager 全绿。CLI 6 个既有 wiremock 502 失败
（`CLI-WIREMOCK-502-PREEXISTING`，TODOLIST 已记录，与本迭代无关）。

## 验收测试落点（B2/B10、C1–C3、G1、U5）

- B2/B10：`startup_injects_bounded_l1_index_not_full_content`（预算 2 行、
  无全文、无 memory-data 包裹）；`zero_l1_budget_renders_no_index`；
  core `l1_block_caps_lines_and_never_injects_full_content`。
- C1–C3：`checkpoint_write_read_roundtrip_and_excluded_from_startup`
  （写读一致、key_info/related_sops 渲染、他人会话不可见、不进启动渲染）；
  `checkpoint_overwrite_replaces_content`；工具 `without_session_reports_failed`。
- G1：`distill_fresh_writes_evidence_file`（EVIDENCE.md 落盘）。
- U5/清理：`session_end_clears_checkpoint`（supersedes tombstone 后
  working_memory_block 为空）；agent loop 退出按会话枚举清理。

## 遗留

- 会话异常退出残留 checkpoint：正常 on_session_end 清理，异常路径残留归
  Wave 5 GC（TODOLIST 条目化跟进）。
- B9 完整强制校验（tool-result 证据绑定）留 Wave 5。
- CLI wiremock 502 排查（独立 TODO，`CLI-WIREMOCK-502-PREEXISTING`）。
