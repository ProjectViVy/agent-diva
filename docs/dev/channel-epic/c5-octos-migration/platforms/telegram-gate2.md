# Telegram C5-I Gate 2 implementation evidence

实现文件：`agent-diva-channels/src/adapters/telegram.rs`。构造器只接收冻结的
`TelegramConfig` 与 `AdapterServices`；模块不调用 legacy `TelegramHandler`，也不引入
Octos runtime。

## Octos 对照与 DIVA 变换

| Octos wire 行为 | DIVA native 行为 | 证据/限制 |
| --- | --- | --- |
| `getUpdates` long polling | `getUpdates` 带 offset/timeout/allowed_updates；取消和退避由 adapter token 控制 | `start`, `get_updates`；需 C5-V mock listener 证明持续运行 |
| `getFile` + `/file/bot...` | 权限检查后下载并写入 `ChannelAttachmentStore`，限制 20 MiB、保留 MIME/文件名 | `fetch_attachment`; `inbound-group-media.json` |
| HTML `sendMessage` + reply | HTML 转义/分块，首块携带 `reply_parameters`，返回真实 `message_id` | `execute_send`; `api-responses.json` |
| `sendPhoto/Audio/Video/Document` | typed parts 分流为 multipart；无可发送内容返回 typed error | `execute_send`; 真实 multipart mock 尚待 C5-V |
| callback ACK | callback 先入 Fabric，再调用 `answerCallbackQuery`，保留 callback ID/data | `process_callback`; `callback-query.json` |
| typing/listening/edit/delete | `sendChatAction`、编辑、删除、finalize-stream 均绑定 chat/message ID | `execute_typing/execute_edit/execute_delete` |

`IngressMarkdown`、`EgressCard`、通用 reaction 和 heartbeat/resume 未被声明；未知或不支持的
命令在第一次 HTTP 调用前返回 `UnsupportedCapability`。allowlist 为空按 DIVA 约定 allow-all，
非空支持 user ID/username，未授权消息不会触发媒体下载。

## 待补证明

目前单元测试已覆盖能力快照、Unicode 分块、HTML 转义和 unsupported reaction 零副作用；
`evidence-manifest.md` 中 TG-01～TG-06 仍为 `partial`，原因是需要本地 mock Bot API、
Fabric busy/cancel、429/plain fallback、媒体存储和 listener cancellation 的 wire transcript。
图片识别不在频道 adapter 内：入站只提供 typed `AttachmentRef`，是否调用 vision provider
由 provider capability/Agent admission 决定。
