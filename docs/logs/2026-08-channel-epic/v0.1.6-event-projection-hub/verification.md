# Verification

| 命令 | 结果 |
| --- | --- |
| `cargo fmt --all` | 通过 |
| `cargo test -p agent-diva-core --lib` | 通过，730 passed |
| `cargo test -p agent-diva-agent --lib` | 通过，433 passed |
| `cargo test -p agent-diva-manager --lib` | 通过，124 passed，1 ignored（初次发现 transient 初始 cursor 语义缺口，已修正） |
| `cargo test -p agent-diva-manager --lib neuro_link_projection` | 通过，5 passed |
| `cargo test -p agent-diva-manager --lib projection_journal::tests::transient_rows_advance_live_cursor_but_are_not_replayed` | 通过 |
| `cargo clippy -p agent-diva-manager --lib -- -D warnings` | 通过（`submit` 使用 boxed error，避免大错误类型） |
| `git diff --check` | 通过（仅 CRLF 转换提示） |

新增覆盖：

- AgentBusEvent 的关联/非 Neuro-Link 过滤与事件映射；
- terminal 事件持久化、工具结果正文不泄漏；
- bus → hub → SQLite → live subscriber 的一次写入路径；
- transient 行推进 live cursor 但不出现在 durable replay；
- WebSocket 现有 hello/session/open/turn/admission 冒烟在 hub 接入后保持通过。

完整 workspace `just fmt-check && just check && just test` 作为交付前门禁执行并在最终结果中记录。
