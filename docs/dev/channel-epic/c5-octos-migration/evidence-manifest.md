# C5-V/C5-Q capability evidence manifest

本清单对应 `agent-diva-channels/tests/fixtures/c5/capability-evidence.json` 和六份
`platforms/*-gate3.md`。`verified` 只表示该行的窄能力组已经同时具备源码 symbol、脱敏
fixture/local transport、passing Rust test、精确请求/响应或 Fabric 结果及所需生命周期
断言；不能从一行推导同频道其它能力已完成。`partial` 表示仍缺一项或多项证据，
`blocked/unsupported` 表示按决策明确不宣称。

| ID | Channel | Capability group | 状态 | Fixture / test evidence | 当前精确结果与缺口 |
| --- | --- | --- | --- | --- | --- |
| TG-01 | Telegram | text/caption ingress | `verified` | `telegram/inbound-text.json`; `wire_telegram_ingress_preserves_identity_media_and_callback_ack` | typed envelope 保留 chat/sender/thread/reply/message ID；getUpdates long-poll lifecycle 仍在 TG-06 |
| TG-02 | Telegram | group mention/reply/command gating | `partial` | `telegram/inbound-group-media.json`; same ingress test | 群/reply 可解析且 policy 先于媒体；全量 command/mention 矩阵未闭合 |
| TG-03 | Telegram | photo/voice/audio/document | `partial` | `inbound-group-media.json`, `api-responses.json`; ingress/multipart tests | getFile、受限下载、AttachmentRef、sendPhoto multipart 有证据；全媒体逐类和损坏读回缺失 |
| TG-04 | Telegram | callback/keyboard | `partial` | `callback-query.json`; `wire_telegram_ingress_preserves_identity_media_and_callback_ack` | callback ID/data 与 answerCallbackQuery 已断言；keyboard outbound 未闭合 |
| TG-05 | Telegram | send/reply/edit/delete/typing | `partial` | `api-responses.json`; `wire_telegram_egress_uses_html_reply_real_id_and_plain_fallback`, `wire_telegram_multipart_and_rate_limit_are_truthful` | HTML/plain fallback、reply、multipart、429/真实 ID 已断言；edit/delete/typing/finalize 组合缺失 |
| TG-06 | Telegram | reconnect/dedup/health | `partial` | `api-responses.json`; `start/get_updates/probe_health` | unsupported 零 HTTP 与 cancellation 分支存在；offset/timeout/cancel、dedup、getMe health transcript 缺失 |
| DC-01 | Discord | Gateway opcode/heartbeat/resume | `partial` | `discord/gateway-frames.json`; `wire_discord_gateway_handles_hello_identify_ready_heartbeat_and_cancel` | HELLO/IDENTIFY/READY/heartbeat ACK/cancel 已断言；RESUME/RECONNECT/INVALID_SESSION 缺失 |
| DC-02 | Discord | DM/guild/thread/mention | `partial` | gateway fixture; incoming parser/gateway tests | thread/reply correlation 已断言；DM/guild/mention/bot filter 全矩阵缺失 |
| DC-03 | Discord | attachments/multipart | `partial` | `discord/rest-responses.json`; `wire_discord_rest_preserves_reply_embed_and_attachment_receipts` | attachment-only multipart 与 snowflake receipt 已断言；全媒体/下载/ingress store 缺失 |
| DC-04 | Discord | edit/delete/reaction/embed | `verified` | `discord/rest-responses.json`; `wire_discord_rest_preserves_reply_embed_and_attachment_receipts` | embed/reply POST、PATCH、DELETE、reaction、typing exact path/status 及真实 ID 已断言 |
| DC-05 | Discord | 429/rate-limit/health | `partial` | `discord/rest-responses.json`; `wire_discord_rate_limit_and_unsupported_listening_are_explicit` | body 1.25s 优先于 header 9，返回 1250ms；health/permission transcript 缺失 |
| FS-01 | Feishu | region/token/WS | `partial` | `feishu/protobuf-frame.json`, `region-endpoints.json`; protocol/token tests | protobuf ACK `biz_rt=0`、region mapping、401 refresh 已断言；WS lifecycle/single-flight压力缺失 |
| FS-02 | Feishu | webhook signature/AES | `verified` | `webhook-url-verification.json`, `webhook-encrypted-event.json`; webhook/AES tests | URL challenge 可校验；缺 headers、bad signature/base64/padding 均 fail-closed |
| FS-03 | Feishu | typed image/file/audio/media | `partial` | `media-resource.json`; `media_fixture_is_stored_as_typed_attachment_after_authentication` | authenticated image resource → bounded typed AttachmentRef；其它媒体/timeout/corruption 缺失 |
| FS-04 | Feishu | upload/send/reply/edit/delete | `partial` | `send-responses.json`; send/reply test | send/reply exact endpoint 与真实 ID；upload/edit/delete/429/malformed response 缺失 |
| FS-05 | Feishu | reaction seen/dedup | `partial` | `protobuf-event.json`; duplicate test | dedup 只 admission 一次；seen reaction failure isolation 尚无独立 wire spy |
| DT-01 | DingTalk | Stream OAuth/register/ACK | `partial` | `dingtalk/stream-callback.json`, `token-response.json`; stream wire test | register、callback、双 ACK、cancel、token shape 已断言；reconnect/backoff 缺失 |
| DT-02 | DingTalk | group/private policy | `verified` | `stream-callback.json`; policy/media ingress tests | allowlist 在 media URL 前；group typed envelope admission 已断言 |
| DT-03 | DingTalk | media upload/send | `partial` | `media-upload-response.json`, `group-send-response.json`; media test | multipart bytes/MIME/name、media ID、send 和 429 partial failure 已断言；audio/video/file 逐类缺失 |
| DT-04 | DingTalk | HMAC/sessionWebhook | `partial` | `stream-callback.json`; signature test | valid/bad/empty HMAC fail-closed；TTL/signed callback transcript 缺失 |
| EM-01 | Email | IMAP UNSEEN/thread headers | `verified` | `email/plain.eml`, `reply.eml`; parser/fake IMAP tests | Message-ID 优先、UID fallback、References/In-Reply-To/thread root、Fabric admission 已断言 |
| EM-02 | Email | consent/auto-reply/TLS | `partial` | plain/reply EML; policy/unsupported tests | allowlist/self-reply/unsupported 在 SMTP 前拒绝；consent/empty recipient/TLS matrix 缺失 |
| EM-03 | Email | multipart attachments | `partial` | `email/multipart.eml`; parser/fake SMTP test | MIME/name/bytes 进入 fake SMTP 输入并返回真实 receipt；全部 MIME/size/raw SMTP wire 缺失 |
| EM-04 | Email | mark-seen/health/cancel | `partial` | reply EML; fake IMAP poll test | `STORE \Seen` 只在 Fabric admission 后执行；blocking cancel/reconnect/real health 缺失 |
| QQ-01 | QQ | token/gateway/Identify | `partial` | `qq/token-response.json`, `gateway-response.json`, `official-intents.json`; QQ gateway test | token/Bearer discovery/Hello→Identify 有 mock 证据；D-013 官方投递证据仍 blocked |
| QQ-02 | QQ | heartbeat/resume/invalid/cooldown | `partial` | `qq/gateway-frames.json`; QQ gateway test | heartbeat op1/op11 与 cancel 已断言；resume/invalid-session cooldown/reconnect 缺失 |
| QQ-03 | QQ | C2C ingress/egress | `verified` | `qq/c2c-message.json`, `send-responses.json`; QQ gateway/outbound tests | C2C identity、admission dedup、`msg_seq`、reply `msg_id`、真实 response ID 已断言 |
| QQ-04 | QQ | group ingress/egress | `verified` | `qq/group-message.json`, `send-responses.json`; QQ gateway/outbound tests | group/member open ID、显式 group routing、`msg_seq`、真实 response ID 已断言 |
| QQ-05 | QQ | media | `blocked/unsupported` | `qq/official-intents.json`; `unsupported_media_fails_before_transport` | media/card 在首次 HTTP 前 typed reject；D-014 无官方 endpoint+wire proof，保持 blocked |

## Verification rule

每个 `verified` 条目必须同时提供：

- Octos/DIVA 源码 symbol 与 endpoint 定位；
- 最小脱敏 fixture 或私有 local mock server/WS/IMAP/SMTP transport transcript；
- passing Rust test 名称；
- 精确请求、响应、receipt 或 typed error；
- 对 health、dedup、retry、timeout、cancel 的状态/生命周期证据。

当前已验证的是窄行，不代表 C5-V/C5-Q 已完成。全量 workspace 门禁和 QQ ignored live
smoke 仍是退出条件；C6 Manager 切换保持禁止。
