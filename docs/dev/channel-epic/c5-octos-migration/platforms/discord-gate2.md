# Discord C5-I Gate 2 implementation evidence

实现文件：`agent-diva-channels/src/adapters/discord.rs`。该模块直接使用现有 reqwest 与
tokio-tungstenite，实现 Octos 的 Gateway/REST 语义；不依赖 Serenity、legacy
`DiscordHandler` 或 Octos manager/bus。

## 迁移结果

- Gateway discovery、Hello、Identify/Resume、heartbeat ACK、READY/RESUMED、Reconnect 与
  Invalid Session 均映射为本地状态和可重试 typed error。
- `MESSAGE_CREATE` 保留 DM/guild/thread/channel/reply/mention/bot policy；线程只从明确的
  thread 对象或 Discord thread channel type 推导，不再把普通 DM 的 channel ID 误标为 thread。
- 文本/Markdown 分块、embed card、multipart typed attachment、reply reference、edit、delete、
  reaction、typing 和 health probe 均使用真实 REST path；429 的 header/body `retry_after`
  映射为 `AdapterError::RateLimited`。
- card-only payload 会在 `embeds` 字段发送，不会因空文本退化成空成功；所有成功发送返回
  Discord 返回的 message ID，失败不会伪造 receipt。

## 证据/剩余项

`capabilities_match_frozen_matrix`、`incoming_message_preserves_thread_and_reply`、
`channel_thread_types_use_channel_id_without_marking_dms_as_threads`、Unicode 分块和 Gateway
URL 单测已通过。DC-01～DC-05 在 `evidence-manifest.md` 仍为 `partial`，C5-V 还需 scripted
WebSocket、REST multipart、reaction/embed、429 和 admission/cancellation mock transcript。
附件会先通过 `AdapterServices` 内容寻址存储，图片识别仍由 provider vision gate 负责。
