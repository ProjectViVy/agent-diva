# Telegram C5-V Gate 3 evidence

状态：`partial`。本文件记录本轮可复现的本地 Bot API transcript；没有真实 Telegram
凭据或外网依赖，故不把 listener/全量 media lifecycle 误标为完成。

| 目标 | 源码 symbol / endpoint | fixture 与 Rust 测试 | 精确证据 | 生命周期/剩余项 |
| --- | --- | --- | --- | --- |
| text/caption ingress | `TelegramAdapter::process_update`, `process_message`; `getUpdates` | `inbound-text.json`, `inbound-group-media.json`; `wire_telegram_ingress_preserves_identity_media_and_callback_ack` | Fabric envelope 保留 chat/sender/thread/reply/message ID；媒体请求使用 `getFile` 与受限 `/file/bot...` 下载 | 需补真实 `getUpdates` offset/timeout/cancel transcript |
| group mention/reply/commands | `process_message`, `is_sender_allowed` | `inbound-group-media.json`; 同上 | 未授权在媒体读取前返回；群 topic 与 reply ID 被保留 | command routing 的全量 `/start`/`/reset`/`/stop` 仍是 partial |
| typed media | `fetch_attachment`, `AttachmentStore` seam | `inbound-group-media.json`, `api-responses.json`; 同上 | `getFile` path、受限下载、文件名/MIME 进入 typed `AttachmentRef`；HTTP failure 为 `telegram_media_http` | voice/audio/video 全矩阵和损坏读回尚未逐类 wire 覆盖 |
| callback/keyboard | `process_callback`; `answerCallbackQuery` | `callback-query.json`; 同上 | callback envelope 保留 callback ID/data，随后 exact `answerCallbackQuery` 请求；失败不伪造 receipt | keyboard outbound 的完整按钮回传仍 partial |
| send/reply/media/rate-limit | `send_text_message`, `execute_send`, `call_multipart`；`sendMessage`/`sendPhoto` 等 | `api-responses.json`; `wire_telegram_egress_uses_html_reply_real_id_and_plain_fallback`, `wire_telegram_multipart_and_rate_limit_are_truthful` | 首块带 reply 参数；HTML parse error 自动 plain fallback；multipart caption 限制 1024；成功使用真实 message ID；429 映射 `RateLimited` | edit/delete/typing/finalize 与 partial chunk failure 需补组合 transcript |
| health/stop/dedup | `start`, `get_updates`, `probe_health` | `api-responses.json`; capability/unsupported tests | unsupported reaction 在首次 HTTP 前返回 `UnsupportedCapability`；start 使用 cancellation select | `getMe` health、重复 update、long-poll cancel 仍 partial |

敏感信息：fixture 只用 redacted IDs；测试请求断言不会输出 token、signed URL 或原始媒体。
图片只作为 typed attachment 进入 Fabric，vision provider 不在 channel 内伪造识别结果。
