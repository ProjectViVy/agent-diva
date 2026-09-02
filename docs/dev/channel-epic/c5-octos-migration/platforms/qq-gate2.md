# QQ Bot C5-I Gate 2 implementation evidence

实现文件：`agent-diva-channels/src/adapters/qq.rs`。实现对照 Octos 固定 SHA
`5ea987813de4fd2afdd1d78f2106ad2868f0d923` 的 `qq_bot_channel.rs`：保留官方 token、Gateway
WebSocket、C2C/group event、`msg_seq`、reply `msg_id`、heartbeat、resume、invalid-session
重连和 allowlist，但映射到 DIVA `ChannelEnvelopeV1`/Fabric/receipt。

## 本次实际声明

已声明：文本、C2C/群 @ 入站解析、dedup、文本分块、C2C/group 回复、health、heartbeat、resume、
token refresh、pacing、supervised restart。Gateway discovery 使用 `GET /gateway` + Bearer，
消息使用 `/v2/users/{openid}/messages` 或 `/v2/groups/{group_openid}/messages` + `QQBot`
header，并解析 `id`/`message_id`。Identify 暂时保留 DIVA 既有 `(1<<25)|(1<<12)`，不会把
Octos 的 `(1<<25)|(1<<30)` 未经官方事件投递证据直接带入生产。

未声明：typed media、image/audio/video/file egress、Markdown、thread、typing、edit/delete、
reaction、card、editable stream。Octos 本身只有 text/media-skip；官方文件上传操作尚未形成
经审查的 endpoint+fixture，因此 D-014 保持 blocked，unsupported 必须在 HTTP 前返回。

## 证据/剩余项

`tests/fixtures/c5/qq/` 提供 token、gateway、Hello/Ready/heartbeat/resume、C2C/group event
和 send response 形状；单元测试覆盖 capability truthfulness、Unicode 4000 限制、响应 ID 和
card zero-side-effect。QQ 的 C2C/group admission、真实 mock gateway、官方 intents 位图、
invalid-session cooldown、429、token single-flight 和真机 smoke 仍属于 C5-V/C5-Q；在这些
证据完成前不得宣称 QQ 或整个 C5 已关闭。
