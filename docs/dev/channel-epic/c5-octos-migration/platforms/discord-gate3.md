# Discord C5-V Gate 3 evidence

状态：`partial`；本地 HTTP/WS fixture 已闭合主要 REST/Gateway 形状，尚无真实 Gateway
权限 smoke。

| 目标 | 源码 symbol / endpoint | fixture 与 Rust 测试 | 精确证据 | 生命周期/剩余项 |
| --- | --- | --- | --- | --- |
| Gateway lifecycle | `discover_gateway`, `run_gateway`, `consume_gateway`; `/gateway/bot` | `gateway-frames.json`; `wire_discord_gateway_handles_hello_identify_ready_heartbeat_and_cancel` | local WS 发送 HELLO；客户端发送 IDENTIFY；READY 后发送 heartbeat，server 回 ACK；取消结束 task | RESUME/RECONNECT/INVALID_SESSION 仍需逐帧 transcript；heartbeat ACK timeout path 有代码但未单独 wire 覆盖 |
| DM/guild/thread/reply | `handle_incoming`, `parse_incoming_message` | `gateway-frames.json`; `incoming_message_preserves_thread_and_reply`, `channel_thread_types_use_channel_id_without_marking_dms_as_threads`, gateway wire test | envelope 保留 guild/channel/thread/message reference；普通 DM 不误标 thread | mention/bot filter 与多种 chat 变体需补 fixture |
| REST text/embed/reply | `send_message`, `post_json`; `/channels/{id}/messages` | `rest-responses.json`; `wire_discord_rest_preserves_reply_embed_and_attachment_receipts` | 首块 JSON 包含 message reference；card 进入 `embeds`；成功 receipt 使用 snowflake | 多块 reply/admission ordering 仍 partial |
| attachments/interactions | `send_attachments`, `execute_edit`, `execute_delete`, `execute_reaction`, `execute_typing` | `rest-responses.json`; 同一 REST wire test | multipart attachment-only 不发送空文本；PATCH/DELETE/reaction/typing 使用 exact path；各 success/status 有断言 | 全部 image/audio/video/file 与权限失败仍需扩展 |
| 429/unsupported/health | `parse_response`, `probe_health`, `ensure_supported` | `rest-responses.json`; `wire_discord_rate_limit_and_unsupported_listening_are_explicit` | body `retry_after=1.25` 优先于 header 9，返回 1250ms `RateLimited`；listening 首次调用前 `UnsupportedCapability`，fixture 只收到一条请求 | health success/permission error 与 reconnect close 仍 partial |

敏感信息：token 只进入 request header 断言，不写日志；snowflake 仅为脱敏 fixture 值。
