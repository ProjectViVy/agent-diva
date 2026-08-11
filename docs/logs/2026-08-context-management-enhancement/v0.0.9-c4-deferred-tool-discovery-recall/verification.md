# Verification

已完成的聚焦验证：

- `cargo test -p agent-diva-tooling -p agent-diva-tools -p agent-diva-agent`：通过。
- 同回合 E2E 单测 `deferred_tool_search_mount_applies_to_the_next_same_turn_call`：通过；第 1 次 provider call 无 custom schema，第 2 次仍无，第 3 次包含并成功执行 mounted tool。
- registry 错误矩阵：`tool_not_discovered`、`tool_not_mounted`、`tool_unavailable`、未知工具 not-found：通过。
- registry 搜索：大小写、空查询、无匹配、limit、字典序、session state 隔离/恢复：通过。
- Recall：无注入、失败不破坏消息、成功顺序、层级超限 Drop、总预算超限 Drop：通过。
- `cargo fmt --all -- --check`：通过。

交付前继续执行：受影响 crate 严格 Clippy、`just fmt-check`、`just check`、`just test`、`just ci`，以及 CLI `--help` smoke；全量门禁中的既有 CLI wiremock 502 失败按项目基线记录。
