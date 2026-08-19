# Verification — v0.1.0 empty allow_from + QQ reject group

## 自动化

| 命令 | 结果 |
|---|---|
| `cargo fmt -p agent-diva-channels` | 已执行 |
| `cargo clippy -p agent-diva-channels --lib -- -D warnings` | 通过 |
| `cargo test -p agent-diva-channels --lib` | 84/84 通过 |

未跑全量 `just test`：本次只改 `agent-diva-channels` 的 allowlist 默认与 QQ 事件过滤。

## 新增测试

- `test_base_channel_is_allowed_empty_list_allow_all`：`new()` 空名单放行。
- `test_empty_allow_from_allows_anyone`：QQ 空名单放行。
- `test_c2c_message_is_forwarded_when_allow_from_empty`：C2C 进 bus。
- `test_group_at_message_is_rejected` / `test_guild_at_message_is_rejected`：群/频道事件不进 bus。
