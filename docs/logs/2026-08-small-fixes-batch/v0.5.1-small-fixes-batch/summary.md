# v0.5.1-small-fixes-batch Summary

## What changed

1. **GATEWAY-PORT-CONFIG-IGNORED 修复（sev-P3）**
   - `agent-diva-cli/src/main.rs` `build_gateway_runtime_config` 此前硬编码
     `DEFAULT_GATEWAY_PORT`（3000），`config.json` 的 `gateway.port` 不生效。
     现改为优先使用 `config.gateway.port`（schema 默认仍为 3000，未配置行为不变）。
   - 新增两个单元测试：配置端口生效、未配置时回退默认端口。

2. **PROVIDERS-EXAMPLE-1.94-CLIPPY 修复（sev-P3）**
   - `agent-diva-providers/examples/minimax_sync_tts.rs` 移除冗余 `.into()`
     （Rust 1.94 `useless_conversion`）。
   - 同批核查发现并修复 providers 其余 3 处 Rust 1.94 `--all-targets` lint：
     - `examples/siliconflow_tts.rs` `too_many_arguments`（按仓库惯例
       `#[allow]`，与 `agent-diva-agent` 现有用法一致）；
     - `src/retry.rs` 两处 `bool_assert_comparison`（测试断言改 `assert!`）。
   - 修复后 `cargo clippy -p agent-diva-providers --all-targets -- -D warnings` 全绿。

3. **LAPUTA-SERVICE-INT-PLUS-ONE 现状核查（sev-P3）**
   - `cargo clippy -p agent-diva-laputa --tests` 下 `tests/service.rs`
     已无 `int_plus_one` 命中；该问题已被后续迭代消解，本批无代码改动。
   - 核查过程中发现 laputa 其他 test target 存在 Rust 1.94 预存在 lint
     （`authority_boundary_guard.rs` / `direct_write_guard.rs` /
     `governance_proof_loop.rs` dead_code ×5、`context_plane_invariants.rs`
     cmp_owned ×1、`authority_boundaries.rs` ×1），**不在本批锁范围内**，
     已条目化记录 TODOLIST（LAPUTA-TESTS-1.94-ALL-TARGETS-CLIPPY）。

## Impact range

- 生产行为变化仅一处：`agent-diva gateway` 启动端口跟随 `config.gateway.port`。
- providers `retry.rs` 仅改测试断言写法，语义不变；example 不影响生产二进制。

## Related

- TODOLIST: `GATEWAY-PORT-CONFIG-IGNORED`、`PROVIDERS-EXAMPLE-1.94-CLIPPY`、
  `Laputa service 预存 clippy int_plus_one`。
- 并行说明：本批在隔离 worktree（`fix/small-fixes-batch`，基于
  `agent-diva-pro` HEAD `faf54fa9`）执行，与主线 CTX-C2 锁范围无重叠；
  共享树 `TODOLIST.md` 更新延后至 CTX-C2 释放锁后执行。
