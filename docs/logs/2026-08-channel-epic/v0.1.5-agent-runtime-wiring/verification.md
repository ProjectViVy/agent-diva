# Verification

| 命令 | 结果 |
| --- | --- |
| `cargo test -p agent-diva-core --lib` | 通过，730 passed |
| `cargo test -p agent-diva-agent --lib` | 通过，433 passed |
| `cargo test -p agent-diva-manager --lib` | 通过，117 passed，1 ignored |
| `cargo clippy -p agent-diva-agent --lib -- -D warnings` | 通过 |
| `cargo clippy -p agent-diva-manager --lib -- -D warnings` | 通过 |

新增覆盖：

- typed envelope 转换保留 opaque session、request/trace、locale，并拒绝非 Message payload；
- Manager adapter 通过 runtime-control channel round-trip admission；
- 真实 loopback WebSocket smoke 仍通过 hello/session/open/turn/admission 链路。

