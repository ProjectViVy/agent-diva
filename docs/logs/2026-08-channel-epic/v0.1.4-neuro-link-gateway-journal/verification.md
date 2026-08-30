# Verification

## 自动化验证

| 命令 | 结果 |
| --- | --- |
| `cargo test -p agent-diva-core --lib` | 通过，730 tests |
| `cargo test -p agent-diva-manager --lib` | 通过，117 passed，1 ignored |
| `cargo clippy -p agent-diva-manager --all-targets -- -D warnings` | 通过 |
| `pnpm test -- --run`（`agent-diva-gui`） | 通过，74 files / 532 tests |
| `pnpm build`（`agent-diva-gui`） | 通过，`vue-tsc` 与 Vite production build |

## 回环 WebSocket 冒烟

`neuro_link::tests::loopback_websocket_handshake_session_and_turn_smoke` 启动临时
127.0.0.1 listener，以真实 WebSocket client 完成 hello、session/open、turn/start，
并确认 admission notification 与 correlation/session cursor 字段。Fake runtime 只用于
测试注入；生产默认不伪造 turn accepted。

## 边界覆盖

- JSON-RPC frame/message 上限为 1 MiB；turn parts 上限为 64；attachment ref 上限为 50 MiB。
- session cursor 跨 stream、未来 cursor、过期 cursor 分别映射为稳定 cursor 错误。
- 同一 `client_message_id` 的不同参数返回 `idempotency_conflict`，原结果不被覆盖。
- 非 loopback listener 会被 Manager server 拒绝。

