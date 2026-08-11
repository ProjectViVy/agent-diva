# Verification

- `cargo check -p agent-diva-providers -p agent-diva-agent`：通过。
- `cargo test -p agent-diva-providers final_wire`：3 项通过。
- `cargo test -p agent-diva-agent cache_observe`：3 项通过。
- `cargo test -p agent-diva-providers final_wire_snapshot_is_emitted_once_across_internal_retries`：通过，内部 retry 仅产生一次结构快照。
- `cargo test -p agent-diva-agent retired_guessing_state_cannot_reenter_the_observer`：通过。
- `cargo clippy -p agent-diva-providers -p agent-diva-agent --all-targets -- -D warnings`：通过。
- `just fmt-check`、`just check`、`just test`、`just ci`：全部通过。
- `cargo run -p agent-diva-cli -- --help`：通过，CLI 帮助正常输出。
- `rg` 删除证明：生产代码中不存在 `warmup_pending`、`consecutive_misses`、
  `SuspectedCacheMiss`、`MissSample`、`last_tools_hash`、`per_tool_hashes`。

注：首次使用 `just run -- --help` 会把额外的 `--` 传给 CLI，因参数语法错误退出；改用
上述直接 `cargo run` 命令后 smoke 通过，与产品行为无关。
